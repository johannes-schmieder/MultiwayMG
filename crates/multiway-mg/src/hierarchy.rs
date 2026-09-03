//! Structure-preserving three-way hierarchy and symmetric V-cycle.

use crate::{
    AffinityAggregationOptions, DensePseudoinverse, DiagonalPreconditioner, FactorAggregation,
    MultiwayError, Preconditioner, ThreeWayProblem, build_affinity_aggregation,
};

/// Method used to construct one hard factor-respecting aggregation per level.
#[derive(Debug, Clone, PartialEq)]
pub enum AggregationStrategy {
    /// Shared-context affinity matching.
    Affinity(AffinityAggregationOptions),
    /// Merge consecutive level pairs. Mainly useful for manufactured oracle tests.
    Consecutive,
    /// Use caller-supplied maps in level order.
    Supplied(Vec<FactorAggregation>),
}

impl Default for AggregationStrategy {
    fn default() -> Self {
        Self::Affinity(AffinityAggregationOptions::default())
    }
}

/// Construction and cycle options for a three-way hierarchy.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchyOptions {
    /// Maximum number of nonterminal aggregation levels.
    pub max_levels: usize,
    /// Largest coefficient dimension admitted to the dense spectral terminal.
    pub terminal_dimension: usize,
    /// Minimum fractional coefficient-dimension reduction for an automatic level.
    pub minimum_dimension_reduction: f64,
    /// Minimum fractional unique-tuple reduction for an automatic level.
    pub minimum_tuple_reduction: f64,
    /// Relative eigenvalue threshold in the dense terminal.
    pub terminal_relative_tolerance: f64,
    /// Weighted-Jacobi damping factor.
    pub jacobi_omega: f64,
    /// Number of pre-smoothing sweeps.
    pub pre_sweeps: usize,
    /// Number of post-smoothing sweeps.
    pub post_sweeps: usize,
    /// Aggregation strategy.
    pub aggregation: AggregationStrategy,
}

impl Default for HierarchyOptions {
    fn default() -> Self {
        Self {
            max_levels: 12,
            terminal_dimension: 192,
            minimum_dimension_reduction: 0.05,
            minimum_tuple_reduction: 0.02,
            terminal_relative_tolerance: 1.0e-12,
            jacobi_omega: 0.5,
            pre_sweeps: 1,
            post_sweeps: 1,
            aggregation: AggregationStrategy::default(),
        }
    }
}

impl HierarchyOptions {
    fn validate(&self) -> Result<(), MultiwayError> {
        if self.max_levels == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "max_levels",
                message: "must be positive".to_owned(),
            });
        }
        if self.terminal_dimension == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "terminal_dimension",
                message: "must be positive".to_owned(),
            });
        }
        for (name, value) in [
            (
                "minimum_dimension_reduction",
                self.minimum_dimension_reduction,
            ),
            ("minimum_tuple_reduction", self.minimum_tuple_reduction),
        ] {
            if !value.is_finite() || !(0.0..1.0).contains(&value) {
                return Err(MultiwayError::InvalidOption {
                    name,
                    message: format!("must be finite and lie in [0, 1), got {value}"),
                });
            }
        }
        if !self.terminal_relative_tolerance.is_finite()
            || self.terminal_relative_tolerance <= 0.0
        {
            return Err(MultiwayError::InvalidOption {
                name: "terminal_relative_tolerance",
                message: format!(
                    "must be finite and positive, got {}",
                    self.terminal_relative_tolerance
                ),
            });
        }
        if !self.jacobi_omega.is_finite()
            || !(0.0..(2.0 / 3.0)).contains(&self.jacobi_omega)
        {
            return Err(MultiwayError::InvalidOption {
                name: "jacobi_omega",
                message: format!("must lie in (0, 2/3), got {}", self.jacobi_omega),
            });
        }
        Ok(())
    }
}

/// Observable dimensions and terminal diagnostics from hierarchy construction.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchyBuildReport {
    dimensions: Vec<usize>,
    tuple_counts: Vec<usize>,
    component_counts: Vec<usize>,
    aggregation_bytes: usize,
    terminal_rank: usize,
    terminal_threshold: f64,
}

