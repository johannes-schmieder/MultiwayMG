//! Bounded complete-cycle witness repair for hard factor aggregations.
//!
//! Compatible-relaxation witnesses diagnose the smoother on the complement of
//! a proposed coarse space. This module instead uses the slowest witness of the
//! complete two-grid error operator. It monotonically enriches a hard coarse
//! space by splitting one aggregate at a time, and admits a split only when the
//! full cycle probe improves by a declared multiplicative amount while every
//! structural budget remains satisfied.

use crate::{
    CycleQualityCriteria, CycleQualityDecision, CycleQualityOptions, CycleQualityReport,
    DiagonalAggregationProjector, FactorAggregation, MultiwayError, Preconditioner,
    ThreeWayProblem, analyze_cycle_quality, evaluate_cycle_quality,
};

/// Explicit policy for bounded complete-cycle split repair.
#[derive(Debug, Clone, Copy)]
pub struct CycleSplitRepairOptions {
    /// Matrix-free complete-cycle power probe.
    pub probe: CycleQualityOptions,
    /// Complete-cycle acceptance criteria.
    pub criteria: CycleQualityCriteria,
    /// Maximum number of admitted aggregate splits.
    pub maximum_rounds: usize,
    /// Largest coarse coefficient dimension divided by fine dimension.
    pub maximum_coarse_dimension_ratio: f64,
    /// Minimum required reduction in unique tuple count.
    pub minimum_tuple_reduction: f64,
    /// Largest `(fine tuples + coarse tuples) / fine tuples`.
    pub maximum_two_level_tuple_complexity: f64,
    /// Smallest fraction of the slow witness's diagonal energy that an
    /// aggregate must contain to be eligible for splitting.
    pub minimum_split_score_fraction: f64,
    /// Largest accepted candidate factor divided by the current factor.
    /// Values below one require a strict complete-cycle improvement.
    pub maximum_candidate_factor_ratio: f64,
}

impl CycleSplitRepairOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if self.maximum_rounds == 0 {
            return Err(invalid("cycle_split_maximum_rounds", "must be positive"));
        }
        validate_unit_interval(
            "cycle_split_maximum_coarse_dimension_ratio",
            self.maximum_coarse_dimension_ratio,
            false,
        )?;
        validate_unit_interval(
            "cycle_split_minimum_tuple_reduction",
            self.minimum_tuple_reduction,
            true,
        )?;
        if !self.maximum_two_level_tuple_complexity.is_finite()
            || !(1.0..=2.0).contains(&self.maximum_two_level_tuple_complexity)
        {
            return Err(invalid(
                "cycle_split_maximum_two_level_tuple_complexity",
                format!(
                    "must be finite and lie in [1, 2], got {}",
                    self.maximum_two_level_tuple_complexity
                ),
            ));
        }
        validate_unit_interval(
            "cycle_split_minimum_split_score_fraction",
            self.minimum_split_score_fraction,
            true,
        )?;
        if !self.maximum_candidate_factor_ratio.is_finite()
            || !(0.0..1.0).contains(&self.maximum_candidate_factor_ratio)
        {
            return Err(invalid(
                "cycle_split_maximum_candidate_factor_ratio",
                format!(
                    "must be finite and lie in (0, 1), got {}",
                    self.maximum_candidate_factor_ratio
                ),
            ));
        }
        Ok(self)
    }
}

