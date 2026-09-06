//! Borrowed exact-topology projection scratch, without duplicated components.
use super::{StructuralProjectionScratch, kernel::ProjectionData};
use crate::{
    IncidenceError, PreparedThreeWayTopology,
    construction::{array_bytes, reserve},
};

/// Mutable projection scratch bound to one exact prepared structural owner.
///
/// Component labels and factor sizes are borrowed, not reconstructed or copied.
/// Projection depends only on topology; different weight frames on that same
/// topology may explicitly share this scratch. Dimensions and exact ownership
/// are checked before values or scratch are mutated. The scalar arithmetic is
/// shared with ordinary `IncidenceComponents`, including its numerical limits.
/// Solvers must reject unrepresentable results before accepting convergence.
///
/// ```compile_fail
/// use multiway_incidence::PreparedThreeWayTopology;
/// let w = {
///     let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
///     t.try_projection_workspace().unwrap()
/// };
/// assert_eq!(w.dimension(), 3);
/// ```
#[derive(Debug)]
pub struct PreparedStructuralProjectionWorkspace<'topology> {
    topology: &'topology PreparedThreeWayTopology,
    scratch: Vec<StructuralProjectionScratch>,
}
impl PreparedStructuralProjectionWorkspace<'_> {
    /// Coefficient dimension of the borrowed exact structural owner.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.topology.topology().total_levels()
    }

    /// Number of exact incidence components.
    #[must_use]
    pub fn component_count(&self) -> usize {
        self.scratch.len()
    }

    /// Exclusive retained scratch-array capacities; excludes borrowed topology.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        array_bytes::<StructuralProjectionScratch>(self.scratch.capacity())
    }

    /// Exact owner check, including rejection of value-equal reconstructions.
    pub fn validate_for(&self, topology: &PreparedThreeWayTopology) -> Result<(), IncidenceError> {
        self.topology.binding().validate_for(topology)
    }
}
impl PreparedThreeWayTopology {
    /// Requested exclusive scratch-array payload for structural projection.
    pub fn projection_workspace_required_bytes(&self) -> Result<usize, IncidenceError> {
        array_bytes::<StructuralProjectionScratch>(self.component_factor_sizes().len())
    }

    /// Fallibly allocate scratch borrowing this exact prepared owner.
    pub fn try_projection_workspace(
        &self,
    ) -> Result<PreparedStructuralProjectionWorkspace<'_>, IncidenceError> {
        let mut scratch = reserve(
            self.component_factor_sizes().len(),
            "prepared projection scratch",
            &mut |_| Ok(()),
        )?;
        scratch.resize(
            self.component_factor_sizes().len(),
            StructuralProjectionScratch::default(),
        );
        Ok(PreparedStructuralProjectionWorkspace {
            topology: self,
            scratch,
        })
    }

    /// Remove the two structural factor shifts per component, without allocation.
    ///
    /// Returns the norm of the removed projection. This low-level arithmetic has
    /// the same floating-point range as ordinary component projection; a finite
    /// result is not a rank or original-operator convergence certificate.
    pub fn project_structural_range_with_workspace(
        &self,
        values: &mut [f64],
        workspace: &mut PreparedStructuralProjectionWorkspace<'_>,
    ) -> Result<f64, IncidenceError> {
        self.validate_projection(values, workspace)?;
        Ok(self
            .projection_data()
            .project(values, &mut workspace.scratch))
    }

    /// Maximum absolute structural shift defect using the shared scalar arithmetic.
    pub fn maximum_structural_defect_with_workspace(
        &self,
        values: &[f64],
        workspace: &mut PreparedStructuralProjectionWorkspace<'_>,
    ) -> Result<f64, IncidenceError> {
        self.validate_projection(values, workspace)?;
        Ok(self
            .projection_data()
            .defect(values, &mut workspace.scratch))
    }

    fn validate_projection(
        &self,
        values: &[f64],
        workspace: &PreparedStructuralProjectionWorkspace<'_>,
    ) -> Result<(), IncidenceError> {
        if values.len() != self.topology().total_levels() {
            return Err(crate::error::dimension(
                "prepared structural projection values",
                self.topology().total_levels(),
                values.len(),
            ));
        }
        workspace.validate_for(self)
    }

    fn projection_data(&self) -> ProjectionData<'_> {
        ProjectionData {
            labels: self.component_labels(),
            factor_sizes: self.component_factor_sizes(),
            offsets: self.topology().offsets(),
        }
    }
}
