//! Oracle multilevel hierarchies with an explicit per-level smoother schedule.
//!
//! The ordinary [`crate::ThreeWayHierarchy`] intentionally has a compact
//! production-oriented policy. Issue #2 needs controlled comparisons in which
//! pair-CMG is used only on the finest level, on the first two levels, or on
//! every nonterminal level. This module supplies that research harness without
//! changing automatic routing.

use std::time::{Duration, Instant};

use crate::{
    DensePseudoinverse, DiagonalPreconditioner, FactorAggregation, MultiwayError,
    PairCmgBuildTiming, PairCmgMemoryReport, PairCmgOptions, PairSubsetCmgPreconditioner,
    Preconditioner, SymmetricMapPreconditioner, ThreeWayProblem,
    memory_estimate::estimate_three_way_problem_bytes,
};

/// Fixed smoother selected for one nonterminal oracle level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OracleLevelSmootherSpec {
    /// One weighted-Jacobi correction.
    Jacobi {
        /// Damping included in the diagonal approximate inverse.
        omega: f64,
    },
    /// One symmetric factor MAP/block-Gauss--Seidel correction.
    SymmetricMap,
    /// One all-three-pair fixed CMG correction.
    AllPairsCmg {
        /// CMG construction and partition options.
        options: PairCmgOptions,
    },
}

/// Construction options for a supplied-map oracle hierarchy.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledOracleHierarchyOptions {
    /// Exact fine-to-coarse maps in hierarchy order.
    pub aggregations: Vec<FactorAggregation>,
    /// One smoother specification for every nonterminal level.
    pub smoothers: Vec<OracleLevelSmootherSpec>,
    /// Equal number of pre- and post-smoothing sweeps.
    pub sweeps: usize,
    /// Relative rank threshold used by the dense terminal pseudoinverse.
    pub terminal_relative_tolerance: f64,
}

impl ScheduledOracleHierarchyOptions {
    fn validate(&self, problem: &ThreeWayProblem) -> Result<(), MultiwayError> {
        if self.aggregations.is_empty() {
            return Err(MultiwayError::InvalidOption {
                name: "oracle_aggregations",
                message: "at least one supplied aggregation is required".to_owned(),
            });
        }
        if self.smoothers.len() != self.aggregations.len() {
            return Err(crate::error::dimension(
                "ScheduledOracleHierarchyOptions smoothers",
                self.aggregations.len(),
                self.smoothers.len(),
            ));
        }
        if self.sweeps == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "oracle_sweeps",
                message: "must be positive".to_owned(),
            });
        }
        if !self.terminal_relative_tolerance.is_finite()
            || self.terminal_relative_tolerance <= 0.0
        {
            return Err(MultiwayError::InvalidOption {
                name: "oracle_terminal_relative_tolerance",
                message: format!(
                    "must be finite and positive, got {}",
                    self.terminal_relative_tolerance
                ),
            });
        }
        if self.aggregations[0].fine_counts() != problem.topology().level_counts() {
            return Err(MultiwayError::InvalidSuppliedAggregation { level: 0 });
        }
        Ok(())
    }
}

/// Build-phase timing for a supplied-map scheduled oracle hierarchy.
///
/// Timings are diagnostics only and are never consumed by routing logic.
/// `smoother_setup` includes pair graph, CMG, and workspace construction for
/// pair-CMG levels; the narrower fields expose those subphases separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledOracleBuildTiming {
    coarsening_setup: Duration,
    smoother_setup: Duration,
    pair_graph_setup: Duration,
    cmg_setup: Duration,
    pair_workspace_setup: Duration,
    terminal_setup: Duration,
    total: Duration,
}

impl ScheduledOracleBuildTiming {
    /// Exact tuple mapping, duplicate collapse, and coarse-problem construction.
    #[must_use]
    pub const fn coarsening_setup(self) -> Duration {
        self.coarsening_setup
    }

