//! Sealed, statically dispatched immutable owners for the shared prepared solvers.
use crate::{
    MultiwayError, PreparedHierarchyPayloadReport, PreparedHierarchyWorkspace, PreparedMapHierarchy,
};
use multiway_incidence::{PreparedTupleGrouping, ThreeWayWeightFrame};

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A fixed current-generation action and fine operator for prepared Krylov solves.
///
/// Implementations are sealed to the crate. Solver storage is statically dispatched
/// and bound to the exact action owner; no virtual call, implicit pool, numerical
/// copy or RHS-dependent preconditioner preparation is introduced by this boundary.
/// The supplied-map hierarchy remains the default prepared solver owner.
pub trait PreparedSolverAction: sealed::Sealed + std::fmt::Debug {
    /// Complete mutable action scratch, borrowing its exact immutable owner.
    type Workspace<'owner>: std::fmt::Debug
    where
        Self: 'owner;
    /// Original fine frame used by the recurrence and independent certificate.
    fn fine_frame(&self) -> &ThreeWayWeightFrame<'_>;
    /// Checked optional grouping used by the fine weighted adjoint.
    fn fine_grouping(&self) -> Option<&PreparedTupleGrouping<'_>>;
    /// Original fine coefficient dimension.
    fn dimension(&self) -> usize {
        self.fine_frame().diagonal().len()
    }
    /// Fallibly allocate all application scratch at an explicit setup boundary.
    fn application_workspace(&self) -> Result<Self::Workspace<'_>, MultiwayError>;
    /// Fixed symmetric action, with exact owner checks and transactional output.
    fn apply_with_workspace<'owner>(
        &'owner self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut Self::Workspace<'owner>,
    ) -> Result<(), MultiwayError>;
    /// Current original Gramian, using only already-admitted action scratch.
    fn fine_gramian_with_workspace<'owner>(
        &'owner self,
        x: &[f64],
        out: &mut [f64],
        workspace: &mut Self::Workspace<'owner>,
    ) -> Result<(), MultiwayError>;
    /// Complete direct-owner and actual workspace array inventory.
    /// Coarse and dense-terminal categories are zero for a baseline action.
    fn payload_report<'owner>(
        &'owner self,
        workspace: &Self::Workspace<'owner>,
        additional_live_payload_bytes: usize,
    ) -> Result<PreparedHierarchyPayloadReport, MultiwayError>;
    /// Exclusive actual application-scratch array capacities, checked for this owner.
    fn workspace_payload_bytes<'owner>(
        &'owner self,
        workspace: &Self::Workspace<'owner>,
    ) -> Result<usize, MultiwayError>;
    /// Ordered requested-payload categories for preallocation admission.
    /// Fine topology, coarse topology, fine frame, coarse frames, terminal,
    /// grouping, and application scratch; caller and outer solver arrays are extra.
    #[doc(hidden)]
    fn requested_payload_parts(&self) -> Result<[usize; 7], MultiwayError>;
}

impl sealed::Sealed for PreparedMapHierarchy<'_> {}
impl PreparedSolverAction for PreparedMapHierarchy<'_> {
    type Workspace<'owner>
        = PreparedHierarchyWorkspace<'owner>
    where
        Self: 'owner;
    #[inline]
    fn fine_frame(&self) -> &ThreeWayWeightFrame<'_> {
        self.frames().fine()
    }
    #[inline]
    fn fine_grouping(&self) -> Option<&PreparedTupleGrouping<'_>> {
        self.level_grouping(0)
    }
    #[inline]
    fn application_workspace(&self) -> Result<Self::Workspace<'_>, MultiwayError> {
        PreparedMapHierarchy::application_workspace(self)
    }
    #[inline]
    fn apply_with_workspace<'owner>(
        &'owner self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut Self::Workspace<'owner>,
    ) -> Result<(), MultiwayError> {
        PreparedMapHierarchy::apply_with_workspace(self, rhs, out, workspace)
    }
    #[inline]
    fn fine_gramian_with_workspace<'owner>(
        &'owner self,
        x: &[f64],
        out: &mut [f64],
        workspace: &mut Self::Workspace<'owner>,
    ) -> Result<(), MultiwayError> {
        PreparedMapHierarchy::fine_gramian_with_workspace(self, x, out, workspace)
    }
    #[inline]
    fn payload_report<'owner>(
        &'owner self,
        workspace: &Self::Workspace<'owner>,
        additional_live_payload_bytes: usize,
    ) -> Result<PreparedHierarchyPayloadReport, MultiwayError> {
        PreparedMapHierarchy::payload_report(self, workspace, additional_live_payload_bytes)
    }
    #[inline]
    fn workspace_payload_bytes<'owner>(
        &'owner self,
        workspace: &Self::Workspace<'owner>,
    ) -> Result<usize, MultiwayError> {
        workspace.validate_for(self)?;
        workspace.retained_payload_bytes()
    }
    #[inline]
    fn requested_payload_parts(&self) -> Result<[usize; 7], MultiwayError> {
        let frames = self.frames();
        Ok([
            frames.hierarchy().fine().retained_payload_bytes()?,
            frames.hierarchy().retained_payload_bytes()?,
            frames.fine().retained_payload_bytes()?,
            frames.retained_payload_bytes()?,
            self.retained_payload_bytes()?,
            self.grouping_payload_bytes()?,
            self.workspace_required_bytes()?,
        ])
    }
}
