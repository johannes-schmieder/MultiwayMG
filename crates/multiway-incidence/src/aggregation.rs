//! Factor-respecting piecewise-constant aggregation.

use std::collections::BTreeMap;

use crate::problem::CompensatedSum;
use crate::{IncidenceError, ThreeWayProblem};

/// One hard aggregation map per factor.
///
/// Each fine level maps to exactly one coarse level in the same factor. This
/// restriction preserves the three-way incidence class under Galerkin
/// coarsening: `G_c = P^T G P` is represented by mapped coarse tuples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactorAggregation {
    fine_counts: [usize; 3],
    coarse_counts: [usize; 3],
    parents: [Vec<u32>; 3],
}

impl FactorAggregation {
    /// Validate factor-local parent labels and construct an aggregation.
    pub fn new(fine_counts: [usize; 3], parents: [Vec<u32>; 3]) -> Result<Self, IncidenceError> {
        Self::try_new(fine_counts, parents)
    }

    /// Validate dense factor-local parent labels using fallible, bounded scratch.
    ///
    /// Valid maps use at most one byte per fine coefficient in the largest
    /// factor during validation. Invalid large labels cannot request a huge
    /// array: the first missing dense label is at most the fine factor count.
    /// Consumed parent arrays and partially validated state are dropped on error.
    pub fn try_new(
        fine_counts: [usize; 3],
        parents: [Vec<u32>; 3],
    ) -> Result<Self, IncidenceError> {
        Self::validate_with(fine_counts, parents, &mut |_| Ok(()))
    }

    fn validate_with<F>(
        fine_counts: [usize; 3],
        parents: [Vec<u32>; 3],
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let mut coarse_counts = [0; 3];
        for factor in 0..3 {
            if parents[factor].len() != fine_counts[factor] {
                return Err(IncidenceError::ParentLengthMismatch {
                    factor,
                    expected: fine_counts[factor],
                    actual: parents[factor].len(),
                });
            }
            let Some(maximum) = parents[factor].iter().copied().max() else {
                return Err(IncidenceError::EmptyFactor { factor });
            };
            let coarse_count =
                (maximum as usize)
                    .checked_add(1)
                    .ok_or(IncidenceError::DimensionOverflow {
                        context: "aggregation parent extent",
                    })?;
            let validation_count = coarse_count.min(fine_counts[factor].saturating_add(1));
            let mut seen = crate::construction::reserve(
                validation_count,
                "aggregation parent validation",
                before,
            )?;
            seen.resize(validation_count, false);
            for &parent in &parents[factor] {
                let index = parent as usize;
                if index >= coarse_count {
                    return Err(IncidenceError::InvalidParent { factor, parent });
                }
                if let Some(slot) = seen.get_mut(index) {
                    *slot = true;
                }
            }
            if let Some(parent) = seen.iter().position(|&value| !value) {
                return Err(IncidenceError::EmptyAggregate { factor, parent });
            }
            coarse_counts[factor] = coarse_count;
        }
        Ok(Self {
            fine_counts,
            coarse_counts,
            parents,
        })
    }

    /// Identity aggregation.
    pub fn identity(fine_counts: [usize; 3]) -> Result<Self, IncidenceError> {
        let parents = core::array::from_fn(|factor| {
            (0..fine_counts[factor]).map(|level| level as u32).collect()
        });
        Self::new(fine_counts, parents)
    }

    /// Deterministically merge consecutive pairs of levels in every factor.
    pub fn consecutive_halving(fine_counts: [usize; 3]) -> Result<Self, IncidenceError> {
        let parents = core::array::from_fn(|factor| {
            (0..fine_counts[factor])
                .map(|level| (level / 2) as u32)
                .collect()
        });
        Self::new(fine_counts, parents)
    }

    /// Fine level counts.
    #[must_use]
    pub const fn fine_counts(&self) -> [usize; 3] {
        self.fine_counts
    }

    /// Coarse level counts.
    #[must_use]
    pub const fn coarse_counts(&self) -> [usize; 3] {
        self.coarse_counts
    }

    /// Parent labels for one factor.
    #[must_use]
    pub fn parents(&self, factor: usize) -> &[u32] {
        &self.parents[factor]
    }

