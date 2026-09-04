//! Fail-closed recursive hierarchy construction from bootstrap aggregations.
//!
//! This module separates automatic coarse-space discovery from the fixed
//! numerical V-cycle. Every accepted level is screened by the single-level
//! bootstrap builder and by cumulative hierarchy-complexity gates. The plan
//! contains exact hard maps that can be submitted to a fixed scheduled
//! hierarchy only when construction reaches an admitted terminal.

use crate::{
    BootstrapAggregationOptions, BootstrapAggregationResult, BootstrapAggregationStopReason,
    DiagonalPreconditioner, FactorAggregation, MultiwayError, ThreeWayProblem,
    build_bootstrap_aggregation,
};

/// Controls recursive automatic hierarchy discovery.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootstrapHierarchyOptions {
    /// Maximum number of admitted nonterminal levels.
    pub maximum_levels: usize,
    /// Largest coefficient dimension accepted as a numerical terminal.
    pub terminal_dimension: usize,
    /// Minimum fractional coefficient-dimension reduction at every level.
    pub minimum_dimension_reduction: f64,
    /// Minimum fractional unique-tuple reduction at every level.
    pub minimum_tuple_reduction: f64,
    /// Maximum cumulative coefficient-dimension complexity.
    pub maximum_dimension_complexity: f64,
    /// Maximum cumulative unique-tuple complexity.
    pub maximum_tuple_complexity: f64,
    /// Conservative weighted-Jacobi damping used for map screening.
    pub screen_jacobi_omega: f64,
    /// Single-level bootstrap construction policy replayed at every level.
    pub aggregation: BootstrapAggregationOptions,
}

impl Default for BootstrapHierarchyOptions {
    fn default() -> Self {
        Self {
            maximum_levels: 12,
            terminal_dimension: 192,
            minimum_dimension_reduction: 0.05,
            minimum_tuple_reduction: 0.02,
            maximum_dimension_complexity: 2.5,
            maximum_tuple_complexity: 3.0,
            screen_jacobi_omega: 0.5,
            aggregation: BootstrapAggregationOptions::default(),
        }
    }
}

impl BootstrapHierarchyOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if self.maximum_levels == 0 {
            return Err(invalid("bootstrap_hierarchy_maximum_levels", "must be positive"));
        }
        if self.terminal_dimension == 0 {
            return Err(invalid("bootstrap_hierarchy_terminal_dimension", "must be positive"));
        }
        validate_fraction(
            "bootstrap_hierarchy_minimum_dimension_reduction",
            self.minimum_dimension_reduction,
        )?;
        validate_fraction(
            "bootstrap_hierarchy_minimum_tuple_reduction",
            self.minimum_tuple_reduction,
        )?;
        if !self.maximum_dimension_complexity.is_finite()
            || self.maximum_dimension_complexity < 1.0
        {
            return Err(invalid(
                "bootstrap_hierarchy_maximum_dimension_complexity",
                format!(
                    "must be finite and at least one, got {}",
                    self.maximum_dimension_complexity
                ),
            ));
        }
        if !self.maximum_tuple_complexity.is_finite() || self.maximum_tuple_complexity < 1.0 {
            return Err(invalid(
                "bootstrap_hierarchy_maximum_tuple_complexity",
                format!(
                    "must be finite and at least one, got {}",
                    self.maximum_tuple_complexity
                ),
            ));
        }
        if !self.screen_jacobi_omega.is_finite()
            || !(0.0..(2.0 / 3.0)).contains(&self.screen_jacobi_omega)
        {
            return Err(invalid(
                "bootstrap_hierarchy_screen_jacobi_omega",
                format!(
                    "must be finite and lie in (0, 2/3), got {}",
                    self.screen_jacobi_omega
                ),
            ));
        }
        Ok(self)
    }
}

