//! Recursive automatic hierarchy construction with complete-cycle screening.
//!
//! Each level is proposed by the conservative bootstrap/repair algorithm and
//! accepted only after the actual symmetric-MAP two-grid cycle passes a
//! deterministic matrix-free quality probe. Cumulative dimension and tuple
//! complexity are enforced before a map is admitted. The resulting research
//! V-cycle uses symmetric MAP on every nonterminal level and a rank-revealing
//! dense pseudoinverse terminal.

use crate::{
    BootstrapAggregationOptions, CyclePortfolioCandidateSource, CycleQualityCriteria,
    CycleQualityOptions, CycleScreenedBootstrapResult, DensePseudoinverse, FactorAggregation,
    MultiwayError, SymmetricMapPreconditioner, SymmetricTwoGridPreconditioner, ThreeWayProblem,
    build_cycle_screened_bootstrap_aggregation,
};

mod payload;
mod workspace;
pub use payload::MapHierarchyPayloadReport;

pub use workspace::CycleScreenedMapHierarchyWorkspace;

/// Options for recursive complete-cycle-screened hierarchy construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CycleScreenedHierarchyOptions {
    /// Maximum number of accepted aggregation levels.
    pub maximum_levels: usize,
    /// Largest coefficient dimension admitted to the dense terminal.
    pub terminal_dimension: usize,
    /// Largest cumulative coefficient-dimension complexity.
    pub maximum_dimension_complexity: f64,
    /// Largest cumulative unique-tuple complexity.
    pub maximum_tuple_complexity: f64,
    /// Conservative bootstrap and structural admission options at every level.
    pub bootstrap: BootstrapAggregationOptions,
    /// Matrix-free complete-cycle probe options at every level.
    pub cycle_probe: CycleQualityOptions,
    /// Complete-cycle acceptance criteria at every level.
    pub cycle_criteria: CycleQualityCriteria,
    /// Relative eigenvalue threshold for exact coarse probes and the terminal.
    pub terminal_relative_tolerance: f64,
}

impl CycleScreenedHierarchyOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if self.maximum_levels == 0 {
            return Err(invalid(
                "cycle_hierarchy_maximum_levels",
                "must be positive",
            ));
        }
        if self.terminal_dimension == 0 {
            return Err(invalid(
                "cycle_hierarchy_terminal_dimension",
                "must be positive",
            ));
        }
        if !self.maximum_dimension_complexity.is_finite() || self.maximum_dimension_complexity < 1.0
        {
            return Err(invalid(
                "cycle_hierarchy_maximum_dimension_complexity",
                format!(
                    "must be finite and at least one, got {}",
                    self.maximum_dimension_complexity
                ),
            ));
        }
        if !self.maximum_tuple_complexity.is_finite() || self.maximum_tuple_complexity < 1.0 {
            return Err(invalid(
                "cycle_hierarchy_maximum_tuple_complexity",
                format!(
                    "must be finite and at least one, got {}",
                    self.maximum_tuple_complexity
                ),
            ));
        }
        if !self.terminal_relative_tolerance.is_finite() || self.terminal_relative_tolerance <= 0.0
        {
            return Err(invalid(
                "cycle_hierarchy_terminal_relative_tolerance",
                format!(
                    "must be finite and positive, got {}",
                    self.terminal_relative_tolerance
                ),
            ));
        }
        Ok(self)
    }
}