impl HierarchyBuildReport {
    /// Coefficient dimension at every level, including the terminal.
    #[must_use]
    pub fn dimensions(&self) -> &[usize] {
        &self.dimensions
    }

    /// Unique tuple count at every level, including the terminal.
    #[must_use]
    pub fn tuple_counts(&self) -> &[usize] {
        &self.tuple_counts
    }

    /// Incidence component count at every level.
    #[must_use]
    pub fn component_counts(&self) -> &[usize] {
        &self.component_counts
    }

    /// Retained bytes in hard parent maps.
    #[must_use]
    pub const fn aggregation_bytes(&self) -> usize {
        self.aggregation_bytes
    }

    /// Numerical rank of the terminal Gramian.
    #[must_use]
    pub const fn terminal_rank(&self) -> usize {
        self.terminal_rank
    }

    /// Absolute terminal eigenvalue threshold.
    #[must_use]
    pub const fn terminal_threshold(&self) -> f64 {
        self.terminal_threshold
    }

    /// Sum of tuple counts divided by the finest tuple count.
    #[must_use]
    pub fn tuple_complexity(&self) -> f64 {
        let Some(&finest) = self.tuple_counts.first() else {
            return 0.0;
        };
        if finest == 0 {
            return 0.0;
        }
        self.tuple_counts.iter().sum::<usize>() as f64 / finest as f64
    }

    /// Sum of coefficient dimensions divided by the finest dimension.
    #[must_use]
    pub fn dimension_complexity(&self) -> f64 {
        let Some(&finest) = self.dimensions.first() else {
            return 0.0;
        };
        if finest == 0 {
            return 0.0;
        }
        self.dimensions.iter().sum::<usize>() as f64 / finest as f64
    }
}

/// Immutable structure-preserving hierarchy with a symmetric Jacobi V-cycle.
#[derive(Debug, Clone)]
pub struct ThreeWayHierarchy {
    problems: Vec<ThreeWayProblem>,
    aggregations: Vec<FactorAggregation>,
    smoothers: Vec<DiagonalPreconditioner>,
    terminal: DensePseudoinverse,
    options: HierarchyOptions,
    report: HierarchyBuildReport,
}

impl ThreeWayHierarchy {
    /// Build an automatic hierarchy or consume supplied oracle maps.
    pub fn build(
        finest: ThreeWayProblem,
        options: HierarchyOptions,
    ) -> Result<Self, MultiwayError> {
        options.validate()?;
        let mut problems = vec![finest];
        let mut aggregations = Vec::new();
        let mut smoothers = Vec::new();

        while problems
            .last()
            .is_some_and(|problem| problem.dimension() > options.terminal_dimension)
        {
            if aggregations.len() >= options.max_levels {
                break;
            }
            let level = aggregations.len();
            let current = problems.last().expect("hierarchy always has a finest level");
            let aggregation = match &options.aggregation {
                AggregationStrategy::Affinity(affinity) => {
                    build_affinity_aggregation(current, *affinity)?
                }
                AggregationStrategy::Consecutive => {
                    FactorAggregation::consecutive_halving(current.topology().level_counts())?
                }
                AggregationStrategy::Supplied(supplied) => supplied
                    .get(level)
                    .cloned()
                    .ok_or(MultiwayError::HierarchyStagnated {
                        dimension: current.dimension(),
                        tuples: current.tuple_count(),
                        limit: options.terminal_dimension,
                    })?,
            };
            if aggregation.fine_counts() != current.topology().level_counts() {
                return Err(MultiwayError::InvalidSuppliedAggregation { level });
            }
            let coarse = aggregation.coarsen(current)?;
            let dimension_reduction =
                1.0 - coarse.dimension() as f64 / current.dimension() as f64;
            let tuple_reduction =
                1.0 - coarse.tuple_count() as f64 / current.tuple_count() as f64;
            let made_progress = coarse.dimension() < current.dimension()
                && (dimension_reduction >= options.minimum_dimension_reduction
                    || tuple_reduction >= options.minimum_tuple_reduction);
            if !made_progress {
                break;
            }
            smoothers.push(DiagonalPreconditioner::new(
                current,
                options.jacobi_omega,
            )?);
            aggregations.push(aggregation);
            problems.push(coarse);
        }

        let terminal_problem = problems.last().expect("hierarchy has a terminal candidate");
        if terminal_problem.dimension() > options.terminal_dimension {
            return Err(MultiwayError::HierarchyStagnated {
                dimension: terminal_problem.dimension(),
                tuples: terminal_problem.tuple_count(),
                limit: options.terminal_dimension,
            });
        }
        let terminal = DensePseudoinverse::from_problem(
            terminal_problem,
            options.terminal_relative_tolerance,
        )?;
        let report = HierarchyBuildReport {
            dimensions: problems.iter().map(ThreeWayProblem::dimension).collect(),
            tuple_counts: problems.iter().map(ThreeWayProblem::tuple_count).collect(),
            component_counts: problems
                .iter()
                .map(|problem| problem.components().count())
                .collect(),
            aggregation_bytes: aggregations
                .iter()
                .map(FactorAggregation::retained_bytes)
                .sum(),
            terminal_rank: terminal.rank(),
            terminal_threshold: terminal.threshold(),
        };
        Ok(Self {
            problems,
            aggregations,
            smoothers,
            terminal,
            options,
            report,
        })
    }

