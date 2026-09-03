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
    fn validate(self) -> Result<Self, MultiwayError> {
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

    let mut projected_rhs = rhs.to_vec();
    let rhs_projection_norm = problem
        .components()
        .project_structural_range(&mut projected_rhs)?;
    ensure_finite("projected PCG right-hand side", &projected_rhs)?;
    let rhs_norm = norm(&projected_rhs);
    if rhs_norm == 0.0 {
        return Ok(PcgResult {
            solution: vec![0.0; dimension],
            iterations: 0,
            converged: true,
            residual_norm: 0.0,
            relative_residual: 0.0,
            rhs_projection_norm,
            stop_reason: PcgStopReason::ZeroRightHandSide,
        });
    }
    let tolerance = options
        .absolute_tolerance
        .max(options.relative_tolerance * rhs_norm);

    let mut solution = vec![0.0; dimension];
    let mut residual = projected_rhs.clone();
    let mut preconditioned = vec![0.0; dimension];
    preconditioner.apply(&residual, &mut preconditioned)?;
    problem
        .components()
        .project_structural_range(&mut preconditioned)?;
    ensure_finite("initial preconditioned residual", &preconditioned)?;
    let mut rho = dot(&residual, &preconditioned);
    if !rho.is_finite() || rho <= 0.0 {
        return Err(MultiwayError::PcgBreakdown {
            iteration: 0,
            message: format!("initial preconditioned metric is {rho}"),
        });
    }
    let mut direction = preconditioned.clone();
    let mut applied = vec![0.0; dimension];

    for iteration in 1..=options.max_iterations {
        problem.apply_gramian(&direction, &mut applied)?;
        let curvature = dot(&direction, &applied);
        if !curvature.is_finite() || curvature <= 0.0 {
            return Err(MultiwayError::PcgBreakdown {
                iteration: iteration - 1,
                message: format!("search-direction curvature is {curvature}"),
            });
        }
        let alpha = rho / curvature;
        if !alpha.is_finite() {
            return Err(MultiwayError::PcgBreakdown {
                iteration: iteration - 1,
                message: format!("step length is {alpha}"),
            });
        }
        axpy(alpha, &direction, &mut solution);
        axpy(-alpha, &applied, &mut residual);
        problem
            .components()
            .project_structural_range(&mut residual)?;

        if iteration % options.residual_recompute_interval == 0 || norm(&residual) <= tolerance {
            residual = problem.residual(&projected_rhs, &solution)?;
            problem
                .components()
                .project_structural_range(&mut residual)?;
            let residual_norm = norm(&residual);
            if residual_norm <= tolerance {
                problem
                    .components()
                    .project_structural_range(&mut solution)?;
                return Ok(PcgResult {
                    solution,
                    iterations: iteration,
                    converged: true,
                    residual_norm,
                    relative_residual: residual_norm / rhs_norm,
                    rhs_projection_norm,
                    stop_reason: PcgStopReason::Converged,
                });
            }
        }

        preconditioner.apply(&residual, &mut preconditioned)?;
        problem
            .components()
            .project_structural_range(&mut preconditioned)?;
        ensure_finite("preconditioned residual", &preconditioned)?;
        let new_rho = dot(&residual, &preconditioned);
        if !new_rho.is_finite() || new_rho <= 0.0 {
            return Err(MultiwayError::PcgBreakdown {
                iteration,
                message: format!("preconditioned metric is {new_rho}"),
            });
        }
        let beta = new_rho / rho;
        for (search, &z) in direction.iter_mut().zip(&preconditioned) {
            *search = beta.mul_add(*search, z);
        }
        problem
            .components()
            .project_structural_range(&mut direction)?;
        rho = new_rho;
    }

    residual = problem.residual(&projected_rhs, &solution)?;
    problem
        .components()
        .project_structural_range(&mut residual)?;
    problem
        .components()
        .project_structural_range(&mut solution)?;
    let residual_norm = norm(&residual);
    Ok(PcgResult {
        solution,
        iterations: options.max_iterations,
        converged: false,
        residual_norm,
        relative_residual: residual_norm / rhs_norm,
        rhs_projection_norm,
        stop_reason: PcgStopReason::MaximumIterations,
    })
}

fn axpy(alpha: f64, x: &[f64], y: &mut [f64]) {
    for (destination, &source) in y.iter_mut().zip(x) {
        *destination = alpha.mul_add(source, *destination);
    }
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (&a, &b) in left.iter().zip(right) {
        let value = a * b;
        let updated = sum + value;
        if sum.abs() >= value.abs() {
            correction += (sum - updated) + value;
        } else {
            correction += (value - updated) + sum;
        }
        sum = updated;
    }
    sum + correction
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

fn ensure_finite(context: &'static str, values: &[f64]) -> Result<(), MultiwayError> {
    if let Some((index, value)) = values
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(MultiwayError::PcgBreakdown {
            iteration: 0,
            message: format!("{context} entry {index} is non-finite: {value}"),
        });
    }
    Ok(())
}