/// Why recursive automatic hierarchy construction stopped.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CycleScreenedHierarchyStopReason {
    /// The current problem fits the declared dense terminal.
    ReachedTerminal,
    /// The accepted-level budget was exhausted before reaching the terminal.
    MaximumLevels {
        /// Configured accepted-level budget.
        maximum_levels: usize,
        /// Remaining coefficient dimension.
        remaining_dimension: usize,
    },
    /// No candidate at this level passed structural and complete-cycle gates.
    LevelRejected {
        /// Zero-based attempted hierarchy level.
        level: usize,
    },
    /// An accepted candidate did not strictly reduce coefficient dimension.
    NoDimensionProgress {
        /// Zero-based attempted hierarchy level.
        level: usize,
        /// Fine coefficient dimension.
        fine_dimension: usize,
        /// Candidate coarse coefficient dimension.
        coarse_dimension: usize,
    },
    /// Admitting the candidate would exceed cumulative dimension complexity.
    DimensionComplexityBudget {
        /// Zero-based attempted hierarchy level.
        level: usize,
        /// Attempted cumulative complexity.
        attempted: f64,
        /// Maximum admitted complexity.
        maximum: f64,
    },
    /// Admitting the candidate would exceed cumulative tuple complexity.
    TupleComplexityBudget {
        /// Zero-based attempted hierarchy level.
        level: usize,
        /// Attempted cumulative complexity.
        attempted: f64,
        /// Maximum admitted complexity.
        maximum: f64,
    },
}

/// Diagnostics for one attempted recursive level.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleScreenedHierarchyLevelReport {
    level: usize,
    fine_dimension: usize,
    fine_tuple_count: usize,
    coarse_dimension: usize,
    coarse_tuple_count: usize,
    selected_source: Option<CyclePortfolioCandidateSource>,
    selected_cycle_factor: Option<f64>,
    cumulative_dimension_complexity: f64,
    cumulative_tuple_complexity: f64,
    admitted: bool,
    portfolio: CycleScreenedBootstrapResult,
}

impl CycleScreenedHierarchyLevelReport {
    /// Zero-based attempted level.
    #[must_use]
    pub const fn level(&self) -> usize {
        self.level
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

    /// Candidate coarse coefficient dimension.
    #[must_use]
    pub const fn coarse_dimension(&self) -> usize {
        self.coarse_dimension
    }

    /// Candidate coarse unique-tuple count.
    #[must_use]
    pub const fn coarse_tuple_count(&self) -> usize {
        self.coarse_tuple_count
    }

    /// Source selected by complete-cycle screening.
    #[must_use]
    pub const fn selected_source(&self) -> Option<CyclePortfolioCandidateSource> {
        self.selected_source
    }

    /// Worst estimated complete-cycle energy factor for the selected map.
    #[must_use]
    pub const fn selected_cycle_factor(&self) -> Option<f64> {
        self.selected_cycle_factor
    }

    /// Cumulative coefficient-dimension complexity if the candidate is admitted.
    #[must_use]
    pub const fn cumulative_dimension_complexity(&self) -> f64 {
        self.cumulative_dimension_complexity
    }

    /// Cumulative unique-tuple complexity if the candidate is admitted.
    #[must_use]
    pub const fn cumulative_tuple_complexity(&self) -> f64 {
        self.cumulative_tuple_complexity
    }

    /// Whether the candidate entered the recursive hierarchy.
    #[must_use]
    pub const fn admitted(&self) -> bool {
        self.admitted
    }

    /// Full bootstrap and candidate-screening result.
    #[must_use]
    pub const fn portfolio(&self) -> &CycleScreenedBootstrapResult {
        &self.portfolio
    }
}

/// Immutable recursive map plan and its complete audit trail.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleScreenedHierarchyPlan {
    problems: Vec<ThreeWayProblem>,
    aggregations: Vec<FactorAggregation>,
    levels: Vec<CycleScreenedHierarchyLevelReport>,
    accepted: bool,
    stop_reason: CycleScreenedHierarchyStopReason,
    dimension_complexity: f64,
    tuple_complexity: f64,
    terminal_relative_tolerance: f64,
}

