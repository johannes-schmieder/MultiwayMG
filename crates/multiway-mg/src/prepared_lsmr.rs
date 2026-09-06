//! Complete serial prepared LSMR with independent original-operator acceptance.
use crate::{
    CertificateWorkReport, LeastSquaresStopReason, MultiwayError, PreparedCertificateWorkspace,
    PreparedHierarchyPayloadReport, PreparedHierarchyWorkspace, PreparedMapHierarchy,
    certificate::{bytes, ensure_finite, vector},
    certify_prepared_normal_equations,
};
use multiway_incidence::{PreparedStructuralProjectionWorkspace, ThreeWayOperatorView};
use schwarz_precond::{
    LsmrCandidateGate, MlsmrWorkspace, MlsmrWorkspaceOptions, OperatorMut, SolveError,
    mlsmr_with_workspace, mlsmr_with_workspace_and_candidate_gate,
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

/// Additional original-operator gate work, including admitted failed attempts.
///
/// Native operator/hierarchy actions and the final certificate remain in
/// [`PreparedLsmrWorkReport`]. Add this candidate certificate work to that final
/// certificate inventory. Projection counts include candidate and final outer
/// projections, excluding projections inside the fixed hierarchy.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PreparedLsmrGateWorkReport {
    /// Resumable native tolerance candidates checked by the original certificate.
    pub candidate_checks: usize,
    /// Candidates whose finite certificate required continuing the same recurrence.
    pub candidate_vetoes: usize,
    /// All attempted outer structural projections, including the final candidate.
    pub projection_applications: usize,
    /// Additional candidate certificates; the final certificate is counted separately.
    pub candidate_certificate: CertificateWorkReport,
}
/// Complete native diagnostics, independent final acceptance and continuation work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedGatedLsmrReport {
    /// Native solve/final original certificate; inspect `solve.accepted` before use.
    pub solve: PreparedLsmrReport,
    /// Extra gate work and outer projections, including all candidate vetoes.
    pub gate: PreparedLsmrGateWorkReport,
}
/// Borrowed result from original-certificate-gated serial LSMR.
#[derive(Debug)]
pub struct PreparedGatedLsmrResult<'workspace> {
    /// Structurally projected coefficients; final acceptance is separate.
    pub coefficients: &'workspace [f64],
    /// Complete diagnostics, acceptance and gate work.
    pub report: PreparedGatedLsmrReport,
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
    last_gate_work: PreparedLsmrGateWorkReport,
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
            last_gate_work: PreparedLsmrGateWorkReport::default(),
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
    /// Extra gate/candidate work and outer projections from the last admitted solve.
    ///
    /// Native solves perform no candidate checks but still record their final
    /// outer projection here. Static rejection preserves both work inventories.
    #[must_use]
    pub const fn last_gate_work(&self) -> PreparedLsmrGateWorkReport {
        self.last_gate_work
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
    let report = solve_impl(hierarchy, targets, workspace, false)?;
    Ok(PreparedLsmrResult {
        coefficients: &workspace.coefficients,
        report,
    })
}

/// Continue resumable native LSMR stops until the original certificate allows stopping.
///
/// Uses exactly the same native recurrence, tolerances, history and workspace.
/// A veto preserves the current Krylov state; it does not restart or reprepare.
/// Each gate projects into existing candidate scratch and certifies against the
/// exact submitted tuple operator. All gate actions are counted separately.
///
/// Limits, exact breakdown and non-resumable native exits still return candidates.
/// Every exit gets a fresh independent final certificate; inspect
/// `report.solve.accepted`. Errors publish no candidate and preserve attempted
/// work. The ordinary native route remains available without gate checks.
pub fn solve_prepared_least_squares_with_certificate_gate<'workspace>(
    hierarchy: &PreparedMapHierarchy<'_>,
    targets: &[f64],
    workspace: &'workspace mut PreparedLsmrWorkspace<'_>,
) -> Result<PreparedGatedLsmrResult<'workspace>, MultiwayError> {
    let solve = solve_impl(hierarchy, targets, workspace, true)?;
    Ok(PreparedGatedLsmrResult {
        coefficients: &workspace.coefficients,
        report: PreparedGatedLsmrReport {
            solve,
            gate: workspace.last_gate_work,
        },
    })
}

