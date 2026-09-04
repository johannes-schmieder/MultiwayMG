//! Production-shaped pair-CMG local solvers hosted by the generic Schwarz executor.
//!
//! The original [`crate::PairCmgPreconditioner`] is intentionally simple, but
//! its three pair systems retain mutex-protected workspaces and are traversed
//! serially.  This module instead splits every pair marginal into connected
//! components, gives each component its own immutable CMG hierarchy and
//! reusable workspace, and lets `schwarz-precond` schedule the independent
//! local solves with pooled caller-owned gather/scatter buffers.

use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use cmg::{CmgOptions, CmgPreconditioner, CmgWorkspace, Components, Laplacian};
use schwarz_precond::{
    LocalSolveError, LocalSolver, PartitionWeights, ReductionStrategy, SchwarzPreconditioner,
    SubdomainCore, SubdomainEntry,
};

use crate::{
    FactorPair, MultiwayError, Preconditioner, ThreeWayProblem,
    memory_estimate::estimate_three_way_problem_bytes,
    structural_projection::StructuralRangeProjector,
};

/// Configuration for the component-local CMG Schwarz adapter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairCmgSchwarzOptions {
    /// CMG hierarchy options used for every admitted pair component.
    pub cmg: CmgOptions,
    /// Fixed number of stationary residual-correction cycles per local solve.
    pub fixed_cycles: usize,
    /// Two-sided partition-of-unity weight attached to every local occurrence.
    pub partition_weight: f64,
    /// Reduction backend used by the outer additive Schwarz executor.
    pub reduction: ReductionStrategy,
}

impl Default for PairCmgSchwarzOptions {
    fn default() -> Self {
        Self {
            cmg: CmgOptions::default(),
            fixed_cycles: 1,
            partition_weight: std::f64::consts::FRAC_1_SQRT_2,
            reduction: ReductionStrategy::default(),
        }
    }
}

impl PairCmgSchwarzOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        self.cmg
            .validate()
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        if self.fixed_cycles == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "pair_cmg_fixed_cycles",
                message: "must be positive".to_owned(),
            });
        }
        if !self.partition_weight.is_finite() || self.partition_weight <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "pair_cmg_partition_weight",
                message: format!("must be finite and positive, got {}", self.partition_weight),
            });
        }
        Ok(self)
    }
}

/// Phase-separated setup time for a component-local pair-CMG Schwarz build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairCmgSchwarzBuildTiming {
    pair_graph_setup: Duration,
    cmg_setup: Duration,
    workspace_setup: Duration,
    schwarz_setup: Duration,
    total: Duration,
}

impl PairCmgSchwarzBuildTiming {
    /// Pair marginal accumulation, graph construction, and component splitting.
    #[must_use]
    pub const fn pair_graph_setup(self) -> Duration {
        self.pair_graph_setup
    }

    /// Sum of immutable CMG hierarchy construction times.
    #[must_use]
    pub const fn cmg_setup(self) -> Duration {
        self.cmg_setup
    }

    /// Sum of retained CMG workspace construction times.
    #[must_use]
    pub const fn workspace_setup(self) -> Duration {
        self.workspace_setup
    }

    /// Subdomain-entry validation and generic Schwarz executor construction.
    #[must_use]
    pub const fn schwarz_setup(self) -> Duration {
        self.schwarz_setup
    }

    /// Complete constructor wall time, including validation and bookkeeping.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

/// Structural description of one connected pair component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairComponentReport {
    pair: FactorPair,
    component: usize,
    vertices: usize,
    edges: usize,
    cycle_excess: usize,
    cmg_levels: usize,
    cmg_retained_bytes: usize,
    cmg_workspace_bytes: usize,
    local_scratch_values: usize,
}

impl PairComponentReport {
    /// Factor pair containing this component.
    #[must_use]
    pub const fn pair(self) -> FactorPair {
        self.pair
    }

    /// Zero-based component label within the factor-pair graph.
    #[must_use]
    pub const fn component(self) -> usize {
        self.component
    }

    /// Number of local coefficient vertices.
    #[must_use]
    pub const fn vertices(self) -> usize {
        self.vertices
    }

