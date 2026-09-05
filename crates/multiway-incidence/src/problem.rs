//! Weighted three-way problem construction and matrix-free kernels.

use std::{collections::BTreeMap, sync::Arc};

use crate::{IncidenceComponents, IncidenceError, ThreeWayTopology};

/// A collapsed weighted three-way incidence problem.
///
/// Cloning is intentionally cheap: immutable topology, numerical weights,
/// diagonal state, and component metadata are shared through reference-counted
/// storage. Distinct hierarchy levels still own distinct problem states.
#[derive(Debug, Clone, PartialEq)]
pub struct ThreeWayProblem {
    topology: Arc<ThreeWayTopology>,
    weights: Arc<[f64]>,
    square_root_weights: Arc<[f64]>,
    diagonal: Arc<[f64]>,
    components: Arc<IncidenceComponents>,
}

impl ThreeWayProblem {
    /// Validate observations, collapse duplicate tuples, and construct a problem.
    pub fn from_observations(
        level_counts: [usize; 3],
        tuples: &[[u32; 3]],
        weights: &[f64],
    ) -> Result<Self, IncidenceError> {
        if tuples.len() != weights.len() {
            return Err(IncidenceError::WeightLengthMismatch {
                tuples: tuples.len(),
                weights: weights.len(),
            });
        }
        if tuples.is_empty() {
            return Err(IncidenceError::EmptyProblem);
        }

        ThreeWayTopology::new(level_counts, tuples.to_vec())?;
        let mut collapsed: BTreeMap<[u32; 3], CompensatedSum> = BTreeMap::new();
        for (tuple_index, (&tuple, &weight)) in tuples.iter().zip(weights).enumerate() {
            if !weight.is_finite() || weight <= 0.0 {
                return Err(IncidenceError::InvalidWeight {
                    tuple_index,
                    weight,
                });
            }
            collapsed.entry(tuple).or_default().add(weight);
        }

        let mut unique_tuples = Vec::with_capacity(collapsed.len());
        let mut unique_weights = Vec::with_capacity(collapsed.len());
        for (tuple, accumulator) in collapsed {
            let weight = accumulator.total();
            if !weight.is_finite() || weight <= 0.0 {
                return Err(IncidenceError::InvalidCollapsedWeight { tuple, weight });
            }
            unique_tuples.push(tuple);
            unique_weights.push(weight);
        }
        Self::from_collapsed_parts(level_counts, unique_tuples, unique_weights)
    }

    pub(crate) fn from_collapsed_parts(
        level_counts: [usize; 3],
        tuples: Vec<[u32; 3]>,
        weights: Vec<f64>,
    ) -> Result<Self, IncidenceError> {
        if tuples.is_empty() {
            return Err(IncidenceError::EmptyProblem);
        }
        if tuples.len() != weights.len() {
            return Err(IncidenceError::WeightLengthMismatch {
                tuples: tuples.len(),
                weights: weights.len(),
            });
        }
        let topology = Arc::new(ThreeWayTopology::new(level_counts, tuples)?);
        let mut square_root_weights = Vec::with_capacity(weights.len());
        for (tuple_index, &weight) in weights.iter().enumerate() {
            if !weight.is_finite() || weight <= 0.0 {
                return Err(IncidenceError::InvalidWeight {
                    tuple_index,
                    weight,
                });
            }
            square_root_weights.push(weight.sqrt());
        }

        let mut diagonal = vec![0.0; topology.total_levels()];
        let mut diagonal_correction = vec![0.0; topology.total_levels()];
        for (&tuple, &weight) in topology.tuples().iter().zip(&weights) {
            for factor in 0..3 {
                let index = topology.global_index(factor, tuple[factor]);
                neumaier_add(
                    &mut diagonal[index],
                    &mut diagonal_correction[index],
                    weight,
                );
            }
        }
        for (value, correction) in diagonal.iter_mut().zip(diagonal_correction) {
            *value += correction;
        }
        for factor in 0..3 {
            for (level, &value) in diagonal[topology.factor_range(factor)].iter().enumerate() {
                if value == 0.0 {
                    return Err(IncidenceError::UnusedLevel { factor, level });
                }
            }
        }

        let components = Arc::new(IncidenceComponents::from_topology(&topology));
        Ok(Self {
            topology,
            weights: Arc::from(weights),
            square_root_weights: Arc::from(square_root_weights),
            diagonal: Arc::from(diagonal),
            components,
        })
    }

    /// Immutable tuple topology.
    #[must_use]
    pub fn topology(&self) -> &ThreeWayTopology {
        self.topology.as_ref()
    }

    /// Positive collapsed tuple weights.
    #[must_use]
    pub fn weights(&self) -> &[f64] {
        self.weights.as_ref()
    }

    /// Square roots of the collapsed tuple weights.
    #[must_use]
    pub fn square_root_weights(&self) -> &[f64] {
        self.square_root_weights.as_ref()
    }

