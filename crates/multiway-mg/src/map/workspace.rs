//! Caller-owned MAP vectors and explicitly prepared projection scratch.

use multiway_incidence::StructuralProjectionWorkspace;

use super::SymmetricMapPreconditioner;
use crate::MultiwayError;

/// Caller-owned storage for one symmetric MAP application.
///
/// Vectors hold scratch, not numerical operators. The projection subworkspace
/// is bound to an immutable component decomposition. Ordinary problem clones
/// share that binding; independent builds require [`Self::try_prepare_for`].
#[derive(Debug)]
pub struct SymmetricMapWorkspace {
    pub(super) compatible_rhs: Vec<f64>,
    pub(super) forward: Vec<f64>,
    pub(super) middle: Vec<f64>,
    pub(super) solution: Vec<f64>,
    pub(super) projection: StructuralProjectionWorkspace,
}

impl SymmetricMapPreconditioner {
    /// Minimum exclusive scratch payload bytes for this MAP application.
    ///
    /// Excludes inline descriptors, shared identity metadata, and the operator.
    pub fn workspace_required_bytes(&self) -> Result<usize, MultiwayError> {
        let vectors = self
            .problem()
            .dimension()
            .checked_mul(4)
            .and_then(|n| n.checked_mul(core::mem::size_of::<f64>()))
            .ok_or_else(overflow)?;
        vectors
            .checked_add(
                self.problem()
                    .components()
                    .projection_workspace_required_bytes()?,
            )
            .ok_or_else(overflow)
    }

    /// Prepare all caller-owned MAP scratch at a fallible setup boundary.
    pub fn application_workspace(&self) -> Result<SymmetricMapWorkspace, MultiwayError> {
        self.workspace_required_bytes()?;
        let mut workspace = SymmetricMapWorkspace {
            compatible_rhs: Vec::new(),
            forward: Vec::new(),
            middle: Vec::new(),
            solution: Vec::new(),
            projection: self.problem().components().try_projection_workspace()?,
        };
        workspace.try_prepare_for(self)?;
        Ok(workspace)
    }
}

impl SymmetricMapWorkspace {
    /// Explicitly prepare for this operator, retaining existing vector capacities.
    ///
    /// No numerical factors or weights are retained. Partial reservation failure
    /// may grow capacities, but does not change vector lengths or the projection
    /// binding. Application never performs this preparation implicitly.
    pub fn try_prepare_for(
        &mut self,
        operator: &SymmetricMapPreconditioner,
    ) -> Result<(), MultiwayError> {
        operator.workspace_required_bytes()?;
        let dimension = operator.problem().dimension();
        for vector in [
            &mut self.compatible_rhs,
            &mut self.forward,
            &mut self.middle,
            &mut self.solution,
        ] {
            if dimension > vector.len() {
                vector
                    .try_reserve_exact(dimension - vector.len())
                    .map_err(|source| MultiwayError::WorkspaceAllocation {
                        context: "SymmetricMapWorkspace",
                        source,
                    })?;
            }
        }
        self.projection
            .try_prepare_for(operator.problem().components())?;
        for vector in [
            &mut self.compatible_rhs,
            &mut self.forward,
            &mut self.middle,
            &mut self.solution,
        ] {
            vector.resize(dimension, 0.0);
        }
        Ok(())
    }

    /// Exact exclusive retained payload based on vector capacities.
    ///
    /// Excludes inline descriptors, shared identity metadata, allocator overhead,
    /// the immutable operator and caller input/output. It is not process peak RSS.
    pub fn retained_bytes(&self) -> Result<usize, MultiwayError> {
        [
            &self.compatible_rhs,
            &self.forward,
            &self.middle,
            &self.solution,
        ]
        .into_iter()
        .try_fold(self.projection.retained_bytes(), |total, vector| {
            let bytes = vector
                .capacity()
                .checked_mul(core::mem::size_of::<f64>())
                .ok_or_else(overflow)?;
            total.checked_add(bytes).ok_or_else(overflow)
        })
    }

    pub(super) fn validate(
        &self,
        operator: &SymmetricMapPreconditioner,
    ) -> Result<(), MultiwayError> {
        let dimension = operator.problem().dimension();
        for vector in [
            &self.compatible_rhs,
            &self.forward,
            &self.middle,
            &self.solution,
        ] {
            if vector.len() != dimension {
                return Err(crate::error::dimension(
                    "SymmetricMapWorkspace",
                    dimension,
                    vector.len(),
                ));
            }
        }
        if !self
            .projection
            .is_compatible_with(operator.problem().components())
        {
            return Err(
                multiway_incidence::IncidenceError::WorkspaceBindingMismatch {
                    context: "SymmetricMapWorkspace",
                }
                .into(),
            );
        }
        Ok(())
    }
}

fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow {
        context: "SymmetricMapWorkspace",
    }
}

impl SymmetricMapWorkspace {
    pub(crate) fn is_prepared_for(&self, operator: &SymmetricMapPreconditioner) -> bool {
        self.validate(operator).is_ok()
    }
}
