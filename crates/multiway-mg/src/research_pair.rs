//! Research pair-CMG variants with explicit pair selection and memory reports.
//!
//! The production-facing [`crate::PairCmgPreconditioner`] always builds all
//! three factor pairs. Issue #2 also needs controlled experiments using one
//! dominant pair, selected pair portfolios, and explicit setup/memory
//! accounting.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cmg::{CmgOptions, CmgPreconditioner, CmgWorkspace, Components, Laplacian};

use crate::{
    MultiwayError, PairCmgOptions, Preconditioner, ThreeWayProblem,
    memory_estimate::estimate_three_way_problem_bytes,
};

/// One of the three bipartite factor pairs in a three-way problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FactorPair {
    /// Factors one and two.
    OneTwo,
    /// Factors one and three.
    OneThree,
    /// Factors two and three.
    TwoThree,
}

impl FactorPair {
    /// Canonical list of all factor pairs.
    pub const ALL: [Self; 3] = [Self::OneTwo, Self::OneThree, Self::TwoThree];

    /// Zero-based factor indices.
    #[must_use]
    pub const fn factors(self) -> (usize, usize) {
        match self {
            Self::OneTwo => (0, 1),
            Self::OneThree => (0, 2),
            Self::TwoThree => (1, 2),
        }
    }

    /// Stable human-readable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OneTwo => "1-2",
            Self::OneThree => "1-3",
            Self::TwoThree => "2-3",
        }
    }
}

/// Build-phase timing for a selected pair-CMG portfolio.
///
/// Timings are diagnostics from one construction and are not deterministic
/// routing inputs. `total` includes validation and small bookkeeping in
/// addition to the three named phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairCmgBuildTiming {
    pair_graph_setup: Duration,
    cmg_setup: Duration,
    workspace_setup: Duration,
    total: Duration,
}

impl PairCmgBuildTiming {
    /// Marginal accumulation, graph construction, and component discovery.
    #[must_use]
    pub const fn pair_graph_setup(self) -> Duration {
        self.pair_graph_setup
    }

    /// CMG hierarchy/preconditioner construction.
    #[must_use]
    pub const fn cmg_setup(self) -> Duration {
        self.cmg_setup
    }

    /// Retained pair RHS, solution, and CMG workspace construction.
    #[must_use]
    pub const fn workspace_setup(self) -> Duration {
        self.workspace_setup
    }

    /// Complete selected-pair constructor time.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

/// Principal heap-memory report for a selected pair-CMG preconditioner.
///
/// The problem-state estimate counts immutable tuple, weight, diagonal, and
/// component arrays once. It deliberately excludes allocator metadata, mutex
/// object headers, stack fields, and sharing with another owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairCmgMemoryReport {
    problem_state_bytes_estimate: usize,
    cmg_preconditioner_bytes: usize,
    pair_workspace_bytes: usize,
    pair_metadata_bytes_estimate: usize,
    total_retained_bytes_estimate: usize,
}

impl PairCmgMemoryReport {
    /// Estimated principal bytes in the shared three-way problem state.
    #[must_use]
    pub const fn problem_state_bytes_estimate(self) -> usize {
        self.problem_state_bytes_estimate
    }

    /// Exact principal bytes reported by all retained CMG preconditioners.
    #[must_use]
    pub const fn cmg_preconditioner_bytes(self) -> usize {
        self.cmg_preconditioner_bytes
    }

    /// Exact CMG workspace bytes plus explicit pair RHS/solution buffers.
    #[must_use]
    pub const fn pair_workspace_bytes(self) -> usize {
        self.pair_workspace_bytes
    }

    /// Conservative pair labels and component-label estimate.
    #[must_use]
    pub const fn pair_metadata_bytes_estimate(self) -> usize {
        self.pair_metadata_bytes_estimate
    }

    /// Sum of the reported principal retained categories.
    #[must_use]
    pub const fn total_retained_bytes_estimate(self) -> usize {
        self.total_retained_bytes_estimate
    }
}

/// Fixed additive CMG correction over an explicit subset of factor pairs.
///
/// A strict subset is generally positive semidefinite rather than positive
/// definite on the complete three-way range. Combine it with a positive
/// background smoother before using ordinary PCG.
#[derive(Debug, Clone)]
pub struct PairSubsetCmgPreconditioner {
    problem: ThreeWayProblem,
    selected_pairs: Vec<FactorPair>,
    systems: Vec<PairSystem>,
    partition_weight: f64,
    build_timing: PairCmgBuildTiming,
}