/// Why recursive automatic hierarchy construction stopped.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum BootstrapHierarchyStopReason {
    /// The finest problem already satisfied the terminal-size gate.
    FinestAlreadyTerminal,
    /// An admitted sequence reached the terminal-size gate.
    ReachedTerminal,
    /// The configured level budget was exhausted before reaching a terminal.
    MaximumLevels {
        /// Maximum admitted nonterminal levels.
        maximum_levels: usize,
        /// Remaining coefficient dimension.
        remaining_dimension: usize,
    },
    /// The single-level bootstrap builder rejected a proposed level.
    AggregationRejected {
        /// Zero-based hierarchy level.
        level: usize,
        /// Deterministic single-level stop reason.
        reason: BootstrapAggregationStopReason,
    },
    /// Coefficient contraction was too small.
    InsufficientDimensionReduction {
        /// Zero-based hierarchy level.
        level: usize,
        /// Observed fractional reduction.
        observed: f64,
        /// Minimum admitted reduction.
        minimum: f64,
    },
    /// Unique-tuple contraction was too small.
    InsufficientTupleReduction {
        /// Zero-based hierarchy level.
        level: usize,
        /// Observed fractional reduction.
        observed: f64,
        /// Minimum admitted reduction.
        minimum: f64,
    },
    /// Adding a level would exceed cumulative dimension complexity.
    DimensionComplexityBudget {
        /// Zero-based hierarchy level.
        level: usize,
        /// Attempted cumulative complexity.
        attempted: f64,
        /// Maximum admitted complexity.
        maximum: f64,
    },
    /// Adding a level would exceed cumulative tuple complexity.
    TupleComplexityBudget {
        /// Zero-based hierarchy level.
        level: usize,
        /// Attempted cumulative complexity.
        attempted: f64,
        /// Maximum admitted complexity.
        maximum: f64,
    },
}

/// Diagnostics for one admitted automatic hierarchy level.
#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapHierarchyLevelReport {
    index: usize,
    fine_dimension: usize,
    fine_tuple_count: usize,
    coarse_dimension: usize,
    coarse_tuple_count: usize,
    dimension_reduction: f64,
    tuple_reduction: f64,
    cumulative_dimension_complexity: f64,
    cumulative_tuple_complexity: f64,
    aggregation: BootstrapAggregationResult,
}

impl BootstrapHierarchyLevelReport {
    /// Zero-based hierarchy level.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Fine coefficient dimension.
    #[must_use]
    pub const fn fine_dimension(&self) -> usize {
        self.fine_dimension
    }

    /// Fine unique-tuple count.
    #[must_use]
    pub const fn fine_tuple_count(&self) -> usize {
        self.fine_tuple_count
    }

    /// Accepted coarse coefficient dimension.
    #[must_use]
    pub const fn coarse_dimension(&self) -> usize {
        self.coarse_dimension
    }

    /// Accepted unique coarse-tuple count.
    #[must_use]
    pub const fn coarse_tuple_count(&self) -> usize {
        self.coarse_tuple_count
    }

    /// Fractional coefficient-dimension reduction.
    #[must_use]
    pub const fn dimension_reduction(&self) -> f64 {
        self.dimension_reduction
    }

    /// Fractional unique-tuple reduction.
    #[must_use]
    pub const fn tuple_reduction(&self) -> f64 {
        self.tuple_reduction
    }

    /// Cumulative coefficient-dimension complexity after this level.
    #[must_use]
    pub const fn cumulative_dimension_complexity(&self) -> f64 {
        self.cumulative_dimension_complexity
    }

    /// Cumulative unique-tuple complexity after this level.
    #[must_use]
    pub const fn cumulative_tuple_complexity(&self) -> f64 {
        self.cumulative_tuple_complexity
    }

    /// Complete single-level bootstrap result.
    #[must_use]
    pub const fn aggregation_result(&self) -> &BootstrapAggregationResult {
        &self.aggregation
    }
}

/// Exact hard maps and diagnostics discovered by recursive bootstrap setup.
#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapHierarchyPlan {
    problems: Vec<ThreeWayProblem>,
    aggregations: Vec<FactorAggregation>,
    levels: Vec<BootstrapHierarchyLevelReport>,
    completed: bool,
    stop_reason: BootstrapHierarchyStopReason,
    dimension_complexity: f64,
    tuple_complexity: f64,
}