    /// Complete construction time for all level smoothers.
    #[must_use]
    pub const fn smoother_setup(self) -> Duration {
        self.smoother_setup
    }

    /// Pair marginal, graph, and component construction within CMG levels.
    #[must_use]
    pub const fn pair_graph_setup(self) -> Duration {
        self.pair_graph_setup
    }

    /// CMG hierarchy/preconditioner construction within CMG levels.
    #[must_use]
    pub const fn cmg_setup(self) -> Duration {
        self.cmg_setup
    }

    /// Retained pair RHS, solution, and CMG workspace construction.
    #[must_use]
    pub const fn pair_workspace_setup(self) -> Duration {
        self.pair_workspace_setup
    }

    /// Rank-revealing dense terminal construction.
    #[must_use]
    pub const fn terminal_setup(self) -> Duration {
        self.terminal_setup
    }

    /// Complete constructor time, including validation and bookkeeping.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

/// Principal retained and apply-scratch memory accounting for a scheduled
/// oracle hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledOracleMemoryReport {
    problem_state_bytes_estimate: usize,
    aggregation_bytes: usize,
    smoother_bytes_estimate: usize,
    pair_cmg_preconditioner_bytes: usize,
    pair_cmg_workspace_bytes: usize,
    terminal_bytes_estimate: usize,
    total_retained_bytes_estimate: usize,
    maximum_apply_scratch_bytes_estimate: usize,
}

impl ScheduledOracleMemoryReport {
    /// Estimated immutable weighted-problem bytes across all levels.
    #[must_use]
    pub const fn problem_state_bytes_estimate(self) -> usize {
        self.problem_state_bytes_estimate
    }

    /// Exact retained bytes in hard parent maps.
    #[must_use]
    pub const fn aggregation_bytes(self) -> usize {
        self.aggregation_bytes
    }

    /// Estimated non-CMG smoother bytes.
    #[must_use]
    pub const fn smoother_bytes_estimate(self) -> usize {
        self.smoother_bytes_estimate
    }

    /// Exact principal immutable bytes reported by retained CMG pair solvers.
    #[must_use]
    pub const fn pair_cmg_preconditioner_bytes(self) -> usize {
        self.pair_cmg_preconditioner_bytes
    }

    /// Exact CMG workspace bytes plus explicit pair RHS/solution buffers.
    #[must_use]
    pub const fn pair_cmg_workspace_bytes(self) -> usize {
        self.pair_cmg_workspace_bytes
    }

    /// Dense terminal eigenvector/eigenvalue estimate.
    #[must_use]
    pub const fn terminal_bytes_estimate(self) -> usize {
        self.terminal_bytes_estimate
    }

    /// Sum of principal retained categories.
    #[must_use]
    pub const fn total_retained_bytes_estimate(self) -> usize {
        self.total_retained_bytes_estimate
    }

    /// Conservative peak temporary vector bytes for one serial application.
    #[must_use]
    pub const fn maximum_apply_scratch_bytes_estimate(self) -> usize {
        self.maximum_apply_scratch_bytes_estimate
    }
}

/// Immutable supplied-map V-cycle with a fixed level-specific smoother schedule.
#[derive(Debug, Clone)]
pub struct ScheduledOracleHierarchy {
    problems: Vec<ThreeWayProblem>,
    aggregations: Vec<FactorAggregation>,
    smoothers: Vec<LevelSmoother>,
    terminal: DensePseudoinverse,
    sweeps: usize,
    memory: ScheduledOracleMemoryReport,
    build_timing: ScheduledOracleBuildTiming,
}

