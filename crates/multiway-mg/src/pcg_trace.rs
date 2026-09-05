//! Projected PCG with a true residual sample after every iteration.

use crate::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, MultiwayError, Preconditioner,
    ThreeWayProblem,
};

/// Options for the issue #2 traced PCG driver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcgTraceOptions {
    /// Relative true-residual tolerance.
    pub relative_tolerance: f64,
    /// Absolute true-residual tolerance.
    pub absolute_tolerance: f64,
    /// Maximum iterations.
    pub max_iterations: usize,
}

impl Default for PcgTraceOptions {
    fn default() -> Self {
        Self {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 0.0,
            max_iterations: 2_000,
        }
    }
}

/// One original-Gramian residual sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcgTraceSample {
    iteration: usize,
    residual_norm: f64,
    relative_residual: f64,
}

impl PcgTraceSample {
    /// Completed iterations at this sample.
    #[must_use]
    pub const fn iteration(self) -> usize {
        self.iteration
    }

    /// Euclidean norm of the recomputed projected residual.
    #[must_use]
    pub const fn residual_norm(self) -> f64 {
        self.residual_norm
    }

    /// Residual norm divided by the projected RHS norm.
    #[must_use]
    pub const fn relative_residual(self) -> f64 {
        self.relative_residual
    }
}

/// Traced PCG result.
#[derive(Debug, Clone, PartialEq)]
pub struct PcgTraceResult {
    solution: Vec<f64>,
    iterations: usize,
    converged: bool,
    rhs_projection_norm: f64,
    gramian_applications: usize,
    preconditioner_applications: usize,
    samples: Vec<PcgTraceSample>,
}

impl PcgTraceResult {
    /// Final normalized solution candidate.
    #[must_use]
    pub fn solution(&self) -> &[f64] {
        &self.solution
    }

    /// Completed iterations.
    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }

    /// Whether a recomputed residual met tolerance.
    #[must_use]
    pub const fn converged(&self) -> bool {
        self.converged
    }

    /// Euclidean norm removed by the structural-range projection of the RHS.
    #[must_use]
    pub const fn rhs_projection_norm(&self) -> f64 {
        self.rhs_projection_norm
    }

    /// Number of original-Gramian applications, including residual audits.
    #[must_use]
    pub const fn gramian_applications(&self) -> usize {
        self.gramian_applications
    }

    /// Number of preconditioner applications.
    #[must_use]
    pub const fn preconditioner_applications(&self) -> usize {
        self.preconditioner_applications
    }

    /// Initial and per-iteration true-residual samples.
    #[must_use]
    pub fn samples(&self) -> &[PcgTraceSample] {
        &self.samples
    }

    /// Final relative true residual.
    #[must_use]
    pub fn final_relative_residual(&self) -> f64 {
        self.samples
            .last()
            .map_or(0.0, |sample| sample.relative_residual)
    }
}

/// Solve with residual replacement and recording after every iteration.
pub fn solve_projected_pcg_traced<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    rhs: &[f64],
    preconditioner: &P,
    options: PcgTraceOptions,
) -> Result<PcgTraceResult, MultiwayError> {
    solve_projected_pcg_traced_with_apply(problem, rhs, preconditioner, options, |rhs, out| {
        preconditioner.apply(rhs, out)
    })
}

/// Solve traced PCG with one caller-owned hierarchy workspace for the full solve.
///
/// This uses the same private recurrence, validation order, true-residual
/// samples, and work accounting as [`solve_projected_pcg_traced`]. Only the
/// preconditioner application closure differs. The workspace can also be reused
/// across subsequent solves and independently constructed hierarchies.
///
/// Invalid options or dimensions are rejected before the workspace is touched.
/// A zero projected RHS needs no preconditioner application and does not prepare
/// an empty workspace. Otherwise a changed hierarchy layout can grow scratch on
/// its first application. Outer PCG vectors, trace storage, and the remaining
/// MAP/projection internals still allocate; this is not a full solver workspace.
pub fn solve_projected_pcg_traced_with_hierarchy_workspace(
    problem: &ThreeWayProblem,
    rhs: &[f64],
    hierarchy: &CycleScreenedMapHierarchy,
    options: PcgTraceOptions,
    workspace: &mut CycleScreenedMapHierarchyWorkspace,
) -> Result<PcgTraceResult, MultiwayError> {
    solve_projected_pcg_traced_with_apply(problem, rhs, hierarchy, options, |rhs, out| {
        hierarchy.apply_with_workspace(rhs, out, workspace)
    })
}

