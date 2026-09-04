//! Frozen `within` approximate-Cholesky Schwarz comparator.
//!
//! This module deliberately uses the public production preconditioner from the
//! exact dependency revision pinned by the workspace. It does not copy or
//! reimplement approximate Cholesky, factor-pair construction, component
//! splitting, partition-of-unity weights, or the generic Schwarz executor.

use std::time::{Duration, Instant};

use schwarz_precond::ReductionStrategy;
use within::{Effect, LocalSolverConfig, PreconditionerConfig, Solver};

use crate::{
    MultiwayError, Preconditioner, ThreeWayProblem,
    memory_estimate::estimate_three_way_problem_bytes,
    structural_projection::StructuralRangeProjector,
};

/// Frozen `within` local-solver and Schwarz reduction configuration.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WithinApproxCholOptions {
    /// Existing approximate-Cholesky/block-elimination local solver settings.
    pub local_solver: LocalSolverConfig,
    /// Existing generic additive-Schwarz reduction backend.
    pub reduction: ReductionStrategy,
}

/// Phase-separated setup timing for the frozen `within` comparator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WithinApproxCholBuildTiming {
    design_input_setup: Duration,
    within_solver_setup: Duration,
    within_preconditioner_setup: Duration,
    projection_setup: Duration,
    total: Duration,
}

impl WithinApproxCholBuildTiming {
    /// Copy canonical tuple codes and weights into the comparator input shape.
    #[must_use]
    pub const fn design_input_setup(self) -> Duration {
        self.design_input_setup
    }

    /// Complete public `within::Solver::new` wall time.
    #[must_use]
    pub const fn within_solver_setup(self) -> Duration {
        self.within_solver_setup
    }

    /// Subset of solver setup reported by the retained `within` preconditioner.
    #[must_use]
    pub const fn within_preconditioner_setup(self) -> Duration {
        self.within_preconditioner_setup
    }

    /// Retained allocation-free three-way range-projector setup.
    #[must_use]
    pub const fn projection_setup(self) -> Duration {
        self.projection_setup
    }

    /// Complete wrapper construction wall time.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

/// Retained-state accounting available through the frozen public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WithinApproxCholMemoryReport {
    problem_state_bytes_estimate: usize,
    projection_workspace_bytes: usize,
    within_retained_bytes: Option<usize>,
}

impl WithinApproxCholMemoryReport {
    /// Estimated immutable three-way problem bytes shared by the wrapper.
    #[must_use]
    pub const fn problem_state_bytes_estimate(self) -> usize {
        self.problem_state_bytes_estimate
    }

    /// Retained allocation-free structural-range projection workspace.
    #[must_use]
    pub const fn projection_workspace_bytes(self) -> usize {
        self.projection_workspace_bytes
    }

    /// Retained bytes in the `within` preconditioner when exposed upstream.
    ///
    /// The pinned comparator revision does not expose this quantity, so the
    /// value is currently `None`. Issue-4 isolated benchmarks must measure the
    /// complete-process retained and peak RSS rather than inventing a number.
    #[must_use]
    pub const fn within_retained_bytes(self) -> Option<usize> {
        self.within_retained_bytes
    }

    /// Sum of retained categories known exactly or estimated in this wrapper.
    ///
    /// This intentionally excludes the opaque `within` preconditioner state.
    #[must_use]
    pub const fn known_retained_bytes_estimate(self) -> usize {
        self.problem_state_bytes_estimate
            .saturating_add(self.projection_workspace_bytes)
    }
}

/// Production `within` additive pair Schwarz, wrapped as a MultiwayMG
/// coefficient-space preconditioner.
///
/// The submitted three-way problem is converted to three intercept-only
/// effects over the same canonical unique tuples and positive weights. The
/// retained public `within::Preconditioner` therefore owns the existing pair
/// cross-tabs, component-local block-elimination/approximate-Cholesky solvers,
/// partition weights, pooled scratch buffers, and parallel executor.
pub struct WithinApproxCholPreconditioner {
    problem: ThreeWayProblem,
    inner: within::Preconditioner,
    warnings: Vec<String>,
    build_timing: WithinApproxCholBuildTiming,
    memory_report: WithinApproxCholMemoryReport,
    projection: StructuralRangeProjector,
}