/// Why complete-cycle split repair stopped.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CycleSplitRepairStopReason {
    /// No admissible aggregate carried enough slow-witness energy.
    NoSplittableAggregate {
        /// Minimum required witness-energy share.
        minimum_score_fraction: f64,
    },
    /// The next split would exceed the coarse-dimension budget.
    CoarseDimensionBudget {
        /// Candidate coarse dimension.
        attempted_dimension: usize,
        /// Maximum admitted coarse dimension.
        maximum_dimension: usize,
    },
    /// The next split would leave insufficient tuple reduction.
    TupleReductionBudget {
        /// Candidate tuple reduction.
        attempted_reduction: f64,
        /// Minimum admitted tuple reduction.
        minimum_reduction: f64,
    },
    /// The next split would exceed the two-level tuple-complexity budget.
    TupleComplexityBudget {
        /// Candidate two-level tuple complexity.
        attempted_complexity: f64,
        /// Maximum admitted complexity.
        maximum_complexity: f64,
    },
    /// The proposed split did not improve the complete-cycle factor enough.
    InsufficientCycleImprovement {
        /// Current worst estimated energy factor.
        current_factor: f64,
        /// Candidate worst estimated energy factor.
        candidate_factor: f64,
        /// Maximum admitted candidate/current ratio.
        maximum_ratio: f64,
    },
    /// The configured split budget was exhausted.
    MaximumRounds {
        /// Configured maximum admitted splits.
        maximum_rounds: usize,
    },
}

/// One split proposed from the slowest complete-cycle witness.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleAggregateSplit {
    factor: usize,
    parent: u32,
    witness_index: usize,
    witness_factor: f64,
    score_fraction: f64,
    separating_gap: f64,
    left_members: Vec<u32>,
    right_members: Vec<u32>,
}

impl CycleAggregateSplit {
    /// Zero-based factor index.
    #[must_use]
    pub const fn factor(&self) -> usize {
        self.factor
    }

    /// Parent label before the split.
    #[must_use]
    pub const fn parent(&self) -> u32 {
        self.parent
    }

    /// Complete-cycle power-start index supplying the witness.
    #[must_use]
    pub const fn witness_index(&self) -> usize {
        self.witness_index
    }

    /// Tail-estimated energy factor of the selected witness.
    #[must_use]
    pub const fn witness_factor(&self) -> f64 {
        self.witness_factor
    }

    /// Fraction of witness diagonal energy in the selected aggregate.
    #[must_use]
    pub const fn score_fraction(&self) -> f64 {
        self.score_fraction
    }

    /// Largest adjacent witness-value gap used to divide members.
    #[must_use]
    pub const fn separating_gap(&self) -> f64 {
        self.separating_gap
    }

    /// Members retaining the original parent label.
    #[must_use]
    pub fn left_members(&self) -> &[u32] {
        &self.left_members
    }

    /// Members assigned to the inserted parent label.
    #[must_use]
    pub fn right_members(&self) -> &[u32] {
        &self.right_members
    }
}

/// Structural diagnostics for one evaluated hard map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CycleSplitStructuralMetrics {
    coarse_dimension: usize,
    coarse_tuple_count: usize,
    coarse_dimension_ratio: f64,
    tuple_reduction: f64,
    two_level_tuple_complexity: f64,
}

impl CycleSplitStructuralMetrics {
    /// Coarse coefficient dimension.
    #[must_use]
    pub const fn coarse_dimension(self) -> usize {
        self.coarse_dimension
    }

    /// Unique coarse tuple count.
    #[must_use]
    pub const fn coarse_tuple_count(self) -> usize {
        self.coarse_tuple_count
    }

    /// Coarse dimension divided by fine dimension.
    #[must_use]
    pub const fn coarse_dimension_ratio(self) -> f64 {
        self.coarse_dimension_ratio
    }

    /// Fractional unique-tuple reduction.
    #[must_use]
    pub const fn tuple_reduction(self) -> f64 {
        self.tuple_reduction
    }

    /// Two-level tuple complexity.
    #[must_use]
    pub const fn two_level_tuple_complexity(self) -> f64 {
        self.two_level_tuple_complexity
    }
}

/// One accepted split transition and the probes bracketing it.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleSplitRepairRound {
    index: usize,
    before_metrics: CycleSplitStructuralMetrics,
    before_report: CycleQualityReport,
    before_decision: CycleQualityDecision,
    split: CycleAggregateSplit,
    after_metrics: CycleSplitStructuralMetrics,
    after_report: CycleQualityReport,
    after_decision: CycleQualityDecision,
}