    /// Diagonal of `B^T W B` in global factor-block order.
    #[must_use]
    pub fn diagonal(&self) -> &[f64] {
        self.diagonal.as_ref()
    }

    /// Connected incidence components.
    #[must_use]
    pub fn components(&self) -> &IncidenceComponents {
        self.components.as_ref()
    }

    /// Number of coefficient coordinates.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.topology.total_levels()
    }

    /// Number of unique collapsed tuples.
    #[must_use]
    pub fn tuple_count(&self) -> usize {
        self.topology.tuple_count()
    }

    /// Compute `out = B x`.
    pub fn apply_incidence(&self, x: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        validate_len(
            "ThreeWayProblem::apply_incidence input",
            self.dimension(),
            x.len(),
        )?;
        validate_len(
            "ThreeWayProblem::apply_incidence output",
            self.tuple_count(),
            out.len(),
        )?;
        for (value, tuple) in out.iter_mut().zip(self.topology.tuples()) {
            *value = x[self.topology.global_index(0, tuple[0])]
                + x[self.topology.global_index(1, tuple[1])]
                + x[self.topology.global_index(2, tuple[2])];
        }
        Ok(())
    }

    /// Compute `out = B^T y`.
    pub fn apply_adjoint(&self, y: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        validate_len(
            "ThreeWayProblem::apply_adjoint input",
            self.tuple_count(),
            y.len(),
        )?;
        validate_len(
            "ThreeWayProblem::apply_adjoint output",
            self.dimension(),
            out.len(),
        )?;
        out.fill(0.0);
        for (tuple, &value) in self.topology.tuples().iter().zip(y) {
            for factor in 0..3 {
                out[self.topology.global_index(factor, tuple[factor])] += value;
            }
        }
        Ok(())
    }

    /// Compute `out = sqrt(W) B x`.
    pub fn apply_weighted_incidence(
        &self,
        x: &[f64],
        out: &mut [f64],
    ) -> Result<(), IncidenceError> {
        self.apply_incidence(x, out)?;
        for (value, &sqrt_weight) in out.iter_mut().zip(self.square_root_weights.iter()) {
            *value *= sqrt_weight;
        }
        Ok(())
    }

    /// Compute `out = B^T sqrt(W) y`.
    pub fn apply_weighted_adjoint(&self, y: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        validate_len(
            "ThreeWayProblem::apply_weighted_adjoint input",
            self.tuple_count(),
            y.len(),
        )?;
        validate_len(
            "ThreeWayProblem::apply_weighted_adjoint output",
            self.dimension(),
            out.len(),
        )?;
        out.fill(0.0);
        for ((tuple, &value), &sqrt_weight) in self
            .topology
            .tuples()
            .iter()
            .zip(y)
            .zip(self.square_root_weights.iter())
        {
            let contribution = sqrt_weight * value;
            for factor in 0..3 {
                out[self.topology.global_index(factor, tuple[factor])] += contribution;
            }
        }
        Ok(())
    }

    /// Compute `out = G x`, where `G = B^T W B`.
    pub fn apply_gramian(&self, x: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        validate_len(
            "ThreeWayProblem::apply_gramian input",
            self.dimension(),
            x.len(),
        )?;
        validate_len(
            "ThreeWayProblem::apply_gramian output",
            self.dimension(),
            out.len(),
        )?;
        out.fill(0.0);
        for (&tuple, &weight) in self.topology.tuples().iter().zip(self.weights.iter()) {
            let indices = [
                self.topology.global_index(0, tuple[0]),
                self.topology.global_index(1, tuple[1]),
                self.topology.global_index(2, tuple[2]),
            ];
            let value = weight * (x[indices[0]] + x[indices[1]] + x[indices[2]]);
            for index in indices {
                out[index] += value;
            }
        }
        Ok(())
    }

    /// Compute the quadratic energy `x^T G x` from tuple contributions.
    pub fn energy(&self, x: &[f64]) -> Result<f64, IncidenceError> {
        validate_len("ThreeWayProblem::energy", self.dimension(), x.len())?;
        let mut sum = 0.0;
        let mut correction = 0.0;
        for (&tuple, &weight) in self.topology.tuples().iter().zip(self.weights.iter()) {
            let value = x[self.topology.global_index(0, tuple[0])]
                + x[self.topology.global_index(1, tuple[1])]
                + x[self.topology.global_index(2, tuple[2])];
            neumaier_add(&mut sum, &mut correction, weight * value * value);
        }
        Ok(sum + correction)
    }

    /// Form `rhs = B^T W targets` in caller-owned storage.
    ///
    /// Dimensions are checked before `rhs` is modified.
    pub fn rhs_from_targets_into(
        &self,
        targets: &[f64],
        rhs: &mut [f64],
    ) -> Result<(), IncidenceError> {
        validate_len(
            "ThreeWayProblem::rhs_from_targets_into targets",
            self.tuple_count(),
            targets.len(),
        )?;
        validate_len(
            "ThreeWayProblem::rhs_from_targets_into rhs",
            self.dimension(),
            rhs.len(),
        )?;
        rhs.fill(0.0);
        for ((&tuple, &weight), &target) in self
            .topology
            .tuples()
            .iter()
            .zip(self.weights.iter())
            .zip(targets)
        {
            let value = weight * target;
            for factor in 0..3 {
                rhs[self.topology.global_index(factor, tuple[factor])] += value;
            }
        }
        Ok(())
    }

    /// Form the normal-equation right-hand side `B^T W targets`.
    pub fn rhs_from_targets(&self, targets: &[f64]) -> Result<Vec<f64>, IncidenceError> {
        let mut rhs = vec![0.0; self.dimension()];
        self.rhs_from_targets_into(targets, &mut rhs)?;
        Ok(rhs)
    }

    /// Compute `out = rhs - G x` in caller-owned storage.
    ///
    /// Dimensions are checked before `out` is modified.
    pub fn residual_into(
        &self,
        rhs: &[f64],
        x: &[f64],
        out: &mut [f64],
    ) -> Result<(), IncidenceError> {
        validate_len(
            "ThreeWayProblem::residual_into rhs",
            self.dimension(),
            rhs.len(),
        )?;
        validate_len(
            "ThreeWayProblem::residual_into x",
            self.dimension(),
            x.len(),
        )?;
        validate_len(
            "ThreeWayProblem::residual_into output",
            self.dimension(),
            out.len(),
        )?;
        self.apply_gramian(x, out)?;
        for (value, &right) in out.iter_mut().zip(rhs) {
            *value = right - *value;
        }
        Ok(())
    }

    /// Compute `rhs - G x` into a newly allocated vector.
    pub fn residual(&self, rhs: &[f64], x: &[f64]) -> Result<Vec<f64>, IncidenceError> {
        let mut residual = vec![0.0; self.dimension()];
        self.residual_into(rhs, x, &mut residual)?;
        Ok(residual)
    }

    /// Materialize the dense Gramian for reference tests and small terminals.
    pub fn dense_gramian(&self) -> Vec<Vec<f64>> {
        let mut matrix = vec![vec![0.0; self.dimension()]; self.dimension()];
        for (&tuple, &weight) in self.topology.tuples().iter().zip(self.weights.iter()) {
            let indices = [
                self.topology.global_index(0, tuple[0]),
                self.topology.global_index(1, tuple[1]),
                self.topology.global_index(2, tuple[2]),
            ];
            for &row in &indices {
                for &column in &indices {
                    matrix[row][column] += weight;
                }
            }
        }
        matrix
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    pub(crate) fn add(&mut self, value: f64) {
        neumaier_add(&mut self.sum, &mut self.correction, value);
    }

    pub(crate) const fn total(self) -> f64 {
        self.sum + self.correction
    }
}