impl core::fmt::Debug for WithinApproxCholPreconditioner {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("WithinApproxCholPreconditioner")
            .field("warnings", &self.warnings)
            .field("build_timing", &self.build_timing)
            .field("memory_report", &self.memory_report)
            .finish_non_exhaustive()
    }
}

impl WithinApproxCholPreconditioner {
    /// Build the exact pinned `within` all-pair additive Schwarz comparator.
    pub fn build(
        problem: ThreeWayProblem,
        options: WithinApproxCholOptions,
    ) -> Result<Self, MultiwayError> {
        let total_start = Instant::now();

        let input_start = Instant::now();
        let mut levels: [Vec<u32>; 3] =
            std::array::from_fn(|_| Vec::with_capacity(problem.topology().tuple_count()));
        for tuple in problem.topology().tuples() {
            for factor in 0..3 {
                levels[factor].push(tuple[factor]);
            }
        }
        let weights = problem.weights().to_vec();
        let effects = levels
            .iter()
            .map(|codes| Effect::new(codes, true, std::iter::empty::<&[f64]>()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| MultiwayError::Within(error.to_string()))?;
        let design_input_setup = input_start.elapsed();

        let within_start = Instant::now();
        let solver = Solver::new(
            effects,
            Some(weights),
            PreconditionerConfig::Additive {
                local_solver: options.local_solver,
                reduction: options.reduction,
            },
        )
        .map_err(|error| MultiwayError::Within(error.to_string()))?;
        let within_solver_setup = within_start.elapsed();
        let inner = solver
            .preconditioner()
            .cloned()
            .ok_or_else(|| MultiwayError::Within("within returned no preconditioner".to_owned()))?;
        if inner.nrows() != problem.dimension() || inner.ncols() != problem.dimension() {
            return Err(MultiwayError::Within(format!(
                "within preconditioner shape {}x{} does not match three-way dimension {}",
                inner.nrows(),
                inner.ncols(),
                problem.dimension()
            )));
        }
        let within_preconditioner_setup = inner.build_duration();
        let warnings = solver.warnings().iter().map(ToString::to_string).collect();

        let projection_start = Instant::now();
        let projection = StructuralRangeProjector::new(&problem);
        let projection_setup = projection_start.elapsed();
        let memory_report = WithinApproxCholMemoryReport {
            problem_state_bytes_estimate: estimate_three_way_problem_bytes(&problem),
            projection_workspace_bytes: projection.workspace_bytes(),
            within_retained_bytes: None,
        };
        let build_timing = WithinApproxCholBuildTiming {
            design_input_setup,
            within_solver_setup,
            within_preconditioner_setup,
            projection_setup,
            total: total_start.elapsed(),
        };

        Ok(Self {
            problem,
            inner,
            warnings,
            build_timing,
            memory_report,
            projection,
        })
    }

    /// Non-fatal warnings emitted by the frozen comparator build.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Phase-separated setup timing.
    #[must_use]
    pub const fn build_timing(&self) -> WithinApproxCholBuildTiming {
        self.build_timing
    }

    /// Retained-state accounting exposed by the frozen public API.
    #[must_use]
    pub const fn memory_report(&self) -> WithinApproxCholMemoryReport {
        self.memory_report
    }

    /// Number of emergency range-projector allocations after construction.
    #[must_use]
    pub fn projection_fallback_allocations(&self) -> usize {
        self.projection.fallback_allocations()
    }
}

impl Preconditioner for WithinApproxCholPreconditioner {
    fn dimension(&self) -> usize {
        self.problem.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        if rhs.len() != self.dimension() {
            return Err(crate::error::dimension(
                "WithinApproxCholPreconditioner::apply rhs",
                self.dimension(),
                rhs.len(),
            ));
        }
        if out.len() != self.dimension() {
            return Err(crate::error::dimension(
                "WithinApproxCholPreconditioner::apply output",
                self.dimension(),
                out.len(),
            ));
        }
        out.fill(0.0);
        if let Err(error) = self.inner.apply(rhs, out) {
            out.fill(0.0);
            return Err(MultiwayError::Within(error.to_string()));
        }
        if let Err(error) = self.projection.project(&self.problem, out) {
            out.fill(0.0);
            return Err(error);
        }
        Ok(())
    }
}
