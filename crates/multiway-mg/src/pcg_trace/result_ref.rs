//! Borrowed result ownership for caller-owned traced-PCG storage.

use super::{PcgTraceResult, PcgTraceSample};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PcgTraceSummary {
    pub(super) iterations: usize,
    pub(super) converged: bool,
    pub(super) rhs_projection_norm: f64,
    pub(super) gramian_applications: usize,
    pub(super) preconditioner_applications: usize,
}

/// Completed solution and residual trace borrowed from a [`super::PcgTraceWorkspace`].
///
/// A successful call produces this view without allocating or copying either
/// array. Holding a live view prevents another mutable use of that workspace.
/// Use [`Self::to_owned`] explicitly to retain an independent result across solves.
///
/// ```compile_fail
/// use multiway_mg::{
///     PcgTraceOptions, PcgTraceWorkspace, SymmetricMapPreconditioner, ThreeWayProblem,
///     solve_projected_pcg_traced_with_workspace,
/// };
/// fn cannot_overwrite_live_result(problem: &ThreeWayProblem, rhs: &[f64]) {
///     let options = PcgTraceOptions::default();
///     let map = SymmetricMapPreconditioner::new(problem.clone());
///     let mut workspace = PcgTraceWorkspace::try_new(problem, options).unwrap();
///     let first = solve_projected_pcg_traced_with_workspace(
///         problem, rhs, &map, options, &mut workspace,
///     ).unwrap();
///     let second = solve_projected_pcg_traced_with_workspace(
///         problem, rhs, &map, options, &mut workspace,
///     ).unwrap();
///     assert_eq!(first.solution(), second.solution());
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcgTraceResultRef<'a> {
    solution: &'a [f64],
    samples: &'a [PcgTraceSample],
    summary: PcgTraceSummary,
}

impl<'a> PcgTraceResultRef<'a> {
    pub(super) const fn new(
        solution: &'a [f64],
        samples: &'a [PcgTraceSample],
        summary: PcgTraceSummary,
    ) -> Self {
        Self {
            solution,
            samples,
            summary,
        }
    }

    /// Final normalized solution candidate, valid for this workspace borrow.
    #[must_use]
    pub const fn solution(&self) -> &'a [f64] {
        self.solution
    }

    /// Completed iterations.
    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.summary.iterations
    }

    /// Whether the recomputed projected residual met tolerance.
    #[must_use]
    pub const fn converged(&self) -> bool {
        self.summary.converged
    }

    /// Euclidean norm removed by the structural-range projection of the RHS.
    #[must_use]
    pub const fn rhs_projection_norm(&self) -> f64 {
        self.summary.rhs_projection_norm
    }

    /// Original-Gramian applications, including the residual audits.
    #[must_use]
    pub const fn gramian_applications(&self) -> usize {
        self.summary.gramian_applications
    }

    /// Preconditioner applications, including the original iteration-limit call.
    #[must_use]
    pub const fn preconditioner_applications(&self) -> usize {
        self.summary.preconditioner_applications
    }

    /// Initial and per-iteration residual samples; unused capacity is not exposed.
    #[must_use]
    pub const fn samples(&self) -> &'a [PcgTraceSample] {
        self.samples
    }

    /// Final relative true residual, using the same convention as the owned result.
    #[must_use]
    pub fn final_relative_residual(&self) -> f64 {
        self.samples
            .last()
            .map_or(0.0, |sample| sample.relative_residual())
    }

    /// Explicitly allocate and copy both arrays into an independent owned result.
    ///
    /// This allocation is outside the zero-allocation borrowed-solve contract.
    #[must_use]
    pub fn to_owned(&self) -> PcgTraceResult {
        PcgTraceResult::from_parts(self.solution.to_vec(), self.samples.to_vec(), self.summary)
    }
}

impl PcgTraceResult {
    pub(super) fn from_parts(
        solution: Vec<f64>,
        samples: Vec<PcgTraceSample>,
        summary: PcgTraceSummary,
    ) -> Self {
        Self {
            solution,
            samples,
            iterations: summary.iterations,
            converged: summary.converged,
            rhs_projection_norm: summary.rhs_projection_norm,
            gramian_applications: summary.gramian_applications,
            preconditioner_applications: summary.preconditioner_applications,
        }
    }
}
