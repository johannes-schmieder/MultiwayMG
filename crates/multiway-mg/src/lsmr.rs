//! Rectangular weighted least-squares driver using modified LSMR.

use std::sync::atomic::{AtomicUsize, Ordering};

use schwarz_precond::{LsmrStopReason, MlsmrOptions, Operator, SolveError, mlsmr};

use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

/// Options for the rectangular weighted least-squares solve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeastSquaresOptions {
    /// Relative normal-equation tolerance used by modified LSMR.
    pub tolerance: f64,
    /// Maximum LSMR iterations.
    pub max_iterations: usize,
    /// Optional local reorthogonalization window.
    pub local_size: Option<usize>,
}

impl Default for LeastSquaresOptions {
    fn default() -> Self {
        Self {
            tolerance: 1.0e-8,
            max_iterations: 1_000,
            local_size: Some(8),
        }
    }
}

impl LeastSquaresOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if !self.tolerance.is_finite() || self.tolerance <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "least_squares_tolerance",
                message: format!("must be finite and positive, got {}", self.tolerance),
            });
        }
        if self.max_iterations == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "least_squares_max_iterations",
                message: "must be positive".to_owned(),
            });
        }
        Ok(self)
    }
}

/// Stable local copy of the external LSMR stop reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeastSquaresStopReason {
    /// Submitted weighted RHS was zero.
    ZeroRightHandSide,
    /// Initial normal-equation residual was zero.
    InitialNormalEquationResidualZero,
    /// Weighted least-squares residual tolerance was reached.
    ResidualTolerance,
    /// Relative normal-equation tolerance was reached.
    NormalEquationTolerance,
    /// A warm start was already exact.
    WarmStartExact,
    /// Recurrence convergence was refuted by the true-residual audit.
    FalseConvergence,
    /// Iteration budget was exhausted.
    MaximumIterations,
    /// The external driver requested escalation.
    Escalated,
}

/// Exact matrix-free work counts for one modified-LSMR solve and its certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeastSquaresWorkReport {
    solver_weighted_incidence_applications: usize,
    solver_weighted_adjoint_applications: usize,
    preconditioner_applications: usize,
    certification_incidence_applications: usize,
    certification_adjoint_applications: usize,
}

impl LeastSquaresWorkReport {
    /// `sqrt(W) B` applications performed inside modified LSMR.
    #[must_use]
    pub const fn solver_weighted_incidence_applications(self) -> usize {
        self.solver_weighted_incidence_applications
    }

    /// `B' sqrt(W)` applications performed inside modified LSMR.
    #[must_use]
    pub const fn solver_weighted_adjoint_applications(self) -> usize {
        self.solver_weighted_adjoint_applications
    }

    /// Fixed Gram-preconditioner applications performed inside modified LSMR.
    #[must_use]
    pub const fn preconditioner_applications(self) -> usize {
        self.preconditioner_applications
    }

    /// Unweighted `B` applications used by the independent final certificate.
    #[must_use]
    pub const fn certification_incidence_applications(self) -> usize {
        self.certification_incidence_applications
    }

    /// Weighted `B'` applications used by the independent final certificate.
    #[must_use]
    pub const fn certification_adjoint_applications(self) -> usize {
        self.certification_adjoint_applications
    }

    /// Total rectangular outer-operator applications inside modified LSMR.
    #[must_use]
    pub const fn solver_outer_operator_applications(self) -> usize {
        self.solver_weighted_incidence_applications
            .saturating_add(self.solver_weighted_adjoint_applications)
    }

    /// Total incidence/adjoint applications including final certification.
    #[must_use]
    pub const fn complete_incidence_applications(self) -> usize {
        self.solver_outer_operator_applications()
            .saturating_add(self.certification_incidence_applications)
            .saturating_add(self.certification_adjoint_applications)
    }
}

/// Result plus an independent normal-equation residual in the original problem.
#[derive(Debug, Clone, PartialEq)]
pub struct LeastSquaresResult {
    coefficients: Vec<f64>,
    converged: bool,
    iterations: usize,
    weighted_residual_norm: f64,
    solver_normal_equation_residual: f64,
    certified_normal_equation_residual: f64,
    stop_reason: LeastSquaresStopReason,
    work: LeastSquaresWorkReport,
}