impl ScheduledOracleHierarchy {
    /// Build and validate every supplied level and its fixed smoother.
    pub fn build(
        finest: ThreeWayProblem,
        options: ScheduledOracleHierarchyOptions,
    ) -> Result<Self, MultiwayError> {
        let total_start = Instant::now();
        options.validate(&finest)?;
        let mut problems = vec![finest];
        let mut smoothers = Vec::with_capacity(options.smoothers.len());
        let mut coarsening_setup = Duration::ZERO;
        let mut smoother_setup = Duration::ZERO;
        let mut pair_graph_setup = Duration::ZERO;
        let mut cmg_setup = Duration::ZERO;
        let mut pair_workspace_setup = Duration::ZERO;

        for (level, (aggregation, smoother_spec)) in options
            .aggregations
            .iter()
            .zip(&options.smoothers)
            .enumerate()
        {
            let problem = problems
                .last()
                .expect("oracle hierarchy retains its finest problem");
            if aggregation.fine_counts() != problem.topology().level_counts() {
                return Err(MultiwayError::InvalidSuppliedAggregation { level });
            }

            let smoother_start = Instant::now();
            let smoother = LevelSmoother::build(problem, *smoother_spec)?;
            smoother_setup += smoother_start.elapsed();
            if let Some(pair_timing) = smoother.pair_timing() {
                pair_graph_setup += pair_timing.pair_graph_setup();
                cmg_setup += pair_timing.cmg_setup();
                pair_workspace_setup += pair_timing.workspace_setup();
            }
            smoothers.push(smoother);

            let coarsening_start = Instant::now();
            problems.push(aggregation.coarsen(problem)?);
            coarsening_setup += coarsening_start.elapsed();
        }

        let terminal_problem = problems
            .last()
            .expect("oracle hierarchy contains a terminal problem");
        let terminal_start = Instant::now();
        let terminal = DensePseudoinverse::from_problem(
            terminal_problem,
            options.terminal_relative_tolerance,
        )?;
        let terminal_setup = terminal_start.elapsed();
        let memory = memory_report(&problems, &options.aggregations, &smoothers);
        let build_timing = ScheduledOracleBuildTiming {
            coarsening_setup,
            smoother_setup,
            pair_graph_setup,
            cmg_setup,
            pair_workspace_setup,
            terminal_setup,
            total: total_start.elapsed(),
        };
        Ok(Self {
            problems,
            aggregations: options.aggregations,
            smoothers,
            terminal,
            sweeps: options.sweeps,
            memory,
            build_timing,
        })
    }

    /// Finest weighted problem.
    #[must_use]
    pub fn finest_problem(&self) -> &ThreeWayProblem {
        &self.problems[0]
    }

