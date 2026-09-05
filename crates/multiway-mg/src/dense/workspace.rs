//! Anonymous modal scratch for rank-revealing terminal applications.

use super::DensePseudoinverse;
use crate::MultiwayError;

/// Caller-owned modal vector, with no terminal factors or numerical binding.
#[derive(Debug, Default)]
pub struct DensePseudoinverseWorkspace {
    pub(super) modal: Vec<f64>,
}

impl DensePseudoinverseWorkspace {
    /// Construct empty scratch; prepare it before applying a terminal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Explicitly resize scratch for a terminal, retaining available capacity.
    ///
    /// Independent terminals with the same dimension can share this anonymous
    /// workspace without preparation. Every modal entry is overwritten on apply.
    pub fn try_prepare_for(&mut self, terminal: &DensePseudoinverse) -> Result<(), MultiwayError> {
        resize(&mut self.modal, terminal.dimension())
    }

    /// Exclusive retained modal-vector payload; excludes all inline/shared state.
    pub fn retained_bytes(&self) -> Result<usize, MultiwayError> {
        bytes(self.modal.capacity())
    }
}

impl DensePseudoinverse {
    /// Required modal scratch payload, excluding the workspace's inline descriptor.
    pub fn workspace_required_bytes(&self) -> Result<usize, MultiwayError> {
        bytes(self.dimension())
    }

    /// Allocate reusable terminal application scratch at a fallible boundary.
    pub fn application_workspace(&self) -> Result<DensePseudoinverseWorkspace, MultiwayError> {
        let mut workspace = DensePseudoinverseWorkspace::new();
        workspace.try_prepare_for(self)?;
        Ok(workspace)
    }
}

fn bytes(dimension: usize) -> Result<usize, MultiwayError> {
    dimension
        .checked_mul(core::mem::size_of::<f64>())
        .ok_or(MultiwayError::WorkspaceSizeOverflow {
            context: "DensePseudoinverseWorkspace",
        })
}

fn resize(vector: &mut Vec<f64>, dimension: usize) -> Result<(), MultiwayError> {
    bytes(dimension)?;
    if dimension > vector.len() {
        vector
            .try_reserve_exact(dimension - vector.len())
            .map_err(|source| MultiwayError::WorkspaceAllocation {
                context: "DensePseudoinverseWorkspace",
                source,
            })?;
    }
    vector.resize(dimension, 0.0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impossible_size_preserves_existing_modal_values_and_capacity() {
        let mut modal = vec![1.0, -2.0];
        let capacity = modal.capacity();
        assert!(resize(&mut modal, usize::MAX).is_err());
        assert!(resize(&mut modal, isize::MAX as usize / 8 + 1).is_err());
        assert_eq!(modal, [1.0, -2.0]);
        assert_eq!(modal.capacity(), capacity);
    }
}
