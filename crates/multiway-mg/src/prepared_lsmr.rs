//! Complete serial prepared LSMR with independent original-operator acceptance.
use crate::{
    CertificateWorkReport, LeastSquaresStopReason, MultiwayError, PreparedCertificateWorkspace,
    PreparedHierarchyPayloadReport, PreparedHierarchyWorkspace, PreparedMapHierarchy,
    certificate::{bytes, ensure_finite, vector},
    certify_prepared_normal_equations,
};
use multiway_incidence::{PreparedStructuralProjectionWorkspace, ThreeWayOperatorView};
use schwarz_precond::{
    MlsmrWorkspace, MlsmrWorkspaceOptions, OperatorMut, SolveError, mlsmr_with_workspace,
};

/// Fixed configuration of one prepared serial LSMR workspace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedLsmrOptions {
    /// Native modified-LSMR tolerance, separate from original-operator acceptance.
    pub tolerance: f64,
    /// Required finite positive original-operator relative gradient tolerance.
    pub certificate_tolerance: f64,
    /// Positive maximum iteration count.
    pub max_iterations: usize,
    /// Local reorthogonalization capacity; capped by tuple and coefficient counts.
    pub local_size: Option<usize>,
}
impl Default for PreparedLsmrOptions {
    fn default() -> Self {
        Self {
            tolerance: 1e-8,
            certificate_tolerance: 1e-8,
            max_iterations: 1_000,
            local_size: Some(8),
        }
    }
}
impl PreparedLsmrOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if !self.tolerance.is_finite()
            || self.tolerance <= 0.0
            || !self.certificate_tolerance.is_finite()
            || self.certificate_tolerance <= 0.0
            || self.max_iterations == 0
        {
            return Err(MultiwayError::InvalidOption {
                name: "prepared_lsmr_options",
                message: "tolerances must be finite and positive, iterations must be positive"
                    .to_owned(),
            });
        }
        Ok(self)
    }
}

/// Actual outer actions attempted in the last admitted solve, including failure.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PreparedLsmrWorkReport {
    /// Weighted incidence actions inside the recurrence and its audit.
    pub weighted_incidence_applications: usize,
    /// Weighted adjoint actions inside the recurrence and its audit.
    pub weighted_adjoint_applications: usize,
    /// Complete fixed hierarchy applications, including failed attempts.
    pub hierarchy_applications: usize,
    /// Original-operator actions in final independent certification.
    pub certificate: CertificateWorkReport,
}

/// Native stopping information and separate original-operator acceptance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedLsmrReport {
    /// Native recurrence/audit convergence; not independent acceptance.
    pub native_converged: bool,
    /// Native stop reason, preserved even if independent acceptance disagrees.
    pub native_stop_reason: LeastSquaresStopReason,
    /// Completed native iterations.
    pub iterations: usize,
    /// Native weighted residual diagnostic.
    pub native_residual_norm: f64,
    /// Native preconditioned normal-equation residual diagnostic.
    pub native_normal_equation_residual: f64,
    /// Finite original `||B'W(y-Bx)|| / ||B'Wy||` certificate.
    pub certified_normal_equation_residual: f64,
    /// Whether that certificate meets the workspace's declared positive tolerance.
    pub accepted: bool,
    /// Exact outer action counts including independent certification.
    pub work: PreparedLsmrWorkReport,
}
/// Borrowed coefficient candidate; workspace reuse requires releasing this borrow.
#[derive(Debug)]
pub struct PreparedLsmrResult<'workspace> {
    /// Structurally projected coefficients. Check `report.accepted` before use.
    pub coefficients: &'workspace [f64],
    /// Independent acceptance and complete native diagnostics/work.
    pub report: PreparedLsmrReport,
}

/// Complete retained payload for a prepared LSMR solve and declared caller state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedLsmrPayloadReport {
    /// All hierarchy owners and its complete application workspace; additional=0.
    pub hierarchy: PreparedHierarchyPayloadReport,
    /// Outer LSMR, projection, candidate, weighted-target and certificate arrays.
    pub outer_workspace_payload_bytes: usize,
    /// Other live arrays, including RHS/output panels and old independent owners.
    pub additional_live_payload_bytes: usize,
    /// Checked complete sum, excluding inline roots, allocator overhead and RSS.
    pub total_payload_bytes: usize,
}