    /// Number of unique positive-weight pair edges.
    #[must_use]
    pub const fn edges(self) -> usize {
        self.edges
    }

    /// Graph cycle excess `edges - vertices + 1` for this connected component.
    #[must_use]
    pub const fn cycle_excess(self) -> usize {
        self.cycle_excess
    }

    /// Number of retained CMG hierarchy levels.
    #[must_use]
    pub const fn cmg_levels(self) -> usize {
        self.cmg_levels
    }

    /// Principal immutable bytes reported by CMG.
    #[must_use]
    pub const fn cmg_retained_bytes(self) -> usize {
        self.cmg_retained_bytes
    }

    /// Bytes in the preallocated reusable CMG workspace.
    #[must_use]
    pub const fn cmg_workspace_bytes(self) -> usize {
        self.cmg_workspace_bytes
    }

    /// Length required for each of the two generic Schwarz local scratch arrays.
    #[must_use]
    pub const fn local_scratch_values(self) -> usize {
        self.local_scratch_values
    }
}

/// Principal retained-state and scratch accounting for pair-CMG Schwarz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairCmgSchwarzMemoryReport {
    problem_state_bytes_estimate: usize,
    cmg_preconditioner_bytes: usize,
    cmg_workspace_pool_bytes: usize,
    subdomain_metadata_bytes_estimate: usize,
    projection_workspace_bytes: usize,
    total_retained_bytes_estimate: usize,
    maximum_local_scratch_bytes_per_worker: usize,
}

impl PairCmgSchwarzMemoryReport {
    /// Estimated immutable three-way problem bytes shared by the adapter.
    #[must_use]
    pub const fn problem_state_bytes_estimate(self) -> usize {
        self.problem_state_bytes_estimate
    }

    /// Sum of principal immutable bytes reported by component CMG objects.
    #[must_use]
    pub const fn cmg_preconditioner_bytes(self) -> usize {
        self.cmg_preconditioner_bytes
    }

    /// Sum of bytes in one preallocated CMG workspace per component.
    #[must_use]
    pub const fn cmg_workspace_pool_bytes(self) -> usize {
        self.cmg_workspace_pool_bytes
    }

    /// Conservative estimate for global indices and partition weights.
    #[must_use]
    pub const fn subdomain_metadata_bytes_estimate(self) -> usize {
        self.subdomain_metadata_bytes_estimate
    }

    /// Retained allocation-free structural-range projection workspace.
    #[must_use]
    pub const fn projection_workspace_bytes(self) -> usize {
        self.projection_workspace_bytes
    }

    /// Sum of the reported principal retained categories.
    #[must_use]
    pub const fn total_retained_bytes_estimate(self) -> usize {
        self.total_retained_bytes_estimate
    }

    /// Two local f64 scratch arrays at the largest component size.
    #[must_use]
    pub const fn maximum_local_scratch_bytes_per_worker(self) -> usize {
        self.maximum_local_scratch_bytes_per_worker
    }
}

/// Fixed additive Schwarz correction using one CMG hierarchy per connected
/// factor-pair component.
///
/// The timed apply path assumes the submitted coefficient-space right-hand side
/// is already in the structural range, as it is for exact `B'` products.  It
/// performs no global RHS copy.  Local pair RHS vectors are sign-switched and
/// projected by CMG, and the assembled output is projected back into the
/// three-way structural range with retained scratch.
pub struct PairCmgSchwarzPreconditioner {
    problem: ThreeWayProblem,
    selected_pairs: Vec<FactorPair>,
    inner: SchwarzPreconditioner<CmgPairLocalSolver>,
    component_reports: Vec<PairComponentReport>,
    build_timing: PairCmgSchwarzBuildTiming,
    memory_report: PairCmgSchwarzMemoryReport,
    projection: StructuralRangeProjector,
}

impl core::fmt::Debug for PairCmgSchwarzPreconditioner {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("PairCmgSchwarzPreconditioner")
            .field("selected_pairs", &self.selected_pairs)
            .field("component_reports", &self.component_reports)
            .field("build_timing", &self.build_timing)
            .field("memory_report", &self.memory_report)
            .finish_non_exhaustive()
    }
}