impl PairSubsetCmgPreconditioner {
    /// Build a canonical selected-pair portfolio.
    pub fn build(
        problem: ThreeWayProblem,
        selected_pairs: &[FactorPair],
        options: PairCmgOptions,
    ) -> Result<Self, MultiwayError> {
        let total_start = Instant::now();
        options
            .cmg
            .validate()
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        if !options.partition_weight.is_finite() || options.partition_weight <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "partition_weight",
                message: format!(
                    "must be finite and positive, got {}",
                    options.partition_weight
                ),
            });
        }
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

        let mut systems = Vec::with_capacity(selected_pairs.len());
        let mut pair_graph_setup = Duration::ZERO;
        let mut cmg_setup = Duration::ZERO;
        let mut workspace_setup = Duration::ZERO;
        for pair in &selected_pairs {
            let (system, timing) = PairSystem::build(&problem, *pair, options.cmg)?;
            pair_graph_setup += timing.pair_graph_setup;
            cmg_setup += timing.cmg_setup;
            workspace_setup += timing.workspace_setup;
            systems.push(system);
        }
        let build_timing = PairCmgBuildTiming {
            pair_graph_setup,
            cmg_setup,
            workspace_setup,
            total: total_start.elapsed(),
        };
        Ok(Self {
            problem,
            selected_pairs,
            systems,
            partition_weight: options.partition_weight,
            build_timing,
        })
    }

    /// Build all three factor pairs using the ordinary partition weight.
    pub fn build_all(
        problem: ThreeWayProblem,
        options: PairCmgOptions,
    ) -> Result<Self, MultiwayError> {
        Self::build(problem, &FactorPair::ALL, options)
    }

    /// Selected pairs in canonical order.
    #[must_use]
    pub fn selected_pairs(&self) -> &[FactorPair] {
        &self.selected_pairs
    }

    /// Underlying weighted problem.
    #[must_use]
    pub const fn problem(&self) -> &ThreeWayProblem {
        &self.problem
    }

    /// Build-phase timing from construction.
    #[must_use]
    pub const fn build_timing(&self) -> PairCmgBuildTiming {
        self.build_timing
    }

    /// Principal retained-memory report.
    #[must_use]
    pub fn memory_report(&self) -> PairCmgMemoryReport {
        let problem_state_bytes_estimate = estimate_problem_bytes(&self.problem);
        let cmg_preconditioner_bytes = self
            .systems
            .iter()
            .map(|system| system.cmg_retained_bytes)
            .sum();
        let pair_workspace_bytes = self
            .systems
            .iter()
            .map(|system| system.workspace_bytes)
            .sum();
        let pair_metadata_bytes_estimate = self
            .systems
            .iter()
            .map(|system| {
                system
                    .local_dimension()
                    .saturating_mul(core::mem::size_of::<usize>())
            })
            .sum::<usize>()
            .saturating_add(
                self.selected_pairs
                    .capacity()
                    .saturating_mul(core::mem::size_of::<FactorPair>()),
            );
        let total_retained_bytes_estimate = problem_state_bytes_estimate
            .saturating_add(cmg_preconditioner_bytes)
            .saturating_add(pair_workspace_bytes)
            .saturating_add(pair_metadata_bytes_estimate);
        PairCmgMemoryReport {
            problem_state_bytes_estimate,
            cmg_preconditioner_bytes,
            pair_workspace_bytes,
            pair_metadata_bytes_estimate,
            total_retained_bytes_estimate,
        }
    }
}

