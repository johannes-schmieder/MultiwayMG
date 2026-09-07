//! Shared serial arithmetic for owned problems and immutable frame views.
//!
//! Constructor-validated arrays; public entry points here check dimensions
//! before writes. Keep tuple order and arithmetic association unchanged.
use crate::problem::neumaier_add;
use crate::{IncidenceError, ThreeWayTopology};

pub(crate) struct OperatorData<'a> {
    pub(crate) topology: &'a ThreeWayTopology,
    pub(crate) weights: &'a [f64],
    pub(crate) square_root_weights: &'a [f64],
}

impl OperatorData<'_> {
    pub(crate) fn dimension(&self) -> usize {
        self.topology.total_levels()
    }
    pub(crate) fn tuple_count(&self) -> usize {
        self.topology.tuple_count()
    }
    /// Compute `out = B x`.
    pub(crate) fn apply_incidence(&self, x: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _profile_span = crate::profiling::span(crate::profiling::Phase::Incidence);

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
    pub(crate) fn apply_adjoint(&self, y: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _profile_span = crate::profiling::span(crate::profiling::Phase::Adjoint);

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
    pub(crate) fn apply_weighted_incidence(
        &self,
        x: &[f64],
        out: &mut [f64],
    ) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _profile_span = crate::profiling::span(crate::profiling::Phase::WeightedIncidence);

        self.apply_incidence(x, out)?;
        for (value, &sqrt_weight) in out.iter_mut().zip(self.square_root_weights.iter()) {
            *value *= sqrt_weight;
        }
        Ok(())
    }

    /// Compute `out = B^T sqrt(W) y`.
    pub(crate) fn apply_weighted_adjoint(
        &self,
        y: &[f64],
        out: &mut [f64],
    ) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _profile_span = crate::profiling::span(crate::profiling::Phase::WeightedAdjoint);

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
    pub(crate) fn apply_gramian(&self, x: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _profile_span = crate::profiling::span(crate::profiling::Phase::Gramian);

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
    pub(crate) fn energy(&self, x: &[f64]) -> Result<f64, IncidenceError> {
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
    pub(crate) fn rhs_from_targets_into(
        &self,
        targets: &[f64],
        rhs: &mut [f64],
    ) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _profile_span = crate::profiling::span(crate::profiling::Phase::Rhs);

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
    pub(crate) fn rhs_from_targets(&self, targets: &[f64]) -> Result<Vec<f64>, IncidenceError> {
        let mut rhs = vec![0.0; self.dimension()];
        self.rhs_from_targets_into(targets, &mut rhs)?;
        Ok(rhs)
    }

    /// Compute `out = rhs - G x` in caller-owned storage.
    ///
    /// Dimensions are checked before `out` is modified.
    pub(crate) fn residual_into(
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
    pub(crate) fn residual(&self, rhs: &[f64], x: &[f64]) -> Result<Vec<f64>, IncidenceError> {
        let mut residual = vec![0.0; self.dimension()];
        self.residual_into(rhs, x, &mut residual)?;
        Ok(residual)
    }

    /// Materialize the dense Gramian for reference tests and small terminals.
    pub(crate) fn dense_gramian(&self) -> Vec<Vec<f64>> {
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

pub(crate) fn validate_len(
    context: &'static str,
    expected: usize,
    actual: usize,
) -> Result<(), IncidenceError> {
    if expected != actual {
        return Err(crate::error::dimension(context, expected, actual));
    }
    Ok(())
}