impl CycleSplitRepairRound {
    /// Zero-based accepted split index.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Structural metrics before the split.
    #[must_use]
    pub const fn before_metrics(&self) -> CycleSplitStructuralMetrics {
        self.before_metrics
    }

    /// Complete-cycle probe before the split.
    #[must_use]
    pub const fn before_report(&self) -> &CycleQualityReport {
        &self.before_report
    }

    /// Complete-cycle decision before the split.
    #[must_use]
    pub const fn before_decision(&self) -> &CycleQualityDecision {
        &self.before_decision
    }

    /// Accepted witness-driven split.
    #[must_use]
    pub const fn split(&self) -> &CycleAggregateSplit {
        &self.split
    }

    /// Structural metrics after the split.
    #[must_use]
    pub const fn after_metrics(&self) -> CycleSplitStructuralMetrics {
        self.after_metrics
    }

    /// Complete-cycle probe after the split.
    #[must_use]
    pub const fn after_report(&self) -> &CycleQualityReport {
        &self.after_report
    }

    /// Complete-cycle decision after the split.
    #[must_use]
    pub const fn after_decision(&self) -> &CycleQualityDecision {
        &self.after_decision
    }
}

/// Complete bounded cycle-aware split-repair result.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleSplitRepairResult {
    initial_aggregation: FactorAggregation,
    final_aggregation: FactorAggregation,
    initial_metrics: CycleSplitStructuralMetrics,
    final_metrics: CycleSplitStructuralMetrics,
    initial_report: CycleQualityReport,
    initial_decision: CycleQualityDecision,
    final_report: CycleQualityReport,
    final_decision: CycleQualityDecision,
    accepted_splits: usize,
    stop_reason: CycleSplitRepairStopReason,
    rounds: Vec<CycleSplitRepairRound>,
}

impl CycleSplitRepairResult {
    /// Submitted map before complete-cycle repair.
    #[must_use]
    pub const fn initial_aggregation(&self) -> &FactorAggregation {
        &self.initial_aggregation
    }

    /// Last map admitted by both cycle-improvement and structural gates.
    #[must_use]
    pub const fn final_aggregation(&self) -> &FactorAggregation {
        &self.final_aggregation
    }

    /// Whether the final complete cycle passes its declared criteria.
    #[must_use]
    pub fn accepted(&self) -> bool {
        self.final_decision.accepted()
    }

    /// Structural metrics before repair.
    #[must_use]
    pub const fn initial_metrics(&self) -> CycleSplitStructuralMetrics {
        self.initial_metrics
    }

    /// Structural metrics after repair.
    #[must_use]
    pub const fn final_metrics(&self) -> CycleSplitStructuralMetrics {
        self.final_metrics
    }

    /// Initial complete-cycle report.
    #[must_use]
    pub const fn initial_report(&self) -> &CycleQualityReport {
        &self.initial_report
    }

    /// Final complete-cycle report.
    #[must_use]
    pub const fn final_report(&self) -> &CycleQualityReport {
        &self.final_report
    }

    /// Number of admitted aggregate splits.
    #[must_use]
    pub const fn accepted_splits(&self) -> usize {
        self.accepted_splits
    }

    /// Deterministic stop reason.
    #[must_use]
    pub const fn stop_reason(&self) -> &CycleSplitRepairStopReason {
        &self.stop_reason
    }

    /// Accepted split transitions.
    #[must_use]
    pub fn rounds(&self) -> &[CycleSplitRepairRound] {
        &self.rounds
    }
}

