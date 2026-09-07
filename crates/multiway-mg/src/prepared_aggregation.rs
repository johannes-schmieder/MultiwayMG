//! Bounded flat-array pair-neighborhood proposals for prepared frames.
use crate::{MultiwayError, PairNeighborhoodAggregationOptions};
use multiway_incidence::{
    FactorAggregation, PreparedHierarchyBudget, ThreeWayWeightFrame, WeightFrameBinding,
};

/// Work actually completed while proposing one factor-respecting map.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PreparedAggregationWork {
    /// Original tuple entries visited while constructing pair marginals.
    pub tuple_visits: usize,
    /// Distinct entries in the six directed pair marginals.
    pub pair_entries: usize,
    /// Shared-neighbor pair contributions emitted before duplicate collapse.
    pub proposals: usize,
    /// Distinct factor-local candidate pairs after duplicate collapse.
    pub unique_candidates: usize,
    /// Neighbor lists exceeding the explicit degree cap.
    pub truncated_neighbors: usize,
    /// Disjoint pairs selected by deterministic greedy matching.
    pub accepted_pairs: usize,
}

/// Conservative live requested-array admission, not an allocator or RSS quota.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedAggregationSetup {
    /// Borrowed fine topology and current weight-frame capacities, counted once.
    pub borrowed_payload_bytes: usize,
    /// Caller-declared old hierarchies, input arrays or other live owners.
    pub additional_live_payload_bytes: usize,
    /// New retained parent arrays, one u32 per fine coefficient.
    pub parent_payload_bytes: usize,
    /// Maximum flat scratch or subsequent dense-parent validation scratch.
    pub scratch_payload_bound: usize,
    /// Bytes per source tuple ID: u32 when the maximum ID fits, otherwise usize.
    pub source_index_bytes: usize,
    /// Largest proposal capacity needed by any of the three factors.
    pub maximum_proposals: usize,
    /// Checked sum of borrowed, additional, parent and scratch payload.
    pub total_payload_bound: usize,
}

/// A failed candidate retains its attempted-work and admission records.
#[derive(Debug, thiserror::Error)]
#[error("prepared aggregation failed: {source}")]
pub struct PreparedAggregationFailure {
    /// Original typed validation, allocation or numerical failure.
    pub source: MultiwayError,
    /// Completed sparse preparation work, including unsuccessful attempts.
    pub work: PreparedAggregationWork,
    /// Requested live peak after checked sizing, including an over-budget request.
    /// Detailed categories are available through the allocation-free size query.
    pub setup_payload_bound: Option<usize>,
}

/// An unscreened map selected from one exact current numerical frame.
///
/// This is only a structural proposal. It builds no coarse frame, smoother or
/// dense factor and makes no claim about recursive-cycle quality. Selecting a
/// map never removes a positive tuple weight from the submitted operator.
#[derive(Debug)]
pub struct PreparedPairNeighborhoodCandidate<'frame, 'topology> {
    binding: WeightFrameBinding<'frame, 'topology>,
    aggregation: FactorAggregation,
    setup: PreparedAggregationSetup,
    work: PreparedAggregationWork,
}

#[derive(Clone, Copy)]
struct PairMass {
    neighbor: u32,
    level: u32,
    mass: f64,
}
#[derive(Clone, Copy)]
struct Proposal {
    left: u32,
    right: u32,
    value: f64,
    ordinal: usize,
}

