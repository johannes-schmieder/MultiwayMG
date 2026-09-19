//! Explicit terminal-first one-shot batches with original-problem acceptance.
use crate::{
    CertificateWorkReport, CycleQualityCriteria, CycleQualityOptions, DensePseudoinverse,
    MultiwayError, PREPARED_DENSE_TERMINAL_LIMIT, PairNeighborhoodAggregationOptions,
    PreparedBaseline, PreparedBaselineKind, PreparedCertificateWorkspace,
    PreparedLsmrGateWorkReport, PreparedLsmrOptions, PreparedLsmrWorkReport, PreparedLsmrWorkspace,
    PreparedPairProposalCoverage, PreparedSolverAction,
    certificate::{bytes, ensure_finite, vector},
    certify_prepared_normal_equations, solve_prepared_least_squares_with_certificate_gate,
};
use multiway_incidence::{
    PreparedComponentLayout, PreparedComponentRecoding, PreparedComponentRoot,
    PreparedComponentView, PreparedHierarchyBudget, ThreeWayWeightFrame,
};
mod hierarchy;
mod layout;
pub use layout::{
    PreparedAutomaticGroupingLocation, PreparedAutomaticGroupingScope, PreparedAutomaticLayout,
    PreparedAutomaticLayoutProgress,
};

struct Progress<'a> {
    inner: &'a mut PreparedAutomaticProgress,
    layout: &'a mut PreparedAutomaticLayoutProgress,
}
impl std::ops::Deref for Progress<'_> {
    type Target = PreparedAutomaticProgress;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}
impl std::ops::DerefMut for Progress<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

/// Explicit construction/screening policy for large components; no timed routing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedAutomaticHierarchyOptions {
    /// At most 63 transitions; no sparse unqualified terminal is substituted.
    pub maximum_transitions: usize,
    /// Sum of fine/coarse tuple counts may not exceed this integer times fine E.
    pub maximum_tuple_multiplier: usize,
    /// Sum of fine/coarse dimensions may not exceed this integer times fine V.
    pub maximum_coefficient_multiplier: usize,
    /// Explicit legacy or complete-row candidate coverage.
    pub coverage: PreparedPairProposalCoverage,
    /// Explicit affinity and legacy neighbor cap; adjacent uses only affinity.
    pub candidate: PairNeighborhoodAggregationOptions,
    /// Bounded deterministic starts/iterations on actual recursive tails.
    pub screen: CycleQualityOptions,
    /// Predeclared quality gates, separate from final solve acceptance.
    pub criteria: CycleQualityCriteria,
}
/// Explicit one-shot policy; defaults are deliberately not selected here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedAutomaticOptions {
    /// None uses the selected baseline directly on large components.
    pub hierarchy: Option<PreparedAutomaticHierarchyOptions>,
    /// Current-generation terminal rank threshold; hard dimension cap is 256.
    pub terminal_relative_tolerance: f64,
    /// Fixed large-component and final original-problem fallback action.
    pub fallback: PreparedBaselineKind,
    /// Native/gated LSMR configuration and authoritative original tolerance.
    /// This bounded one-shot entry caps maximum iterations at 1,000,000.
    pub lsmr: PreparedLsmrOptions,
}