/// Improve a hard aggregation using complete-cycle slow witnesses.
///
/// The cycle builder must return the exact fixed preconditioner whose quality is
/// being optimized. Candidate splits are evaluated with the same deterministic
/// probe and accepted only after exact coarse tuple construction and every
/// structural budget check.
pub fn repair_cycle_aggregation_by_splitting<C, F>(
    problem: &ThreeWayProblem,
    initial_aggregation: &FactorAggregation,
    options: CycleSplitRepairOptions,
    mut build_cycle: F,
) -> Result<CycleSplitRepairResult, MultiwayError>
where
    C: Preconditioner,
    F: FnMut(&FactorAggregation) -> Result<C, MultiwayError>,
{
    let options = options.validate()?;
    let initial_aggregation = initial_aggregation.clone();
    let maximum_coarse_dimension =
        (options.maximum_coarse_dimension_ratio * problem.dimension() as f64).floor() as usize;
    let mut current_aggregation = initial_aggregation.clone();
    let mut current_metrics = structural_metrics(problem, &current_aggregation)?;
    let current_cycle = build_cycle(&current_aggregation)?;
    let mut current_report = analyze_cycle_quality(problem, &current_cycle, options.probe)?;
    let mut current_decision = evaluate_cycle_quality(&current_report, options.criteria)?;
    let initial_metrics = current_metrics;
    let initial_report = current_report.clone();
    let initial_decision = current_decision.clone();
    let mut rounds = Vec::with_capacity(options.maximum_rounds);
    if let Err(reason) = validate_metrics(current_metrics, maximum_coarse_dimension, options) {
        return Ok(result(
            initial_aggregation,
            current_aggregation,
            initial_metrics,
            current_metrics,
            initial_report,
            initial_decision,
            current_report,
            current_decision,
            rounds,
            reason,
        ));
    }

    for round_index in 0..options.maximum_rounds {
        let Some(split) = choose_split(
            problem,
            &current_aggregation,
            &current_report,
            options.minimum_split_score_fraction,
        ) else {
            return Ok(result(
                initial_aggregation,
                current_aggregation,
                initial_metrics,
                current_metrics,
                initial_report,
                initial_decision,
                current_report,
                current_decision,
                rounds,
                CycleSplitRepairStopReason::NoSplittableAggregate {
                    minimum_score_fraction: options.minimum_split_score_fraction,
                },
            ));
        };
        let candidate_aggregation = apply_split(&current_aggregation, &split)?;
        let candidate_metrics = structural_metrics(problem, &candidate_aggregation)?;
        if let Err(reason) = validate_metrics(candidate_metrics, maximum_coarse_dimension, options)
        {
            return Ok(result(
                initial_aggregation,
                current_aggregation,
                initial_metrics,
                current_metrics,
                initial_report,
                initial_decision,
                current_report,
                current_decision,
                rounds,
                reason,
            ));
        }
        let candidate_cycle = build_cycle(&candidate_aggregation)?;
        let candidate_report = analyze_cycle_quality(problem, &candidate_cycle, options.probe)?;
        let candidate_decision = evaluate_cycle_quality(&candidate_report, options.criteria)?;
        let current_factor = current_report.maximum_estimated_energy_factor();
        let candidate_factor = candidate_report.maximum_estimated_energy_factor();
        if candidate_factor > current_factor * options.maximum_candidate_factor_ratio {
            return Ok(result(
                initial_aggregation,
                current_aggregation,
                initial_metrics,
                current_metrics,
                initial_report,
                initial_decision,
                current_report,
                current_decision,
                rounds,
                CycleSplitRepairStopReason::InsufficientCycleImprovement {
                    current_factor,
                    candidate_factor,
                    maximum_ratio: options.maximum_candidate_factor_ratio,
                },
            ));
        }
        rounds.push(CycleSplitRepairRound {
            index: round_index,
            before_metrics: current_metrics,
            before_report: current_report,
            before_decision: current_decision,
            split,
            after_metrics: candidate_metrics,
            after_report: candidate_report.clone(),
            after_decision: candidate_decision.clone(),
        });
        current_aggregation = candidate_aggregation;
        current_metrics = candidate_metrics;
        current_report = candidate_report;
        current_decision = candidate_decision;
    }

    Ok(result(
        initial_aggregation,
        current_aggregation,
        initial_metrics,
        current_metrics,
        initial_report,
        initial_decision,
        current_report,
        current_decision,
        rounds,
        CycleSplitRepairStopReason::MaximumRounds {
            maximum_rounds: options.maximum_rounds,
        },
    ))
}

