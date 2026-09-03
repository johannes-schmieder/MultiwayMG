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
            let coarse_count = maximum as usize + 1;
            let mut seen = vec![false; coarse_count];
            for &parent in &parents[factor] {
                let index = parent as usize;
                if index >= coarse_count {
                    return Err(IncidenceError::InvalidParent { factor, parent });
                }
                seen[index] = true;
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

    /// Apply transpose restriction `coarse = P^T fine`.
    pub fn restrict(&self, fine: &[f64], coarse: &mut [f64]) -> Result<(), IncidenceError> {
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