/// Column-major canonical tuple targets and global coefficient outputs, K=1..32.
///
/// Static validation precedes any mutation. During execution, incomplete outputs
/// may contain partial component coefficients or normal-RHS scratch. Consume a
/// column only when its report is Some and accepted. Existing prepared scalar
/// batch APIs keep their stronger failed-output preservation contract unchanged.
#[derive(Debug)]
pub struct PreparedAutomaticBatch<'a> {
    /// K complete canonical tuple target columns.
    pub targets: &'a [f64],
    /// Number of independent columns, one through 32.
    pub columns: usize,
    /// K complete original factor-major coefficient columns.
    pub coefficients: &'a mut [f64],
    /// Exactly K acceptance records; None means no complete certificate yet.
    pub reports: &'a mut [Option<PreparedAutomaticColumnReport>],
}
/// Original full-problem certificate; local acceptance alone cannot publish this.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedAutomaticColumnReport {
    /// Initial full-problem certificate, when component execution reached it.
    pub initial_certificate: Option<f64>,
    /// Certificate of the candidate currently stored in the output column.
    pub certified_normal_equation_residual: f64,
    /// Whether that certificate meets the explicit original tolerance.
    pub accepted: bool,
    /// Whether this column required a final whole-original baseline solve.
    pub global_fallback: bool,
}
/// Last entered stage; counters retain admitted attempts, including failures.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PreparedAutomaticStage {
    /// Static validation has not admitted execution.
    #[default]
    Validation,
    /// Flat original component layout or inverse preparation.
    Partition,
    /// Component-local structural/numerical input materialization.
    LocalRoot,
    /// Direct current small-component dense factorization and application.
    DenseTerminal,
    /// Bounded structural candidates and provisional frames.
    Construction,
    /// Fresh complete numerical replay and bounded dense terminal.
    NumericalReplay,
    /// Actual recursive-tail quality screening.
    Screening,
    /// Complete local hierarchy or baseline solver execution.
    ComponentSolve,
    /// Independent original full-problem projection and certification.
    Certification,
    /// Final baseline fallback on the original full problem.
    GlobalFallback,
    /// Every requested output has a completed original certificate.
    Complete,
}
/// Bounded retained failure detail; earlier rejections remain in aggregate counts.
#[derive(Debug)]
pub struct PreparedAutomaticRejection {
    /// Component label, or None for full-problem setup/certification.
    pub component: Option<usize>,
    /// Last entered stage at this rejection.
    pub stage: PreparedAutomaticStage,
    /// Original typed cause; successful fallback does not erase it.
    pub source: MultiwayError,
}
/// Bounded execution/accounting record; no per-component descriptor array.
///
/// Array payload includes source/frame, target/output/report slices and explicitly
/// declared other live state. Add unused caller capacities to the budget's other
/// category. Inline roots, fixed dense 6KiB stack scratch, allocator metadata and
/// new allocator excess are excluded; external total time and RSS must still be
/// measured. The report keeps the last typed rejection and complete aggregate
/// counts, not an unbounded event history. Native action counts do not expand
/// internal hierarchy traversal; all actual failed work remains in total timing.
#[derive(Debug, Default)]
pub struct PreparedAutomaticProgress {
    /// Last entered stage, retained on a returned error.
    pub stage: PreparedAutomaticStage,
    /// Component currently processed, or None for the full original problem.
    pub component: Option<usize>,
    /// Current RHS, or None during setup.
    pub column: Option<usize>,
    /// Original supported incidence components; not numerical rank.
    pub components: usize,
    /// Components executed analytically without a local numerical owner.
    pub singleton_components: usize,
    /// Entered bounded small-component dense factorization attempts.
    pub dense_components: usize,
    /// Entered large component routes.
    pub large_components: usize,
    /// Entered complete automatic hierarchy attempts.
    pub hierarchy_attempts: usize,
    /// Fully screened hierarchies selected for component solves.
    pub accepted_hierarchies: usize,
    /// Rejected complete hierarchy attempts, including quality/numerical failure.
    pub hierarchy_rejections: usize,
    /// Explicit quality-criterion rejections among those attempts.
    pub quality_rejections: usize,
    /// Components routed to the selected fixed baseline.
    pub baseline_components: usize,
    /// RHS columns entering a final original-problem baseline solve.
    pub global_fallback_columns: usize,
    /// All retained/reported rejected attempts, including component/final gates.
    pub rejections: usize,
    /// Largest attempted requested payload, including over-budget requests.
    pub maximum_requested_payload_bytes: usize,
    /// Largest admitted requested or checked actual-retained payload.
    pub maximum_admitted_payload_bytes: usize,
    /// Complete sparse candidate work, including rejected construction.
    pub candidate_work: crate::PreparedAggregationWork,
    /// Entered structural appends, including a rejected extension.
    pub structural_attempts: usize,
    /// Tuple inputs submitted to those appends; not exact CPU work.
    pub structural_input_tuples: usize,
    /// Provisional replay input tuples, including admitted failed attempts.
    pub provisional_input_tuples: usize,
    /// Actual outer actions entered by recursive quality screens.
    pub screen_work: crate::PreparedCycleScreenWork,
    /// Last quality-criterion rejection: original component label and rejected tail.
    pub last_quality_rejection: Option<(usize, crate::PreparedLevelCycleQuality)>,
    /// Explicit global structural projections before initial certification.
    pub global_projection_applications: usize,
    /// Complete local/global LSMR work, including failed attempts.
    pub solve_work: PreparedLsmrWorkReport,
    /// Candidate-gate work, in addition to native/final-certificate work above.
    pub gate_work: PreparedLsmrGateWorkReport,
    /// Separate global certificates after component assembly.
    pub global_certificate_work: CertificateWorkReport,
    /// Latest typed rejection, preserved even after successful fallback.
    pub last_rejection: Option<PreparedAutomaticRejection>,
}

