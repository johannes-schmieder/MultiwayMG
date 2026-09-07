//! Separately admitted weights-free grouping of an explicit nonterminal prefix.
use crate::{
    IncidenceError, PreparedHierarchyBudget, PreparedHierarchyTopology, PreparedTupleGrouping,
    construction::{array_bytes, reserve, sum_bytes},
};

/// Optional exact factor rows for a declared prefix of a structural hierarchy.
///
/// Owns only selected grouping arrays and their descriptor array, borrowing the
/// exact immutable hierarchy. `grouped_levels` may be zero through the number of
/// nonterminal levels. One selects fine-only for a nonterminal fine level; the
/// full nonterminal count selects every cycle level. A terminal-only hierarchy
/// therefore accepts zero. This explicit prefix is not an automatic selector.
/// Different current-weight replays can share the same structural grouping.
///
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, PreparedHierarchyTopology,
///     PreparedHierarchyGrouping};
/// let fine = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let groups = {
///     let hierarchy = PreparedHierarchyTopology::try_new(&fine, vec![]).unwrap();
///     PreparedHierarchyGrouping::try_new(&hierarchy, 0).unwrap()
/// };
/// assert_eq!(groups.grouped_levels(), 0);
/// ```
#[derive(Debug)]
pub struct PreparedHierarchyGrouping<'hierarchy, 'fine> {
    hierarchy: &'hierarchy PreparedHierarchyTopology<'fine>,
    groups: Vec<PreparedTupleGrouping<'hierarchy>>,
    setup_peak_payload_bound: usize,
}
impl<'hierarchy, 'fine> PreparedHierarchyGrouping<'hierarchy, 'fine> {
    /// Prepare an explicit nonterminal grouping prefix without a payload limit.
    pub fn try_new(
        hierarchy: &'hierarchy PreparedHierarchyTopology<'fine>,
        grouped_levels: usize,
    ) -> Result<Self, IncidenceError> {
        Self::try_new_with_budget(
            hierarchy,
            grouped_levels,
            PreparedHierarchyBudget::UNLIMITED,
        )
    }
    /// Admit the entire requested build peak before the first reservation.
    ///
    /// Charges fine and owned coarse structure once, grouping descriptors/arrays,
    /// the currently live construction cursor and declared other arrays. Old
    /// groups, frames and workspaces belong in the caller's additional live bytes.
    /// This is requested-array admission, not a stack/allocator/RSS quota.
    pub fn try_new_with_budget(
        hierarchy: &'hierarchy PreparedHierarchyTopology<'fine>,
        grouped_levels: usize,
        budget: PreparedHierarchyBudget,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(hierarchy, grouped_levels, budget, &mut |_| Ok(()))
    }
    /// Requested exclusive retained grouping arrays and descriptors.
    pub fn required_payload_bytes(
        hierarchy: &PreparedHierarchyTopology<'_>,
        grouped_levels: usize,
    ) -> Result<usize, IncidenceError> {
        validate_prefix(hierarchy, grouped_levels)?;
        let mut result = array_bytes::<PreparedTupleGrouping<'_>>(grouped_levels)?;
        for level in 0..grouped_levels {
            result = sum_bytes(&[
                result,
                PreparedTupleGrouping::required_payload_bytes(
                    hierarchy.level(level).expect("validated grouped level"),
                )?,
            ])?;
        }
        Ok(result)
    }
    /// Complete requested construction peak, including borrowed owners exactly once.
    ///
    /// Cursors are dropped between levels, so only the current cursor is charged.
    /// Excludes inline roots, stack, metadata and allocator-provided excess new
    /// capacity; actual retained capacities are measured separately.
    pub fn setup_payload_bound(
        hierarchy: &PreparedHierarchyTopology<'_>,
        grouped_levels: usize,
        additional_live_payload_bytes: usize,
    ) -> Result<usize, IncidenceError> {
        validate_prefix(hierarchy, grouped_levels)?;
        let mut retained = sum_bytes(&[
            base_bytes(hierarchy, additional_live_payload_bytes)?,
            array_bytes::<PreparedTupleGrouping<'_>>(grouped_levels)?,
        ])?;
        let mut peak = retained;
        for level in 0..grouped_levels {
            let topology = hierarchy.level(level).expect("validated grouped level");
            let new_peak = PreparedTupleGrouping::setup_payload_bound(topology, 0)?
                .checked_sub(topology.retained_payload_bytes()?)
                .ok_or_else(overflow)?;
            peak = peak.max(sum_bytes(&[retained, new_peak])?);
            retained = sum_bytes(&[
                retained,
                PreparedTupleGrouping::required_payload_bytes(topology)?,
            ])?;
        }
        Ok(peak.max(retained))
    }
    fn build_with<F>(
        hierarchy: &'hierarchy PreparedHierarchyTopology<'fine>,
        grouped_levels: usize,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let requested = Self::setup_payload_bound(
            hierarchy,
            grouped_levels,
            budget.additional_live_payload_bytes,
        )?;
        admit(requested, budget)?;
        let groups = reserve(
            grouped_levels,
            "hierarchy tuple grouping descriptors",
            before,
        )?;
        let base = base_bytes(hierarchy, budget.additional_live_payload_bytes)?;
        let mut result = Self {
            hierarchy,
            groups,
            setup_peak_payload_bound: requested,
        };
        for level in 0..grouped_levels {
            let topology = hierarchy.level(level).expect("validated grouped level");
            // Single-level admission adds its borrowed topology; it is already
            // present in the full structural base, so remove that one charge.
            let live = sum_bytes(&[base, result.retained_payload_bytes()?])?;
            let additional = live
                .checked_sub(topology.retained_payload_bytes()?)
                .ok_or_else(overflow)?;
            let peak = PreparedTupleGrouping::setup_payload_bound(topology, additional)?;
            admit(peak, budget)?;
            result.setup_peak_payload_bound = result.setup_peak_payload_bound.max(peak);
            result.groups.push(PreparedTupleGrouping::build_with_hook(
                topology,
                budget.maximum_payload_bytes,
                additional,
                before,
            )?);
        }
        let live = sum_bytes(&[base, result.retained_payload_bytes()?])?;
        admit(live, budget)?;
        result.setup_peak_payload_bound = result.setup_peak_payload_bound.max(live);
        Ok(result)
    }
    /// Exact borrowed structural hierarchy; numerical frames are not retained.
    #[must_use]
    pub const fn hierarchy(&self) -> &'hierarchy PreparedHierarchyTopology<'fine> {
        self.hierarchy
    }
    /// Number of explicitly grouped nonterminal levels.
    #[must_use]
    pub fn grouped_levels(&self) -> usize {
        self.groups.len()
    }
    /// Exact group owner at a selected level; terminal/unselected levels return None.
    #[must_use]
    pub fn level(&self, level: usize) -> Option<&PreparedTupleGrouping<'hierarchy>> {
        self.groups.get(level)
    }
    /// Reject an equal-but-foreign structural hierarchy before numerical mutation.
    pub fn validate_for(
        &self,
        hierarchy: &PreparedHierarchyTopology<'_>,
    ) -> Result<(), IncidenceError> {
        if core::ptr::eq(self.hierarchy, hierarchy) {
            Ok(())
        } else {
            Err(IncidenceError::HierarchyBindingMismatch)
        }
    }
    /// Largest charged setup bound, including the caller's declared other live state.
    #[must_use]
    pub const fn setup_peak_payload_bound(&self) -> usize {
        self.setup_peak_payload_bound
    }
    /// Actual exclusive grouping array/descriptor capacities; no borrowed owner copies.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        let mut result = array_bytes::<PreparedTupleGrouping<'_>>(self.groups.capacity())?;
        for group in &self.groups {
            result = sum_bytes(&[result, group.retained_payload_bytes()?])?;
        }
        Ok(result)
    }
}
fn validate_prefix(
    hierarchy: &PreparedHierarchyTopology<'_>,
    levels: usize,
) -> Result<(), IncidenceError> {
    let depth = hierarchy.level_count() - 1;
    if levels > depth {
        return Err(crate::error::dimension(
            "maximum grouped nonterminal prefix",
            depth,
            levels,
        ));
    }
    Ok(())
}
fn base_bytes(
    hierarchy: &PreparedHierarchyTopology<'_>,
    additional: usize,
) -> Result<usize, IncidenceError> {
    sum_bytes(&[
        hierarchy.fine().retained_payload_bytes()?,
        hierarchy.retained_payload_bytes()?,
        additional,
    ])
}
fn admit(required: usize, budget: PreparedHierarchyBudget) -> Result<(), IncidenceError> {
    if required > budget.maximum_payload_bytes {
        Err(IncidenceError::HierarchyBudgetExceeded {
            required,
            budget: budget.maximum_payload_bytes,
        })
    } else {
        Ok(())
    }
}
fn overflow() -> IncidenceError {
    IncidenceError::DimensionOverflow {
        context: "hierarchy tuple grouping",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FactorAggregation, PreparedThreeWayTopology};
    #[test]
    fn prefix_peak_admission_exact_owners_and_every_reservation_unwind() {
        let tuples: Vec<_> = (0..4)
            .flat_map(|i| (0..4).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
            .collect();
        let topology = PreparedThreeWayTopology::try_from_collapsed([4; 3], &tuples).unwrap();
        let make = || {
            PreparedHierarchyTopology::try_new(
                &topology,
                vec![
                    FactorAggregation::consecutive_halving([4; 3]).unwrap(),
                    FactorAggregation::consecutive_halving([2; 3]).unwrap(),
                ],
            )
            .unwrap()
        };
        let h = make();
        let foreign = make();
        for prefix in 0..=2 {
            let peak = PreparedHierarchyGrouping::setup_payload_bound(&h, prefix, 17).unwrap();
            let budget = PreparedHierarchyBudget {
                maximum_payload_bytes: peak,
                additional_live_payload_bytes: 17,
            };
            assert!(
                PreparedHierarchyGrouping::build_with(
                    &h,
                    prefix,
                    PreparedHierarchyBudget {
                        maximum_payload_bytes: peak - 1,
                        ..budget
                    },
                    &mut |_| panic!("reject full peak before allocation")
                )
                .is_err()
            );
            let mut calls = 0;
            let g = PreparedHierarchyGrouping::build_with(&h, prefix, budget, &mut |_| {
                calls += 1;
                Ok(())
            })
            .unwrap();
            assert_eq!(calls, if prefix == 0 { 0 } else { 1 + 3 * prefix });
            assert_eq!(g.grouped_levels(), prefix);
            assert_eq!(g.setup_peak_payload_bound(), peak);
            assert_eq!(
                g.retained_payload_bytes().unwrap(),
                PreparedHierarchyGrouping::required_payload_bytes(&h, prefix).unwrap()
            );
            assert!(g.validate_for(&foreign).is_err());
            g.validate_for(&h).unwrap();
            for level in 0..prefix {
                g.level(level)
                    .unwrap()
                    .validate_for(h.level(level).unwrap())
                    .unwrap();
            }
            assert!(g.level(prefix).is_none());
            for unwind in [false, true] {
                for fail in 0..calls {
                    let mut next = 0;
                    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        PreparedHierarchyGrouping::build_with(&h, prefix, budget, &mut |context| {
                            let n = next;
                            next += 1;
                            if n == fail {
                                assert!(!unwind, "injected hierarchy grouping unwind");
                                return Err(IncidenceError::TopologyAllocation { context });
                            }
                            Ok(())
                        })
                    }));
                    assert_eq!(next, fail + 1);
                    if unwind {
                        assert!(r.is_err());
                    } else {
                        assert!(r.unwrap().is_err());
                    }
                    g.validate_for(&h).unwrap();
                }
            }
        }
        assert!(PreparedHierarchyGrouping::try_new(&h, 3).is_err());
        assert!(PreparedHierarchyGrouping::setup_payload_bound(&h, 1, usize::MAX).is_err());
        let terminal = PreparedHierarchyTopology::try_new(&topology, vec![]).unwrap();
        assert_eq!(
            PreparedHierarchyGrouping::try_new(&terminal, 0)
                .unwrap()
                .retained_payload_bytes()
                .unwrap(),
            0
        );
        assert!(PreparedHierarchyGrouping::try_new(&terminal, 1).is_err());
    }
    #[test]
    fn coarse_cursor_reuse_does_not_charge_all_cursors_as_simultaneously_live() {
        let tuples: Vec<_> = (0..2)
            .flat_map(|i| (0..128).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
            .collect();
        let t = PreparedThreeWayTopology::try_from_collapsed([2, 128, 2], &tuples).unwrap();
        let first = FactorAggregation::new(
            [2, 128, 2],
            [vec![0, 1], (0..128).map(|j| j % 2).collect(), vec![0, 1]],
        )
        .unwrap();
        let h = PreparedHierarchyTopology::try_new(
            &t,
            vec![
                first,
                FactorAggregation::consecutive_halving([2; 3]).unwrap(),
            ],
        )
        .unwrap();
        let base =
            base_bytes(&h, 17).unwrap() + array_bytes::<PreparedTupleGrouping<'_>>(2).unwrap();
        let expected = base
            + PreparedTupleGrouping::required_payload_bytes(&t).unwrap()
            + 128 * core::mem::size_of::<usize>();
        let peak = PreparedHierarchyGrouping::setup_payload_bound(&h, 2, 17).unwrap();
        assert_eq!(peak, expected);
        let groups = PreparedHierarchyGrouping::try_new_with_budget(
            &h,
            2,
            PreparedHierarchyBudget {
                maximum_payload_bytes: peak,
                additional_live_payload_bytes: 17,
            },
        )
        .unwrap();
        assert_eq!(groups.setup_peak_payload_bound(), peak);
    }
}