// The branch is outside each sort/visit loop; no dynamic iterator allocation.
enum SourceIds {
    Compact(Vec<u32>),
    Wide(Vec<usize>),
}
impl SourceIds {
    fn compact(count: usize) -> bool {
        count.saturating_sub(1) <= u32::MAX as usize
    }
    fn index_bytes(count: usize) -> usize {
        if Self::compact(count) {
            std::mem::size_of::<u32>()
        } else {
            std::mem::size_of::<usize>()
        }
    }
    fn try_new<F>(count: usize, before: &mut F) -> Result<Self, MultiwayError>
    where
        F: FnMut(&'static str) -> Result<(), MultiwayError>,
    {
        if Self::compact(count) {
            let mut ids = reserve(count, "candidate source IDs", before)?;
            ids.extend((0..count).map(|i| i as u32));
            Ok(Self::Compact(ids))
        } else {
            let mut ids = reserve(count, "candidate source IDs", before)?;
            ids.extend(0..count);
            Ok(Self::Wide(ids))
        }
    }
    fn sort(&mut self, tuples: &[[u32; 3]], factor: usize, neighbor: usize) {
        match self {
            Self::Compact(ids) => ids.sort_unstable_by_key(|&id| {
                let i = id as usize;
                (tuples[i][neighbor], tuples[i][factor], id)
            }),
            Self::Wide(ids) => {
                ids.sort_unstable_by_key(|&id| (tuples[id][neighbor], tuples[id][factor], id))
            }
        }
    }
    fn visit<F>(&self, mut f: F) -> Result<(), MultiwayError>
    where
        F: FnMut(usize) -> Result<(), MultiwayError>,
    {
        match self {
            Self::Compact(ids) => {
                for &id in ids {
                    f(id as usize)?;
                }
            }
            Self::Wide(ids) => {
                for &id in ids {
                    f(id)?;
                }
            }
        }
        Ok(())
    }
}

impl<'frame, 'topology> PreparedPairNeighborhoodCandidate<'frame, 'topology> {
    /// Size the entire proposal stage before its first array reservation.
    ///
    /// New excess capacity, inline roots, allocator metadata and sort stack are
    /// excluded. Sorting is in-place; no recursive dense state is constructed.
    pub fn setup_payload_report(
        frame: &ThreeWayWeightFrame<'_>,
        options: PairNeighborhoodAggregationOptions,
        budget: PreparedHierarchyBudget,
    ) -> Result<PreparedAggregationSetup, MultiwayError> {
        if !options.minimum_affinity.is_finite()
            || !(0.0..=1.0).contains(&options.minimum_affinity)
            || options.maximum_neighbor_degree < 2
        {
            return Err(MultiwayError::InvalidOption {
                name: "prepared_pair_neighborhood",
                message: "affinity must be finite in [0,1] and neighbor degree at least two"
                    .to_owned(),
            });
        }
        let counts = frame.topology().topology().level_counts();
        let e = frame.weights().len();
        let v = frame.diagonal().len();
        let mut maximum_proposals = 0;
        for factor in 0..3 {
            let k = options.maximum_neighbor_degree.min(counts[factor]);
            let pairs = mul(k, k.saturating_sub(1))? / 2;
            let edge_bound = mul(e, k.saturating_sub(1))? / 2;
            let mut factor_bound = 0;
            for neighbor in 0..3 {
                if neighbor != factor {
                    factor_bound =
                        add(factor_bound, mul(counts[neighbor], pairs)?.min(edge_bound))?;
                }
            }
            maximum_proposals = maximum_proposals.max(factor_bound);
        }
        let max_count = *counts.iter().max().expect("three factors");
        let flat = sum(&[
            mul(SourceIds::index_bytes(e), e)?,
            bytes::<PairMass>(e)?,
            bytes::<Proposal>(maximum_proposals)?,
            bytes::<u32>(max_count)?,
        ])?;
        // Flat scratch is dropped before validating the completed dense labels.
        let scratch_payload_bound = flat.max(bytes::<bool>(max_count)?);
        let parent_payload_bytes = bytes::<u32>(v)?;
        let borrowed_payload_bytes = add(
            frame.topology().retained_payload_bytes()?,
            frame.retained_payload_bytes()?,
        )?;
        let total_payload_bound = sum(&[
            borrowed_payload_bytes,
            budget.additional_live_payload_bytes,
            parent_payload_bytes,
            scratch_payload_bound,
        ])?;
        Ok(PreparedAggregationSetup {
            borrowed_payload_bytes,
            additional_live_payload_bytes: budget.additional_live_payload_bytes,
            parent_payload_bytes,
            scratch_payload_bound,
            source_index_bytes: SourceIds::index_bytes(e),
            maximum_proposals,
            total_payload_bound,
        })
    }