    /// Number of retained bytes in parent arrays.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.parents
            .iter()
            .map(|parents| parents.capacity() * core::mem::size_of::<u32>())
            .sum()
    }

    /// Apply piecewise-constant prolongation `fine = P coarse`.
    pub fn prolong(&self, coarse: &[f64], fine: &mut [f64]) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _profile_span = crate::profiling::span(crate::profiling::Phase::Prolongation);

        let fine_dimension: usize = self.fine_counts.iter().sum();
        let coarse_dimension: usize = self.coarse_counts.iter().sum();
        if coarse.len() != coarse_dimension {
            return Err(crate::error::dimension(
                "FactorAggregation::prolong coarse",
                coarse_dimension,
                coarse.len(),
            ));
        }
        if fine.len() != fine_dimension {
            return Err(crate::error::dimension(
                "FactorAggregation::prolong fine",
                fine_dimension,
                fine.len(),
            ));
        }
        let mut fine_offset = 0;
        let mut coarse_offset = 0;
        for factor in 0..3 {
            for level in 0..self.fine_counts[factor] {
                fine[fine_offset + level] =
                    coarse[coarse_offset + self.parents[factor][level] as usize];
            }
            fine_offset += self.fine_counts[factor];
            coarse_offset += self.coarse_counts[factor];
        }
        Ok(())
    }

    /// Add piecewise-constant prolongation directly: `fine += P coarse`.
    ///
    /// Checks both dimensions before any output write. Each coefficient uses the
    /// same addition as materializing `P coarse` and then adding it, without a
    /// temporary vector or a second fine-vector pass. This action does not check
    /// finiteness; complete solver boundaries retain their numerical checks.
    pub fn prolong_add(&self, coarse: &[f64], fine: &mut [f64]) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _profile_span = crate::profiling::span(crate::profiling::Phase::Prolongation);

        let fine_dimension: usize = self.fine_counts.iter().sum();
        let coarse_dimension: usize = self.coarse_counts.iter().sum();
        if coarse.len() != coarse_dimension {
            return Err(crate::error::dimension(
                "FactorAggregation::prolong_add coarse",
                coarse_dimension,
                coarse.len(),
            ));
        }
        if fine.len() != fine_dimension {
            return Err(crate::error::dimension(
                "FactorAggregation::prolong_add fine",
                fine_dimension,
                fine.len(),
            ));
        }
        let mut fine_offset = 0;
        let mut coarse_offset = 0;
        for factor in 0..3 {
            let end = fine_offset + self.fine_counts[factor];
            for (value, &parent) in fine[fine_offset..end].iter_mut().zip(&self.parents[factor]) {
                *value += coarse[coarse_offset + parent as usize];
            }
            fine_offset = end;
            coarse_offset += self.coarse_counts[factor];
        }
        Ok(())
    }

    /// Apply transpose restriction `coarse = P^T fine`.
    pub fn restrict(&self, fine: &[f64], coarse: &mut [f64]) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _profile_span = crate::profiling::span(crate::profiling::Phase::Restriction);

        let fine_dimension: usize = self.fine_counts.iter().sum();
        let coarse_dimension: usize = self.coarse_counts.iter().sum();
        if fine.len() != fine_dimension {
            return Err(crate::error::dimension(
                "FactorAggregation::restrict fine",
                fine_dimension,
                fine.len(),
            ));
        }
        if coarse.len() != coarse_dimension {
            return Err(crate::error::dimension(
                "FactorAggregation::restrict coarse",
                coarse_dimension,
                coarse.len(),
            ));
        }
        coarse.fill(0.0);
        let mut fine_offset = 0;
        let mut coarse_offset = 0;
        for factor in 0..3 {
            for level in 0..self.fine_counts[factor] {
                coarse[coarse_offset + self.parents[factor][level] as usize] +=
                    fine[fine_offset + level];
            }
            fine_offset += self.fine_counts[factor];
            coarse_offset += self.coarse_counts[factor];
        }
        Ok(())
    }

    /// Map and merge tuples to construct the exact Galerkin coarse problem.
    pub fn coarsen(&self, fine: &ThreeWayProblem) -> Result<ThreeWayProblem, IncidenceError> {
        if fine.topology().level_counts() != self.fine_counts {
            return Err(IncidenceError::DimensionMismatch {
                context: "FactorAggregation::coarsen level counts",
                expected: self.fine_counts.iter().sum(),
                actual: fine.dimension(),
            });
        }
        let mut collapsed: BTreeMap<[u32; 3], CompensatedSum> = BTreeMap::new();
        for (&tuple, &weight) in fine.topology().tuples().iter().zip(fine.weights()) {
            let mapped = [
                self.parents[0][tuple[0] as usize],
                self.parents[1][tuple[1] as usize],
                self.parents[2][tuple[2] as usize],
            ];
            collapsed.entry(mapped).or_default().add(weight);
        }
        let mut tuples = Vec::with_capacity(collapsed.len());
        let mut weights = Vec::with_capacity(collapsed.len());
        for (tuple, accumulator) in collapsed {
            tuples.push(tuple);
            weights.push(accumulator.total());
        }
        ThreeWayProblem::from_collapsed_parts(self.coarse_counts, tuples, weights)
    }
}

