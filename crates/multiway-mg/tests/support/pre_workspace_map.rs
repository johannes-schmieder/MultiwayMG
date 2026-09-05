// Test-only allocating reference from e60809b; do not use in production.
//! Symmetric block Gauss--Seidel/MAP preconditioning.

use multiway_mg::{MultiwayError, Preconditioner, ThreeWayProblem};

/// One symmetric factor sweep, equivalent to a block symmetric
/// Gauss--Seidel preconditioner for the three-way Gramian.
///
/// Forward and reverse triangular solves use the exact diagonal factor blocks.
/// Orthogonal structural-range projections are applied on both sides so the
/// exposed operator remains symmetric on the complete coefficient space.
#[derive(Debug, Clone)]
pub struct AllocatingMapReference {
    problem: ThreeWayProblem,
}

impl AllocatingMapReference {
    /// Build one fixed symmetric MAP correction.
    #[must_use]
    pub const fn new(problem: ThreeWayProblem) -> Self {
        Self { problem }
    }

    /// Underlying weighted problem.
    #[must_use]
    pub const fn problem(&self) -> &ThreeWayProblem {
        &self.problem
    }
}

impl Preconditioner for AllocatingMapReference {
    fn dimension(&self) -> usize {
        self.problem.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        let dimension = self.dimension();
        if rhs.len() != dimension {
            return Err(dimension(
                "AllocatingMapReference::apply rhs",
                dimension,
                rhs.len(),
            ));
        }
        if out.len() != dimension {
            return Err(dimension(
                "AllocatingMapReference::apply output",
                dimension,
                out.len(),
            ));
        }

        let mut compatible_rhs = rhs.to_vec();
        self.problem
            .components()
            .project_structural_range(&mut compatible_rhs)?;
        let topology = self.problem.topology();
        let offsets = topology.offsets();
        let diagonal = self.problem.diagonal();
        let mut forward = vec![0.0; dimension];

        for factor in 0..3 {
            let start = offsets[factor];
            let end = offsets[factor + 1];
            forward[start..end].copy_from_slice(&compatible_rhs[start..end]);
            for (&tuple, &weight) in topology.tuples().iter().zip(self.problem.weights()) {
                let target = topology.global_index(factor, tuple[factor]);
                let mut coupling = 0.0;
                for previous in 0..factor {
                    coupling = forward[topology.global_index(previous, tuple[previous])]
                        .mul_add(weight, coupling);
                }
                forward[target] -= coupling;
            }
            for index in start..end {
                forward[index] /= diagonal[index];
            }
        }

        let middle: Vec<f64> = forward
            .iter()
            .zip(diagonal)
            .map(|(&value, &degree)| value * degree)
            .collect();
        out.fill(0.0);
        for factor in (0..3).rev() {
            let start = offsets[factor];
            let end = offsets[factor + 1];
            out[start..end].copy_from_slice(&middle[start..end]);
            for (&tuple, &weight) in topology.tuples().iter().zip(self.problem.weights()) {
                let target = topology.global_index(factor, tuple[factor]);
                let mut coupling = 0.0;
                for following in (factor + 1)..3 {
                    coupling = out[topology.global_index(following, tuple[following])]
                        .mul_add(weight, coupling);
                }
                out[target] -= coupling;
            }
            for index in start..end {
                out[index] /= diagonal[index];
            }
        }
        self.problem.components().project_structural_range(out)?;
        Ok(())
    }
}

fn dimension(context: &'static str, expected: usize, actual: usize) -> MultiwayError {
    MultiwayError::DimensionMismatch {
        context,
        expected,
        actual,
    }
}
