//! Complete prepared serial projected PCG with independent tuple-space certification.
use crate::{
    CertificateWorkReport, MultiwayError, PcgOptions, PcgStopReason, PreparedCertificateWorkspace,
    PreparedHierarchyPayloadReport, PreparedHierarchyWorkspace, PreparedMapHierarchy,
    certificate::{bytes, ensure_finite, vector},
    certify_prepared_normal_equations,
    pcg_kernel::{self, PcgActions, PcgStorage},
};
use multiway_incidence::{PreparedStructuralProjectionWorkspace, ThreeWayOperatorView};

/// Fixed native projected-PCG and independent acceptance configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedPcgOptions {
    /// Native tolerances, iteration limit and true-residual recomputation interval.
    pub pcg: PcgOptions,
    /// Finite positive tolerance for the original weighted incidence certificate.
    pub certificate_tolerance: f64,
}
impl Default for PreparedPcgOptions {
    fn default() -> Self {
        Self {
            pcg: PcgOptions::default(),
            certificate_tolerance: 1e-8,
        }
    }
}
impl PreparedPcgOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        self.pcg.validate()?;
        if !self.certificate_tolerance.is_finite() || self.certificate_tolerance <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "pcg_certificate_tolerance",
                message: "must be finite and positive".to_owned(),
            });
        }
        Ok(self)
    }
}
/// Actual attempted actions in the last admitted prepared PCG solve, even on error.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PreparedPcgWorkReport {
    /// Original weighted adjoint used to construct the coefficient RHS.
    pub rhs_adjoint_applications: usize,
    /// Fine Gramian applications, including true-residual recomputations.
    pub gramian_applications: usize,
    /// Complete fixed hierarchy applications.
    pub hierarchy_applications: usize,
    /// Outer structural projections, excluding projections inside the hierarchy.
    pub projection_applications: usize,
    /// Independent final tuple-space certificate work.
    pub certificate: CertificateWorkReport,
}
/// Native projected-PCG diagnostics and separate original-operator acceptance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedPcgReport {
    /// Native projected true-residual stopping, not independent acceptance.
    pub native_converged: bool,
    /// Native projected-PCG stopping reason.
    pub native_stop_reason: PcgStopReason,
    /// Completed native iterations.
    pub iterations: usize,
    /// Native projected residual norm.
    pub native_residual_norm: f64,
    /// Native projected residual relative to the projected coefficient RHS.
    pub native_relative_residual: f64,
    /// Norm removed by initial structural projection of the coefficient RHS.
    pub rhs_projection_norm: f64,
    /// Finite original `||B'W(y-Bx)|| / ||B'Wy||` certificate.
    pub certified_normal_equation_residual: f64,
    /// Original-operator certificate meets the declared positive tolerance.
    pub accepted: bool,
    /// Actual action counts including independent certification.
    pub work: PreparedPcgWorkReport,
}
/// Borrowed coefficient candidate; release its borrow before reusing the workspace.
#[derive(Debug)]
pub struct PreparedPcgResult<'workspace> {
    /// Candidate coefficients; inspect `report.accepted` before use.
    pub coefficients: &'workspace [f64],
    /// Native diagnostics and independent acceptance.
    pub report: PreparedPcgReport,
}
/// Complete retained payload of one prepared PCG solve and declared caller state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedPcgPayloadReport {
    /// All direct hierarchy owners/application scratch; additional=0.
    pub hierarchy: PreparedHierarchyPayloadReport,
    /// Outer PCG, coefficient RHS, projection and certificate arrays.
    pub outer_workspace_payload_bytes: usize,
    /// Other live arrays explicitly declared by the caller.
    pub additional_live_payload_bytes: usize,
    /// Checked complete sum, excluding inline roots, allocator overhead and RSS.
    pub total_payload_bytes: usize,
}