/// Solve one complete batch with explicit terminal-first component scheduling.
///
/// Process all K RHS for each component before releasing its numerical/Krylov
/// storage. Dense factors and large workspaces are never retained for all
/// components at once. Every complete assembled column gets a fresh original
/// certificate. Rejected construction/screens use a fixed local baseline; a
/// failed component route or rejected global certificate may use one final
/// original-problem baseline attempt. No retries, tolerance changes or elapsed-
/// time decisions occur. A returned error retains progress and report semantics.
pub fn solve_prepared_automatic_batch_into(
    frame: &ThreeWayWeightFrame<'_>,
    batch: PreparedAutomaticBatch<'_>,
    options: PreparedAutomaticOptions,
    budget: PreparedHierarchyBudget,
    progress: &mut PreparedAutomaticProgress,
) -> Result<(), MultiwayError> {
    let mut layout = PreparedAutomaticLayoutProgress::default();
    run(
        frame,
        batch,
        options,
        budget,
        &mut Progress {
            inner: progress,
            layout: &mut layout,
        },
        PreparedAutomaticLayout::Scalar,
    )
}

/// Complete automatic execution with an explicit, separately charged kernel layout.
///
/// Uses the same component, construction, quality and certificate policy as the
/// scalar entry point. Fine/all prefixes affect nonterminal hierarchy levels;
/// large-component and final global baselines group their single fine level.
/// Those LSMR baselines need no Gramian tuple image and allocate none.
/// Dense/singleton execution needs no grouping. There is no layout selector or
/// silent scalar recovery: rejected group setup follows ordinary typed hierarchy/
/// component fallback, and failed final group setup returns its error.
///
/// Groups and at most one shared tuple image belong to the current component.
/// Their capacities, setup cursor and failed attempts are included in admission.
/// Original certificates remain scalar. Both progress records and caller outputs
/// keep their prior values on static validation failure; after admission the
/// ordinary partial-output/accepted-report contract applies.
pub fn solve_prepared_automatic_batch_into_with_layout(
    frame: &ThreeWayWeightFrame<'_>,
    batch: PreparedAutomaticBatch<'_>,
    options: PreparedAutomaticOptions,
    budget: PreparedHierarchyBudget,
    selected_layout: PreparedAutomaticLayout,
    progress: &mut PreparedAutomaticProgress,
    layout_progress: &mut PreparedAutomaticLayoutProgress,
) -> Result<(), MultiwayError> {
    run(
        frame,
        batch,
        options,
        budget,
        &mut Progress {
            inner: progress,
            layout: layout_progress,
        },
        selected_layout,
    )
}

