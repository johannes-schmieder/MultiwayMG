//! Explicit fallible preparation of component-bound projection scratch.

use std::sync::Arc;

use super::{IncidenceComponents, StructuralProjectionScratch, StructuralProjectionWorkspace};
use crate::IncidenceError;

const CONTEXT: &str = "structural projection workspace preparation";

impl IncidenceComponents {
    /// Required scratch-array payload bytes, excluding inline and shared state.
    pub fn projection_workspace_required_bytes(&self) -> Result<usize, IncidenceError> {
        required_bytes(self.count())
    }

    /// Allocate projection scratch fallibly and bind it to this decomposition.
    ///
    /// The ordinary [`Self::projection_workspace`] constructor remains available.
    /// No new identity token is allocated here; this clones the owner's token.
    pub fn try_projection_workspace(&self) -> Result<StructuralProjectionWorkspace, IncidenceError> {
        let mut scratch = Vec::new();
        prepare_scratch(&mut scratch, self.count())?;
        Ok(StructuralProjectionWorkspace {
            dimension: self.labels.len(),
            scratch,
            binding: Arc::clone(&self.binding),
        })
    }
}

impl StructuralProjectionWorkspace {
    /// Whether application is valid without an explicit preparation step.
    ///
    /// Equal dimensions or value-equal metadata do not replace exact identity.
    #[must_use]
    pub fn is_compatible_with(&self, components: &IncidenceComponents) -> bool {
        self.dimension == components.labels.len()
            && self.scratch.len() == components.count()
            && Arc::ptr_eq(&self.binding, &components.binding)
    }

    /// Explicitly prepare scratch for another immutable component decomposition.
    ///
    /// Applications never rebind implicitly. This setup operation retains existing
    /// capacity, clears scratch when changing owners, and replaces the binding
    /// only after successful reservation. A failed reservation leaves the previous
    /// dimensions, contents and binding usable. Preparing an already compatible
    /// workspace does nothing and allocates nothing.
    pub fn try_prepare_for(&mut self, components: &IncidenceComponents) -> Result<(), IncidenceError> {
        if self.is_compatible_with(components) {
            return Ok(());
        }
        prepare_scratch(&mut self.scratch, components.count())?;
        self.dimension = components.labels.len();
        self.binding = Arc::clone(&components.binding);
        Ok(())
    }
}

fn required_bytes(count: usize) -> Result<usize, IncidenceError> {
    count
        .checked_mul(core::mem::size_of::<StructuralProjectionScratch>())
        .ok_or(IncidenceError::DimensionOverflow { context: CONTEXT })
}

fn prepare_scratch(
    scratch: &mut Vec<StructuralProjectionScratch>,
    count: usize,
) -> Result<(), IncidenceError> {
    required_bytes(count)?;
    if count > scratch.len() {
        scratch
            .try_reserve_exact(count - scratch.len())
            .map_err(|_| IncidenceError::WorkspaceAllocation { context: CONTEXT })?;
    }
    scratch.resize(count, StructuralProjectionScratch::default());
    scratch.fill(StructuralProjectionScratch::default());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impossible_preparation_preserves_live_scratch() {
        let mut scratch = vec![StructuralProjectionScratch {
            sums: [1.0, -2.0, 3.0],
            ..StructuralProjectionScratch::default()
        }];
        let before = scratch.clone();
        let pointer = scratch.as_ptr();
        let capacity = scratch.capacity();
        assert!(prepare_scratch(&mut scratch, usize::MAX).is_err());
        let count = (isize::MAX as usize / core::mem::size_of::<StructuralProjectionScratch>()) + 1;
        assert!(matches!(
            prepare_scratch(&mut scratch, count),
            Err(IncidenceError::WorkspaceAllocation { .. })
        ));
        assert_eq!(scratch, before);
        assert_eq!(scratch.capacity(), capacity);
        assert_eq!(scratch.as_ptr(), pointer);
    }
}