/// Caller-owned complete serial solve storage attached to one exact hierarchy.
///
/// Weights, topology and factors are borrowed. The workspace owns every mutable
/// recurrence, hierarchy, projection and certificate buffer. No pool or automatic
/// rebind is used. Targets must be in canonical unique-tuple order. Repeated RHS
/// calls reuse all storage; the returned coefficients borrow that storage.
pub struct PreparedLsmrWorkspace<'owner> {
    hierarchy: &'owner PreparedMapHierarchy<'owner>,
    hierarchy_scratch: PreparedHierarchyWorkspace<'owner>,
    recurrence: MlsmrWorkspace,
    projection: PreparedStructuralProjectionWorkspace<'owner>,
    certificate: PreparedCertificateWorkspace<'owner, 'owner>,
    weighted_targets: Vec<f64>,
    coefficients: Vec<f64>,
    options: PreparedLsmrOptions,
    last_work: PreparedLsmrWorkReport,
}
impl std::fmt::Debug for PreparedLsmrWorkspace<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedLsmrWorkspace")
            .field("options", &self.options)
            .field("dimension", &self.coefficients.len())
            .field("tuples", &self.weighted_targets.len())
            .field("last_work", &self.last_work)
            .finish_non_exhaustive()
    }
}
impl<'owner> PreparedLsmrWorkspace<'owner> {
    /// Fallibly allocate complete scratch using the supplied fixed configuration.
    pub fn try_new(
        hierarchy: &'owner PreparedMapHierarchy<'owner>,
        options: PreparedLsmrOptions,
    ) -> Result<Self, MultiwayError> {
        Self::try_new_with_payload_budget(hierarchy, options, usize::MAX, 0)
    }
    /// Admit complete requested live payload before any scratch reservation.
    ///
    /// Includes immutable owners and all new mutable arrays. Add other live
    /// buffers/generations explicitly. This is not an allocator quota or RSS cap.
    pub fn try_new_with_payload_budget(
        hierarchy: &'owner PreparedMapHierarchy<'owner>,
        options: PreparedLsmrOptions,
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
            recurrence: MlsmrWorkspace::try_new(
                fine.weights().len(),
                fine.diagonal().len(),
                options.local_size,
            )
            .map_err(MultiwayError::PreparedLsmr)?,
            projection: fine.topology().try_projection_workspace()?,
            certificate: PreparedCertificateWorkspace::try_new(fine)?,
            weighted_targets: vector(fine.weights().len())?,
            coefficients: vector(fine.diagonal().len())?,
            options,
            last_work: PreparedLsmrWorkReport::default(),
        })
    }
    /// Complete requested construction payload including exact immutable owners.
    pub fn setup_payload_bound(
        hierarchy: &PreparedMapHierarchy<'_>,
        options: PreparedLsmrOptions,
        additional_live_payload_bytes: usize,
    ) -> Result<usize, MultiwayError> {
        let options = options.validate()?;
        let frames = hierarchy.frames();
        let fine = frames.fine();
        let mut total = additional_live_payload_bytes;
        for part in [
            frames.hierarchy().fine().retained_payload_bytes()?,
            frames.hierarchy().retained_payload_bytes()?,
            fine.retained_payload_bytes()?,
            frames.retained_payload_bytes()?,
            hierarchy.retained_payload_bytes()?,
            hierarchy.workspace_required_bytes()?,
            fine.topology().projection_workspace_required_bytes()?,
            PreparedCertificateWorkspace::required_payload_bytes(fine)?,
            bytes(fine.weights().len())?,
            bytes(fine.diagonal().len())?,
            MlsmrWorkspace::required_payload_bytes(
                fine.weights().len(),
                fine.diagonal().len(),
                options.local_size,
            )
            .map_err(MultiwayError::PreparedLsmr)?,
        ] {
            total = add(total, part)?;
        }
        Ok(total)
    }
    /// Fixed solver and acceptance configuration.
    #[must_use]
    pub const fn options(&self) -> PreparedLsmrOptions {
        self.options
    }
    /// Actual work for the last solve admitted past static validation, even on error.
    #[must_use]
    pub const fn last_work(&self) -> PreparedLsmrWorkReport {
        self.last_work
    }
    /// Exact hierarchy validation, without preparation or allocation.
    pub fn validate_for(&self, hierarchy: &PreparedMapHierarchy<'_>) -> Result<(), MultiwayError> {
        if !core::ptr::eq(self.hierarchy, hierarchy) {
            return Err(MultiwayError::WorkspaceNotPrepared {
                context: "prepared LSMR hierarchy",
            });
        }
        Ok(())
    }
    /// Complete exclusive scratch capacities, including the hierarchy application.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        add(
            self.outer_payload_bytes()?,
            self.hierarchy_scratch.retained_payload_bytes()?,
        )
    }
    /// Full retained lifetime inventory with explicitly declared caller state.
    pub fn payload_report(
        &self,
        additional_live_payload_bytes: usize,
    ) -> Result<PreparedLsmrPayloadReport, MultiwayError> {
        let hierarchy = self.hierarchy.payload_report(&self.hierarchy_scratch, 0)?;
        let outer_workspace_payload_bytes = self.outer_payload_bytes()?;
        let total_payload_bytes = add(
            add(hierarchy.total_payload_bytes, outer_workspace_payload_bytes)?,
            additional_live_payload_bytes,
        )?;
        Ok(PreparedLsmrPayloadReport {
            hierarchy,
            outer_workspace_payload_bytes,
            additional_live_payload_bytes,
            total_payload_bytes,
        })
    }
    fn outer_payload_bytes(&self) -> Result<usize, MultiwayError> {
        let mut total = 0;
        for part in [
            self.recurrence
                .retained_payload_bytes()
                .map_err(MultiwayError::PreparedLsmr)?,
            self.projection.retained_payload_bytes()?,
            self.certificate.retained_payload_bytes()?,
            bytes(self.weighted_targets.capacity())?,
            bytes(self.coefficients.capacity())?,
        ] {
            total = add(total, part)?;
        }
        Ok(total)
    }
}

