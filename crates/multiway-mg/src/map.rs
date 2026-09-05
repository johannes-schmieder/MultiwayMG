//! Symmetric block Gauss--Seidel/MAP preconditioning.

use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

mod workspace;
pub use workspace::SymmetricMapWorkspace;

/// One symmetric factor sweep, equivalent to a block symmetric
/// Gauss--Seidel preconditioner for the three-way Gramian.
///
/// Forward and reverse triangular solves use the exact diagonal factor blocks.
/// Orthogonal structural-range projections are applied on both sides so the
/// exposed operator remains symmetric on the complete coefficient space.
#[derive(Debug, Clone)]
pub struct SymmetricMapPreconditioner {
    problem: ThreeWayProblem,
}

impl SymmetricMapPreconditioner {
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

impl SymmetricMapPreconditioner {
    /// Apply one symmetric MAP sweep using explicitly prepared caller scratch.
    ///
    /// Dimensions and projection binding are checked before mutation. No
    /// allocation occurs on a valid prepared call; output is transactional.
    pub fn apply_with_workspace(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut SymmetricMapWorkspace,
    ) -> Result<(), MultiwayError> {
        let dimension = self.dimension();
        if rhs.len() != dimension {
            return Err(crate::error::dimension(
                "SymmetricMapPreconditioner::apply rhs",
                dimension,
                rhs.len(),
            ));
        }
        if out.len() != dimension {
            return Err(crate::error::dimension(
                "SymmetricMapPreconditioner::apply output",
                dimension,
                out.len(),
            ));
        }

        workspace.validate(self)?;
        let SymmetricMapWorkspace {
            compatible_rhs,
            forward,
            middle,
            solution,
            projection,
        } = workspace;
        compatible_rhs.copy_from_slice(rhs);
        self.problem
            .components()
            .project_structural_range_with_workspace(compatible_rhs, projection)?;
        let topology = self.problem.topology();
        let offsets = topology.offsets();
        let diagonal = self.problem.diagonal();
        forward.fill(0.0);

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

        for ((middle, &value), &degree) in middle.iter_mut().zip(forward.iter()).zip(diagonal) {
            *middle = value * degree;
        }
        solution.fill(0.0);
        for factor in (0..3).rev() {
            let start = offsets[factor];
            let end = offsets[factor + 1];
            solution[start..end].copy_from_slice(&middle[start..end]);
            for (&tuple, &weight) in topology.tuples().iter().zip(self.problem.weights()) {
                let target = topology.global_index(factor, tuple[factor]);
                let mut coupling = 0.0;
                for following in (factor + 1)..3 {
                    coupling = solution[topology.global_index(following, tuple[following])]
                        .mul_add(weight, coupling);
                }
                solution[target] -= coupling;
            }
            for index in start..end {
                solution[index] /= diagonal[index];
            }
        }
        self.problem
            .components()
            .project_structural_range_with_workspace(solution, projection)?;
        out.copy_from_slice(solution);
        Ok(())
    }
}

impl Preconditioner for SymmetricMapPreconditioner {
    fn dimension(&self) -> usize {
        self.problem.dimension()
    }
    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        if rhs.len() != self.dimension() {
            return Err(crate::error::dimension(
                "SymmetricMapPreconditioner::apply rhs",
                self.dimension(),
                rhs.len(),
            ));
        }
        if out.len() != self.dimension() {
            return Err(crate::error::dimension(
                "SymmetricMapPreconditioner::apply output",
                self.dimension(),
                out.len(),
            ));
        }
        let mut workspace = self.application_workspace()?;
        self.apply_with_workspace(rhs, out, &mut workspace)
    }
}