    /// Number of exact aggregation levels.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.aggregations.len()
    }

    /// Coefficient dimension at every level, including the terminal.
    #[must_use]
    pub fn dimensions(&self) -> Vec<usize> {
        self.problems
            .iter()
            .map(ThreeWayProblem::dimension)
            .collect()
    }

    /// Unique tuple count at every level, including the terminal.
    #[must_use]
    pub fn tuple_counts(&self) -> Vec<usize> {
        self.problems
            .iter()
            .map(ThreeWayProblem::tuple_count)
            .collect()
    }

    /// Sum of level tuple counts divided by finest tuple count.
    #[must_use]
    pub fn tuple_complexity(&self) -> f64 {
        self.problems
            .iter()
            .map(ThreeWayProblem::tuple_count)
            .sum::<usize>() as f64
            / self.finest_problem().tuple_count() as f64
    }

    /// Sum of level dimensions divided by finest dimension.
    #[must_use]
    pub fn dimension_complexity(&self) -> f64 {
        self.problems
            .iter()
            .map(ThreeWayProblem::dimension)
            .sum::<usize>() as f64
            / self.finest_problem().dimension() as f64
    }

    /// Numerical terminal rank.
    #[must_use]
    pub const fn terminal_rank(&self) -> usize {
        self.terminal.rank()
    }

    /// Principal retained and apply-scratch memory report.
    #[must_use]
    pub const fn memory_report(&self) -> ScheduledOracleMemoryReport {
        self.memory
    }

    /// Build-phase timing from construction.
    #[must_use]
    pub const fn build_timing(&self) -> ScheduledOracleBuildTiming {
        self.build_timing
    }

    fn apply_level(&self, level: usize, rhs: &[f64]) -> Result<Vec<f64>, MultiwayError> {
        let problem = &self.problems[level];
        if rhs.len() != problem.dimension() {
            return Err(crate::error::dimension(
                "ScheduledOracleHierarchy::apply_level",
                problem.dimension(),
                rhs.len(),
            ));
        }
        if level == self.aggregations.len() {
            let mut solution = vec![0.0; problem.dimension()];
            self.terminal.solve_into(rhs, &mut solution)?;
            problem
                .components()
                .project_structural_range(&mut solution)?;
            return Ok(solution);
        }

        let mut compatible_rhs = rhs.to_vec();
        problem
            .components()
            .project_structural_range(&mut compatible_rhs)?;
        let mut solution = vec![0.0; problem.dimension()];
        for _ in 0..self.sweeps {
            smoothing_step(
                problem,
                &self.smoothers[level],
                &compatible_rhs,
                &mut solution,
            )?;
        }
        let residual = problem.residual(&compatible_rhs, &solution)?;
        let coarse_problem = &self.problems[level + 1];
        let mut coarse_rhs = vec![0.0; coarse_problem.dimension()];
        self.aggregations[level].restrict(&residual, &mut coarse_rhs)?;
        coarse_problem
            .components()
            .project_structural_range(&mut coarse_rhs)?;
        let coarse_solution = self.apply_level(level + 1, &coarse_rhs)?;
        let mut prolongated = vec![0.0; problem.dimension()];
        self.aggregations[level].prolong(&coarse_solution, &mut prolongated)?;
        add_assign(&mut solution, &prolongated);
        for _ in 0..self.sweeps {
            smoothing_step(
                problem,
                &self.smoothers[level],
                &compatible_rhs,
                &mut solution,
            )?;
        }
        problem
            .components()
            .project_structural_range(&mut solution)?;
        Ok(solution)
    }
}

impl Preconditioner for ScheduledOracleHierarchy {
    fn dimension(&self) -> usize {
        self.finest_problem().dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        if rhs.len() != self.dimension() {
            return Err(crate::error::dimension(
                "ScheduledOracleHierarchy::apply rhs",
                self.dimension(),
                rhs.len(),
            ));
        }
        if out.len() != self.dimension() {
            return Err(crate::error::dimension(
                "ScheduledOracleHierarchy::apply output",
                self.dimension(),
                out.len(),
            ));
        }
        let solution = self.apply_level(0, rhs)?;
        out.copy_from_slice(&solution);
        Ok(())
    }
}

#[derive(Debug, Clone)]
enum LevelSmoother {
    Jacobi(DiagonalPreconditioner),
    SymmetricMap(SymmetricMapPreconditioner),
    AllPairsCmg(PairSubsetCmgPreconditioner),
}

impl LevelSmoother {
    fn build(
        problem: &ThreeWayProblem,
        spec: OracleLevelSmootherSpec,
    ) -> Result<Self, MultiwayError> {
        match spec {
            OracleLevelSmootherSpec::Jacobi { omega } => {
                Ok(Self::Jacobi(DiagonalPreconditioner::new(problem, omega)?))
            }
            OracleLevelSmootherSpec::SymmetricMap => Ok(Self::SymmetricMap(
                SymmetricMapPreconditioner::new(problem.clone()),
            )),
            OracleLevelSmootherSpec::AllPairsCmg { options } => Ok(Self::AllPairsCmg(
                PairSubsetCmgPreconditioner::build_all(problem.clone(), options)?,
            )),
        }
    }

    fn pair_memory(&self) -> Option<PairCmgMemoryReport> {
        match self {
            Self::AllPairsCmg(pair) => Some(pair.memory_report()),
            Self::Jacobi(_) | Self::SymmetricMap(_) => None,
        }
    }