#[allow(clippy::too_many_arguments)]
fn result(
    initial_aggregation: FactorAggregation,
    final_aggregation: FactorAggregation,
    initial_metrics: CycleSplitStructuralMetrics,
    final_metrics: CycleSplitStructuralMetrics,
    initial_report: CycleQualityReport,
    initial_decision: CycleQualityDecision,
    final_report: CycleQualityReport,
    final_decision: CycleQualityDecision,
    rounds: Vec<CycleSplitRepairRound>,
    stop_reason: CycleSplitRepairStopReason,
) -> CycleSplitRepairResult {
    let accepted_splits = rounds.len();
    CycleSplitRepairResult {
        initial_aggregation,
        final_aggregation,
        initial_metrics,
        final_metrics,
        initial_report,
        initial_decision,
        final_report,
        final_decision,
        accepted_splits,
        stop_reason,
        rounds,
    }
}

fn choose_split(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    report: &CycleQualityReport,
    minimum_score_fraction: f64,
) -> Option<CycleAggregateSplit> {
    let witness_index = report.slowest_vector_index()?;
    let witness_report = &report.vectors()[witness_index];
    let witness = witness_report.final_error();
    let counts = problem.topology().level_counts();
    let offsets = problem.topology().offsets();
    let total_energy = problem
        .diagonal()
        .iter()
        .zip(witness)
        .map(|(&degree, &value)| degree * value * value)
        .sum::<f64>();
    if !total_energy.is_finite() || total_energy <= 0.0 {
        return None;
    }

    let mut best: Option<(f64, usize, u32, Vec<usize>)> = None;
    for factor in 0..3 {
        let mut members = vec![Vec::new(); aggregation.coarse_counts()[factor]];
        for level in 0..counts[factor] {
            members[aggregation.parents(factor)[level] as usize].push(level);
        }
        for (parent, levels) in members.into_iter().enumerate() {
            if levels.len() < 2 {
                continue;
            }
            let score = levels
                .iter()
                .map(|&level| {
                    let index = offsets[factor] + level;
                    problem.diagonal()[index] * witness[index] * witness[index]
                })
                .sum::<f64>();
            let score_fraction = score / total_energy;
            if score_fraction < minimum_score_fraction {
                continue;
            }
            let replace = best.as_ref().is_none_or(|current| {
                score_fraction.total_cmp(&current.0).is_gt()
                    || score_fraction.to_bits() == current.0.to_bits()
                        && (factor, parent) < (current.1, current.2 as usize)
            });
            if replace {
                best = Some((score_fraction, factor, parent as u32, levels));
            }
        }
    }
    let (score_fraction, factor, parent, mut members) = best?;
    members.sort_by(|&left, &right| {
        witness[offsets[factor] + left]
            .total_cmp(&witness[offsets[factor] + right])
            .then_with(|| left.cmp(&right))
    });

    let mut best_cut = 1;
    let mut best_gap = f64::NEG_INFINITY;
    let mut best_balance = f64::INFINITY;
    let total_mass = members
        .iter()
        .map(|&level| problem.diagonal()[offsets[factor] + level])
        .sum::<f64>();
    let mut left_mass = 0.0;
    for cut in 1..members.len() {
        left_mass += problem.diagonal()[offsets[factor] + members[cut - 1]];
        let gap =
            witness[offsets[factor] + members[cut]] - witness[offsets[factor] + members[cut - 1]];
        let balance = (2.0 * left_mass - total_mass).abs();
        if gap.total_cmp(&best_gap).is_gt()
            || (gap.to_bits() == best_gap.to_bits()
                && (balance.total_cmp(&best_balance).is_lt()
                    || (balance.to_bits() == best_balance.to_bits() && cut < best_cut)))
        {
            best_cut = cut;
            best_gap = gap;
            best_balance = balance;
        }
    }

    Some(CycleAggregateSplit {
        factor,
        parent,
        witness_index,
        witness_factor: witness_report.estimated_energy_factor(),
        score_fraction,
        separating_gap: best_gap,
        left_members: members[..best_cut]
            .iter()
            .map(|&level| level as u32)
            .collect(),
        right_members: members[best_cut..]
            .iter()
            .map(|&level| level as u32)
            .collect(),
    })
}