fn run(
    frame: &ThreeWayWeightFrame<'_>,
    mut batch: PreparedAutomaticBatch<'_>,
    options: PreparedAutomaticOptions,
    budget: PreparedHierarchyBudget,
    progress: &mut Progress<'_>,
    selected_layout: PreparedAutomaticLayout,
) -> Result<(), MultiwayError> {
    validate(frame, &batch, options)?;
    let caller = add(
        budget.additional_live_payload_bytes,
        add(
            bytes(batch.targets.len())?,
            add(
                bytes(batch.coefficients.len())?,
                bytes_of::<Option<PreparedAutomaticColumnReport>>(batch.reports.len())?,
            )?,
        )?,
    )?;
    let base = add(caller, fine_payload(frame)?)?;
    *progress.inner = PreparedAutomaticProgress {
        components: frame.topology().component_factor_sizes().len(),
        ..PreparedAutomaticProgress::default()
    };
    *progress.layout = PreparedAutomaticLayoutProgress {
        requested_layout: selected_layout,
        ..PreparedAutomaticLayoutProgress::default()
    };
    batch.reports.fill(None);
    admit(base, budget.maximum_payload_bytes, progress)?;
    let component_result = components(
        frame,
        &mut batch,
        options,
        budget.maximum_payload_bytes,
        caller,
        progress,
    );
    let mut fallback = [false; 32];
    match component_result {
        Ok(()) => {
            progress.component = None;
            progress.column = None;
            progress.stage = PreparedAutomaticStage::Certification;
            match certify_columns(
                frame,
                &mut batch,
                options,
                budget.maximum_payload_bytes,
                base,
                progress,
            ) {
                Ok(rejected) => fallback = rejected,
                Err(source) => {
                    reject(source, progress)?;
                    fallback[..batch.columns].fill(true);
                }
            }
        }
        Err(source) => {
            reject(source, progress)?;
            fallback[..batch.columns].fill(true);
        }
    }
    // All layout/recoder/local owners and initial certificate scratch are dead.
    if fallback[..batch.columns].iter().any(|&x| x) {
        progress.component = None;
        progress.stage = PreparedAutomaticStage::GlobalFallback;
        global_fallback(
            frame,
            &mut batch,
            options,
            budget.maximum_payload_bytes,
            caller,
            &fallback,
            progress,
        )?;
    }
    progress.stage = PreparedAutomaticStage::Complete;
    progress.component = None;
    progress.column = None;
    Ok(())
}

fn validate(
    frame: &ThreeWayWeightFrame<'_>,
    batch: &PreparedAutomaticBatch<'_>,
    options: PreparedAutomaticOptions,
) -> Result<(), MultiwayError> {
    if !(1..=32).contains(&batch.columns) {
        return Err(invalid("automatic RHS count"));
    }
    length(
        "automatic targets",
        mul(frame.weights().len(), batch.columns)?,
        batch.targets.len(),
    )?;
    length(
        "automatic coefficients",
        mul(frame.diagonal().len(), batch.columns)?,
        batch.coefficients.len(),
    )?;
    length("automatic reports", batch.columns, batch.reports.len())?;
    ensure_finite(batch.targets, "automatic targets")?;
    options.lsmr.validate()?;
    if options.lsmr.max_iterations > 1_000_000 {
        return Err(invalid("automatic iteration cap"));
    }
    if !options.terminal_relative_tolerance.is_finite() || options.terminal_relative_tolerance <= 0.
    {
        return Err(invalid("automatic terminal rank tolerance"));
    }
    if let Some(h) = options.hierarchy {
        if h.maximum_transitions > 63
            || h.maximum_tuple_multiplier == 0
            || h.maximum_coefficient_multiplier == 0
        {
            return Err(invalid("automatic hierarchy limits"));
        }
        if !h.candidate.minimum_affinity.is_finite()
            || !(0.0..=1.0).contains(&h.candidate.minimum_affinity)
            || h.candidate.maximum_neighbor_degree < 2
        {
            return Err(invalid("automatic candidates"));
        }
        crate::prepared_cycle_quality::validate(h.screen, h.criteria)?;
    }
    Ok(())
}