impl PairCmgSchwarzPreconditioner {
    /// Build a canonical selected-pair portfolio.
    pub fn build(
        problem: ThreeWayProblem,
        selected_pairs: &[FactorPair],
        options: PairCmgSchwarzOptions,
    ) -> Result<Self, MultiwayError> {
        let total_start = Instant::now();
        let options = options.validate()?;
        let selected_pairs: Vec<_> = FactorPair::ALL
            .into_iter()
            .filter(|pair| selected_pairs.contains(pair))
            .collect();
        if selected_pairs.is_empty() {
            return Err(MultiwayError::InvalidOption {
                name: "selected_pairs",
                message: "at least one factor pair must be selected".to_owned(),
            });
        }

        let mut entries = Vec::new();
        let mut component_reports = Vec::new();
        let mut pair_graph_setup = Duration::ZERO;
        let mut cmg_setup = Duration::ZERO;
        let mut workspace_setup = Duration::ZERO;
        let mut cmg_preconditioner_bytes = 0usize;
        let mut cmg_workspace_pool_bytes = 0usize;
        let mut subdomain_metadata_bytes_estimate = 0usize;
        let mut maximum_local_scratch_values = 0usize;

        for &pair in &selected_pairs {
            let graph_start = Instant::now();
            let components = build_pair_components(&problem, pair)?;
            pair_graph_setup += graph_start.elapsed();

            for component in components {
                let cmg_start = Instant::now();
                let preconditioner = CmgPreconditioner::build(&component.graph, options.cmg)
                    .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
                cmg_setup += cmg_start.elapsed();

                let workspace_start = Instant::now();
                let workspace = preconditioner
                    .try_workspace()
                    .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
                let workspace_bytes = workspace.byte_len();
                workspace_setup += workspace_start.elapsed();

                let local_scratch_values = if options.fixed_cycles == 1 {
                    component.graph.vertex_count()
                } else {
                    component
                        .graph
                        .vertex_count()
                        .checked_mul(2)
                        .ok_or_else(|| MultiwayError::InvalidOption {
                            name: "pair_cmg_fixed_cycles",
                            message: "local scratch dimension overflow".to_owned(),
                        })?
                };
                maximum_local_scratch_values =
                    maximum_local_scratch_values.max(local_scratch_values);
                let retained_bytes = preconditioner.retained_bytes();
                cmg_preconditioner_bytes = cmg_preconditioner_bytes.saturating_add(retained_bytes);
                cmg_workspace_pool_bytes = cmg_workspace_pool_bytes.saturating_add(workspace_bytes);
                subdomain_metadata_bytes_estimate = subdomain_metadata_bytes_estimate
                    .saturating_add(component.global_indices.len().saturating_mul(12));

                let report = PairComponentReport {
                    pair,
                    component: component.component,
                    vertices: component.graph.vertex_count(),
                    edges: component.graph.edge_count(),
                    cycle_excess: component
                        .graph
                        .edge_count()
                        .saturating_sub(component.graph.vertex_count().saturating_sub(1)),
                    cmg_levels: preconditioner.hierarchy().levels().len(),
                    cmg_retained_bytes: retained_bytes,
                    cmg_workspace_bytes: workspace_bytes,
                    local_scratch_values,
                };

                let solver = CmgPairLocalSolver {
                    graph: component.graph,
                    preconditioner,
                    second_start: component.second_start,
                    fixed_cycles: options.fixed_cycles,
                    scratch_size: local_scratch_values,
                    workspace_pool: Mutex::new(vec![workspace]),
                    fallback_workspace_allocations: AtomicUsize::new(0),
                };
                let weights = if options.partition_weight == 1.0 {
                    PartitionWeights::Uniform(component.global_indices.len())
                } else {
                    PartitionWeights::NonUniform(vec![
                        options.partition_weight;
                        component.global_indices.len()
                    ])
                };
                let core = SubdomainCore::with_partition_weights(component.global_indices, weights)
                    .map_err(|error| MultiwayError::Lsmr(error.to_string()))?;
                let entry = SubdomainEntry::try_new(core, solver)
                    .map_err(|error| MultiwayError::Lsmr(error.to_string()))?;
                entries.push(entry);
                component_reports.push(report);
            }
        }

        let schwarz_start = Instant::now();
        let inner =
            SchwarzPreconditioner::with_n_dofs(entries, problem.dimension(), options.reduction);
        let schwarz_setup = schwarz_start.elapsed();

        let projection = StructuralRangeProjector::new(&problem);
        let projection_workspace_bytes = projection.workspace_bytes();
        let problem_state_bytes_estimate = estimate_three_way_problem_bytes(&problem);
        let total_retained_bytes_estimate = problem_state_bytes_estimate
            .saturating_add(cmg_preconditioner_bytes)
            .saturating_add(cmg_workspace_pool_bytes)
            .saturating_add(subdomain_metadata_bytes_estimate)
            .saturating_add(projection_workspace_bytes);
        let maximum_local_scratch_bytes_per_worker = maximum_local_scratch_values
            .saturating_mul(2)
            .saturating_mul(core::mem::size_of::<f64>());
        let memory_report = PairCmgSchwarzMemoryReport {
            problem_state_bytes_estimate,
            cmg_preconditioner_bytes,
            cmg_workspace_pool_bytes,
            subdomain_metadata_bytes_estimate,
            projection_workspace_bytes,
            total_retained_bytes_estimate,
            maximum_local_scratch_bytes_per_worker,
        };
        let build_timing = PairCmgSchwarzBuildTiming {
            pair_graph_setup,
            cmg_setup,
            workspace_setup,
            schwarz_setup,
            total: total_start.elapsed(),
        };
        Ok(Self {
            problem,
            selected_pairs,
            inner,
            component_reports,
            build_timing,
            memory_report,
            projection,
        })
    }