impl FactorAggregation {
    /// Checked parent-array payload, including unused capacities.
    ///
    /// Excludes the inline aggregation descriptor and allocator overhead.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        self.parents.iter().try_fold(0usize, |total, parents| {
            parents
                .capacity()
                .checked_mul(core::mem::size_of::<u32>())
                .and_then(|bytes| total.checked_add(bytes))
                .ok_or(IncidenceError::DimensionOverflow {
                    context: "aggregation payload",
                })
        })
    }
}

#[cfg(test)]
mod payload_tests {
    use super::*;
    #[test]
    fn unused_parent_capacity_is_charged() {
        let parents: [Vec<u32>; 3] = core::array::from_fn(|factor| {
            let mut values = Vec::with_capacity(8 + factor * 8);
            values.extend([0, 0]);
            values
        });
        let expected: usize = parents.iter().map(|p| p.capacity() * 4).sum();
        let map = FactorAggregation::new([2; 3], parents).unwrap();
        assert_eq!(map.retained_payload_bytes().unwrap(), expected);
        assert!(expected > 6 * core::mem::size_of::<u32>());
        assert_eq!(map.retained_payload_bytes().unwrap(), map.retained_bytes());
    }
}

#[cfg(test)]
mod fallible_validation_tests {
    use super::*;
    #[test]
    fn huge_sparse_parent_labels_reject_without_huge_validation_arrays() {
        for parents in [
            [vec![u32::MAX], vec![0], vec![0]],
            [vec![0, u32::MAX], vec![0], vec![0]],
        ] {
            let n = parents[0].len();
            let mut calls = 0;
            let error = FactorAggregation::validate_with([n, 1, 1], parents, &mut |_| {
                calls += 1;
                Ok(())
            })
            .unwrap_err();
            assert_eq!(
                error,
                IncidenceError::EmptyAggregate {
                    factor: 0,
                    parent: n - 1
                }
            );
            assert_eq!(calls, 1);
        }
        let relabel =
            FactorAggregation::try_new([4, 3, 2], [vec![1, 0, 1, 0], vec![0, 1, 0], vec![0, 0]])
                .unwrap();
        assert_eq!(relabel.coarse_counts(), [2, 2, 1]);
    }
    #[test]
    fn each_validation_reservation_failure_or_unwind_drops_unpublished_state() {
        for unwind in [false, true] {
            for fail_at in 0..3 {
                let mut calls = 0;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    FactorAggregation::validate_with(
                        [2; 3],
                        [vec![0, 1], vec![1, 0], vec![0, 0]],
                        &mut |context| {
                            let at = calls;
                            calls += 1;
                            if at == fail_at {
                                if unwind {
                                    panic!("injected parent validation unwind");
                                }
                                return Err(IncidenceError::TopologyAllocation { context });
                            }
                            Ok(())
                        },
                    )
                }));
                assert_eq!(calls, fail_at + 1);
                if unwind {
                    assert!(result.is_err());
                } else {
                    assert!(result.unwrap().is_err());
                }
                assert!(FactorAggregation::try_new([1; 3], [vec![0], vec![0], vec![0]]).is_ok());
            }
        }
    }
}