fn components(
    frame: &ThreeWayWeightFrame<'_>,
    batch: &mut PreparedAutomaticBatch<'_>,
    options: PreparedAutomaticOptions,
    maximum: usize,
    caller: usize,
    progress: &mut Progress<'_>,
) -> Result<(), MultiwayError> {
    let topology = frame.topology();
    if progress.components == frame.weights().len() {
        // Every supported component has >=1 tuple, hence all have exactly one.
        for id in 0..frame.weights().len() {
            singleton(frame, batch, id);
        }
        progress.singleton_components = progress.components;
        return Ok(());
    }
    progress.stage = PreparedAutomaticStage::Partition;
    let b = payload_budget(maximum, add(caller, frame.retained_payload_bytes()?)?);
    admit(
        PreparedComponentLayout::setup_payload_bound(topology, b)?,
        maximum,
        progress,
    )?;
    let layout = PreparedComponentLayout::try_new(topology, b)?;
    admit(
        add(
            add(caller, fine_payload(frame)?)?,
            layout.retained_payload_bytes()?,
        )?,
        maximum,
        progress,
    )?;
    admit(
        PreparedComponentRecoding::setup_payload_bound(&layout, b)?,
        maximum,
        progress,
    )?;
    let recoding = PreparedComponentRecoding::try_new(&layout, b)?;
    let mappings = add(
        layout.retained_payload_bytes()?,
        recoding.retained_payload_bytes()?,
    )?;
    let outer = add(caller, mappings)?;
    let original_live = add(outer, fine_payload(frame)?)?;
    admit(original_live, maximum, progress)?;
    for c in 0..layout.component_count() {
        progress.component = Some(c);
        progress.column = None;
        let view = layout.component(c).expect("bounded component index");
        if view.tuple_count() == 1 {
            view.tuple_ids().for_each(|id| singleton(frame, batch, id));
            increment(&mut progress.singleton_components, 1)?;
        } else if view.dimension() <= PREPARED_DENSE_TERMINAL_LIMIT {
            progress.stage = PreparedAutomaticStage::DenseTerminal;
            increment(&mut progress.dense_components, 1)?;
            admit(
                add(
                    original_live,
                    DensePseudoinverse::factorization_payload_bound(view.dimension())?,
                )?,
                maximum,
                progress,
            )?;
            let terminal = DensePseudoinverse::from_component(
                frame,
                &recoding,
                c,
                options.terminal_relative_tolerance,
            )?;
            admit(
                add(original_live, terminal.retained_payload_bytes()?)?,
                maximum,
                progress,
            )?;
            dense_columns(frame, view, &terminal, batch, progress)?;
        } else {
            increment(&mut progress.large_components, 1)?;
            if layout.is_identity() {
                large(frame, None, batch, options, maximum, outer, progress)?;
            } else {
                progress.stage = PreparedAutomaticStage::LocalRoot;
                let b = payload_budget(maximum, add(caller, frame.retained_payload_bytes()?)?);
                admit(
                    PreparedComponentRoot::setup_payload_bound(&recoding, c, b)?,
                    maximum,
                    progress,
                )?;
                let root = PreparedComponentRoot::try_new(&recoding, c, b)?;
                let b = payload_budget(maximum, add(caller, recoding.retained_payload_bytes()?)?);
                admit(
                    root.weight_frame_setup_payload_bound(frame, b)?,
                    maximum,
                    progress,
                )?;
                let local = root.try_weight_frame(frame, b)?;
                admit(
                    add(original_live, fine_payload(&local)?)?,
                    maximum,
                    progress,
                )?;
                large(
                    &local,
                    Some(view),
                    batch,
                    options,
                    maximum,
                    original_live,
                    progress,
                )?;
            }
        }
    }
    Ok(())
}

fn singleton(frame: &ThreeWayWeightFrame<'_>, batch: &mut PreparedAutomaticBatch<'_>, id: usize) {
    let t = frame.topology().topology();
    let key = t.tuples()[id];
    for j in 0..batch.columns {
        let value = batch.targets[j * frame.weights().len() + id] / 3.0;
        for (q, &level) in key.iter().enumerate() {
            batch.coefficients[j * frame.diagonal().len() + t.global_index(q, level)] = value;
        }
    }
}
fn dense_columns(
    frame: &ThreeWayWeightFrame<'_>,
    view: PreparedComponentView<'_, '_>,
    terminal: &DensePseudoinverse,
    batch: &mut PreparedAutomaticBatch<'_>,
    progress: &mut Progress<'_>,
) -> Result<(), MultiwayError> {
    // Explicit 6KiB maximum stack scratch, independent of component/RHS counts.
    let mut rhs = [0.; PREPARED_DENSE_TERMINAL_LIMIT];
    let mut out = [0.; PREPARED_DENSE_TERMINAL_LIMIT];
    let mut modal = [0.; PREPARED_DENSE_TERMINAL_LIMIT];
    let n = view.dimension();
    let e = frame.weights().len();
    let v = frame.diagonal().len();
    for j in 0..batch.columns {
        progress.column = Some(j);
        let target = &batch.targets[j * e..(j + 1) * e];
        let destination = &mut batch.coefficients[j * v..(j + 1) * v];
        view.rhs_from_targets_in_global(frame, target, destination)?;
        view.gather_coefficients(destination, &mut rhs[..n])?;
        ensure_finite(&rhs[..n], "automatic component normal RHS")?;
        terminal.solve_with_modal(&rhs[..n], &mut out[..n], &mut modal[..n])?;
        ensure_finite(&out[..n], "automatic component dense coefficients")?;
        view.scatter_coefficients(&out[..n], destination)?;
    }
    Ok(())
}

