//! Exact coarse corrections and symmetric research two-grid cycles.

use crate::{
    DensePseudoinverse, FactorAggregation, MultiwayError, Preconditioner, ThreeWayProblem,
    memory_estimate::estimate_three_way_problem_bytes,
};

/// Exact pseudoinverse correction in one hard factor-preserving coarse space.
#[derive(Debug, Clone)]
pub struct ExactCoarseCorrection {
    fine_problem: ThreeWayProblem,
    aggregation: FactorAggregation,
    coarse_problem: ThreeWayProblem,
    coarse_inverse: DensePseudoinverse,
}

impl ExactCoarseCorrection {
    /// Construct the exact Galerkin coarse correction.
    pub fn build(
        fine_problem: ThreeWayProblem,
        aggregation: FactorAggregation,
        relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        if aggregation.fine_counts() != fine_problem.topology().level_counts() {
            return Err(MultiwayError::InvalidAggregation {
                message: format!(
                    "aggregation fine counts {:?} do not match problem counts {:?}",
                    aggregation.fine_counts(),
                    fine_problem.topology().level_counts()
                ),
            });
        }
        let coarse_problem = aggregation.coarsen(&fine_problem)?;
        let coarse_inverse = DensePseudoinverse::from_problem(&coarse_problem, relative_tolerance)?;
        Ok(Self {
            fine_problem,
            aggregation,
            coarse_problem,
            coarse_inverse,
        })
    }

    /// Fine weighted problem.
    #[must_use]
    pub const fn fine_problem(&self) -> &ThreeWayProblem {
        &self.fine_problem
    }

    /// Exact hard aggregation.
    #[must_use]
    pub const fn aggregation(&self) -> &FactorAggregation {
        &self.aggregation
    }

    /// Coarse weighted problem.
    #[must_use]
    pub const fn coarse_problem(&self) -> &ThreeWayProblem {
        &self.coarse_problem
    }

    /// Numerical rank of the coarse terminal.
    #[must_use]
    pub const fn coarse_rank(&self) -> usize {
        self.coarse_inverse.rank()
    }

    /// Principal retained-memory estimate.
    #[must_use]
    pub fn retained_bytes_estimate(&self) -> usize {
        estimate_three_way_problem_bytes(&self.fine_problem)
            .saturating_add(estimate_three_way_problem_bytes(&self.coarse_problem))
            .saturating_add(self.aggregation.retained_bytes())
            .saturating_add(
                self.coarse_problem
                    .dimension()
                    .saturating_mul(self.coarse_problem.dimension().saturating_add(1))
                    .saturating_mul(8),
            )
    }
}

impl Preconditioner for ExactCoarseCorrection {
    fn dimension(&self) -> usize {
        self.fine_problem.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        validate_dimensions("ExactCoarseCorrection::apply", self.dimension(), rhs, out)?;
        let mut compatible_rhs = rhs.to_vec();
        self.fine_problem
            .components()
            .project_structural_range(&mut compatible_rhs)?;
        let mut coarse_rhs = vec![0.0; self.coarse_problem.dimension()];
        self.aggregation
            .restrict(&compatible_rhs, &mut coarse_rhs)?;
        self.coarse_problem
            .components()
            .project_structural_range(&mut coarse_rhs)?;
        let mut coarse_solution = vec![0.0; self.coarse_problem.dimension()];
        self.coarse_inverse
            .solve_into(&coarse_rhs, &mut coarse_solution)?;
        self.aggregation.prolong(&coarse_solution, out)?;
        self.fine_problem
            .components()
            .project_structural_range(out)?;
        Ok(())
    }
}

/// Symmetric multiplicative two-grid cycle with exact coarse correction.
#[derive(Debug, Clone)]
pub struct SymmetricTwoGridPreconditioner<S> {
    problem: ThreeWayProblem,
    smoother: S,
    coarse: ExactCoarseCorrection,
    sweeps: usize,
    damping: f64,
}

impl<S: Preconditioner> SymmetricTwoGridPreconditioner<S> {
    /// Build one fixed cycle with equal pre- and post-smoothing counts.
    pub fn build(
        problem: ThreeWayProblem,
        aggregation: FactorAggregation,
        smoother: S,
        sweeps: usize,
        damping: f64,
        terminal_relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        if smoother.dimension() != problem.dimension() {
            return Err(crate::error::dimension(
                "SymmetricTwoGridPreconditioner::build smoother",
                problem.dimension(),
                smoother.dimension(),
            ));
        }
        if sweeps == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "two_grid_sweeps",
                message: "must be positive".to_owned(),
            });
        }
        if !damping.is_finite() || damping <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "two_grid_damping",
                message: format!("must be finite and positive, got {damping}"),
            });
        }
        let coarse = ExactCoarseCorrection::build(
            problem.clone(),
            aggregation,
            terminal_relative_tolerance,
        )?;
        Ok(Self {
            problem,
            smoother,
            coarse,
            sweeps,
            damping,
        })
    }

    /// Smoother used before and after the coarse correction.
    #[must_use]
    pub const fn smoother(&self) -> &S {
        &self.smoother
    }

    /// Exact coarse correction.
    #[must_use]
    pub const fn coarse_correction(&self) -> &ExactCoarseCorrection {
        &self.coarse
    }

    /// Number of pre- and post-smoothing sweeps.
    #[must_use]
    pub const fn sweeps(&self) -> usize {
        self.sweeps
    }

    /// Fixed smoother damping.
    #[must_use]
    pub const fn damping(&self) -> f64 {
        self.damping
    }
}

impl<S: Preconditioner> Preconditioner for SymmetricTwoGridPreconditioner<S> {
    fn dimension(&self) -> usize {
        self.problem.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        validate_dimensions(
            "SymmetricTwoGridPreconditioner::apply",
            self.dimension(),
            rhs,
            out,
        )?;
        let mut compatible_rhs = rhs.to_vec();
        self.problem
            .components()
            .project_structural_range(&mut compatible_rhs)?;
        out.fill(0.0);
        for _ in 0..self.sweeps {
            smoothing_step(
                &self.problem,
                &self.smoother,
                self.damping,
                &compatible_rhs,
                out,
            )?;
        }
        let residual = self.problem.residual(&compatible_rhs, out)?;
        let mut coarse_correction = vec![0.0; self.dimension()];
        self.coarse.apply(&residual, &mut coarse_correction)?;
        add_assign(out, &coarse_correction);
        for _ in 0..self.sweeps {
            smoothing_step(
                &self.problem,
                &self.smoother,
                self.damping,
                &compatible_rhs,
                out,
            )?;
        }
        self.problem.components().project_structural_range(out)?;
        Ok(())
    }
}

fn smoothing_step<S: Preconditioner>(
    problem: &ThreeWayProblem,
    smoother: &S,
    damping: f64,
    rhs: &[f64],
    solution: &mut [f64],
) -> Result<(), MultiwayError> {
    let residual = problem.residual(rhs, solution)?;
    let mut correction = vec![0.0; problem.dimension()];
    smoother.apply(&residual, &mut correction)?;
    problem
        .components()
        .project_structural_range(&mut correction)?;
    for (value, &step) in solution.iter_mut().zip(&correction) {
        *value = damping.mul_add(step, *value);
    }
    Ok(())
}

fn add_assign(destination: &mut [f64], source: &[f64]) {
    for (left, &right) in destination.iter_mut().zip(source) {
        *left += right;
    }
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
