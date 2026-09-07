//! Exact weights-free factor rows over the sole canonical tuple array.
use crate::{
    IncidenceError, PreparedThreeWayTopology,
    construction::{array_bytes, reserve, sum_bytes},
};
use core::ops::Range;

/// Actual stored tuple-ID width; factor zero uses implicit canonical ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupedIndexWidth {
    /// Four bytes per tuple ID when its maximum value fits.
    Narrow,
    /// Native usize tuple IDs, checked against addressable array limits.
    Wide,
}
#[derive(Debug)]
enum Indices {
    Narrow(Vec<u32>),
    Wide(Vec<usize>),
}

/// One immutable coefficient row in ascending canonical tuple order.
///
/// Matching the representation once per row permits monomorphic hot loops.
/// Slices name canonical tuple IDs, never copied numerical weights or tuples.
#[derive(Debug, Clone)]
pub enum GroupedTupleRow<'a> {
    /// Factor zero's canonical contiguous tuple range, with no stored IDs.
    Contiguous(Range<usize>),
    /// Stable narrow tuple IDs for factor one or two.
    Narrow(&'a [u32]),
    /// Stable native-width tuple IDs for factor one or two.
    Wide(&'a [usize]),
}
impl GroupedTupleRow<'_> {
    /// Number of incident canonical tuples.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Contiguous(r) => r.len(),
            Self::Narrow(r) => r.len(),
            Self::Wide(r) => r.len(),
        }
    }
    /// Whether the row has no incident tuples.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Visit canonical tuple IDs in ascending order, dispatching outside the loop.
    #[inline]
    pub fn for_each(self, mut f: impl FnMut(usize)) {
        match self {
            Self::Contiguous(r) => r.for_each(f),
            Self::Narrow(r) => r.iter().for_each(|&id| f(id as usize)),
            Self::Wide(r) => r.iter().for_each(|&id| f(id)),
        }
    }
}

/// Optional weights-free stable row grouping bound to one exact prepared topology.
///
/// Retains one V+3 offset array and one 2E tuple-ID array. Canonical factor-zero
/// rows are implicit; factors one/two use stable counting placement. No tuples,
/// components, observation groups or numerical weights are duplicated. The same
/// grouping can serve any immutable weight frame of this exact topology.
///
/// The borrowed structural owner cannot be moved or dropped while used:
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, PreparedTupleGrouping};
/// let groups = {
///     let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
///     PreparedTupleGrouping::try_new(&t).unwrap()
/// };
/// assert_eq!(groups.row(0, 0).unwrap().len(), 1);
/// ```
#[derive(Debug)]
pub struct PreparedTupleGrouping<'topology> {
    topology: &'topology PreparedThreeWayTopology,
    offsets: Vec<usize>,
    indices: Indices,
}
impl<'topology> PreparedTupleGrouping<'topology> {
    /// Fallibly prepare stable factor rows; scalar execution does not require them.
    pub fn try_new(topology: &'topology PreparedThreeWayTopology) -> Result<Self, IncidenceError> {
        Self::try_new_with_budget(topology, usize::MAX, 0)
    }

    /// Admit topology, requested grouping/cursor arrays and declared other live payload.
    ///
    /// `maximum_payload_bytes` is a requested-array admission bound, not an
    /// allocator quota or RSS limit. Other live groups, frames and workspaces
    /// belong in `additional_live_payload_bytes`. Failure publishes no owner.
    pub fn try_new_with_budget(
        topology: &'topology PreparedThreeWayTopology,
        maximum_payload_bytes: usize,
        additional_live_payload_bytes: usize,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(
            topology,
            choose_width(topology.topology().tuple_count()),
            maximum_payload_bytes,
            additional_live_payload_bytes,
            &mut |_| Ok(()),
        )
    }

    /// Requested exclusive retained grouping payload, excluding the borrowed topology.
    pub fn required_payload_bytes(
        topology: &PreparedThreeWayTopology,
    ) -> Result<usize, IncidenceError> {
        let t = topology.topology();
        grouping_bytes(
            t.level_counts(),
            t.tuple_count(),
            choose_width(t.tuple_count()),
        )
    }

