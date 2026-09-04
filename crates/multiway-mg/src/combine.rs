//! Fixed weighted sums of two linear preconditioners.

use crate::{MultiwayError, Preconditioner};

/// Fixed weighted sum `left_weight * L + right_weight * R`.
#[derive(Debug, Clone)]
pub struct WeightedSumPreconditioner<L, R> {
    left: L,
    right: R,
    left_weight: f64,
    right_weight: f64,
    dimension: usize,
}

impl<L: Preconditioner, R: Preconditioner> WeightedSumPreconditioner<L, R> {
    /// Validate and construct a fixed linear sum.
    pub fn new(
        left: L,
        left_weight: f64,
        right: R,
        right_weight: f64,
    ) -> Result<Self, MultiwayError> {
        if left.dimension() != right.dimension() {
            return Err(crate::error::dimension(
                "WeightedSumPreconditioner::new",
                left.dimension(),
                right.dimension(),
            ));
        }
        for (name, weight) in [
            ("left_preconditioner_weight", left_weight),
            ("right_preconditioner_weight", right_weight),
        ] {
            if !weight.is_finite() || weight < 0.0 {
                return Err(MultiwayError::InvalidOption {
                    name,
                    message: format!("must be finite and nonnegative, got {weight}"),
                });
            }
        }
        if left_weight == 0.0 && right_weight == 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "preconditioner_weights",
                message: "at least one weight must be positive".to_owned(),
            });
        }
        let dimension = left.dimension();
        Ok(Self {
            left,
            right,
            left_weight,
            right_weight,
            dimension,
        })
    }

    /// Left preconditioner.
    #[must_use]
    pub const fn left(&self) -> &L {
        &self.left
    }

    /// Right preconditioner.
    #[must_use]
    pub const fn right(&self) -> &R {
        &self.right
    }

    /// Left scalar weight.
    #[must_use]
    pub const fn left_weight(&self) -> f64 {
        self.left_weight
    }

    /// Right scalar weight.
    #[must_use]
    pub const fn right_weight(&self) -> f64 {
        self.right_weight
    }
}

impl<L: Preconditioner, R: Preconditioner> Preconditioner for WeightedSumPreconditioner<L, R> {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        if rhs.len() != self.dimension {
            return Err(crate::error::dimension(
                "WeightedSumPreconditioner::apply rhs",
                self.dimension,
                rhs.len(),
            ));
        }
        if out.len() != self.dimension {
            return Err(crate::error::dimension(
                "WeightedSumPreconditioner::apply output",
                self.dimension,
                out.len(),
            ));
        }
        let mut right_output = vec![0.0; self.dimension];
        self.left.apply(rhs, out)?;
        self.right.apply(rhs, &mut right_output)?;
        for (value, &right) in out.iter_mut().zip(&right_output) {
            *value = self.left_weight.mul_add(*value, self.right_weight * right);
        }
        Ok(())
    }
}