    /// Build all three factor pairs.
    pub fn build_all(
        problem: ThreeWayProblem,
        options: PairCmgSchwarzOptions,
    ) -> Result<Self, MultiwayError> {
        Self::build(problem, &FactorPair::ALL, options)
    }

    /// Selected pairs in canonical order.
    #[must_use]
    pub fn selected_pairs(&self) -> &[FactorPair] {
        &self.selected_pairs
    }

    /// Connected-component reports in canonical pair/component order.
    #[must_use]
    pub fn component_reports(&self) -> &[PairComponentReport] {
        &self.component_reports
    }

    /// Phase-separated setup timing.
    #[must_use]
    pub const fn build_timing(&self) -> PairCmgSchwarzBuildTiming {
        self.build_timing
    }

    /// Principal retained-state and local-scratch accounting.
    #[must_use]
    pub const fn memory_report(&self) -> PairCmgSchwarzMemoryReport {
        self.memory_report
    }

    /// Concrete reduction backend selected at the current Rayon width.
    #[must_use]
    pub fn reduction_strategy(&self) -> ReductionStrategy {
        self.inner.reduction_strategy()
    }

    /// Number of emergency CMG workspace allocations after construction.
    ///
    /// Sequential repeated-RHS use should leave this at zero.  A positive value
    /// indicates concurrent re-entry of the same component solver and is
    /// reported rather than hidden from benchmark accounting.
    #[must_use]
    pub fn fallback_workspace_allocations(&self) -> usize {
        self.inner
            .subdomains()
            .iter()
            .map(|entry| {
                entry
                    .solver()
                    .fallback_workspace_allocations
                    .load(Ordering::Relaxed)
            })
            .sum::<usize>()
            .saturating_add(self.projection.fallback_allocations())
    }

    fn project_output(&self, values: &mut [f64]) -> Result<(), MultiwayError> {
        self.projection.project(&self.problem, values)
    }
}

impl Preconditioner for PairCmgSchwarzPreconditioner {
    fn dimension(&self) -> usize {
        self.problem.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        if rhs.len() != self.dimension() {
            return Err(crate::error::dimension(
                "PairCmgSchwarzPreconditioner::apply rhs",
                self.dimension(),
                rhs.len(),
            ));
        }
        if out.len() != self.dimension() {
            return Err(crate::error::dimension(
                "PairCmgSchwarzPreconditioner::apply output",
                self.dimension(),
                out.len(),
            ));
        }
        out.fill(0.0);
        if let Err(error) = self.inner.apply(rhs, out) {
            out.fill(0.0);
            return Err(MultiwayError::Lsmr(error.to_string()));
        }
        if let Err(error) = self.project_output(out) {
            out.fill(0.0);
            return Err(error);
        }
        Ok(())
    }
}

