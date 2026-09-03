//! Shared preconditioner trait.

use crate::MultiwayError;

/// A fixed linear approximate inverse on coefficient space.
///
/// Implementations used inside ordinary PCG must additionally be symmetric and
/// positive on the projected operator range.
pub trait Preconditioner: Send + Sync {
    /// Coefficient-space dimension.
    fn dimension(&self) -> usize;

    /// Compute `out = M^{-1} rhs` without changing solver state.
    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError>;
}