/// Solve one canonical tuple-target column and independently certify its candidate.
///
/// Exact hierarchy and input layout/finite values are checked before state changes.
/// A successful return may have `accepted=false`; native stopping is preserved
/// separately. Numerical errors return no candidate and the same workspace can
/// solve a later RHS. Action counts remain available after admitted failures.
pub fn solve_prepared_least_squares<'workspace>(
    hierarchy: &PreparedMapHierarchy<'_>,
    targets: &[f64],
    workspace: &'workspace mut PreparedLsmrWorkspace<'_>,
) -> Result<PreparedLsmrResult<'workspace>, MultiwayError> {
    workspace.validate_for(hierarchy)?;
    let fine = workspace.hierarchy.frames().fine();
    if targets.len() != fine.weights().len() {
        return Err(crate::error::dimension(
            "prepared LSMR targets",
            fine.weights().len(),
            targets.len(),
        ));
    }
    ensure_finite(targets, "prepared LSMR targets")?;
    workspace.last_work = PreparedLsmrWorkReport::default();
    for ((out, &y), &root) in workspace
        .weighted_targets
        .iter_mut()
        .zip(targets)
        .zip(fine.square_root_weights())
    {
        *out = y * root;
    }
    ensure_finite(
        &workspace.weighted_targets,
        "prepared LSMR weighted targets",
    )?;
    let mut operator = IncidenceAction {
        view: fine.operator_view(),
        forward: 0,
        adjoint: 0,
        error: None,
    };
    let mut preconditioner = HierarchyAction {
        hierarchy: workspace.hierarchy,
        scratch: &mut workspace.hierarchy_scratch,
        count: 0,
        error: None,
    };
    let candidate = mlsmr_with_workspace(
        &mut operator,
        &workspace.weighted_targets,
        &mut preconditioner,
        workspace.options.tolerance,
        workspace.options.max_iterations,
        MlsmrWorkspaceOptions::default(),
        &mut workspace.recurrence,
    );
    workspace.last_work.weighted_incidence_applications = operator.forward;
    workspace.last_work.weighted_adjoint_applications = operator.adjoint;
    workspace.last_work.hierarchy_applications = preconditioner.count;
    if let Some(error) = operator
        .error
        .take()
        .or_else(|| preconditioner.error.take())
    {
        return Err(error);
    }
    let candidate = candidate.map_err(MultiwayError::PreparedLsmr)?;
    let native = candidate.diagnostics;
    workspace.coefficients.copy_from_slice(candidate.x);
    fine.topology().project_structural_range_with_workspace(
        &mut workspace.coefficients,
        &mut workspace.projection,
    )?;
    ensure_finite(&workspace.coefficients, "prepared LSMR projected candidate")?;
    let certificate = certify_prepared_normal_equations(
        fine.operator_view(),
        targets,
        &workspace.coefficients,
        &mut workspace.certificate,
    );
    workspace.last_work.certificate = workspace.certificate.last_work();
    let certificate = certificate?;
    let report = PreparedLsmrReport {
        native_converged: native.converged,
        native_stop_reason: crate::lsmr::convert_stop_reason(native.stop_reason),
        iterations: native.iterations,
        native_residual_norm: native.residual_norm,
        native_normal_equation_residual: native.normal_eq_residual,
        certified_normal_equation_residual: certificate,
        accepted: certificate <= workspace.options.certificate_tolerance,
        work: workspace.last_work,
    };
    Ok(PreparedLsmrResult {
        coefficients: &workspace.coefficients,
        report,
    })
}