fn large(
    frame: &ThreeWayWeightFrame<'_>,
    view: Option<PreparedComponentView<'_, '_>>,
    batch: &mut PreparedAutomaticBatch<'_>,
    options: PreparedAutomaticOptions,
    maximum: usize,
    other: usize,
    progress: &mut Progress<'_>,
) -> Result<(), MultiwayError> {
    if options.hierarchy.is_some() {
        increment(&mut progress.hierarchy_attempts, 1)?;
        match hierarchy::solve(frame, view, batch, options, maximum, other, progress) {
            Ok(()) => return Ok(()),
            Err(source) => {
                increment(&mut progress.hierarchy_rejections, 1)?;
                reject(source, progress)?;
            }
        }
    }
    // The attempted hierarchy/replay/screen/solver owners died before fallback.
    progress.stage = PreparedAutomaticStage::ComponentSolve;
    increment(&mut progress.baseline_components, 1)?;
    let groups = layout::baseline_groups(
        frame,
        maximum,
        other,
        PreparedAutomaticGroupingScope::ComponentBaseline,
        progress,
    )?;
    let owner = PreparedBaseline::new(frame, options.fallback);
    let owner = match &groups {
        // LSMR uses the grouped weighted adjoint and MAP rows, never the fine
        // Gramian action. Its baseline therefore needs no tuple-image buffer.
        Some(groups) => owner.with_grouping(groups, crate::GroupedGramianMode::RowGather)?,
        None => owner,
    };
    solve_columns(&owner, view, batch, options.lsmr, maximum, other, progress)
}

fn solve_columns<P: PreparedSolverAction>(
    owner: &P,
    view: Option<PreparedComponentView<'_, '_>>,
    batch: &mut PreparedAutomaticBatch<'_>,
    options: PreparedLsmrOptions,
    maximum: usize,
    other: usize,
    progress: &mut Progress<'_>,
) -> Result<(), MultiwayError> {
    let gathered = if view.is_some() {
        owner.fine_frame().weights().len()
    } else {
        0
    };
    admit(
        PreparedLsmrWorkspace::setup_payload_bound(owner, options, add(other, bytes(gathered)?)?)?,
        maximum,
        progress,
    )?;
    let mut target = vector(gathered)?;
    let other = add(other, bytes(target.capacity())?)?;
    let mut workspace =
        PreparedLsmrWorkspace::try_new_with_payload_budget(owner, options, maximum, other)?;
    admit(
        workspace.payload_report(other)?.total_payload_bytes,
        maximum,
        progress,
    )?;
    let e = batch.targets.len() / batch.columns;
    let v = batch.coefficients.len() / batch.columns;
    for j in 0..batch.columns {
        progress.column = Some(j);
        let original = &batch.targets[j * e..(j + 1) * e];
        let target = if let Some(view) = view {
            view.gather_tuple_values(original, &mut target)?;
            &target[..]
        } else {
            original
        };
        match solve_prepared_least_squares_with_certificate_gate(owner, target, &mut workspace) {
            Ok(result) => {
                solve_work(result.report.solve.work, result.report.gate, progress)?;
                if let Some(view) = view {
                    view.scatter_coefficients(
                        result.coefficients,
                        &mut batch.coefficients[j * v..(j + 1) * v],
                    )?;
                } else {
                    batch.coefficients[j * v..(j + 1) * v].copy_from_slice(result.coefficients);
                }
                if !result.report.solve.accepted {
                    return Err(numerical("automatic component certificate rejected"));
                }
            }
            Err(source) => {
                solve_work(workspace.last_work(), workspace.last_gate_work(), progress)?;
                return Err(source);
            }
        }
    }
    Ok(())
}