    /// Conservative simultaneously live requested construction payload.
    ///
    /// Includes actual borrowed topology capacity, new grouping arrays, one
    /// reused max(n1,n2) cursor and declared other arrays. Excludes inline roots,
    /// stack, allocator metadata and allocator-provided excess new capacity.
    pub fn setup_payload_bound(
        topology: &PreparedThreeWayTopology,
        additional_live_payload_bytes: usize,
    ) -> Result<usize, IncidenceError> {
        setup_bytes(
            topology,
            choose_width(topology.topology().tuple_count()),
            additional_live_payload_bytes,
        )
    }

    fn build_with<F>(
        topology: &'topology PreparedThreeWayTopology,
        width: GroupedIndexWidth,
        maximum_payload_bytes: usize,
        additional_live_payload_bytes: usize,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let required = setup_bytes(topology, width, additional_live_payload_bytes)?;
        if required > maximum_payload_bytes {
            return Err(IncidenceError::SymbolicSetupBudgetExceeded {
                context: "factor tuple grouping",
                required,
                budget: maximum_payload_bytes,
            });
        }
        let t = topology.topology();
        let counts = t.level_counts();
        let e = t.tuple_count();
        let mut offsets = reserve(offset_count(counts)?, "factor tuple offsets", before)?;
        offsets.resize(offset_count(counts)?, 0usize);
        for factor in 0..3 {
            let start = t.offsets()[factor] + factor;
            let part = &mut offsets[start..start + counts[factor] + 1];
            for tuple in t.tuples() {
                part[tuple[factor] as usize + 1] += 1;
            }
            for i in 0..counts[factor] {
                part[i + 1] += part[i];
            }
            debug_assert_eq!(part[counts[factor]], e);
        }
        let count = e.checked_mul(2).ok_or_else(overflow)?;
        let mut indices = match width {
            GroupedIndexWidth::Narrow => {
                let mut ids = reserve(count, "narrow factor tuple IDs", before)?;
                ids.resize(count, 0u32);
                Indices::Narrow(ids)
            }
            GroupedIndexWidth::Wide => {
                let mut ids = reserve(count, "wide factor tuple IDs", before)?;
                ids.resize(count, 0usize);
                Indices::Wide(ids)
            }
        };
        let cursor_len = counts[1].max(counts[2]);
        let mut cursor = reserve(cursor_len, "factor tuple placement cursor", before)?;
        cursor.resize(cursor_len, 0usize);
        for factor in 1..3 {
            let start = t.offsets()[factor] + factor;
            cursor[..counts[factor]].copy_from_slice(&offsets[start..start + counts[factor]]);
            for (id, tuple) in t.tuples().iter().enumerate() {
                let position = &mut cursor[tuple[factor] as usize];
                let target = (factor - 1) * e + *position;
                match &mut indices {
                    Indices::Narrow(ids) => ids[target] = id as u32, // Checked once before reservation.
                    Indices::Wide(ids) => ids[target] = id,
                }
                *position += 1;
            }
        }
        Ok(Self {
            topology,
            offsets,
            indices,
        })
    }