fn validate_len(
    context: &'static str,
    expected: usize,
    actual: usize,
) -> Result<(), IncidenceError> {
    if expected != actual {
        return Err(crate::error::dimension(context, expected, actual));
    }
    Ok(())
}

fn neumaier_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let updated = *sum + value;
    if sum.abs() >= value.abs() {
        *correction += (*sum - updated) + value;
    } else {
        *correction += (value - updated) + *sum;
    }
    *sum = updated;
}

impl ThreeWayProblem {
    /// Payload reachable through the five shared problem allocations, counted once.
    ///
    /// Counts topology/component objects behind Arc, their vector capacities,
    /// and the three numerical Arc slices. Excludes the inline problem handle,
    /// Arc headers/alignment padding, identity headers and allocator overhead.
    /// Ordinary problem clones share this payload; do not sum their reports.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        let parts = [
            core::mem::size_of::<ThreeWayTopology>(),
            self.topology.retained_payload_bytes()?,
            core::mem::size_of::<IncidenceComponents>(),
            self.components.retained_payload_bytes()?,
            core::mem::size_of_val(self.weights.as_ref()),
            core::mem::size_of_val(self.square_root_weights.as_ref()),
            core::mem::size_of_val(self.diagonal.as_ref()),
        ];
        parts.into_iter().try_fold(0usize, |total, bytes| {
            total
                .checked_add(bytes)
                .ok_or(IncidenceError::DimensionOverflow {
                    context: "shared problem payload",
                })
        })
    }

    /// Whether all five immutable backing allocations are shared with `other`.
    ///
    /// This is storage identity, not value equality or authorization to reuse
    /// numerical state under changed weights. Independent equal builds return false.
    #[must_use]
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.topology, &other.topology)
            && Arc::ptr_eq(&self.weights, &other.weights)
            && Arc::ptr_eq(&self.square_root_weights, &other.square_root_weights)
            && Arc::ptr_eq(&self.diagonal, &other.diagonal)
            && Arc::ptr_eq(&self.components, &other.components)
    }
}