fn apply_split(
    aggregation: &FactorAggregation,
    split: &CycleAggregateSplit,
) -> Result<FactorAggregation, MultiwayError> {
    let fine_counts = aggregation.fine_counts();
    let mut parents: [Vec<u32>; 3] =
        core::array::from_fn(|factor| aggregation.parents(factor).to_vec());
    let mut right = vec![false; fine_counts[split.factor]];
    for &level in &split.right_members {
        right[level as usize] = true;
    }
    for level in 0..fine_counts[split.factor] {
        let old_parent = aggregation.parents(split.factor)[level];
        parents[split.factor][level] = if old_parent < split.parent {
            old_parent
        } else if old_parent > split.parent {
            old_parent + 1
        } else if right[level] {
            split.parent + 1
        } else {
            split.parent
        };
    }
    FactorAggregation::new(fine_counts, parents).map_err(Into::into)
}

fn structural_metrics(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<CycleSplitStructuralMetrics, MultiwayError> {
    DiagonalAggregationProjector::new(problem.clone(), aggregation.clone())?;
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension = coarse.dimension();
    let coarse_tuple_count = coarse.tuple_count();
    let tuple_ratio = coarse_tuple_count as f64 / problem.tuple_count() as f64;
    Ok(CycleSplitStructuralMetrics {
        coarse_dimension,
        coarse_tuple_count,
        coarse_dimension_ratio: coarse_dimension as f64 / problem.dimension() as f64,
        tuple_reduction: 1.0 - tuple_ratio,
        two_level_tuple_complexity: 1.0 + tuple_ratio,
    })
}

fn validate_metrics(
    metrics: CycleSplitStructuralMetrics,
    maximum_coarse_dimension: usize,
    options: CycleSplitRepairOptions,
) -> Result<(), CycleSplitRepairStopReason> {
    if metrics.coarse_dimension > maximum_coarse_dimension {
        return Err(CycleSplitRepairStopReason::CoarseDimensionBudget {
            attempted_dimension: metrics.coarse_dimension,
            maximum_dimension: maximum_coarse_dimension,
        });
    }
    if metrics.tuple_reduction < options.minimum_tuple_reduction {
        return Err(CycleSplitRepairStopReason::TupleReductionBudget {
            attempted_reduction: metrics.tuple_reduction,
            minimum_reduction: options.minimum_tuple_reduction,
        });
    }
    if metrics.two_level_tuple_complexity > options.maximum_two_level_tuple_complexity {
        return Err(CycleSplitRepairStopReason::TupleComplexityBudget {
            attempted_complexity: metrics.two_level_tuple_complexity,
            maximum_complexity: options.maximum_two_level_tuple_complexity,
        });
    }
    Ok(())
}

fn validate_unit_interval(
    name: &'static str,
    value: f64,
    include_zero: bool,
) -> Result<(), MultiwayError> {
    let valid = if include_zero {
        (0.0..=1.0).contains(&value)
    } else {
        (0.0..=1.0).contains(&value) && value > 0.0
    };
    if !value.is_finite() || !valid {
        return Err(invalid(
            name,
            format!("must be finite and lie in the admitted unit interval, got {value}"),
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