// Keep the preconditioner reference for the original dimension-validation path;
// parameterize only its application. No recurrence or interior-mutable adapter
// is duplicated for the workspace entry point.
fn solve_projected_pcg_traced_with_apply<P, F>(
    problem: &ThreeWayProblem,
    rhs: &[f64],
    preconditioner: &P,
    options: PcgTraceOptions,
    mut apply_preconditioner: F,
) -> Result<PcgTraceResult, MultiwayError>
where
    P: Preconditioner + ?Sized,
    F: FnMut(&[f64], &mut [f64]) -> Result<(), MultiwayError>,
{
    validate_options(options)?;
    let dimension = problem.dimension();
    if rhs.len() != dimension {
        return Err(crate::error::dimension(
            "solve_projected_pcg_traced rhs",
            dimension,
            rhs.len(),
        ));
    }
    if preconditioner.dimension() != dimension {
        return Err(crate::error::dimension(
            "solve_projected_pcg_traced preconditioner",
            dimension,
            preconditioner.dimension(),
        ));
    }
    let mut projected_rhs = rhs.to_vec();
    let rhs_projection_norm = problem
        .components()
        .project_structural_range(&mut projected_rhs)?;
    let rhs_norm = norm(&projected_rhs);
    let mut samples = vec![PcgTraceSample {
        iteration: 0,
        residual_norm: rhs_norm,
        relative_residual: if rhs_norm == 0.0 { 0.0 } else { 1.0 },
    }];
    if rhs_norm == 0.0 {
        return Ok(PcgTraceResult {
            solution: vec![0.0; dimension],
            iterations: 0,
            converged: true,
            rhs_projection_norm,
            gramian_applications: 0,
            preconditioner_applications: 0,
            samples,
        });
    }
    let tolerance = options
        .absolute_tolerance
        .max(options.relative_tolerance * rhs_norm);
    let mut solution = vec![0.0; dimension];
    let mut residual = projected_rhs.clone();
    let mut preconditioned = vec![0.0; dimension];
    apply_preconditioner(&residual, &mut preconditioned)?;
    problem
        .components()
        .project_structural_range(&mut preconditioned)?;
    let mut preconditioner_applications = 1;
    let mut rho = dot(&residual, &preconditioned);
    validate_positive_metric(0, "initial preconditioned metric", rho)?;
    let mut direction = preconditioned.clone();
    let mut applied = vec![0.0; dimension];
    let mut gramian_applications = 0;

    for iteration in 1..=options.max_iterations {
        problem.apply_gramian(&direction, &mut applied)?;
        gramian_applications += 1;
        let curvature = dot(&direction, &applied);
        validate_positive_metric(iteration - 1, "search-direction curvature", curvature)?;
        let alpha = rho / curvature;
        if !alpha.is_finite() {
            return Err(MultiwayError::PcgBreakdown {
                iteration: iteration - 1,
                message: format!("step length is {alpha}"),
            });
        }
        axpy(alpha, &direction, &mut solution);
        residual = problem.residual(&projected_rhs, &solution)?;
        gramian_applications += 1;
        problem
            .components()
            .project_structural_range(&mut residual)?;
        let residual_norm = norm(&residual);
        samples.push(PcgTraceSample {
            iteration,
            residual_norm,
            relative_residual: residual_norm / rhs_norm,
        });
        if residual_norm <= tolerance {
            problem
                .components()
                .project_structural_range(&mut solution)?;
            return Ok(PcgTraceResult {
                solution,
                iterations: iteration,
                converged: true,
                rhs_projection_norm,
                gramian_applications,
                preconditioner_applications,
                samples,
            });
        }
        apply_preconditioner(&residual, &mut preconditioned)?;
        preconditioner_applications += 1;
        problem
            .components()
            .project_structural_range(&mut preconditioned)?;
        let new_rho = dot(&residual, &preconditioned);
        validate_positive_metric(iteration, "preconditioned metric", new_rho)?;
        let beta = new_rho / rho;
        for (search, &value) in direction.iter_mut().zip(&preconditioned) {
            *search = beta.mul_add(*search, value);
        }
        problem
            .components()
            .project_structural_range(&mut direction)?;
        rho = new_rho;
    }
    problem
        .components()
        .project_structural_range(&mut solution)?;
    Ok(PcgTraceResult {
        solution,
        iterations: options.max_iterations,
        converged: false,
        rhs_projection_norm,
        gramian_applications,
        preconditioner_applications,
        samples,
    })
}

fn validate_options(options: PcgTraceOptions) -> Result<(), MultiwayError> {
    if !options.relative_tolerance.is_finite() || options.relative_tolerance < 0.0 {
        return Err(MultiwayError::InvalidOption {
            name: "trace_relative_tolerance",
            message: format!(
                "must be finite and nonnegative, got {}",
                options.relative_tolerance
            ),
        });
    }
    if !options.absolute_tolerance.is_finite() || options.absolute_tolerance < 0.0 {
        return Err(MultiwayError::InvalidOption {
            name: "trace_absolute_tolerance",
            message: format!(
                "must be finite and nonnegative, got {}",
                options.absolute_tolerance
            ),
        });
    }
    if options.relative_tolerance == 0.0 && options.absolute_tolerance == 0.0 {
        return Err(MultiwayError::InvalidOption {
            name: "trace_tolerances",
            message: "at least one tolerance must be positive".to_owned(),
        });
    }
    if options.max_iterations == 0 {
        return Err(MultiwayError::InvalidOption {
            name: "trace_max_iterations",
            message: "must be positive".to_owned(),
        });
    }
    Ok(())
}

fn validate_positive_metric(
    iteration: usize,
    name: &'static str,
    value: f64,
) -> Result<(), MultiwayError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(MultiwayError::PcgBreakdown {
            iteration,
            message: format!("{name} is {value}"),
        });
    }
    Ok(())
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
