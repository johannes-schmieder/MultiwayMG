//! Projected preconditioned conjugate gradients for the singular Gramian.

use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

/// Options for the research projected-PCG driver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcgOptions {
    /// Relative Euclidean residual tolerance.
    pub relative_tolerance: f64,
    /// Absolute Euclidean residual tolerance.
    pub absolute_tolerance: f64,
    /// Maximum number of iterations.
    pub max_iterations: usize,
    /// Interval for recomputing the residual against the original operator.
    pub residual_recompute_interval: usize,
}

impl Default for PcgOptions {
    fn default() -> Self {
        Self {
            relative_tolerance: 1.0e-8,
            absolute_tolerance: 0.0,
            max_iterations: 1_000,
            residual_recompute_interval: 25,
        }
    }
}

impl PcgOptions {
    pub(crate) fn validate(self) -> Result<Self, MultiwayError> {
        if !self.relative_tolerance.is_finite() || self.relative_tolerance < 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "relative_tolerance",
                message: format!(
                    "must be finite and nonnegative, got {}",
                    self.relative_tolerance
                ),
            });
        }
        if !self.absolute_tolerance.is_finite() || self.absolute_tolerance < 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "absolute_tolerance",
                message: format!(
                    "must be finite and nonnegative, got {}",
                    self.absolute_tolerance
                ),
            });
        }
        if self.relative_tolerance == 0.0 && self.absolute_tolerance == 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "pcg_tolerances",
                message: "at least one tolerance must be positive".to_owned(),
            });
        }
        if self.max_iterations == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "max_iterations",
                message: "must be positive".to_owned(),
            });
        }
        if self.residual_recompute_interval == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "residual_recompute_interval",
                message: "must be positive".to_owned(),
            });
        }
        Ok(self)
    }
}

/// Why projected PCG stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcgStopReason {
    /// The projected right-hand side was exactly zero.
    ZeroRightHandSide,
    /// A recomputed residual met the requested tolerance.
    Converged,
    /// The iteration budget was exhausted.
    MaximumIterations,
}

/// Result and true projected-residual diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct PcgResult {
    solution: Vec<f64>,
    iterations: usize,
    converged: bool,
    residual_norm: f64,
    relative_residual: f64,
    rhs_projection_norm: f64,
    stop_reason: PcgStopReason,
}

impl PcgResult {
    /// Projected minimum-gauge candidate.
    #[must_use]
    pub fn solution(&self) -> &[f64] {
        &self.solution
    }

    /// Consume the result and return the solution.
    #[must_use]
    pub fn into_solution(self) -> Vec<f64> {
        self.solution
    }

    /// Number of completed PCG iterations.
    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }

    /// Whether an original-operator residual met the tolerance.
    #[must_use]
    pub const fn converged(&self) -> bool {
        self.converged
    }

    /// Final Euclidean residual norm after recomputation.
    #[must_use]
    pub const fn residual_norm(&self) -> f64 {
        self.residual_norm
    }

    /// Final residual norm divided by the projected RHS norm.
    #[must_use]
    pub const fn relative_residual(&self) -> f64 {
        self.relative_residual
    }

    /// Norm removed while projecting the submitted RHS out of known shift modes.
    #[must_use]
    pub const fn rhs_projection_norm(&self) -> f64 {
        self.rhs_projection_norm
    }

    /// Stop reason.
    #[must_use]
    pub const fn stop_reason(&self) -> PcgStopReason {
        self.stop_reason
    }
}

/// Solve the Gramian system after orthogonally removing known factor-shift modes.
///
/// Extra unidentified directions can still cause a breakdown. Production
/// callers should prefer rectangular LSMR and always certify in their original
/// observation-space operator.
pub fn solve_projected_pcg<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    rhs: &[f64],
    preconditioner: &P,
    options: PcgOptions,
) -> Result<PcgResult, MultiwayError> {
    let options = options.validate()?;
    let dimension = problem.dimension();
    if rhs.len() != dimension {
        return Err(crate::error::dimension(
            "solve_projected_pcg rhs",
            dimension,
            rhs.len(),
        ));
    }
    if preconditioner.dimension() != dimension {
        return Err(crate::error::dimension(
            "solve_projected_pcg preconditioner",
            dimension,
            preconditioner.dimension(),
        ));
    }

    crate::pcg_kernel::ensure_finite("PCG right-hand side", rhs)
        .map_err(crate::error::ordinary_pcg_error)?;
    let mut storage = crate::pcg_kernel::PcgStorage::try_new(dimension)?;
    let mut actions = OrdinaryPcgActions {
        problem,
        preconditioner,
        projection: problem.components().try_projection_workspace()?,
    };
    let diagnostics = crate::pcg_kernel::solve(&mut actions, rhs, options, &mut storage)
        .map_err(crate::error::ordinary_pcg_error)?;
    Ok(PcgResult {
        solution: storage.solution,
        iterations: diagnostics.iterations,
        converged: diagnostics.converged,
        residual_norm: diagnostics.residual_norm,
        relative_residual: diagnostics.relative_residual,
        rhs_projection_norm: diagnostics.rhs_projection_norm,
        stop_reason: diagnostics.stop_reason,
    })
}

struct OrdinaryPcgActions<'a, P: Preconditioner + ?Sized> {
    problem: &'a ThreeWayProblem,
    preconditioner: &'a P,
    projection: multiway_incidence::StructuralProjectionWorkspace,
}
impl<P: Preconditioner + ?Sized> crate::pcg_kernel::PcgActions for OrdinaryPcgActions<'_, P> {
    fn project(&mut self, values: &mut [f64]) -> Result<f64, MultiwayError> {
        Ok(self
            .problem
            .components()
            .project_structural_range_with_workspace(values, &mut self.projection)?)
    }
    fn precondition(&mut self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.preconditioner.apply(rhs, out)
    }
    fn gramian(&mut self, x: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.problem.apply_gramian(x, out)?;
        Ok(())
    }
    fn residual(&mut self, rhs: &[f64], x: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.problem.residual_into(rhs, x, out)?;
        Ok(())
    }
}