#[derive(Debug)]
struct CmgPairLocalSolver {
    graph: Laplacian,
    preconditioner: CmgPreconditioner,
    second_start: usize,
    fixed_cycles: usize,
    scratch_size: usize,
    workspace_pool: Mutex<Vec<CmgWorkspace>>,
    fallback_workspace_allocations: AtomicUsize,
}

impl CmgPairLocalSolver {
    fn take_workspace(&self) -> Result<CmgWorkspace, LocalSolveError> {
        let workspace = self
            .workspace_pool
            .lock()
            .map_err(|_| LocalSolveError::BackendFailed {
                context: "pair-cmg workspace borrow",
                message: "workspace pool lock was poisoned".to_owned(),
            })?
            .pop();
        match workspace {
            Some(workspace) => Ok(workspace),
            None => {
                self.fallback_workspace_allocations
                    .fetch_add(1, Ordering::Relaxed);
                self.preconditioner.try_workspace().map_err(|error| {
                    LocalSolveError::BackendFailed {
                        context: "pair-cmg workspace fallback allocation",
                        message: error.to_string(),
                    }
                })
            }
        }
    }

    fn return_workspace(&self, workspace: CmgWorkspace) -> Result<(), LocalSolveError> {
        self.workspace_pool
            .lock()
            .map_err(|_| LocalSolveError::BackendFailed {
                context: "pair-cmg workspace return",
                message: "workspace pool lock was poisoned".to_owned(),
            })?
            .push(workspace);
        Ok(())
    }

    fn solve_fixed_cycles(
        &self,
        rhs: &mut [f64],
        sol: &mut [f64],
        workspace: &mut CmgWorkspace,
    ) -> Result<(), LocalSolveError> {
        let n = self.graph.vertex_count();
        let (active_rhs, saved_rhs) = rhs.split_at_mut(n);
        let (active_sol, correction) = sol.split_at_mut(n);
        active_sol.fill(0.0);

        if self.fixed_cycles == 1 {
            return self
                .preconditioner
                .apply_into(active_rhs, active_sol, workspace)
                .map_err(|error| LocalSolveError::BackendFailed {
                    context: "pair-cmg fixed cycle",
                    message: error.to_string(),
                });
        }

        saved_rhs[..n].copy_from_slice(active_rhs);
        for cycle in 0..self.fixed_cycles {
            if cycle > 0 {
                self.graph
                    .matvec_into(active_sol, &mut correction[..n])
                    .map_err(|error| LocalSolveError::BackendFailed {
                        context: "pair-cmg residual matvec",
                        message: error.to_string(),
                    })?;
                for index in 0..n {
                    active_rhs[index] = saved_rhs[index] - correction[index];
                }
            }
            correction[..n].fill(0.0);
            self.preconditioner
                .apply_into(active_rhs, &mut correction[..n], workspace)
                .map_err(|error| LocalSolveError::BackendFailed {
                    context: "pair-cmg fixed residual-correction cycle",
                    message: error.to_string(),
                })?;
            for index in 0..n {
                active_sol[index] += correction[index];
            }
        }
        active_rhs.copy_from_slice(&saved_rhs[..n]);
        Ok(())
    }
}

impl LocalSolver for CmgPairLocalSolver {
    fn n_local(&self) -> usize {
        self.graph.vertex_count()
    }

    fn scratch_size(&self) -> usize {
        self.scratch_size
    }