impl LeastSquaresResult {
    /// Structurally normalized coefficient candidate.
    #[must_use]
    pub fn coefficients(&self) -> &[f64] {
        &self.coefficients
    }

    /// Consume the result and return coefficients.
    #[must_use]
    pub fn into_coefficients(self) -> Vec<f64> {
        self.coefficients
    }

    /// Whether modified LSMR reported convergence.
    #[must_use]
    pub const fn converged(&self) -> bool {
        self.converged
    }

    /// Iteration count.
    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }

    /// Final weighted observation-space residual norm reported by LSMR.
    #[must_use]
    pub const fn weighted_residual_norm(&self) -> f64 {
        self.weighted_residual_norm
    }

    /// LSMR's own normal-equation residual diagnostic.
    #[must_use]
    pub const fn solver_normal_equation_residual(&self) -> f64 {
        self.solver_normal_equation_residual
    }

    /// Independently recomputed `||B^T W r|| / ||B^T W y||`.
    #[must_use]
    pub const fn certified_normal_equation_residual(&self) -> f64 {
        self.certified_normal_equation_residual
    }

    /// Stop reason.
    #[must_use]
    pub const fn stop_reason(&self) -> LeastSquaresStopReason {
        self.stop_reason
    }

    /// Exact matrix-free work counts for the solve and independent certificate.
    #[must_use]
    pub const fn work(&self) -> LeastSquaresWorkReport {
        self.work
    }
}

/// Solve `min_x ||sqrt(W) (targets - Bx)||_2` using a Gramian preconditioner.
pub fn solve_weighted_least_squares<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    targets: &[f64],
    preconditioner: &P,
    options: LeastSquaresOptions,
) -> Result<LeastSquaresResult, MultiwayError> {
    let options = options.validate()?;
    if targets.len() != problem.tuple_count() {
        return Err(crate::error::dimension(
            "solve_weighted_least_squares targets",
            problem.tuple_count(),
            targets.len(),
        ));
    }
    if preconditioner.dimension() != problem.dimension() {
        return Err(crate::error::dimension(
            "solve_weighted_least_squares preconditioner",
            problem.dimension(),
            preconditioner.dimension(),
        ));
    }

    let weighted_incidence_applications = AtomicUsize::new(0);
    let weighted_adjoint_applications = AtomicUsize::new(0);
    let preconditioner_applications = AtomicUsize::new(0);
    let operator = WeightedIncidenceOperator {
        problem,
        incidence_applications: &weighted_incidence_applications,
        adjoint_applications: &weighted_adjoint_applications,
    };
    let preconditioner = PreconditionerOperator {
        preconditioner,
        applications: &preconditioner_applications,
    };
    let weighted_targets: Vec<f64> = targets
        .iter()
        .zip(problem.square_root_weights())
        .map(|(&target, &sqrt_weight)| target * sqrt_weight)
        .collect();
    let result = mlsmr(
        &operator,
        &weighted_targets,
        &preconditioner,
        options.tolerance,
        options.max_iterations,
        MlsmrOptions {
            warm_start: None,
            escalation: None,
            local_size: options.local_size,
        },
    )
    .map_err(|error| MultiwayError::Lsmr(error.to_string()))?;

    let mut coefficients = result.x;
    problem
        .components()
        .project_structural_range(&mut coefficients)?;
    let certified_normal_equation_residual =
        certify_normal_equations(problem, targets, &coefficients)?;
    Ok(LeastSquaresResult {
        coefficients,
        converged: result.converged,
        iterations: result.iterations,
        weighted_residual_norm: result.residual_norm,
        solver_normal_equation_residual: result.normal_eq_residual,
        certified_normal_equation_residual,
        stop_reason: convert_stop_reason(result.stop_reason),
        work: LeastSquaresWorkReport {
            solver_weighted_incidence_applications: weighted_incidence_applications
                .load(Ordering::Relaxed),
            solver_weighted_adjoint_applications: weighted_adjoint_applications
                .load(Ordering::Relaxed),
            preconditioner_applications: preconditioner_applications.load(Ordering::Relaxed),
            certification_incidence_applications: 1,
            certification_adjoint_applications: 2,
        },
    })
}