    /// Exact borrowed structural owner.
    #[must_use]
    pub const fn topology(&self) -> &'topology PreparedThreeWayTopology {
        self.topology
    }
    /// Reject even an equal-but-foreign topology; numerical weights are irrelevant.
    pub fn validate_for(&self, topology: &PreparedThreeWayTopology) -> Result<(), IncidenceError> {
        self.topology.binding().validate_for(topology)
    }
    /// Actual width of the two stored factor permutations.
    #[must_use]
    pub const fn index_width(&self) -> GroupedIndexWidth {
        match self.indices {
            Indices::Narrow(_) => GroupedIndexWidth::Narrow,
            Indices::Wide(_) => GroupedIndexWidth::Wide,
        }
    }
    /// Local row boundaries for one factor, from zero to E inclusive.
    #[must_use]
    pub fn factor_offsets(&self, factor: usize) -> Option<&[usize]> {
        let t = self.topology.topology();
        let count = *t.level_counts().get(factor)?;
        let start = t.offsets()[factor] + factor;
        Some(&self.offsets[start..start + count + 1])
    }
    /// One checked factor-local row; invalid factor or level returns None.
    #[must_use]
    #[inline]
    pub fn row(&self, factor: usize, level: usize) -> Option<GroupedTupleRow<'_>> {
        let offsets = self.factor_offsets(factor)?;
        let begin = *offsets.get(level)?;
        let end = *offsets.get(level.checked_add(1)?)?;
        if factor == 0 {
            return Some(GroupedTupleRow::Contiguous(begin..end));
        }
        let base = (factor - 1) * self.topology.topology().tuple_count();
        Some(match &self.indices {
            Indices::Narrow(ids) => GroupedTupleRow::Narrow(&ids[base + begin..base + end]),
            Indices::Wide(ids) => GroupedTupleRow::Wide(&ids[base + begin..base + end]),
        })
    }
    /// Actual exclusive array capacities; cursor storage is dropped after construction.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            array_bytes::<usize>(self.offsets.capacity())?,
            match &self.indices {
                Indices::Narrow(ids) => array_bytes::<u32>(ids.capacity())?,
                Indices::Wide(ids) => array_bytes::<usize>(ids.capacity())?,
            },
        ])
    }
}
fn choose_width(tuples: usize) -> GroupedIndexWidth {
    if u32::try_from(tuples.saturating_sub(1)).is_ok() {
        GroupedIndexWidth::Narrow
    } else {
        GroupedIndexWidth::Wide
    }
}
fn overflow() -> IncidenceError {
    IncidenceError::DimensionOverflow {
        context: "factor tuple grouping",
    }
}
fn offset_count(counts: [usize; 3]) -> Result<usize, IncidenceError> {
    counts
        .iter()
        .try_fold(3usize, |sum, &n| sum.checked_add(n).ok_or_else(overflow))
}
fn grouping_bytes(
    counts: [usize; 3],
    tuples: usize,
    width: GroupedIndexWidth,
) -> Result<usize, IncidenceError> {
    let ids = tuples.checked_mul(2).ok_or_else(overflow)?;
    let indices = match width {
        GroupedIndexWidth::Narrow => {
            u32::try_from(tuples.saturating_sub(1)).map_err(|_| overflow())?;
            array_bytes::<u32>(ids)?
        }
        GroupedIndexWidth::Wide => array_bytes::<usize>(ids)?,
    };
    sum_bytes(&[array_bytes::<usize>(offset_count(counts)?)?, indices])
}
fn setup_bytes(
    topology: &PreparedThreeWayTopology,
    width: GroupedIndexWidth,
    additional: usize,
) -> Result<usize, IncidenceError> {
    let t = topology.topology();
    let counts = t.level_counts();
    sum_bytes(&[
        topology.retained_payload_bytes()?,
        grouping_bytes(counts, t.tuple_count(), width)?,
        array_bytes::<usize>(counts[1].max(counts[2]))?,
        additional,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_rows_both_widths_exact_budget_and_every_reservation_failure() {
        let tuples = [[0, 0, 0], [0, 1, 2], [0, 2, 1], [1, 0, 1], [1, 2, 3]];
        let topology = PreparedThreeWayTopology::try_from_collapsed([2, 3, 4], &tuples).unwrap();
        for width in [GroupedIndexWidth::Narrow, GroupedIndexWidth::Wide] {
            let required = setup_bytes(&topology, width, 17).unwrap();
            let mut reservations = 0;
            let groups =
                PreparedTupleGrouping::build_with(&topology, width, required, 17, &mut |_| {
                    reservations += 1;
                    Ok(())
                })
                .unwrap();
            assert_eq!(reservations, 3);
            assert_eq!(groups.index_width(), width);
            assert_eq!(
                groups.retained_payload_bytes().unwrap(),
                grouping_bytes([2, 3, 4], tuples.len(), width).unwrap()
            );
            for (factor, count) in [2, 3, 4].into_iter().enumerate() {
                for level in 0..count {
                    let expected: Vec<_> = tuples
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| t[factor] as usize == level)
                        .map(|(i, _)| i)
                        .collect();
                    let row = groups.row(factor, level).unwrap();
                    assert_eq!(row.len(), expected.len());
                    assert!(!row.is_empty());
                    let mut actual = Vec::new();
                    row.for_each(|id| actual.push(id));
                    assert_eq!(actual, expected);
                }
                assert!(groups.row(factor, count).is_none());
            }
            let weights = [0.5, 2.0, 8.0, 0.25, 16.0];
            let frame = crate::ThreeWayWeightFrame::try_new(
                &topology,
                crate::WeightFrameInput::Tuples(&weights),
            )
            .unwrap();
            let scalar = frame.operator_view();
            let grouped = scalar.with_grouping(&groups).unwrap();
            let x = [0.25, -2.0, 4.0, -8.0, 0.5, -1.0, 2.0, -4.0, 8.0];
            let y = [-1.0, 2.0, -4.0, 8.0, 0.125];
            let mut a = [0.0; 9];
            let mut b = [f64::NAN; 9];
            let mut image = [f64::NAN; 5];
            scalar.apply_gramian(&x, &mut a).unwrap();
            grouped.apply_gramian(&x, &mut b).unwrap();
            assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
            grouped
                .apply_gramian_with_image(&x, &mut b, &mut image)
                .unwrap();
            assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
            scalar.apply_adjoint(&y, &mut a).unwrap();
            grouped.apply_adjoint(&y, &mut b).unwrap();
            assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
            scalar.apply_weighted_adjoint(&y, &mut a).unwrap();
            grouped.apply_weighted_adjoint(&y, &mut b).unwrap();
            assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
            scalar.rhs_from_targets_into(&y, &mut a).unwrap();
            grouped.rhs_from_targets_into(&y, &mut b).unwrap();
            assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
            assert!(groups.row(3, 0).is_none());
            assert!(groups.row(0, usize::MAX).is_none());
            assert!(
                PreparedTupleGrouping::build_with(
                    &topology,
                    width,
                    required - 1,
                    17,
                    &mut |_| panic!("over budget before allocation")
                )
                .is_err()
            );
            for fail in 0..reservations {
                let mut next = 0;
                let result = PreparedTupleGrouping::build_with(
                    &topology,
                    width,
                    required,
                    17,
                    &mut |context| {
                        if next == fail {
                            return Err(IncidenceError::TopologyAllocation { context });
                        }
                        next += 1;
                        Ok(())
                    },
                );
                assert!(matches!(
                    result,
                    Err(IncidenceError::TopologyAllocation { .. })
                ));
            }
            assert!(
                std::panic::catch_unwind(|| {
                    let mut next = 0;
                    let _ = PreparedTupleGrouping::build_with(
                        &topology,
                        width,
                        required,
                        17,
                        &mut |_| {
                            next += 1;
                            if next == 3 {
                                panic!("cursor reservation unwind");
                            }
                            Ok(())
                        },
                    );
                })
                .is_err()
            );
            assert!(PreparedTupleGrouping::try_new(&topology).is_ok());
        }
    }
    #[test]
    fn impossible_dimensions_and_width_boundary_do_not_allocate() {
        assert!(grouping_bytes([usize::MAX, 1, 1], 1, GroupedIndexWidth::Narrow).is_err());
        assert!(grouping_bytes([1; 3], usize::MAX, GroupedIndexWidth::Wide).is_err());
        #[cfg(target_pointer_width = "64")]
        {
            let full = u32::MAX as usize + 1;
            assert_eq!(choose_width(full), GroupedIndexWidth::Narrow);
            assert_eq!(choose_width(full + 1), GroupedIndexWidth::Wide);
            assert!(grouping_bytes([1; 3], full, GroupedIndexWidth::Narrow).is_ok());
            assert!(grouping_bytes([1; 3], full + 1, GroupedIndexWidth::Narrow).is_err());
            assert!(grouping_bytes([1; 3], full + 1, GroupedIndexWidth::Wide).is_ok());
        }
    }
}