impl CycleScreenedHierarchyPlan {
    /// Build a recursively screened hierarchy from one fixed weighted problem.
    pub fn build(
        finest: ThreeWayProblem,
        options: CycleScreenedHierarchyOptions,
    ) -> Result<Self, MultiwayError> {
        let options = options.validate()?;
        let finest_dimension = finest.dimension();
        let finest_tuples = finest.tuple_count();
        let mut problems = vec![finest];
        let mut aggregations = Vec::new();
        let mut levels = Vec::new();
        let mut cumulative_dimensions = finest_dimension;
        let mut cumulative_tuples = finest_tuples;

        let stop_reason = loop {
            let current = problems
                .last()
                .expect("a hierarchy plan always retains its finest problem");
            if current.dimension() <= options.terminal_dimension {
                break CycleScreenedHierarchyStopReason::ReachedTerminal;
            }
            let level = aggregations.len();
            if level >= options.maximum_levels {
                break CycleScreenedHierarchyStopReason::MaximumLevels {
                    maximum_levels: options.maximum_levels,
                    remaining_dimension: current.dimension(),
                };
            }

            let primary = crate::DiagonalPreconditioner::new(current, 0.5)?;
            let problem_for_cycle = current.clone();
            let portfolio = build_cycle_screened_bootstrap_aggregation(
                current,
                &primary,
                options.bootstrap,
                options.cycle_probe,
                options.cycle_criteria,
                |aggregation| {
                    SymmetricTwoGridPreconditioner::build(
                        problem_for_cycle.clone(),
                        aggregation.clone(),
                        SymmetricMapPreconditioner::new(problem_for_cycle.clone()),
                        1,
                        1.0,
                        options.terminal_relative_tolerance,
                    )
                },
            )?;
            let candidate = portfolio.final_aggregation().clone();
            let candidate_coarse = candidate.coarsen(current)?;
            let coarse_dimension = candidate_coarse.dimension();
            let coarse_tuples = candidate_coarse.tuple_count();
            let attempted_dimensions = cumulative_dimensions.saturating_add(coarse_dimension);
            let attempted_tuples = cumulative_tuples.saturating_add(coarse_tuples);
            let dimension_complexity = attempted_dimensions as f64 / finest_dimension as f64;
            let tuple_complexity = attempted_tuples as f64 / finest_tuples as f64;
            let selected_cycle_factor = portfolio
                .selected_evaluation()
                .and_then(|evaluation| evaluation.cycle_report())
                .map(crate::CycleQualityReport::maximum_estimated_energy_factor);

            if !portfolio.accepted() {
                levels.push(CycleScreenedHierarchyLevelReport {
                    level,
                    fine_dimension: current.dimension(),
                    fine_tuple_count: current.tuple_count(),
                    coarse_dimension,
                    coarse_tuple_count: coarse_tuples,
                    selected_source: portfolio.selected_source(),
                    selected_cycle_factor,
                    cumulative_dimension_complexity: dimension_complexity,
                    cumulative_tuple_complexity: tuple_complexity,
                    admitted: false,
                    portfolio,
                });
                break CycleScreenedHierarchyStopReason::LevelRejected { level };
            }
            if coarse_dimension >= current.dimension() {
                levels.push(CycleScreenedHierarchyLevelReport {
                    level,
                    fine_dimension: current.dimension(),
                    fine_tuple_count: current.tuple_count(),
                    coarse_dimension,
                    coarse_tuple_count: coarse_tuples,
                    selected_source: portfolio.selected_source(),
                    selected_cycle_factor,
                    cumulative_dimension_complexity: dimension_complexity,
                    cumulative_tuple_complexity: tuple_complexity,
                    admitted: false,
                    portfolio,
                });
                break CycleScreenedHierarchyStopReason::NoDimensionProgress {
                    level,
                    fine_dimension: current.dimension(),
                    coarse_dimension,
                };
            }
            if dimension_complexity > options.maximum_dimension_complexity {
                levels.push(CycleScreenedHierarchyLevelReport {
                    level,
                    fine_dimension: current.dimension(),
                    fine_tuple_count: current.tuple_count(),
                    coarse_dimension,
                    coarse_tuple_count: coarse_tuples,
                    selected_source: portfolio.selected_source(),
                    selected_cycle_factor,
                    cumulative_dimension_complexity: dimension_complexity,
                    cumulative_tuple_complexity: tuple_complexity,
                    admitted: false,
                    portfolio,
                });
                break CycleScreenedHierarchyStopReason::DimensionComplexityBudget {
                    level,
                    attempted: dimension_complexity,
                    maximum: options.maximum_dimension_complexity,
                };
            }
            if tuple_complexity > options.maximum_tuple_complexity {
                levels.push(CycleScreenedHierarchyLevelReport {
                    level,
                    fine_dimension: current.dimension(),
                    fine_tuple_count: current.tuple_count(),
                    coarse_dimension,
                    coarse_tuple_count: coarse_tuples,
                    selected_source: portfolio.selected_source(),
                    selected_cycle_factor,
                    cumulative_dimension_complexity: dimension_complexity,
                    cumulative_tuple_complexity: tuple_complexity,
                    admitted: false,
                    portfolio,
                });
                break CycleScreenedHierarchyStopReason::TupleComplexityBudget {
                    level,
                    attempted: tuple_complexity,
                    maximum: options.maximum_tuple_complexity,
                };
            }

            levels.push(CycleScreenedHierarchyLevelReport {
                level,
                fine_dimension: current.dimension(),
                fine_tuple_count: current.tuple_count(),
                coarse_dimension,
                coarse_tuple_count: coarse_tuples,
                selected_source: portfolio.selected_source(),
                selected_cycle_factor,
                cumulative_dimension_complexity: dimension_complexity,
                cumulative_tuple_complexity: tuple_complexity,
                admitted: true,
                portfolio,
            });
            cumulative_dimensions = attempted_dimensions;
            cumulative_tuples = attempted_tuples;
            aggregations.push(candidate);
            problems.push(candidate_coarse);
        };

        let accepted = matches!(
            stop_reason,
            CycleScreenedHierarchyStopReason::ReachedTerminal
        );
        Ok(Self {
            problems,
            aggregations,
            levels,
            accepted,
            stop_reason,
            dimension_complexity: cumulative_dimensions as f64 / finest_dimension as f64,
            tuple_complexity: cumulative_tuples as f64 / finest_tuples as f64,
            terminal_relative_tolerance: options.terminal_relative_tolerance,
        })
    }