fn solve_impl(
    hierarchy: &PreparedMapHierarchy<'_>,
    targets: &[f64],
    workspace: &mut PreparedLsmrWorkspace<'_>,
    use_gate: bool,
) -> Result<PreparedLsmrReport, MultiwayError> {
    #[cfg(feature = "profiling")]
    let _profile_span =
        multiway_incidence::profiling::span(multiway_incidence::profiling::Phase::PreparedLsmr);

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
    workspace.last_gate_work = PreparedLsmrGateWorkReport::default();
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
    let mut gate_error = None;
    let candidate = if use_gate {
        let mut gate = OriginalCertificateGate {
            view: fine.operator_view(),
            targets,
            coefficients: &mut workspace.coefficients,
            projection: &mut workspace.projection,
            certificate: &mut workspace.certificate,
            tolerance: workspace.options.certificate_tolerance,
            work: &mut workspace.last_gate_work,
            error: None,
        };
        let result = mlsmr_with_workspace_and_candidate_gate(
            &mut operator,
            &workspace.weighted_targets,
            &mut preconditioner,
            workspace.options.tolerance,
            workspace.options.max_iterations,
            MlsmrWorkspaceOptions::default(),
            &mut gate,
            &mut workspace.recurrence,
        );
        gate_error = gate.error.take();
        result
    } else {
        mlsmr_with_workspace(
            &mut operator,
            &workspace.weighted_targets,
            &mut preconditioner,
            workspace.options.tolerance,
            workspace.options.max_iterations,
            MlsmrWorkspaceOptions::default(),
            &mut workspace.recurrence,
        )
    };
    workspace.last_work.weighted_incidence_applications = operator.forward;
    workspace.last_work.weighted_adjoint_applications = operator.adjoint;
    workspace.last_work.hierarchy_applications = preconditioner.count;
    if let Some(error) = operator
        .error
        .take()
        .or_else(|| preconditioner.error.take())
        .or(gate_error)
    {
        return Err(error);
    }
    let candidate = candidate.map_err(MultiwayError::PreparedLsmr)?;
    let native = candidate.diagnostics;
    workspace.coefficients.copy_from_slice(candidate.x);
    workspace.last_gate_work.projection_applications =
        work_add(workspace.last_gate_work.projection_applications, 1)?;
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
    Ok(PreparedLsmrReport {
        native_converged: native.converged,
        native_stop_reason: crate::lsmr::convert_stop_reason(native.stop_reason),
        iterations: native.iterations,
        native_residual_norm: native.residual_norm,
        native_normal_equation_residual: native.normal_eq_residual,
        certified_normal_equation_residual: certificate,
        accepted: certificate <= workspace.options.certificate_tolerance,
        work: workspace.last_work,
    })
}