    /// Finest problem represented by the hierarchy.
    #[must_use]
    pub const fn finest_problem(&self) -> &ThreeWayProblem {
        &self.problems[0]
    }

    /// Build diagnostics.
    #[must_use]
    pub const fn report(&self) -> &HierarchyBuildReport {
        &self.report
    }

    /// Number of aggregation levels.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.aggregations.len()
    }

    /// Apply only the first coarse correction, with recursive multilevel work
    /// below it but without finest-level Jacobi smoothing.
    pub fn apply_coarse_correction(
        &self,
        rhs: &[f64],
        out: &mut [f64],
    ) -> Result<(), MultiwayError> {
        let dimension = self.finest_problem().dimension();
        validate_dimensions(
            "ThreeWayHierarchy::apply_coarse_correction",
            dimension,
            rhs,
            out,
        )?;
        if self.aggregations.is_empty() {
            self.terminal.solve_into(rhs, out)?;
            self.finest_problem()
                .components()
                .project_structural_range(out)?;
            return Ok(());
        }

        let mut compatible_rhs = rhs.to_vec();
        self.finest_problem()
            .components()
            .project_structural_range(&mut compatible_rhs)?;
        let coarse_dimension = self.problems[1].dimension();
        let mut coarse_rhs = vec![0.0; coarse_dimension];
        self.aggregations[0].restrict(&compatible_rhs, &mut coarse_rhs)?;
        self.problems[1]
            .components()
            .project_structural_range(&mut coarse_rhs)?;
        let coarse_solution = self.apply_level(1, &coarse_rhs)?;
        self.aggregations[0].prolong(&coarse_solution, out)?;
        self.finest_problem()
            .components()
            .project_structural_range(out)?;
        Ok(())
    }

    fn apply_level(&self, level: usize, rhs: &[f64]) -> Result<Vec<f64>, MultiwayError> {
        let problem = &self.problems[level];
        if rhs.len() != problem.dimension() {
            return Err(crate::error::dimension(
                "ThreeWayHierarchy::apply_level rhs",
                problem.dimension(),
                rhs.len(),
            ));
        }
        if level + 1 == self.problems.len() {
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
        for _ in 0..self.options.pre_sweeps {
            smoothing_sweep(
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
        let mut prolonged = vec![0.0; problem.dimension()];
        self.aggregations[level].prolong(&coarse_solution, &mut prolonged)?;
        add_assign(&mut solution, &prolonged);

        for _ in 0..self.options.post_sweeps {
            smoothing_sweep(
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

impl Preconditioner for ThreeWayHierarchy {
    fn dimension(&self) -> usize {
        self.finest_problem().dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        let dimension = self.dimension();
        validate_dimensions("ThreeWayHierarchy::apply", dimension, rhs, out)?;
        let solution = self.apply_level(0, rhs)?;
        out.copy_from_slice(&solution);
        Ok(())
    }
}

fn smoothing_sweep(
    problem: &ThreeWayProblem,
    smoother: &DiagonalPreconditioner,
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
