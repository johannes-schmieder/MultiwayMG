//! Rank-revealing dense terminal pseudoinverse.

use nalgebra::{DMatrix, linalg::SymmetricEigen};

use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

mod workspace;
pub use workspace::DensePseudoinverseWorkspace;

/// Dense spectral pseudoinverse used by small hierarchy terminals and references.
#[derive(Debug, Clone)]
pub struct DensePseudoinverse {
    eigenvectors: DMatrix<f64>,
    inverse_eigenvalues: Vec<f64>,
    rank: usize,
    threshold: f64,
}

impl DensePseudoinverse {
    /// Build a pseudoinverse with a relative eigenvalue threshold.
    pub fn from_problem(
        problem: &ThreeWayProblem,
        relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "terminal_relative_tolerance",
                message: format!("must be finite and positive, got {relative_tolerance}"),
            });
        }
        let dense = problem.dense_gramian();
        let dimension = problem.dimension();
        let flat: Vec<f64> = dense.into_iter().flatten().collect();
        let matrix = DMatrix::from_row_slice(dimension, dimension, &flat);
        Self::from_matrix(matrix, relative_tolerance)
    }

    pub(crate) fn from_frame(
        frame: &multiway_incidence::ThreeWayWeightFrame<'_>,
        relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "terminal_relative_tolerance",
                message: format!("must be finite and positive, got {relative_tolerance}"),
            });
        }
        let topology = frame.topology().topology();
        let n = topology.total_levels();
        let length = n
            .checked_mul(n)
            .filter(|&n| n <= isize::MAX as usize / 8)
            .ok_or(MultiwayError::WorkspaceSizeOverflow {
                context: "prepared dense terminal matrix",
            })?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(length)
            .map_err(|source| MultiwayError::WorkspaceAllocation {
                context: "prepared dense terminal matrix",
                source,
            })?;
        values.resize(length, 0.0);
        // Native column-major assembly follows the ordinary canonical tuple and
        // row/column accumulation order, without row-vector/flattened copies.
        for (&tuple, &weight) in topology.tuples().iter().zip(frame.weights()) {
            let indices = [
                topology.global_index(0, tuple[0]),
                topology.global_index(1, tuple[1]),
                topology.global_index(2, tuple[2]),
            ];
            for &row in &indices {
                for &column in &indices {
                    values[column * n + row] += weight;
                }
            }
        }
        Self::from_matrix(DMatrix::from_vec(n, n, values), relative_tolerance)
    }

    fn from_matrix(matrix: DMatrix<f64>, relative_tolerance: f64) -> Result<Self, MultiwayError> {
        let dimension = matrix.nrows();
        if !matrix.iter().all(|value| value.is_finite()) {
            return Err(MultiwayError::NumericalFailure {
                context: "dense terminal matrix",
            });
        }
        // Bound failure on numerically unrepresentable inputs. This is a setup
        // guard, not an RHS-dependent inner solver or a change in rank policy.
        let decomposition = SymmetricEigen::try_new(matrix, f64::EPSILON, 10_000).ok_or(
            MultiwayError::NumericalFailure {
                context: "dense terminal eigendecomposition",
            },
        )?;
        if !decomposition
            .eigenvalues
            .iter()
            .all(|value| value.is_finite())
            || !decomposition
                .eigenvectors
                .iter()
                .all(|value| value.is_finite())
        {
            return Err(MultiwayError::NumericalFailure {
                context: "dense terminal eigendecomposition",
            });
        }
        let spectral_scale = decomposition
            .eigenvalues
            .iter()
            .copied()
            .map(f64::abs)
            .fold(0.0, f64::max);
        let threshold = relative_tolerance * spectral_scale;
        if !spectral_scale.is_finite()
            || spectral_scale <= 0.0
            || !threshold.is_finite()
            || threshold <= 0.0
        {
            return Err(MultiwayError::NumericalFailure {
                context: "dense terminal spectral threshold",
            });
        }
        let mut inverse_eigenvalues = Vec::with_capacity(dimension);
        let mut rank = 0;
        for &value in decomposition.eigenvalues.iter() {
            if value < -threshold {
                return Err(MultiwayError::NegativeEigenvalue {
                    value,
                    tolerance: threshold,
                });
            }
            if value > threshold {
                let inverse = 1.0 / value;
                if !inverse.is_finite() || inverse <= 0.0 {
                    return Err(MultiwayError::NumericalFailure {
                        context: "dense terminal inverse eigenvalue",
                    });
                }
                inverse_eigenvalues.push(inverse);
                rank += 1;
            } else {
                inverse_eigenvalues.push(0.0);
            }
        }
        Ok(Self {
            eigenvectors: decomposition.eigenvectors,
            inverse_eigenvalues,
            rank,
            threshold,
        })
    }

    /// Numerical rank retained by the pseudoinverse.
    #[must_use]
    pub const fn rank(&self) -> usize {
        self.rank
    }

    /// Absolute eigenvalue threshold used by the factorization.
    #[must_use]
    pub const fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Matrix dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.inverse_eigenvalues.len()
    }

    /// Apply the spectral pseudoinverse.
    pub fn solve_into(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        if rhs.len() != self.dimension() {
            return Err(crate::error::dimension(
                "DensePseudoinverse::solve_into rhs",
                self.dimension(),
                rhs.len(),
            ));
        }
        if out.len() != self.dimension() {
            return Err(crate::error::dimension(
                "DensePseudoinverse::solve_into output",
                self.dimension(),
                out.len(),
            ));
        }
        let mut workspace = self.application_workspace()?;
        self.solve_into_with_workspace(rhs, out, &mut workspace)
    }

    /// Apply the spectral pseudoinverse without allocating prepared scratch.
    ///
    /// All vector lengths are validated before output or scratch is changed.
    pub fn solve_into_with_workspace(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut DensePseudoinverseWorkspace,
    ) -> Result<(), MultiwayError> {
        let dimension = self.dimension();
        if rhs.len() != dimension {
            return Err(crate::error::dimension(
                "DensePseudoinverse::solve_into rhs",
                dimension,
                rhs.len(),
            ));
        }
        if out.len() != dimension {
            return Err(crate::error::dimension(
                "DensePseudoinverse::solve_into output",
                dimension,
                out.len(),
            ));
        }
        if workspace.modal.len() != dimension {
            return Err(crate::error::dimension(
                "DensePseudoinverseWorkspace",
                dimension,
                workspace.modal.len(),
            ));
        }
        let modal = &mut workspace.modal;
        for (mode, modal_value) in modal.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (row, &right) in rhs.iter().enumerate() {
                sum = self.eigenvectors[(row, mode)].mul_add(right, sum);
            }
            *modal_value = sum * self.inverse_eigenvalues[mode];
        }
        for (row, value) in out.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (mode, &modal_value) in modal.iter().enumerate() {
                sum = self.eigenvectors[(row, mode)].mul_add(modal_value, sum);
            }
            *value = sum;
        }
        Ok(())
    }
}

impl Preconditioner for DensePseudoinverse {
    fn dimension(&self) -> usize {
        self.inverse_eigenvalues.len()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.solve_into(rhs, out)
    }
}

impl DensePseudoinverse {
    /// Exclusive retained matrix and inverse-eigenvalue payload, by capacity.
    ///
    /// Excludes the inline descriptor, temporary factorization storage and
    /// allocator overhead. This does not estimate factorization peak memory.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        self.eigenvectors
            .data
            .as_vec()
            .capacity()
            .checked_add(self.inverse_eigenvalues.capacity())
            .and_then(|values| values.checked_mul(core::mem::size_of::<f64>()))
            .ok_or(MultiwayError::WorkspaceSizeOverflow {
                context: "dense terminal payload",
            })
    }

    pub(crate) fn workspace_is_prepared(&self, workspace: &DensePseudoinverseWorkspace) -> bool {
        workspace.modal.len() == self.dimension()
    }
}