/// Caller-owned serial PCG, hierarchy and certificate storage for one exact owner.
///
/// Shares the ordinary untraced PCG recurrence; traced research PCG remains a
/// separate diagnostic driver. Additional unidentified directions can cause PCG
/// breakdown. Rectangular LSMR remains the rank-robust default candidate route.
/// Targets are canonical tuple values. No automatic fallback or generation rebind
/// occurs, and a native stopping flag alone never accepts a solution.
#[derive(Debug)]
pub struct PreparedPcgWorkspace<'owner> {
    hierarchy: &'owner PreparedMapHierarchy<'owner>,
    hierarchy_scratch: PreparedHierarchyWorkspace<'owner>,
    projection: PreparedStructuralProjectionWorkspace<'owner>,
    certificate: PreparedCertificateWorkspace<'owner, 'owner>,
    storage: PcgStorage,
    rhs: Vec<f64>,
    options: PreparedPcgOptions,
    last_work: PreparedPcgWorkReport,
}
impl<'owner> PreparedPcgWorkspace<'owner> {
    /// Fallibly allocate complete scratch at an explicit fixed-configuration boundary.
    pub fn try_new(
        hierarchy: &'owner PreparedMapHierarchy<'owner>,
        options: PreparedPcgOptions,
    ) -> Result<Self, MultiwayError> {
        Self::try_new_with_payload_budget(hierarchy, options, usize::MAX, 0)
    }
    /// Admit exact immutable owners plus complete requested scratch before allocation.
    pub fn try_new_with_payload_budget(
        hierarchy: &'owner PreparedMapHierarchy<'owner>,
        options: PreparedPcgOptions,
        maximum_payload_bytes: usize,
        additional_live_payload_bytes: usize,
    ) -> Result<Self, MultiwayError> {
        let options = options.validate()?;
        let required =
            Self::setup_payload_bound(hierarchy, options, additional_live_payload_bytes)?;
        if required > maximum_payload_bytes {
            return Err(MultiwayError::PayloadBudgetExceeded {
                required,
                budget: maximum_payload_bytes,
            });
        }
        let fine = hierarchy.frames().fine();
        Ok(Self {
            hierarchy,
            hierarchy_scratch: hierarchy.application_workspace()?,
            projection: fine.topology().try_projection_workspace()?,
            certificate: PreparedCertificateWorkspace::try_new(fine)?,
            storage: PcgStorage::try_new(fine.diagonal().len())?,
            rhs: vector(fine.diagonal().len())?,
            options,
            last_work: PreparedPcgWorkReport::default(),
        })
    }
    /// Complete live requested payload bound; caller inputs/old generations are explicit.
    pub fn setup_payload_bound(
        hierarchy: &PreparedMapHierarchy<'_>,
        options: PreparedPcgOptions,
        additional_live_payload_bytes: usize,
    ) -> Result<usize, MultiwayError> {
        options.validate()?;
        let frames = hierarchy.frames();
        let fine = frames.fine();
        let mut total = additional_live_payload_bytes;
        for part in [
            frames.hierarchy().fine().retained_payload_bytes()?,
            frames.hierarchy().retained_payload_bytes()?,
            fine.retained_payload_bytes()?,
            frames.retained_payload_bytes()?,
            hierarchy.retained_payload_bytes()?,
            hierarchy.grouping_payload_bytes()?,
            hierarchy.workspace_required_bytes()?,
            fine.topology().projection_workspace_required_bytes()?,
            PreparedCertificateWorkspace::required_payload_bytes(fine)?,
            PcgStorage::required_payload_bytes(fine.diagonal().len())?,
            bytes(fine.diagonal().len())?,
        ] {
            total = add(total, part)?;
        }
        Ok(total)
    }
    /// Fixed native and certificate options.
    #[must_use]
    pub const fn options(&self) -> PreparedPcgOptions {
        self.options
    }
    /// Actual actions from the last call admitted past static validation, even on error.
    #[must_use]
    pub const fn last_work(&self) -> PreparedPcgWorkReport {
        self.last_work
    }
    /// Exact numerical hierarchy validation without mutation or preparation.
    pub fn validate_for(&self, hierarchy: &PreparedMapHierarchy<'_>) -> Result<(), MultiwayError> {
        if !core::ptr::eq(self.hierarchy, hierarchy) {
            return Err(MultiwayError::WorkspaceNotPrepared {
                context: "prepared PCG hierarchy",
            });
        }
        Ok(())
    }
    /// Complete exclusive retained scratch capacities, including hierarchy application.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        add(
            self.outer_payload_bytes()?,
            self.hierarchy_scratch.retained_payload_bytes()?,
        )
    }
    /// Complete retained direct-owner lifetime inventory with declared caller arrays.
    pub fn payload_report(
        &self,
        additional_live_payload_bytes: usize,
    ) -> Result<PreparedPcgPayloadReport, MultiwayError> {
        let hierarchy = self.hierarchy.payload_report(&self.hierarchy_scratch, 0)?;
        let outer_workspace_payload_bytes = self.outer_payload_bytes()?;
        let total_payload_bytes = add(
            add(hierarchy.total_payload_bytes, outer_workspace_payload_bytes)?,
            additional_live_payload_bytes,
        )?;
        Ok(PreparedPcgPayloadReport {
            hierarchy,
            outer_workspace_payload_bytes,
            additional_live_payload_bytes,
            total_payload_bytes,
        })
    }
    fn outer_payload_bytes(&self) -> Result<usize, MultiwayError> {
        let mut total = 0;
        for part in [
            self.storage.retained_payload_bytes()?,
            self.projection.retained_payload_bytes()?,
            self.certificate.retained_payload_bytes()?,
            bytes(self.rhs.capacity())?,
        ] {
            total = add(total, part)?;
        }
        Ok(total)
    }
}