    fn pair_timing(&self) -> Option<PairCmgBuildTiming> {
        match self {
            Self::AllPairsCmg(pair) => Some(pair.build_timing()),
            Self::Jacobi(_) | Self::SymmetricMap(_) => None,
        }
    }
}

impl Preconditioner for LevelSmoother {
    fn dimension(&self) -> usize {
        match self {
            Self::Jacobi(smoother) => smoother.dimension(),
            Self::SymmetricMap(smoother) => smoother.dimension(),
            Self::AllPairsCmg(smoother) => smoother.dimension(),
        }
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        match self {
            Self::Jacobi(smoother) => smoother.apply(rhs, out),
            Self::SymmetricMap(smoother) => smoother.apply(rhs, out),
            Self::AllPairsCmg(smoother) => smoother.apply(rhs, out),
        }
    }
}

fn smoothing_step(
    problem: &ThreeWayProblem,
    smoother: &LevelSmoother,
    rhs: &[f64],
    solution: &mut [f64],
) -> Result<(), MultiwayError> {
    let residual = problem.residual(rhs, solution)?;
    let mut correction = vec![0.0; problem.dimension()];
    smoother.apply(&residual, &mut correction)?;
    problem
        .components()
        .project_structural_range(&mut correction)?;
    add_assign(solution, &correction);
    Ok(())
}

fn memory_report(
    problems: &[ThreeWayProblem],
    aggregations: &[FactorAggregation],
    smoothers: &[LevelSmoother],
) -> ScheduledOracleMemoryReport {
    let problem_state_bytes_estimate = problems
        .iter()
        .map(estimate_three_way_problem_bytes)
        .sum::<usize>();
    let aggregation_bytes = aggregations
        .iter()
        .map(FactorAggregation::retained_bytes)
        .sum::<usize>();
    let smoother_bytes_estimate = smoothers
        .iter()
        .enumerate()
        .map(|(level, smoother)| match smoother {
            LevelSmoother::Jacobi(_) => problems[level].dimension().saturating_mul(8),
            LevelSmoother::SymmetricMap(_) | LevelSmoother::AllPairsCmg(_) => 0,
        })
        .sum::<usize>();
    let pair_cmg_preconditioner_bytes = smoothers
        .iter()
        .filter_map(LevelSmoother::pair_memory)
        .map(PairCmgMemoryReport::cmg_preconditioner_bytes)
        .sum::<usize>();
    let pair_cmg_workspace_bytes = smoothers
        .iter()
        .filter_map(LevelSmoother::pair_memory)
        .map(PairCmgMemoryReport::pair_workspace_bytes)
        .sum::<usize>();
    let terminal_dimension = problems.last().map_or(0, ThreeWayProblem::dimension);
    let terminal_bytes_estimate = terminal_dimension
        .saturating_mul(terminal_dimension.saturating_add(1))
        .saturating_mul(8);
    let total_retained_bytes_estimate = problem_state_bytes_estimate
        .saturating_add(aggregation_bytes)
        .saturating_add(smoother_bytes_estimate)
        .saturating_add(pair_cmg_preconditioner_bytes)
        .saturating_add(pair_cmg_workspace_bytes)
        .saturating_add(terminal_bytes_estimate);
    let maximum_apply_scratch_bytes_estimate = problems
        .iter()
        .map(|problem| problem.dimension().saturating_mul(7).saturating_mul(8))
        .sum::<usize>();
    ScheduledOracleMemoryReport {
        problem_state_bytes_estimate,
        aggregation_bytes,
        smoother_bytes_estimate,
        pair_cmg_preconditioner_bytes,
        pair_cmg_workspace_bytes,
        terminal_bytes_estimate,
        total_retained_bytes_estimate,
        maximum_apply_scratch_bytes_estimate,
    }
}

fn add_assign(destination: &mut [f64], source: &[f64]) {
    for (left, &right) in destination.iter_mut().zip(source) {
        *left += right;
    }
}