    /// Propose with bounded flat storage, preserving finite legacy accumulation order.
    ///
    /// All scratch is admitted before allocation. An ordinal resolves duplicate
    /// proposal ties without stable-sort allocation. Overflowing pair/overlap or
    /// normalization arithmetic rejects with attempted work; it is never hidden
    /// by affinity clamping. The older research builder remains available.
    pub fn try_new(
        frame: &'frame ThreeWayWeightFrame<'topology>,
        options: PairNeighborhoodAggregationOptions,
        budget: PreparedHierarchyBudget,
    ) -> Result<Self, PreparedAggregationFailure> {
        Self::build_with(frame, options, budget, &mut |_| Ok(()))
    }

    fn build_with<F>(
        frame: &'frame ThreeWayWeightFrame<'topology>,
        options: PairNeighborhoodAggregationOptions,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<Self, PreparedAggregationFailure>
    where
        F: FnMut(&'static str) -> Result<(), MultiwayError>,
    {
        let mut work = PreparedAggregationWork::default();
        let mut setup_payload_bound = None;
        let result = (|| {
            let report = Self::setup_payload_report(frame, options, budget)?;
            setup_payload_bound = Some(report.total_payload_bound);
            if report.total_payload_bound > budget.maximum_payload_bytes {
                return Err(MultiwayError::PayloadBudgetExceeded {
                    required: report.total_payload_bound,
                    budget: budget.maximum_payload_bytes,
                });
            }
            let counts = frame.topology().topology().level_counts();
            let offsets = frame.topology().topology().offsets();
            let tuples = frame.topology().topology().tuples();
            let e = tuples.len();
            let mut parents = [Vec::new(), Vec::new(), Vec::new()];
            for factor in 0..3 {
                parents[factor] = reserve(counts[factor], "candidate parents", before)?;
                parents[factor].resize(counts[factor], u32::MAX);
            }
            let mut ids = SourceIds::try_new(e, before)?;
            let mut masses: Vec<PairMass> = reserve(e, "candidate pair masses", before)?;
            let mut proposals: Vec<Proposal> =
                reserve(report.maximum_proposals, "candidate overlaps", before)?;
            let mut mates: Vec<u32> =
                reserve(*counts.iter().max().unwrap(), "candidate mates", before)?;
            for factor in 0..3 {
                proposals.clear();
                for neighbor in 0..3 {
                    if neighbor != factor {
                        ids.sort(tuples, factor, neighbor);
                        masses.clear();
                        ids.visit(|id| {
                            work.tuple_visits += 1;
                            let tuple = tuples[id];
                            let weight = frame.weights()[id];
                            if let Some(last) = masses.last_mut().filter(|p| {
                                p.neighbor == tuple[neighbor] && p.level == tuple[factor]
                            }) {
                                last.mass += weight;
                                finite(last.mass, "candidate pair mass")?;
                            } else {
                                masses.push(PairMass {
                                    neighbor: tuple[neighbor],
                                    level: tuple[factor],
                                    mass: weight,
                                });
                                work.pair_entries += 1;
                            }
                            Ok(())
                        })?;
                        let mut start = 0;
                        while start < masses.len() {
                            let mut end = start + 1;
                            while end < masses.len()
                                && masses[end].neighbor == masses[start].neighbor
                            {
                                end += 1;
                            }
                            let row = &mut masses[start..end];
                            row.sort_unstable_by(|a, b| {
                                b.mass
                                    .total_cmp(&a.mass)
                                    .then_with(|| a.level.cmp(&b.level))
                            });
                            let kept = row.len().min(options.maximum_neighbor_degree);
                            if kept < row.len() {
                                work.truncated_neighbors += 1;
                            }
                            let row = &mut row[..kept];
                            row.sort_unstable_by_key(|p| p.level);
                            for left in 0..row.len() {
                                for right in left + 1..row.len() {
                                    // Shared neighbors imply the same exact incidence component.
                                    debug_assert_eq!(
                                        frame.topology().component_labels()
                                            [offsets[factor] + row[left].level as usize],
                                        frame.topology().component_labels()
                                            [offsets[factor] + row[right].level as usize]
                                    );
                                    assert!(
                                        proposals.len() < report.maximum_proposals,
                                        "proved proposal bound"
                                    );
                                    proposals.push(Proposal {
                                        left: row[left].level,
                                        right: row[right].level,
                                        value: row[left].mass.min(row[right].mass),
                                        ordinal: proposals.len(),
                                    });
                                    work.proposals += 1;
                                }
                            }
                            start = end;
                        }
                    }
                }
                proposals.sort_unstable_by_key(|p| (p.left, p.right, p.ordinal));
                let mut unique = 0;
                for read in 0..proposals.len() {
                    let p = proposals[read];
                    if unique > 0
                        && (proposals[unique - 1].left, proposals[unique - 1].right)
                            == (p.left, p.right)
                    {
                        proposals[unique - 1].value += p.value;
                        finite(proposals[unique - 1].value, "candidate overlap")?;
                    } else {
                        proposals[unique] = p;
                        unique += 1;
                        work.unique_candidates += 1;
                    }
                }
                proposals.truncate(unique);
                for p in &mut proposals {
                    let a = frame.diagonal()[offsets[factor] + p.left as usize];
                    let b = frame.diagonal()[offsets[factor] + p.right as usize];
                    let denominator = a.sqrt() * b.sqrt();
                    finite(denominator, "candidate normalization")?;
                    if denominator == 0.0 {
                        return Err(MultiwayError::NumericalFailure {
                            context: "candidate normalization",
                        });
                    }
                    let affinity = p.value / denominator;
                    finite(affinity, "candidate affinity")?;
                    p.value = affinity.clamp(0.0, 1.0);
                }
                proposals.retain(|p| p.value >= options.minimum_affinity);
                proposals.sort_unstable_by(|a, b| {
                    b.value
                        .total_cmp(&a.value)
                        .then_with(|| a.left.cmp(&b.left))
                        .then_with(|| a.right.cmp(&b.right))
                });
                mates.resize(counts[factor], u32::MAX);
                mates.fill(u32::MAX);
                for p in &proposals {
                    let a = p.left as usize;
                    let b = p.right as usize;
                    if mates[a] == u32::MAX && mates[b] == u32::MAX {
                        mates[a] = p.right;
                        mates[b] = p.left;
                        work.accepted_pairs += 1;
                    }
                }
                let mut next = 0u32;
                for level in 0..counts[factor] {
                    if parents[factor][level] == u32::MAX {
                        parents[factor][level] = next;
                        if mates[level] != u32::MAX {
                            parents[factor][mates[level] as usize] = next;
                        }
                        next = next.checked_add(1).ok_or_else(overflow)?;
                    }
                }
            }
            drop((ids, masses, proposals, mates));
            // Requires the fallible constructor introduced with this increment.
            let aggregation = FactorAggregation::try_new(counts, parents)?;
            Ok(Self {
                binding: frame.binding(),
                aggregation,
                setup: report,
                work,
            })
        })();
        result.map_err(|source| PreparedAggregationFailure {
            source,
            work,
            setup_payload_bound,
        })
    }

    /// Verify the exact numerical frame from which affinities were computed.
    pub fn validate_for(&self, frame: &ThreeWayWeightFrame<'_>) -> Result<(), MultiwayError> {
        self.binding.validate_for(frame)?;
        Ok(())
    }
    /// Borrow the unscreened structural map.
    pub const fn aggregation(&self) -> &FactorAggregation {
        &self.aggregation
    }
    /// Complete conservative setup admission report.
    pub const fn setup_report(&self) -> PreparedAggregationSetup {
        self.setup
    }
    /// Actual completed sparse work.
    pub const fn work_report(&self) -> PreparedAggregationWork {
        self.work
    }
    /// Exclusive retained parent capacities, excluding the borrowed frame.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        Ok(self.aggregation.retained_payload_bytes()?)
    }
    /// Consume the selection record and return a structural map for later screening.
    /// Changed weights always require fresh numerical replay and quality screening.
    pub fn into_aggregation(self) -> FactorAggregation {
        self.aggregation
    }
}
fn finite(x: f64, context: &'static str) -> Result<(), MultiwayError> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(MultiwayError::NumericalFailure { context })
    }
}
fn overflow() -> MultiwayError {
    multiway_incidence::IncidenceError::DimensionOverflow {
        context: "prepared aggregation",
    }
    .into()
}
fn add(a: usize, b: usize) -> Result<usize, MultiwayError> {
    a.checked_add(b).ok_or_else(overflow)
}
fn mul(a: usize, b: usize) -> Result<usize, MultiwayError> {
    a.checked_mul(b).ok_or_else(overflow)
}
fn sum(xs: &[usize]) -> Result<usize, MultiwayError> {
    xs.iter().try_fold(0, |a, &b| add(a, b))
}
fn bytes<T>(n: usize) -> Result<usize, MultiwayError> {
    let n = mul(n, std::mem::size_of::<T>())?;
    if n <= isize::MAX as usize {
        Ok(n)
    } else {
        Err(overflow())
    }
}

fn reserve<T, F>(n: usize, context: &'static str, before: &mut F) -> Result<Vec<T>, MultiwayError>
where
    F: FnMut(&'static str) -> Result<(), MultiwayError>,
{
    bytes::<T>(n)?;
    let mut v = Vec::new();
    if n > 0 {
        before(context)?;
        v.try_reserve_exact(n)
            .map_err(|_| multiway_incidence::IncidenceError::TopologyAllocation { context })?;
    }
    Ok(v)
}

#[cfg(test)]
mod failure_tests {
    use super::*;
    use multiway_incidence::{PreparedThreeWayTopology, WeightFrameInput};
    #[test]
    fn compact_and_wide_source_orders_match_at_all_factor_pairs() {
        assert!(SourceIds::compact(u32::MAX as usize));
        if let Some(first_wide) = (u32::MAX as usize).checked_add(2) {
            assert!(SourceIds::compact(first_wide - 1));
            assert!(!SourceIds::compact(first_wide));
            assert_eq!(
                SourceIds::index_bytes(first_wide),
                std::mem::size_of::<usize>()
            );
        }
        let tuples: Vec<_> = (0..3)
            .flat_map(|i| (0..2).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
            .collect();
        let mut compact = SourceIds::Compact((0..tuples.len()).rev().map(|i| i as u32).collect());
        let mut wide = SourceIds::Wide((0..tuples.len()).rev().collect());
        for factor in 0..3 {
            for neighbor in 0..3 {
                if factor != neighbor {
                    compact.sort(&tuples, factor, neighbor);
                    wide.sort(&tuples, factor, neighbor);
                    let mut a = Vec::new();
                    let mut b = Vec::new();
                    compact
                        .visit(|i| {
                            a.push(i);
                            Ok(())
                        })
                        .unwrap();
                    wide.visit(|i| {
                        b.push(i);
                        Ok(())
                    })
                    .unwrap();
                    assert_eq!(a, b);
                    assert!(a.windows(2).all(|w| (
                        tuples[w[0]][neighbor],
                        tuples[w[0]][factor],
                        w[0]
                    ) < (
                        tuples[w[1]][neighbor],
                        tuples[w[1]][factor],
                        w[1]
                    )));
                }
            }
        }
        assert!(
            SourceIds::try_new(usize::MAX, &mut |_| panic!("width overflow before reserve"))
                .is_err()
        );
    }
    #[test]
    fn complete_admission_precedes_every_flat_reservation_and_unwind_recovers() {
        let tuples: Vec<_> = (0..4)
            .flat_map(|i| (0..3).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
            .collect();
        let t = PreparedThreeWayTopology::try_from_collapsed([4, 3, 2], &tuples).unwrap();
        let frame = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
        let options = PairNeighborhoodAggregationOptions::default();
        let mut calls = 0;
        let old = PreparedPairNeighborhoodCandidate::build_with(
            &frame,
            options,
            PreparedHierarchyBudget::UNLIMITED,
            &mut |_| {
                calls += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(calls, 7); // Three parents plus source IDs, masses, proposals and mates.
        let required = old.setup_report().total_payload_bound;
        let bad = PreparedPairNeighborhoodCandidate::build_with(
            &frame,
            options,
            PreparedHierarchyBudget {
                maximum_payload_bytes: required - 1,
                additional_live_payload_bytes: 0,
            },
            &mut |_| panic!("not admitted"),
        )
        .unwrap_err();
        assert_eq!(bad.work, Default::default());
        for unwind in [false, true] {
            for fail_at in 0..calls {
                let mut reached = 0;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    PreparedPairNeighborhoodCandidate::build_with(
                        &frame,
                        options,
                        PreparedHierarchyBudget::UNLIMITED,
                        &mut |context| {
                            let at = reached;
                            reached += 1;
                            if at == fail_at {
                                if unwind {
                                    panic!("injected flat candidate unwind");
                                }
                                return Err(
                                    multiway_incidence::IncidenceError::TopologyAllocation {
                                        context,
                                    }
                                    .into(),
                                );
                            }
                            Ok(())
                        },
                    )
                }));
                assert_eq!(reached, fail_at + 1);
                if unwind {
                    assert!(result.is_err());
                } else {
                    assert_eq!(result.unwrap().unwrap_err().work, Default::default());
                }
                let rebuilt = PreparedPairNeighborhoodCandidate::try_new(
                    &frame,
                    options,
                    PreparedHierarchyBudget::UNLIMITED,
                )
                .unwrap();
                assert_eq!(old.aggregation(), rebuilt.aggregation());
                assert_eq!(old.work_report(), rebuilt.work_report());
            }
        }
        let overflowed = PreparedPairNeighborhoodCandidate::build_with(
            &frame,
            options,
            PreparedHierarchyBudget {
                maximum_payload_bytes: usize::MAX,
                additional_live_payload_bytes: usize::MAX,
            },
            &mut |_| panic!("overflow before reservation"),
        )
        .unwrap_err();
        assert!(overflowed.setup_payload_bound.is_none());
        assert_eq!(overflowed.work, Default::default());
        assert!(bytes::<f64>(usize::MAX).is_err());
        assert!(bytes::<u8>(isize::MAX as usize + 1).is_err());
        assert!(mul(usize::MAX, 2).is_err());
    }
    #[test]
    fn per_neighbor_proposal_bound_covers_all_small_ragged_degree_lists() {
        for k in 1usize..=8 {
            for a in 0usize..=7 {
                for b in 0usize..=7 {
                    for c in 0usize..=7 {
                        let actual = [a, b, c]
                            .into_iter()
                            .map(|n| {
                                let n = n.min(k);
                                n * n.saturating_sub(1) / 2
                            })
                            .sum::<usize>();
                        let entries = a + b + c;
                        let bound = (3 * k * k.saturating_sub(1) / 2)
                            .min(entries * k.saturating_sub(1) / 2);
                        assert!(actual <= bound);
                    }
                }
            }
        }
    }
}
