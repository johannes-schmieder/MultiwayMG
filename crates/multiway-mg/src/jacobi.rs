//! Diagonal weighted-Jacobi preconditioner.

use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

/// Damped inverse diagonal of a three-way incidence Gramian.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagonalPreconditioner {
    scaled_inverse_diagonal: Vec<f64>,
    omega: f64,
}

impl DiagonalPreconditioner {
    /// Build a weighted-Jacobi correction.
    ///
    /// The three-way bound `G <= 3 D` makes `0 < omega < 2/3` a conservative
    /// stable range for stationary Jacobi smoothing on the positive subspace.
    pub fn new(problem: &ThreeWayProblem, omega: f64) -> Result<Self, MultiwayError> {
        if !omega.is_finite() || !(0.0..(2.0 / 3.0)).contains(&omega) {
            return Err(MultiwayError::InvalidOption {
                name: "jacobi_omega",
                message: format!("must lie in (0, 2/3), got {omega}"),
            });
        }
        let scaled_inverse_diagonal = problem
            .diagonal()
            .iter()
            .map(|&value| omega / value)
            .collect();
        Ok(Self {
            scaled_inverse_diagonal,
            omega,
        })
    }

    /// Damping factor.
    #[must_use]
    pub const fn omega(&self) -> f64 {
        self.omega
    }
}

impl Preconditioner for DiagonalPreconditioner {
    fn dimension(&self) -> usize {
        self.scaled_inverse_diagonal.len()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        let dimension = self.dimension();
        if rhs.len() != dimension {
            return Err(crate::error::dimension(
                "DiagonalPreconditioner::apply rhs",
                dimension,
                rhs.len(),
            ));
        }
        if out.len() != dimension {
            return Err(crate::error::dimension(
                "DiagonalPreconditioner::apply output",
                dimension,
                out.len(),
            ));
        }
        for ((value, &right), &inverse) in
            out.iter_mut().zip(rhs).zip(&self.scaled_inverse_diagonal)
        {
            *value = inverse * right;
        }
        Ok(())
    }
}