impl Preconditioner for PairSubsetCmgPreconditioner {
    fn dimension(&self) -> usize {
        self.problem.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        validate_dimensions(
            "PairSubsetCmgPreconditioner::apply",
            self.dimension(),
            rhs,
            out,
        )?;
        let mut compatible_rhs = rhs.to_vec();
        self.problem
            .components()
            .project_structural_range(&mut compatible_rhs)?;
        out.fill(0.0);
        for system in &self.systems {
            system.accumulate(&compatible_rhs, out, self.partition_weight)?;
        }
        self.problem.components().project_structural_range(out)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct PairSystem {
    pair: FactorPair,
    first_count: usize,
    second_count: usize,
    first_offset: usize,
    second_offset: usize,
    components: Components,
    preconditioner: CmgPreconditioner,
    workspace: Arc<Mutex<PairWorkspace>>,
    cmg_retained_bytes: usize,
    workspace_bytes: usize,
}

impl PairSystem {
    fn build(
        problem: &ThreeWayProblem,
        pair: FactorPair,
        options: CmgOptions,
    ) -> Result<(Self, PairCmgBuildTiming), MultiwayError> {
        let total_start = Instant::now();
        let pair_graph_start = Instant::now();
        let (first, second) = pair.factors();
        let counts = problem.topology().level_counts();
        let offsets = problem.topology().offsets();
        let first_count = counts[first];
        let second_count = counts[second];
        let mut marginal: BTreeMap<(u32, u32), f64> = BTreeMap::new();
        for (&tuple, &weight) in problem.topology().tuples().iter().zip(problem.weights()) {
            *marginal.entry((tuple[first], tuple[second])).or_insert(0.0) += weight;
        }
        let edges = marginal
            .into_iter()
            .map(|((left, right), weight)| (left as usize, first_count + right as usize, weight));
        let graph = Laplacian::from_edges(first_count + second_count, edges)
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        let components = Components::from_laplacian(&graph);
        let pair_graph_setup = pair_graph_start.elapsed();

        let cmg_start = Instant::now();
        let preconditioner = CmgPreconditioner::build(&graph, options)
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        let cmg_setup = cmg_start.elapsed();

        let workspace_start = Instant::now();
        let local_dimension = first_count + second_count;
        let cmg_workspace = preconditioner.workspace();
        let workspace_bytes = cmg_workspace
            .byte_len()
            .saturating_add(local_dimension.saturating_mul(16));
        let cmg_retained_bytes = preconditioner.retained_bytes();
        let workspace = PairWorkspace {
            rhs: vec![0.0; local_dimension],
            solution: vec![0.0; local_dimension],
            cmg: cmg_workspace,
        };
        let workspace_setup = workspace_start.elapsed();
        let system = Self {
            pair,
            first_count,
            second_count,
            first_offset: offsets[first],
            second_offset: offsets[second],
            components,
            preconditioner,
            workspace: Arc::new(Mutex::new(workspace)),
            cmg_retained_bytes,
            workspace_bytes,
        };
        let timing = PairCmgBuildTiming {
            pair_graph_setup,
            cmg_setup,
            workspace_setup,
            total: total_start.elapsed(),
        };
        Ok((system, timing))
    }

    const fn local_dimension(&self) -> usize {
        self.first_count + self.second_count
    }

    fn accumulate(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        partition_weight: f64,
    ) -> Result<(), MultiwayError> {
        let mut workspace = self.workspace.lock().map_err(|_| {
            MultiwayError::Cmg(format!(
                "pair {} CMG workspace lock was poisoned",
                self.pair.label()
            ))
        })?;
        workspace.rhs.fill(0.0);
        workspace.solution.fill(0.0);
        for level in 0..self.first_count {
            workspace.rhs[level] = partition_weight * rhs[self.first_offset + level];
        }
        for level in 0..self.second_count {
            workspace.rhs[self.first_count + level] =
                -partition_weight * rhs[self.second_offset + level];
        }
        self.components
            .center_in_place(&mut workspace.rhs)
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        let PairWorkspace { rhs, solution, cmg } = &mut *workspace;
        self.preconditioner
            .apply_compatible_into(rhs, solution, cmg)
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        for level in 0..self.first_count {
            out[self.first_offset + level] += partition_weight * solution[level];
        }
        for level in 0..self.second_count {
            out[self.second_offset + level] -=
                partition_weight * solution[self.first_count + level];
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct PairWorkspace {
    rhs: Vec<f64>,
    solution: Vec<f64>,
    cmg: CmgWorkspace,
}

/// Principal immutable state estimate used by issue #2 diagnostics.
#[must_use]
pub fn estimate_problem_bytes(problem: &ThreeWayProblem) -> usize {
    estimate_three_way_problem_bytes(problem)
}

fn validate_dimensions(
    context: &'static str,
    dimension: usize,
    rhs: &[f64],
    out: &[f64],
) -> Result<(), MultiwayError> {
    if rhs.len() != dimension {
        return Err(crate::error::dimension(context, dimension, rhs.len()));
    }
    if out.len() != dimension {
        return Err(crate::error::dimension(context, dimension, out.len()));
    }
    Ok(())
}