fn certify_columns(
    frame: &ThreeWayWeightFrame<'_>,
    batch: &mut PreparedAutomaticBatch<'_>,
    options: PreparedAutomaticOptions,
    maximum: usize,
    base: usize,
    progress: &mut Progress<'_>,
) -> Result<[bool; 32], MultiwayError> {
    let topology = frame.topology();
    admit(
        add(
            base,
            add(
                topology.projection_workspace_required_bytes()?,
                PreparedCertificateWorkspace::required_payload_bytes(frame)?,
            )?,
        )?,
        maximum,
        progress,
    )?;
    let mut projection = topology.try_projection_workspace()?;
    let mut certificate = PreparedCertificateWorkspace::try_new(frame)?;
    admit(
        add(
            base,
            add(
                projection.retained_payload_bytes()?,
                certificate.retained_payload_bytes()?,
            )?,
        )?,
        maximum,
        progress,
    )?;
    let mut rejected = [false; 32];
    let e = frame.weights().len();
    let v = frame.diagonal().len();
    for (j, reject_column) in rejected.iter_mut().enumerate().take(batch.columns) {
        progress.column = Some(j);
        let output = &mut batch.coefficients[j * v..(j + 1) * v];
        increment(&mut progress.global_projection_applications, 1)?;
        let removed = topology.project_structural_range_with_workspace(output, &mut projection)?;
        if !removed.is_finite() {
            return Err(numerical("automatic original projection"));
        }
        let result = certify_prepared_normal_equations(
            frame.operator_view(),
            &batch.targets[j * e..(j + 1) * e],
            output,
            &mut certificate,
        );
        certificate_work(
            certificate.last_work(),
            &mut progress.global_certificate_work,
        )?;
        let residual = result?;
        let accepted = residual <= options.lsmr.certificate_tolerance;
        batch.reports[j] = Some(PreparedAutomaticColumnReport {
            initial_certificate: Some(residual),
            certified_normal_equation_residual: residual,
            accepted,
            global_fallback: false,
        });
        *reject_column = !accepted;
        if !accepted {
            reject(
                numerical("automatic original certificate rejected"),
                progress,
            )?;
        }
    }
    Ok(rejected)
}

fn global_fallback(
    frame: &ThreeWayWeightFrame<'_>,
    batch: &mut PreparedAutomaticBatch<'_>,
    options: PreparedAutomaticOptions,
    maximum: usize,
    other: usize,
    selected: &[bool; 32],
    progress: &mut Progress<'_>,
) -> Result<(), MultiwayError> {
    let groups = layout::baseline_groups(
        frame,
        maximum,
        other,
        PreparedAutomaticGroupingScope::GlobalBaseline,
        progress,
    )?;
    let owner = PreparedBaseline::new(frame, options.fallback);
    let owner = match &groups {
        // LSMR uses the grouped weighted adjoint and MAP rows, never the fine
        // Gramian action. Its baseline therefore needs no tuple-image buffer.
        Some(groups) => owner.with_grouping(groups, crate::GroupedGramianMode::RowGather)?,
        None => owner,
    };
    admit(
        PreparedLsmrWorkspace::setup_payload_bound(&owner, options.lsmr, other)?,
        maximum,
        progress,
    )?;
    let mut workspace =
        PreparedLsmrWorkspace::try_new_with_payload_budget(&owner, options.lsmr, maximum, other)?;
    admit(
        workspace.payload_report(other)?.total_payload_bytes,
        maximum,
        progress,
    )?;
    let e = frame.weights().len();
    let v = frame.diagonal().len();
    for (j, &selected) in selected.iter().enumerate().take(batch.columns) {
        if !selected {
            continue;
        }
        progress.column = Some(j);
        increment(&mut progress.global_fallback_columns, 1)?;
        match solve_prepared_least_squares_with_certificate_gate(
            &owner,
            &batch.targets[j * e..(j + 1) * e],
            &mut workspace,
        ) {
            Ok(result) => {
                solve_work(result.report.solve.work, result.report.gate, progress)?;
                batch.coefficients[j * v..(j + 1) * v].copy_from_slice(result.coefficients);
                let initial_certificate = batch.reports[j].and_then(|r| r.initial_certificate);
                batch.reports[j] = Some(PreparedAutomaticColumnReport {
                    initial_certificate,
                    certified_normal_equation_residual: result
                        .report
                        .solve
                        .certified_normal_equation_residual,
                    accepted: result.report.solve.accepted,
                    global_fallback: true,
                });
                if !result.report.solve.accepted {
                    reject(
                        numerical("automatic global fallback certificate rejected"),
                        progress,
                    )?;
                }
            }
            Err(source) => {
                solve_work(workspace.last_work(), workspace.last_gate_work(), progress)?;
                return Err(source);
            }
        }
    }
    Ok(())
}