impl BootstrapHierarchyPlan {
    /// Build a recursive hierarchy plan without constructing numerical V-cycle smoothers.
    pub fn build(
        finest: ThreeWayProblem,
        options: BootstrapHierarchyOptions,
    ) -> Result<Self, MultiwayError> {
        let options = options.validate()?;
        let finest_dimension = finest.dimension();
        let finest_tuples = finest.tuple_count();
        if finest_dimension <= options.terminal_dimension {
            return Ok(Self {
                problems: vec![finest],
                aggregations: Vec::new(),
                levels: Vec::new(),
                completed: true,
                stop_reason: BootstrapHierarchyStopReason::FinestAlreadyTerminal,
                dimension_complexity: 1.0,
                tuple_complexity: 1.0,
            });
        }

        let mut problems = vec![finest];
        let mut aggregations = Vec::with_capacity(options.maximum_levels);
        let mut levels = Vec::with_capacity(options.maximum_levels);
        let mut dimension_sum = finest_dimension;
        let mut tuple_sum = finest_tuples;

        for level in 0..options.maximum_levels {
            let current = problems
                .last()
                .expect("bootstrap hierarchy always retains its finest problem");
            let screen = DiagonalPreconditioner::new(current, options.screen_jacobi_omega)?;
            let aggregation_result =
                build_bootstrap_aggregation(current, &screen, options.aggregation)?;
            if !aggregation_result.accepted() {
                let reason = aggregation_result.stop_reason().clone();
                return Ok(Self::stopped(
                    problems,
                    aggregations,
                    levels,
                    BootstrapHierarchyStopReason::AggregationRejected { level, reason },
                    dimension_sum,
                    tuple_sum,
                    finest_dimension,
                    finest_tuples,
                ));
            }
            let aggregation = aggregation_result.final_aggregation().clone();
            let coarse = aggregation.coarsen(current)?;
            let dimension_reduction =
                1.0 - coarse.dimension() as f64 / current.dimension() as f64;
            let tuple_reduction =
                1.0 - coarse.tuple_count() as f64 / current.tuple_count() as f64;
            if dimension_reduction < options.minimum_dimension_reduction {
                return Ok(Self::stopped(
                    problems,
                    aggregations,
                    levels,
                    BootstrapHierarchyStopReason::InsufficientDimensionReduction {
                        level,
                        observed: dimension_reduction,
                        minimum: options.minimum_dimension_reduction,
                    },
                    dimension_sum,
                    tuple_sum,
                    finest_dimension,
                    finest_tuples,
                ));
            }
            if tuple_reduction < options.minimum_tuple_reduction {
                return Ok(Self::stopped(
                    problems,
                    aggregations,
                    levels,
                    BootstrapHierarchyStopReason::InsufficientTupleReduction {
                        level,
                        observed: tuple_reduction,
                        minimum: options.minimum_tuple_reduction,
                    },
                    dimension_sum,
                    tuple_sum,
                    finest_dimension,
                    finest_tuples,
                ));
            }

            let attempted_dimension_sum = dimension_sum.saturating_add(coarse.dimension());
            let attempted_tuple_sum = tuple_sum.saturating_add(coarse.tuple_count());
            let attempted_dimension_complexity =
                attempted_dimension_sum as f64 / finest_dimension as f64;
            let attempted_tuple_complexity = attempted_tuple_sum as f64 / finest_tuples as f64;
            if attempted_dimension_complexity > options.maximum_dimension_complexity {
                return Ok(Self::stopped(
                    problems,
                    aggregations,
                    levels,
                    BootstrapHierarchyStopReason::DimensionComplexityBudget {
                        level,
                        attempted: attempted_dimension_complexity,
                        maximum: options.maximum_dimension_complexity,
                    },
                    dimension_sum,
                    tuple_sum,
                    finest_dimension,
                    finest_tuples,
                ));
            }
            if attempted_tuple_complexity > options.maximum_tuple_complexity {
                return Ok(Self::stopped(
                    problems,
                    aggregations,
                    levels,
                    BootstrapHierarchyStopReason::TupleComplexityBudget {
                        level,
                        attempted: attempted_tuple_complexity,
                        maximum: options.maximum_tuple_complexity,
                    },
                    dimension_sum,
                    tuple_sum,
                    finest_dimension,
                    finest_tuples,
                ));
            }

            dimension_sum = attempted_dimension_sum;
            tuple_sum = attempted_tuple_sum;
            levels.push(BootstrapHierarchyLevelReport {
                index: level,
                fine_dimension: current.dimension(),
                fine_tuple_count: current.tuple_count(),
                coarse_dimension: coarse.dimension(),
                coarse_tuple_count: coarse.tuple_count(),
                dimension_reduction,
                tuple_reduction,
                cumulative_dimension_complexity: attempted_dimension_complexity,
                cumulative_tuple_complexity: attempted_tuple_complexity,
                aggregation: aggregation_result,
            });
            aggregations.push(aggregation);
            let reached_terminal = coarse.dimension() <= options.terminal_dimension;
            problems.push(coarse);
            if reached_terminal {
                return Ok(Self {
                    problems,
                    aggregations,
                    levels,
                    completed: true,
                    stop_reason: BootstrapHierarchyStopReason::ReachedTerminal,
                    dimension_complexity: attempted_dimension_complexity,
                    tuple_complexity: attempted_tuple_complexity,
                });
            }
        }

        let remaining_dimension = problems
            .last()
            .expect("bootstrap hierarchy retains its terminal candidate")
            .dimension();
        Ok(Self::stopped(
            problems,
            aggregations,
            levels,
            BootstrapHierarchyStopReason::MaximumLevels {
                maximum_levels: options.maximum_levels,
                remaining_dimension,
            },
            dimension_sum,
            tuple_sum,
            finest_dimension,
            finest_tuples,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn stopped(
        problems: Vec<ThreeWayProblem>,
        aggregations: Vec<FactorAggregation>,
        levels: Vec<BootstrapHierarchyLevelReport>,
        stop_reason: BootstrapHierarchyStopReason,
        dimension_sum: usize,
        tuple_sum: usize,
        finest_dimension: usize,
        finest_tuples: usize,
    ) -> Self {
        Self {
            problems,
            aggregations,
            levels,
            completed: false,
            stop_reason,
            dimension_complexity: dimension_sum as f64 / finest_dimension as f64,
            tuple_complexity: tuple_sum as f64 / finest_tuples as f64,
        }
    }

    /// Whether automatic setup reached an admitted terminal.
    #[must_use]
    pub const fn completed(&self) -> bool {
        self.completed
    }

    /// Deterministic completion or rejection reason.
    #[must_use]
    pub const fn stop_reason(&self) -> &BootstrapHierarchyStopReason {
        &self.stop_reason
    }

    /// Accepted hard maps in finest-to-coarsest order.
    #[must_use]
    pub fn aggregations(&self) -> &[FactorAggregation] {
        &self.aggregations
    }

    /// Fine, intermediate, and terminal weighted problems.
    #[must_use]
    pub fn problems(&self) -> &[ThreeWayProblem] {
        &self.problems
    }

    /// Per-level automatic setup diagnostics.
    #[must_use]
    pub fn level_reports(&self) -> &[BootstrapHierarchyLevelReport] {
        &self.levels
    }

    /// Finest weighted problem.
    #[must_use]
    pub fn finest_problem(&self) -> &ThreeWayProblem {
        &self.problems[0]
    }

    /// Last admitted weighted problem, whether or not it satisfies the terminal gate.
    #[must_use]
    pub fn terminal_candidate(&self) -> &ThreeWayProblem {
        self.problems
            .last()
            .expect("bootstrap hierarchy contains its finest problem")
    }

    /// Cumulative coefficient-dimension complexity.
    #[must_use]
    pub const fn dimension_complexity(&self) -> f64 {
        self.dimension_complexity
    }

    /// Cumulative unique-tuple complexity.
    #[must_use]
    pub const fn tuple_complexity(&self) -> f64 {
        self.tuple_complexity
    }
}

fn validate_fraction(name: &'static str, value: f64) -> Result<(), MultiwayError> {
    if !value.is_finite() || !(0.0..1.0).contains(&value) {
        return Err(invalid(
            name,
            format!("must be finite and lie in [0, 1), got {value}"),
        ));
    }
    Ok(())
}

fn invalid(name: &'static str, message: impl Into<String>) -> MultiwayError {
    MultiwayError::InvalidOption {
        name,
        message: message.into(),
    }
}