struct OriginalCertificateGate<'borrow, 'owner> {
    view: ThreeWayOperatorView<'owner, 'owner>,
    targets: &'borrow [f64],
    coefficients: &'borrow mut [f64],
    projection: &'borrow mut PreparedStructuralProjectionWorkspace<'owner>,
    certificate: &'borrow mut PreparedCertificateWorkspace<'owner, 'owner>,
    tolerance: f64,
    work: &'borrow mut PreparedLsmrGateWorkReport,
    error: Option<MultiwayError>,
}
impl OriginalCertificateGate<'_, '_> {
    fn check(&mut self, correction: &[f64], offset: Option<&[f64]>) -> Result<bool, MultiwayError> {
        self.work.candidate_checks = work_add(self.work.candidate_checks, 1)?;
        ensure_finite(correction, "prepared LSMR gate candidate")?;
        if let Some(offset) = offset {
            for (i, value) in self.coefficients.iter_mut().enumerate() {
                *value = correction[i] + offset[i];
            }
        } else {
            self.coefficients.copy_from_slice(correction);
        }
        self.work.projection_applications = work_add(self.work.projection_applications, 1)?;
        self.view
            .frame()
            .topology()
            .project_structural_range_with_workspace(self.coefficients, self.projection)?;
        ensure_finite(self.coefficients, "prepared LSMR projected gate candidate")?;
        let result = certify_prepared_normal_equations(
            self.view,
            self.targets,
            self.coefficients,
            self.certificate,
        );
        let work = self.certificate.last_work();
        self.work.candidate_certificate.incidence_applications = work_add(
            self.work.candidate_certificate.incidence_applications,
            work.incidence_applications,
        )?;
        self.work.candidate_certificate.adjoint_applications = work_add(
            self.work.candidate_certificate.adjoint_applications,
            work.adjoint_applications,
        )?;
        let allowed = result? <= self.tolerance;
        if !allowed {
            self.work.candidate_vetoes = work_add(self.work.candidate_vetoes, 1)?;
        }
        Ok(allowed)
    }
}
impl LsmrCandidateGate for OriginalCertificateGate<'_, '_> {
    fn allow_stop(
        &mut self,
        correction: &[f64],
        offset: Option<&[f64]>,
    ) -> Result<bool, SolveError> {
        self.check(correction, offset).map_err(|error| {
            self.error = Some(error);
            SolveError::Synchronization {
                context: "prepared LSMR original certificate gate",
            }
        })
    }
}
fn work_add(a: usize, b: usize) -> Result<usize, MultiwayError> {
    a.checked_add(b)
        .ok_or(MultiwayError::WorkspaceSizeOverflow {
            context: "prepared LSMR work counter",
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
    let (rows, dimension) = validate_batch(
        hierarchy,
        targets,
        columns,
        coefficients,
        reports.len(),
        workspace,
    )?;
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

/// Bounded 1–32 RHS scalar reuse with original-certificate-gated continuation.
///
/// Static validation precedes mutation. Completed candidates/reports are retained
/// even when acceptance is false; inspect `report.solve.accepted`. On an error,
/// the failed/unprocessed output suffix is unchanged, its reports are `None`,
/// and both workspace work getters retain the failed attempt. No allocation or
/// parallelism is introduced; targets/coefficients are column-major.
pub fn solve_prepared_least_squares_with_certificate_gate_batch_into(
    hierarchy: &PreparedMapHierarchy<'_>,
    targets: &[f64],
    columns: usize,
    coefficients: &mut [f64],
    reports: &mut [Option<PreparedGatedLsmrReport>],
    workspace: &mut PreparedLsmrWorkspace<'_>,
) -> Result<(), MultiwayError> {
    let (rows, dimension) = validate_batch(
        hierarchy,
        targets,
        columns,
        coefficients,
        reports.len(),
        workspace,
    )?;
    reports.fill(None);
    for (column, report) in reports.iter_mut().enumerate() {
        let result = solve_prepared_least_squares_with_certificate_gate(
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
fn validate_batch(
    hierarchy: &PreparedMapHierarchy<'_>,
    targets: &[f64],
    columns: usize,
    coefficients: &[f64],
    reports_length: usize,
    workspace: &PreparedLsmrWorkspace<'_>,
) -> Result<(usize, usize), MultiwayError> {
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
        ("prepared RHS reports", columns, reports_length),
    ] {
        if expected != actual {
            return Err(crate::error::dimension(context, expected, actual));
        }
    }
    ensure_finite(targets, "prepared RHS panel")?;
    Ok((rows, dimension))
}

#[cfg(test)]
mod gate_tests {
    use super::*;
    use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};

    #[test]
    fn gate_errors_preserve_original_type_and_count_attempted_work() {
        let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
        let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
        let mut projection = topology.try_projection_workspace().unwrap();
        let mut certificate = PreparedCertificateWorkspace::try_new(&fine).unwrap();
        let mut coefficients = [0.0; 3];
        let mut work = PreparedLsmrGateWorkReport::default();
        let mut gate = OriginalCertificateGate {
            view: fine.operator_view(),
            targets: &[0.0],
            coefficients: &mut coefficients,
            projection: &mut projection,
            certificate: &mut certificate,
            tolerance: 1e-8,
            work: &mut work,
            error: None,
        };
        assert!(gate.allow_stop(&[f64::NAN, 0.0, 0.0], None).is_err());
        assert!(matches!(
            gate.error.take(),
            Some(MultiwayError::NumericalFailure {
                context: "prepared LSMR gate candidate"
            })
        ));
        assert_eq!(gate.work.candidate_checks, 1);
        assert_eq!(gate.work.projection_applications, 0);
        assert_eq!(
            gate.work.candidate_certificate,
            CertificateWorkReport::default()
        );
        assert!(
            gate.allow_stop(&[f64::MAX, f64::MAX, -f64::MAX], None)
                .is_err()
        );
        assert!(matches!(
            gate.error.take(),
            Some(MultiwayError::NumericalFailure {
                context: "prepared LSMR projected gate candidate"
            })
        ));
        assert_eq!(gate.work.projection_applications, 1);
        assert_eq!(
            gate.work.candidate_certificate,
            CertificateWorkReport::default()
        );
        assert!(gate.allow_stop(&[f64::MAX; 3], None).is_err());
        assert!(matches!(
            gate.error.take(),
            Some(MultiwayError::NumericalFailure { .. })
        ));
        assert_eq!(gate.work.projection_applications, 2);
        assert_eq!(
            gate.work.candidate_certificate,
            CertificateWorkReport {
                incidence_applications: 1,
                adjoint_applications: 0
            }
        );
        assert!(gate.allow_stop(&[0.0; 3], None).unwrap());
        assert_eq!(gate.work.candidate_checks, 4);
        assert_eq!(gate.work.projection_applications, 3);
        assert_eq!(
            gate.work.candidate_certificate,
            CertificateWorkReport {
                incidence_applications: 2,
                adjoint_applications: 2
            }
        );
        // Finite vectors can still fail the original certificate's representability boundary.
        gate.targets = &[f64::MAX];
        assert!(gate.allow_stop(&[0.0; 3], None).is_err());
        assert!(matches!(
            gate.error.take(),
            Some(MultiwayError::NumericalFailure { .. })
        ));
        assert_eq!(
            gate.work.candidate_certificate,
            CertificateWorkReport {
                incidence_applications: 3,
                adjoint_applications: 4
            }
        );
        gate.targets = &[0.0];
        gate.work.candidate_checks = usize::MAX;
        assert!(gate.allow_stop(&[0.0; 3], None).is_err());
        assert!(matches!(
            gate.error.take(),
            Some(MultiwayError::WorkspaceSizeOverflow {
                context: "prepared LSMR work counter"
            })
        ));
    }
}