fn reject(source: MultiwayError, p: &mut Progress<'_>) -> Result<(), MultiwayError> {
    increment(&mut p.rejections, 1)?;
    p.last_rejection = Some(PreparedAutomaticRejection {
        component: p.component,
        stage: p.stage,
        source,
    });
    Ok(())
}
fn solve_work(
    w: PreparedLsmrWorkReport,
    g: PreparedLsmrGateWorkReport,
    p: &mut Progress<'_>,
) -> Result<(), MultiwayError> {
    increment(
        &mut p.solve_work.weighted_incidence_applications,
        w.weighted_incidence_applications,
    )?;
    increment(
        &mut p.solve_work.weighted_adjoint_applications,
        w.weighted_adjoint_applications,
    )?;
    increment(
        &mut p.solve_work.hierarchy_applications,
        w.hierarchy_applications,
    )?;
    certificate_work(w.certificate, &mut p.solve_work.certificate)?;
    increment(&mut p.gate_work.candidate_checks, g.candidate_checks)?;
    increment(&mut p.gate_work.candidate_vetoes, g.candidate_vetoes)?;
    increment(
        &mut p.gate_work.projection_applications,
        g.projection_applications,
    )?;
    certificate_work(
        g.candidate_certificate,
        &mut p.gate_work.candidate_certificate,
    )
}
fn certificate_work(
    from: CertificateWorkReport,
    to: &mut CertificateWorkReport,
) -> Result<(), MultiwayError> {
    increment(&mut to.incidence_applications, from.incidence_applications)?;
    increment(&mut to.adjoint_applications, from.adjoint_applications)
}
fn fine_payload(frame: &ThreeWayWeightFrame<'_>) -> Result<usize, MultiwayError> {
    add(
        frame.topology().retained_payload_bytes()?,
        frame.retained_payload_bytes()?,
    )
}
fn payload_budget(
    maximum_payload_bytes: usize,
    additional_live_payload_bytes: usize,
) -> PreparedHierarchyBudget {
    PreparedHierarchyBudget {
        maximum_payload_bytes,
        additional_live_payload_bytes,
    }
}
fn admit(required: usize, maximum: usize, p: &mut Progress<'_>) -> Result<(), MultiwayError> {
    p.maximum_requested_payload_bytes = p.maximum_requested_payload_bytes.max(required);
    if required > maximum {
        return Err(MultiwayError::PayloadBudgetExceeded {
            required,
            budget: maximum,
        });
    }
    p.maximum_admitted_payload_bytes = p.maximum_admitted_payload_bytes.max(required);
    Ok(())
}
fn length(context: &'static str, expected: usize, actual: usize) -> Result<(), MultiwayError> {
    if expected == actual {
        Ok(())
    } else {
        Err(crate::error::dimension(context, expected, actual))
    }
}
fn bytes_of<T>(count: usize) -> Result<usize, MultiwayError> {
    mul(count, size_of::<T>())
}
fn add(a: usize, b: usize) -> Result<usize, MultiwayError> {
    a.checked_add(b).ok_or_else(overflow)
}
fn mul(a: usize, b: usize) -> Result<usize, MultiwayError> {
    a.checked_mul(b).ok_or_else(overflow)
}
fn increment(value: &mut usize, amount: usize) -> Result<(), MultiwayError> {
    *value = add(*value, amount)?;
    Ok(())
}
fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow {
        context: "automatic component batch",
    }
}
fn invalid(context: &'static str) -> MultiwayError {
    MultiwayError::InvalidCycleScreenInput { context }
}
fn numerical(context: &'static str) -> MultiwayError {
    MultiwayError::NumericalFailure { context }
}