/// Solve one canonical tuple-target column with projected PCG and independent certification.
///
/// Exact hierarchy, target length and finite values are checked before mutation.
/// Native convergence remains a candidate. Numerical breakdown returns no result;
/// complete attempted work remains available and later calls reinitialize scratch.
pub fn solve_prepared_pcg_least_squares<'workspace>(
    hierarchy: &PreparedMapHierarchy<'_>,
    targets: &[f64],
    workspace: &'workspace mut PreparedPcgWorkspace<'_>,
) -> Result<PreparedPcgResult<'workspace>, MultiwayError> {
    #[cfg(feature = "profiling")]
    let _profile_span =
        multiway_incidence::profiling::span(multiway_incidence::profiling::Phase::PreparedPcg);

    workspace.validate_for(hierarchy)?;
    let fine = workspace.hierarchy.frames().fine();
    if targets.len() != fine.weights().len() {
        return Err(crate::error::dimension(
            "prepared PCG targets",
            fine.weights().len(),
            targets.len(),
        ));
    }
    ensure_finite(targets, "prepared PCG targets")?;
    workspace.last_work = PreparedPcgWorkReport::default();
    workspace.last_work.rhs_adjoint_applications = 1;
    let original = fine.operator_view();
    if let Some(grouping) = hierarchy.level_grouping(0) {
        original
            .with_grouping(grouping)?
            .rhs_from_targets_into(targets, &mut workspace.rhs)?;
    } else {
        original.rhs_from_targets_into(targets, &mut workspace.rhs)?;
    }
    pcg_kernel::ensure_finite("PCG right-hand side", &workspace.rhs)?;
    let mut actions = PreparedPcgActions {
        view: fine.operator_view(),
        hierarchy: workspace.hierarchy,
        hierarchy_scratch: &mut workspace.hierarchy_scratch,
        projection: &mut workspace.projection,
        work: &mut workspace.last_work,
    };
    let native = pcg_kernel::solve(
        &mut actions,
        &workspace.rhs,
        workspace.options.pcg,
        &mut workspace.storage,
    )?;
    let certificate = certify_prepared_normal_equations(
        fine.operator_view(),
        targets,
        &workspace.storage.solution,
        &mut workspace.certificate,
    );
    workspace.last_work.certificate = workspace.certificate.last_work();
    let certificate = certificate?;
    Ok(PreparedPcgResult {
        coefficients: &workspace.storage.solution,
        report: PreparedPcgReport {
            native_converged: native.converged,
            native_stop_reason: native.stop_reason,
            iterations: native.iterations,
            native_residual_norm: native.residual_norm,
            native_relative_residual: native.relative_residual,
            rhs_projection_norm: native.rhs_projection_norm,
            certified_normal_equation_residual: certificate,
            accepted: certificate <= workspace.options.certificate_tolerance,
            work: workspace.last_work,
        },
    })
}