    /// Whether construction reached the declared terminal through admitted maps.
    #[must_use]
    pub const fn accepted(&self) -> bool {
        self.accepted
    }

    /// Deterministic construction stop reason.
    #[must_use]
    pub const fn stop_reason(&self) -> &CycleScreenedHierarchyStopReason {
        &self.stop_reason
    }

    /// Accepted hard maps in finest-to-coarsest order.
    #[must_use]
    pub fn aggregations(&self) -> &[FactorAggregation] {
        &self.aggregations
    }

    /// Every attempted level, including a final rejected candidate.
    #[must_use]
    pub fn level_reports(&self) -> &[CycleScreenedHierarchyLevelReport] {
        &self.levels
    }

    /// Number of accepted nonterminal levels.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.aggregations.len()
    }

    /// Cumulative coefficient-dimension complexity of admitted levels.
    #[must_use]
    pub const fn dimension_complexity(&self) -> f64 {
        self.dimension_complexity
    }

    /// Cumulative unique-tuple complexity of admitted levels.
    #[must_use]
    pub const fn tuple_complexity(&self) -> f64 {
        self.tuple_complexity
    }

    /// Finest problem.
    #[must_use]
    pub fn finest_problem(&self) -> &ThreeWayProblem {
        &self.problems[0]
    }

    /// Current terminal candidate, even when construction stopped early.
    #[must_use]
    pub fn terminal_problem(&self) -> &ThreeWayProblem {
        self.problems
            .last()
            .expect("a hierarchy plan always has a finest problem")
    }

    /// Build the fixed symmetric-MAP V-cycle after successful planning.
    pub fn build_preconditioner(&self) -> Result<CycleScreenedMapHierarchy, MultiwayError> {
        if !self.accepted {
            return Err(MultiwayError::CycleQuality {
                message: format!(
                    "cannot build a hierarchy from rejected plan: {:?}",
                    self.stop_reason
                ),
            });
        }
        CycleScreenedMapHierarchy::from_plan(self)
    }
}