    fn solve_local(
        &self,
        rhs: &mut [f64],
        sol: &mut [f64],
        _allow_inner_parallelism: bool,
    ) -> Result<(), LocalSolveError> {
        let n = self.n_local();
        if rhs.len() < self.scratch_size || sol.len() < self.scratch_size {
            sol.fill(0.0);
            return Err(LocalSolveError::BackendFailed {
                context: "pair-cmg local scratch",
                message: format!(
                    "rhs/solution scratch lengths ({}/{}) are below required {}",
                    rhs.len(),
                    sol.len(),
                    self.scratch_size
                ),
            });
        }
        debug_assert!(self.second_start <= n);
        for value in &mut rhs[self.second_start..n] {
            *value = -*value;
        }

        let mut workspace = match self.take_workspace() {
            Ok(workspace) => workspace,
            Err(error) => {
                for value in &mut rhs[self.second_start..n] {
                    *value = -*value;
                }
                sol[..n].fill(0.0);
                return Err(error);
            }
        };
        let result = self.solve_fixed_cycles(rhs, sol, &mut workspace);
        for value in &mut rhs[self.second_start..n] {
            *value = -*value;
        }
        if result.is_ok() {
            for value in &mut sol[self.second_start..n] {
                *value = -*value;
            }
            if let Err(error) = self.return_workspace(workspace) {
                sol[..n].fill(0.0);
                return Err(error);
            }
        } else {
            sol[..n].fill(0.0);
        }
        result
    }

    fn inner_parallelism_work_estimate(&self) -> usize {
        // The pinned CMG dependency is serial.  Returning zero keeps the outer
        // Schwarz scheduler from enabling nested Rayon around this local solve.
        0
    }
}

struct PairComponent {
    component: usize,
    graph: Laplacian,
    global_indices: Vec<u32>,
    second_start: usize,
}

fn build_pair_components(
    problem: &ThreeWayProblem,
    pair: FactorPair,
) -> Result<Vec<PairComponent>, MultiwayError> {
    let (first, second) = pair.factors();
    let counts = problem.topology().level_counts();
    let offsets = problem.topology().offsets();
    let first_count = counts[first];
    let second_count = counts[second];
    let mut marginal: BTreeMap<(u32, u32), f64> = BTreeMap::new();
    for (&tuple, &weight) in problem.topology().tuples().iter().zip(problem.weights()) {
        *marginal.entry((tuple[first], tuple[second])).or_insert(0.0) += weight;
    }
    let full_graph = Laplacian::from_edges(
        first_count + second_count,
        marginal
            .into_iter()
            .map(|((left, right), weight)| (left as usize, first_count + right as usize, weight)),
    )
    .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
    let labels = Components::from_laplacian(&full_graph);
    let mut vertices = vec![Vec::new(); labels.count()];
    for (vertex, &component) in labels.labels().iter().enumerate() {
        vertices[component].push(vertex);
    }
    let mut compact = vec![usize::MAX; full_graph.vertex_count()];
    for component_vertices in &vertices {
        for (local, &vertex) in component_vertices.iter().enumerate() {
            compact[vertex] = local;
        }
    }
    let mut edges = vec![Vec::new(); labels.count()];
    for edge in full_graph.edges() {
        let component = labels.labels()[edge.u()];
        debug_assert_eq!(component, labels.labels()[edge.v()]);
        edges[component].push((compact[edge.u()], compact[edge.v()], edge.weight()));
    }

    let mut result = Vec::with_capacity(labels.count());
    for (component, (component_vertices, component_edges)) in
        vertices.into_iter().zip(edges).enumerate()
    {
        if component_edges.is_empty() {
            return Err(MultiwayError::Cmg(format!(
                "pair {} component {component} has no positive edge",
                pair.label()
            )));
        }
        let second_start = component_vertices.partition_point(|&vertex| vertex < first_count);
        if second_start == 0 || second_start == component_vertices.len() {
            return Err(MultiwayError::Cmg(format!(
                "pair {} component {component} does not contain both factor blocks",
                pair.label()
            )));
        }
        let global_indices = component_vertices
            .iter()
            .map(|&vertex| {
                let global = if vertex < first_count {
                    offsets[first] + vertex
                } else {
                    offsets[second] + (vertex - first_count)
                };
                u32::try_from(global).map_err(|_| {
                    MultiwayError::Cmg(format!(
                        "pair {} global coefficient index {global} exceeds u32",
                        pair.label()
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let graph = Laplacian::from_edges(component_vertices.len(), component_edges)
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        result.push(PairComponent {
            component,
            graph,
            global_indices,
            second_start,
        });
    }
    Ok(result)
}
