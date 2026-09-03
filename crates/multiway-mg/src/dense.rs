//! Rank-revealing dense terminal pseudoinverse.

use nalgebra::{DMatrix, linalg::SymmetricEigen};

use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

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
        let decomposition = SymmetricEigen::new(matrix);
        let spectral_scale = decomposition
            .eigenvalues
            .iter()
            .copied()
            .map(f64::abs)
            .fold(0.0, f64::max);
        let threshold = relative_tolerance * spectral_scale.max(1.0);
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
                inverse_eigenvalues.push(1.0 / value);
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
        let mut modal = vec![0.0; dimension];
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