/// Fixed symmetric-MAP recursive V-cycle built from an accepted plan.
#[derive(Debug, Clone)]
pub struct CycleScreenedMapHierarchy {
    problems: Vec<ThreeWayProblem>,
    aggregations: Vec<FactorAggregation>,
    smoothers: Vec<SymmetricMapPreconditioner>,
    terminal: DensePseudoinverse,
}

impl CycleScreenedMapHierarchy {
    /// Build a fixed symmetric-MAP hierarchy from caller-supplied hard maps.
    ///
    /// Every map is validated against the current level, must preserve exact
    /// incidence components, and must strictly reduce coefficient dimension.
    /// This constructor is useful for oracle references and independently
    /// generated map sequences; automatic production-shaped construction should
    /// normally use [`CycleScreenedHierarchyPlan`].
    pub fn from_maps(
        finest: ThreeWayProblem,
        aggregations: Vec<FactorAggregation>,
        terminal_relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        if !terminal_relative_tolerance.is_finite() || terminal_relative_tolerance <= 0.0 {
            return Err(invalid(
                "cycle_hierarchy_terminal_relative_tolerance",
                format!("must be finite and positive, got {terminal_relative_tolerance}"),
            ));
        }

        let mut problems = Vec::with_capacity(aggregations.len() + 1);
        problems.push(finest);
        for (level, aggregation) in aggregations.iter().enumerate() {
            let current = problems
                .last()
                .expect("a supplied hierarchy always retains its finest problem");
            if aggregation.fine_counts() != current.topology().level_counts() {
                return Err(MultiwayError::InvalidSuppliedAggregation { level });
            }
            crate::DiagonalAggregationProjector::new(current.clone(), aggregation.clone())?;
            let coarse = aggregation.coarsen(current)?;
            if coarse.dimension() >= current.dimension() {
                return Err(MultiwayError::InvalidAggregation {
                    message: format!(
                        "supplied hierarchy level {level} does not strictly reduce dimension: fine {}, coarse {}",
                        current.dimension(),
                        coarse.dimension(),
                    ),
                });
            }
            problems.push(coarse);
        }

        let smoothers = problems[..aggregations.len()]
            .iter()
            .cloned()
            .map(SymmetricMapPreconditioner::new)
            .collect();
        let terminal_problem = problems
            .last()
            .expect("a supplied hierarchy always has a terminal problem");
        let terminal =
            DensePseudoinverse::from_problem(terminal_problem, terminal_relative_tolerance)?;
        Ok(Self {
            problems,
            aggregations,
            smoothers,
            terminal,
        })
    }

    fn from_plan(plan: &CycleScreenedHierarchyPlan) -> Result<Self, MultiwayError> {
        Self::from_maps(
            plan.finest_problem().clone(),
            plan.aggregations.clone(),
            plan.terminal_relative_tolerance,
        )
    }

    /// Number of accepted aggregation levels.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.aggregations.len()
    }

    /// Finest weighted problem.
    #[must_use]
    pub fn finest_problem(&self) -> &ThreeWayProblem {
        &self.problems[0]
    }
}

fn add_assign(destination: &mut [f64], source: &[f64]) {
    for (left, &right) in destination.iter_mut().zip(source) {
        *left += right;
    }
}

fn invalid(name: &'static str, message: impl Into<String>) -> MultiwayError {
    MultiwayError::InvalidOption {
        name,
        message: message.into(),
    }
}