/// Solve 1–32 independent column-major target columns with shared scalar PCG scratch.
///
/// All static layouts and finite targets are validated first. Reports are then
/// cleared to `None`; each completed candidate is copied with `Some(report)`.
/// Inspect each `accepted` flag. Numerical failure preserves the completed prefix
/// and leaves failed/unprocessed outputs unchanged; the first `None` identifies
/// the failing column and `last_work()` records its attempted actions.
pub fn solve_prepared_pcg_batch_into(
    hierarchy: &PreparedMapHierarchy<'_>,
    targets: &[f64],
    columns: usize,
    coefficients: &mut [f64],
    reports: &mut [Option<PreparedPcgReport>],
    workspace: &mut PreparedPcgWorkspace<'_>,
) -> Result<(), MultiwayError> {
    workspace.validate_for(hierarchy)?;
    if !(1..=32).contains(&columns) {
        return Err(crate::error::dimension(
            "prepared PCG RHS count (1..=32)",
            32,
            columns,
        ));
    }
    let rows = hierarchy.frames().fine().weights().len();
    let n = hierarchy.dimension();
    let target_length = rows.checked_mul(columns).ok_or_else(overflow)?;
    let coefficient_length = n.checked_mul(columns).ok_or_else(overflow)?;
    for (context, expected, actual) in [
        ("prepared PCG RHS panel", target_length, targets.len()),
        (
            "prepared PCG coefficient panel",
            coefficient_length,
            coefficients.len(),
        ),
        ("prepared PCG reports", columns, reports.len()),
    ] {
        if expected != actual {
            return Err(crate::error::dimension(context, expected, actual));
        }
    }
    ensure_finite(targets, "prepared PCG RHS panel")?;
    reports.fill(None);
    for (column, report) in reports.iter_mut().enumerate() {
        let result = solve_prepared_pcg_least_squares(
            hierarchy,
            &targets[column * rows..(column + 1) * rows],
            workspace,
        )?;
        coefficients[column * n..(column + 1) * n].copy_from_slice(result.coefficients);
        *report = Some(result.report);
    }
    Ok(())
}

struct PreparedPcgActions<'borrow, 'owner> {
    view: ThreeWayOperatorView<'owner, 'owner>,
    hierarchy: &'owner PreparedMapHierarchy<'owner>,
    hierarchy_scratch: &'borrow mut PreparedHierarchyWorkspace<'owner>,
    projection: &'borrow mut PreparedStructuralProjectionWorkspace<'owner>,
    work: &'borrow mut PreparedPcgWorkReport,
}
impl PcgActions for PreparedPcgActions<'_, '_> {
    fn project(&mut self, values: &mut [f64]) -> Result<f64, MultiwayError> {
        self.work.projection_applications += 1;
        Ok(self
            .view
            .frame()
            .topology()
            .project_structural_range_with_workspace(values, self.projection)?)
    }
    fn precondition(&mut self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.work.hierarchy_applications += 1;
        self.hierarchy
            .apply_with_workspace(rhs, out, self.hierarchy_scratch)
    }
    fn gramian(&mut self, x: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.work.gramian_applications += 1;
        self.hierarchy
            .fine_gramian_with_workspace(x, out, self.hierarchy_scratch)?;
        Ok(())
    }
    fn residual(&mut self, rhs: &[f64], x: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.work.gramian_applications += 1;
        if rhs.len() != self.view.dimension() {
            return Err(crate::error::dimension(
                "prepared PCG residual RHS",
                self.view.dimension(),
                rhs.len(),
            ));
        }
        self.hierarchy
            .fine_gramian_with_workspace(x, out, self.hierarchy_scratch)?;
        for (value, &right) in out.iter_mut().zip(rhs) {
            *value = right - *value;
        }
        Ok(())
    }
}
fn add(a: usize, b: usize) -> Result<usize, MultiwayError> {
    a.checked_add(b).ok_or_else(overflow)
}
fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow {
        context: "prepared PCG payload",
    }
}