struct IncidenceAction<'state> {
    view: ThreeWayOperatorView<'state, 'state>,
    forward: usize,
    adjoint: usize,
    error: Option<MultiwayError>,
}
impl OperatorMut for IncidenceAction<'_> {
    fn nrows(&self) -> usize {
        self.view.tuple_count()
    }
    fn ncols(&self) -> usize {
        self.view.dimension()
    }
    fn apply(&mut self, x: &[f64], out: &mut [f64]) -> Result<(), SolveError> {
        self.forward += 1;
        capture(
            self.view
                .apply_weighted_incidence(x, out)
                .map_err(Into::into),
            &mut self.error,
        )
    }
    fn apply_adjoint(&mut self, x: &[f64], out: &mut [f64]) -> Result<(), SolveError> {
        self.adjoint += 1;
        capture(
            self.view.apply_weighted_adjoint(x, out).map_err(Into::into),
            &mut self.error,
        )
    }
}
struct HierarchyAction<'borrow, 'owner> {
    hierarchy: &'owner PreparedMapHierarchy<'owner>,
    scratch: &'borrow mut PreparedHierarchyWorkspace<'owner>,
    count: usize,
    error: Option<MultiwayError>,
}
impl OperatorMut for HierarchyAction<'_, '_> {
    fn nrows(&self) -> usize {
        self.hierarchy.dimension()
    }
    fn ncols(&self) -> usize {
        self.hierarchy.dimension()
    }
    fn apply(&mut self, x: &[f64], out: &mut [f64]) -> Result<(), SolveError> {
        self.count += 1;
        capture(
            self.hierarchy.apply_with_workspace(x, out, self.scratch),
            &mut self.error,
        )
    }
    fn apply_adjoint(&mut self, x: &[f64], out: &mut [f64]) -> Result<(), SolveError> {
        self.apply(x, out)
    }
}
fn capture(
    result: Result<(), MultiwayError>,
    error: &mut Option<MultiwayError>,
) -> Result<(), SolveError> {
    result.map_err(|value| {
        *error = Some(value);
        SolveError::Synchronization {
            context: "prepared MultiwayMG action",
        }
    })
}
fn add(a: usize, b: usize) -> Result<usize, MultiwayError> {
    a.checked_add(b)
        .ok_or(MultiwayError::WorkspaceSizeOverflow {
            context: "prepared LSMR payload",
        })
}

/// Solve 1–32 independent RHS columns serially with one bounded scratch owner.
///
/// Inputs and coefficient outputs are column-major. All layouts and finite input
/// values are validated before output/reports/scratch mutation. Reports are then
/// cleared to `None`. Each completed candidate is copied and receives `Some(report)`;
/// callers must inspect its `accepted` flag. A numerical error leaves the completed
/// prefix in place and the failed/unprocessed output columns unchanged. The first
/// `None` identifies the failed column and `workspace.last_work()` records its work.
/// No allocation or concurrency is introduced by this scalar scheduling wrapper.
///
/// This is independent scalar reuse, not fused panels or a block Krylov algorithm.
pub fn solve_prepared_least_squares_batch_into(
    hierarchy: &PreparedMapHierarchy<'_>,
    targets: &[f64],
    columns: usize,
    coefficients: &mut [f64],
    reports: &mut [Option<PreparedLsmrReport>],
    workspace: &mut PreparedLsmrWorkspace<'_>,
) -> Result<(), MultiwayError> {
    workspace.validate_for(hierarchy)?;
    if !(1..=32).contains(&columns) {
        return Err(MultiwayError::DimensionMismatch {
            context: "prepared LSMR RHS count (1..=32)",
            expected: 32,
            actual: columns,
        });
    }
    let rows = hierarchy.frames().fine().weights().len();
    let dimension = hierarchy.dimension();
    let target_length = rows
        .checked_mul(columns)
        .ok_or(MultiwayError::WorkspaceSizeOverflow {
            context: "prepared RHS layout",
        })?;
    let coefficient_length =
        dimension
            .checked_mul(columns)
            .ok_or(MultiwayError::WorkspaceSizeOverflow {
                context: "prepared coefficient layout",
            })?;
    for (context, expected, actual) in [
        ("prepared RHS panel", target_length, targets.len()),
        (
            "prepared coefficient panel",
            coefficient_length,
            coefficients.len(),
        ),
        ("prepared RHS reports", columns, reports.len()),
    ] {
        if expected != actual {
            return Err(crate::error::dimension(context, expected, actual));
        }
    }
    ensure_finite(targets, "prepared RHS panel")?;
    reports.fill(None);
    for (column, report) in reports.iter_mut().enumerate() {
        let result = solve_prepared_least_squares(
            hierarchy,
            &targets[column * rows..(column + 1) * rows],
            workspace,
        )?;
        coefficients[column * dimension..(column + 1) * dimension]
            .copy_from_slice(result.coefficients);
        *report = Some(result.report);
    }
    Ok(())
}