struct WeightedIncidenceOperator<'a> {
    problem: &'a ThreeWayProblem,
    incidence_applications: &'a AtomicUsize,
    adjoint_applications: &'a AtomicUsize,
}

impl Operator for WeightedIncidenceOperator<'_> {
    fn nrows(&self) -> usize {
        self.problem.tuple_count()
    }

    fn ncols(&self) -> usize {
        self.problem.dimension()
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> Result<(), SolveError> {
        self.incidence_applications.fetch_add(1, Ordering::Relaxed);
        self.problem
            .apply_weighted_incidence(x, y)
            .map_err(|error| external_error("weighted incidence apply", error))
    }

    fn apply_adjoint(&self, x: &[f64], y: &mut [f64]) -> Result<(), SolveError> {
        self.adjoint_applications.fetch_add(1, Ordering::Relaxed);
        self.problem
            .apply_weighted_adjoint(x, y)
            .map_err(|error| external_error("weighted incidence adjoint", error))
    }
}

struct PreconditionerOperator<'a, P: Preconditioner + ?Sized> {
    preconditioner: &'a P,
    applications: &'a AtomicUsize,
}

impl<P: Preconditioner + ?Sized> Operator for PreconditionerOperator<'_, P> {
    fn nrows(&self) -> usize {
        self.preconditioner.dimension()
    }

    fn ncols(&self) -> usize {
        self.preconditioner.dimension()
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> Result<(), SolveError> {
        self.applications.fetch_add(1, Ordering::Relaxed);
        self.preconditioner
            .apply(x, y)
            .map_err(|error| external_error("multiway preconditioner apply", error))
    }

    fn apply_adjoint(&self, x: &[f64], y: &mut [f64]) -> Result<(), SolveError> {
        self.apply(x, y)
    }
}

fn certify_normal_equations(
    problem: &ThreeWayProblem,
    targets: &[f64],
    coefficients: &[f64],
) -> Result<f64, MultiwayError> {
    let mut fitted = vec![0.0; problem.tuple_count()];
    problem.apply_incidence(coefficients, &mut fitted)?;
    for (value, &target) in fitted.iter_mut().zip(targets) {
        *value = target - *value;
    }
    let gradient = problem.rhs_from_targets(&fitted)?;
    let reference = problem.rhs_from_targets(targets)?;
    let numerator = norm(&gradient);
    let denominator = norm(&reference);
    Ok(if denominator == 0.0 {
        if numerator == 0.0 { 0.0 } else { f64::INFINITY }
    } else {
        numerator / denominator
    })
}

fn convert_stop_reason(reason: LsmrStopReason) -> LeastSquaresStopReason {
    match reason {
        LsmrStopReason::ZeroRhs => LeastSquaresStopReason::ZeroRightHandSide,
        LsmrStopReason::InitialNormalEquationResidualZero => {
            LeastSquaresStopReason::InitialNormalEquationResidualZero
        }
        LsmrStopReason::ResidualTolerance => LeastSquaresStopReason::ResidualTolerance,
        LsmrStopReason::NormalEquationTolerance => LeastSquaresStopReason::NormalEquationTolerance,
        LsmrStopReason::WarmStartExact => LeastSquaresStopReason::WarmStartExact,
        LsmrStopReason::FalseConvergence => LeastSquaresStopReason::FalseConvergence,
        LsmrStopReason::MaxIterations => LeastSquaresStopReason::MaximumIterations,
        LsmrStopReason::Escalated => LeastSquaresStopReason::Escalated,
    }
}

fn external_error(context: &'static str, error: impl std::fmt::Display) -> SolveError {
    SolveError::InvalidInput {
        context,
        message: error.to_string(),
    }
}

fn norm(values: &[f64]) -> f64 {
    let scale = values.iter().copied().map(f64::abs).fold(0.0, f64::max);
    if scale == 0.0 {
        return 0.0;
    }
    scale
        * values
            .iter()
            .map(|value| (value / scale) * (value / scale))
            .sum::<f64>()
            .sqrt()
}
