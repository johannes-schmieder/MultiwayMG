//! Bounded deterministic repair of hard factor aggregations.
//!
//! The first repair method is deliberately monotone: it only enriches an
//! existing coarse space by splitting one aggregate at a time. A deterministic
//! compatible-relaxation witness identifies the aggregate containing the most
//! remaining diagonal-energy error. Members are separated at the largest gap
//! in that witness, subject to explicit coarse-dimension and tuple-complexity
//! budgets.

use crate::{
    CompatibleRelaxationCriteria, CompatibleRelaxationDecision, CompatibleRelaxationOptions,
    CompatibleRelaxationReport, FactorAggregation, MultiwayError, Preconditioner, ThreeWayProblem,
    analyze_compatible_relaxation, evaluate_compatible_relaxation,
};

/// Explicit budgets and compatible-relaxation policy for aggregate repair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AggregationRepairOptions {
    /// Deterministic compatible-relaxation experiment used at every round.
    pub relaxation: CompatibleRelaxationOptions,
    /// Criteria determining whether the current map is acceptable.
    pub criteria: CompatibleRelaxationCriteria,
    /// Maximum number of accepted aggregate splits.
    pub maximum_rounds: usize,
    /// Largest coarse coefficient dimension divided by the fine dimension.
    pub maximum_coarse_dimension_ratio: f64,
    /// Minimum required reduction in unique tuple count relative to the fine
    /// problem after every accepted split.
    pub minimum_tuple_reduction: f64,
    /// Largest accepted two-level tuple complexity
    /// `(fine_tuples + coarse_tuples) / fine_tuples`.
    pub maximum_two_level_tuple_complexity: f64,
    /// Smallest fraction of the slow witness's diagonal energy that an
    /// aggregate must contain to be eligible for splitting.
    pub minimum_split_score_fraction: f64,
}

impl AggregationRepairOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if self.maximum_rounds == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "repair_maximum_rounds",
                message: "must be positive".to_owned(),
            });
        }
        validate_unit_interval(
            "repair_maximum_coarse_dimension_ratio",
            self.maximum_coarse_dimension_ratio,
            false,
        )?;
        validate_unit_interval(
            "repair_minimum_tuple_reduction",
            self.minimum_tuple_reduction,
            true,
        )?;
        if !self.maximum_two_level_tuple_complexity.is_finite()
            || !(1.0..=2.0).contains(&self.maximum_two_level_tuple_complexity)
        {
            return Err(MultiwayError::InvalidOption {
                name: "repair_maximum_two_level_tuple_complexity",
                message: format!(
                    "must be finite and lie in [1, 2], got {}",
                    self.maximum_two_level_tuple_complexity
                ),
            });
        }
        validate_unit_interval(
            "repair_minimum_split_score_fraction",
            self.minimum_split_score_fraction,
            true,
        )?;
        Ok(self)
    }
}

/// Why a bounded repair process stopped.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AggregationRepairStopReason {
    /// The submitted map already passed the declared criteria.
    AlreadyAccepted,
    /// A repaired map passed the declared criteria.
    Accepted,
    /// The maximum number of accepted splits was exhausted.
    MaximumRounds {
        /// Configured split budget.
        maximum_rounds: usize,
    },
    /// No aggregate with at least two members carried enough witness energy.
    NoSplittableAggregate {
        /// Minimum required fraction of witness diagonal energy.
        minimum_score_fraction: f64,
    },
    /// The current map or proposed split makes the coarse space too large.
    CoarseDimensionBudget {
        /// Coarse dimension after the proposed split.
        attempted_dimension: usize,
        /// Largest admitted coarse dimension.
        maximum_dimension: usize,
    },
    /// The current map or proposed split leaves too many unique coarse tuples.
    TupleReductionBudget {
        /// Reduction after the proposed split.
        attempted_reduction: f64,
        /// Minimum admitted reduction.
        minimum_reduction: f64,
    },
    /// The current map or proposed split exceeds the two-level tuple-work budget.
    TupleComplexityBudget {
        /// Complexity after the proposed split.
        attempted_complexity: f64,
        /// Maximum admitted complexity.
        maximum_complexity: f64,
    },
    /// The current map spans the complete fine coefficient space, leaving no
    /// compatible complement to diagnose.
    NoCompatibleComplement,
}

/// One deterministic split selected from a slow compatible witness.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregateSplit {
    factor: usize,
    parent: u32,
    witness_index: usize,
    witness_diagonal_contraction: f64,
    score_fraction: f64,
    separating_gap: f64,
    left_members: Vec<u32>,
    right_members: Vec<u32>,
}

impl AggregateSplit {
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

    /// Deterministic compatible test-vector index used for scoring.
    #[must_use]
    pub const fn witness_index(&self) -> usize {
        self.witness_index
    }

    /// Final divided by initial diagonal norm of the selected witness.
    #[must_use]
    pub const fn witness_diagonal_contraction(&self) -> f64 {
        self.witness_diagonal_contraction
    }

    /// Fraction of the witness's final diagonal energy contained in the
    /// selected aggregate.
    #[must_use]
    pub const fn score_fraction(&self) -> f64 {
        self.score_fraction
    }

    /// Largest adjacent witness-value gap used to divide the aggregate.
    #[must_use]
    pub const fn separating_gap(&self) -> f64 {
        self.separating_gap
    }

    /// Members assigned to the original parent label.
    #[must_use]
    pub fn left_members(&self) -> &[u32] {
        &self.left_members
    }

    /// Members assigned to the newly inserted parent label.
    #[must_use]
    pub fn right_members(&self) -> &[u32] {
        &self.right_members
    }
}

/// Diagnostics for one evaluated map and an optional proposed split.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregationRepairRound {
    index: usize,
    coarse_dimension: usize,
    coarse_tuple_count: usize,
    coarse_dimension_ratio: f64,
    tuple_reduction: f64,
    two_level_tuple_complexity: f64,
    report: CompatibleRelaxationReport,
    decision: CompatibleRelaxationDecision,
    proposed_split: Option<AggregateSplit>,
}

impl AggregationRepairRound {
    /// Zero-based repair round.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Coarse coefficient dimension of the evaluated map.
    #[must_use]
    pub const fn coarse_dimension(&self) -> usize {
        self.coarse_dimension
    }

    /// Unique coarse tuple count of the evaluated map.
    #[must_use]
    pub const fn coarse_tuple_count(&self) -> usize {
        self.coarse_tuple_count
    }

    /// Coarse dimension divided by fine dimension.
    #[must_use]
    pub const fn coarse_dimension_ratio(&self) -> f64 {
        self.coarse_dimension_ratio
    }

    /// Fractional unique-tuple reduction relative to the fine problem.
    #[must_use]
    pub const fn tuple_reduction(&self) -> f64 {
        self.tuple_reduction
    }

    /// Two-level tuple complexity.
    #[must_use]
    pub const fn two_level_tuple_complexity(&self) -> f64 {
        self.two_level_tuple_complexity
    }

    /// Full compatible-relaxation diagnostics.
    #[must_use]
    pub const fn report(&self) -> &CompatibleRelaxationReport {
        &self.report
    }

    /// Acceptance decision for the evaluated map.
    #[must_use]
    pub const fn decision(&self) -> &CompatibleRelaxationDecision {
        &self.decision
    }

    /// Split proposed from the slowest witness, when one was available.
    #[must_use]
    pub const fn proposed_split(&self) -> Option<&AggregateSplit> {
        self.proposed_split.as_ref()
    }
}

/// Complete bounded repair result.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregationRepairResult {
    initial_aggregation: FactorAggregation,
    final_aggregation: FactorAggregation,
    accepted: bool,
    accepted_splits: usize,
    stop_reason: AggregationRepairStopReason,
    rounds: Vec<AggregationRepairRound>,
}

impl AggregationRepairResult {
    /// Submitted aggregation before repair.
    #[must_use]
    pub const fn initial_aggregation(&self) -> &FactorAggregation {
        &self.initial_aggregation
    }

    /// Last aggregation admitted by all structural budgets.
    #[must_use]
    pub const fn final_aggregation(&self) -> &FactorAggregation {
        &self.final_aggregation
    }

    /// Whether the final map passed the compatible-relaxation criteria.
    #[must_use]
    pub const fn accepted(&self) -> bool {
        self.accepted
    }

    /// Number of aggregate splits admitted before stopping.
    #[must_use]
    pub const fn accepted_splits(&self) -> usize {
        self.accepted_splits
    }

    /// Deterministic stop reason.
    #[must_use]
    pub const fn stop_reason(&self) -> &AggregationRepairStopReason {
        &self.stop_reason
    }

    /// Evaluated maps and proposed splits in chronological order.
    #[must_use]
    pub fn rounds(&self) -> &[AggregationRepairRound] {
        &self.rounds
    }
}

/// Enrich a hard coarse space through bounded witness-driven aggregate splits.
///
/// The routine never merges levels and never accepts a proposed split before
/// constructing its exact coarse tuple problem and checking every declared
/// structural budget. Compatible relaxation is rerun from deterministic test
/// vectors after each accepted split.
pub fn repair_aggregation_by_splitting<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    initial_aggregation: &FactorAggregation,
    smoother: &P,
    options: AggregationRepairOptions,
) -> Result<AggregationRepairResult, MultiwayError> {
    let options = options.validate()?;
    if smoother.dimension() != problem.dimension() {
        return Err(crate::error::dimension(
            "repair_aggregation_by_splitting smoother",
            problem.dimension(),
            smoother.dimension(),
        ));
    }
    let initial = initial_aggregation.clone();
    let mut current = initial.clone();
    let mut rounds = Vec::with_capacity(options.maximum_rounds + 1);
    let maximum_coarse_dimension =
        (options.maximum_coarse_dimension_ratio * problem.dimension() as f64).floor() as usize;

    for round_index in 0..=options.maximum_rounds {
        let coarse_dimension: usize = current.coarse_counts().iter().sum();
        if coarse_dimension >= problem.dimension() {
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::NoCompatibleComplement,
                rounds,
            });
        }
        let coarse = current.coarsen(problem)?;
        let coarse_tuple_count = coarse.tuple_count();
        let metrics = structural_metrics(problem, coarse_dimension, coarse_tuple_count);
        let report =
            analyze_compatible_relaxation(problem, &current, smoother, options.relaxation)?;
        let decision = evaluate_compatible_relaxation(&report, options.criteria)?;
        if coarse_dimension > maximum_coarse_dimension {
            rounds.push(AggregationRepairRound {
                index: round_index,
                coarse_dimension,
                coarse_tuple_count,
                coarse_dimension_ratio: metrics.coarse_dimension_ratio,
                tuple_reduction: metrics.tuple_reduction,
                two_level_tuple_complexity: metrics.two_level_tuple_complexity,
                report,
                decision,
                proposed_split: None,
            });
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::CoarseDimensionBudget {
                    attempted_dimension: coarse_dimension,
                    maximum_dimension: maximum_coarse_dimension,
                },
                rounds,
            });
        }
        if metrics.tuple_reduction < options.minimum_tuple_reduction {
            rounds.push(AggregationRepairRound {
                index: round_index,
                coarse_dimension,
                coarse_tuple_count,
                coarse_dimension_ratio: metrics.coarse_dimension_ratio,
                tuple_reduction: metrics.tuple_reduction,
                two_level_tuple_complexity: metrics.two_level_tuple_complexity,
                report,
                decision,
                proposed_split: None,
            });
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::TupleReductionBudget {
                    attempted_reduction: metrics.tuple_reduction,
                    minimum_reduction: options.minimum_tuple_reduction,
                },
                rounds,
            });
        }
        if metrics.two_level_tuple_complexity > options.maximum_two_level_tuple_complexity {
            rounds.push(AggregationRepairRound {
                index: round_index,
                coarse_dimension,
                coarse_tuple_count,
                coarse_dimension_ratio: metrics.coarse_dimension_ratio,
                tuple_reduction: metrics.tuple_reduction,
                two_level_tuple_complexity: metrics.two_level_tuple_complexity,
                report,
                decision,
                proposed_split: None,
            });
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::TupleComplexityBudget {
                    attempted_complexity: metrics.two_level_tuple_complexity,
                    maximum_complexity: options.maximum_two_level_tuple_complexity,
                },
                rounds,
            });
        }
        if decision.accepted() {
            rounds.push(AggregationRepairRound {
                index: round_index,
                coarse_dimension,
                coarse_tuple_count,
                coarse_dimension_ratio: metrics.coarse_dimension_ratio,
                tuple_reduction: metrics.tuple_reduction,
                two_level_tuple_complexity: metrics.two_level_tuple_complexity,
                report,
                decision,
                proposed_split: None,
            });
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: true,
                accepted_splits: round_index,
                stop_reason: if round_index == 0 {
                    AggregationRepairStopReason::AlreadyAccepted
                } else {
                    AggregationRepairStopReason::Accepted
                },
                rounds,
            });
        }
        if round_index == options.maximum_rounds {
            rounds.push(AggregationRepairRound {
                index: round_index,
                coarse_dimension,
                coarse_tuple_count,
                coarse_dimension_ratio: metrics.coarse_dimension_ratio,
                tuple_reduction: metrics.tuple_reduction,
                two_level_tuple_complexity: metrics.two_level_tuple_complexity,
                report,
                decision,
                proposed_split: None,
            });
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::MaximumRounds {
                    maximum_rounds: options.maximum_rounds,
                },
                rounds,
            });
        }

        let witness_index =
            report
                .slowest_vector_index()
                .ok_or_else(|| MultiwayError::CompatibleRelaxation {
                    message: "compatible-relaxation report contained no witnesses".to_owned(),
                })?;
        let witness_report = &report.vectors()[witness_index];
        let witness = SlowWitness {
            index: witness_index,
            values: witness_report.final_error().to_vec(),
            diagonal_contraction: witness_report.diagonal_contraction(),
        };
        let Some(split) = choose_split(
            problem,
            &current,
            &witness,
            options.minimum_split_score_fraction,
        ) else {
            rounds.push(AggregationRepairRound {
                index: round_index,
                coarse_dimension,
                coarse_tuple_count,
                coarse_dimension_ratio: metrics.coarse_dimension_ratio,
                tuple_reduction: metrics.tuple_reduction,
                two_level_tuple_complexity: metrics.two_level_tuple_complexity,
                report,
                decision,
                proposed_split: None,
            });
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::NoSplittableAggregate {
                    minimum_score_fraction: options.minimum_split_score_fraction,
                },
                rounds,
            });
        };
        let candidate = apply_split(&current, &split)?;
        let candidate_coarse_dimension: usize = candidate.coarse_counts().iter().sum();
        let candidate_coarse = candidate.coarsen(problem)?;
        let candidate_metrics = structural_metrics(
            problem,
            candidate_coarse_dimension,
            candidate_coarse.tuple_count(),
        );
        rounds.push(AggregationRepairRound {
            index: round_index,
            coarse_dimension,
            coarse_tuple_count,
            coarse_dimension_ratio: metrics.coarse_dimension_ratio,
            tuple_reduction: metrics.tuple_reduction,
            two_level_tuple_complexity: metrics.two_level_tuple_complexity,
            report,
            decision,
            proposed_split: Some(split),
        });

        if candidate_coarse_dimension > maximum_coarse_dimension {
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::CoarseDimensionBudget {
                    attempted_dimension: candidate_coarse_dimension,
                    maximum_dimension: maximum_coarse_dimension,
                },
                rounds,
            });
        }
        if candidate_metrics.tuple_reduction < options.minimum_tuple_reduction {
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::TupleReductionBudget {
                    attempted_reduction: candidate_metrics.tuple_reduction,
                    minimum_reduction: options.minimum_tuple_reduction,
                },
                rounds,
            });
        }
        if candidate_metrics.two_level_tuple_complexity > options.maximum_two_level_tuple_complexity
        {
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::TupleComplexityBudget {
                    attempted_complexity: candidate_metrics.two_level_tuple_complexity,
                    maximum_complexity: options.maximum_two_level_tuple_complexity,
                },
                rounds,
            });
        }
        current = candidate;
    }
    unreachable!("bounded repair loop always returns")
}

#[derive(Debug)]
struct SlowWitness {
    index: usize,
    values: Vec<f64>,
    diagonal_contraction: f64,
}

fn choose_split(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    witness: &SlowWitness,
    minimum_score_fraction: f64,
) -> Option<AggregateSplit> {
    let counts = problem.topology().level_counts();
    let offsets = problem.topology().offsets();
    let total_energy = problem
        .diagonal()
        .iter()
        .zip(&witness.values)
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
                    problem.diagonal()[index] * witness.values[index] * witness.values[index]
                })
                .sum::<f64>();
            let score_fraction = score / total_energy;
            if score_fraction < minimum_score_fraction {
                continue;
            }
            let replace = best.as_ref().is_none_or(|current| {
                score_fraction.total_cmp(&current.0).is_gt()
                    || (score_fraction.to_bits() == current.0.to_bits()
                        && (factor, parent) < (current.1, current.2 as usize))
            });
            if replace {
                best = Some((score_fraction, factor, parent as u32, levels));
            }
        }
    }
    let (score_fraction, factor, parent, mut members) = best?;
    members.sort_by(|&left, &right| {
        let left_value = witness.values[offsets[factor] + left];
        let right_value = witness.values[offsets[factor] + right];
        left_value
            .total_cmp(&right_value)
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
        let left_value = witness.values[offsets[factor] + members[cut - 1]];
        let right_value = witness.values[offsets[factor] + members[cut]];
        let gap = right_value - left_value;
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
    let left_members = members[..best_cut]
        .iter()
        .map(|&level| level as u32)
        .collect();
    let right_members = members[best_cut..]
        .iter()
        .map(|&level| level as u32)
        .collect();
    Some(AggregateSplit {
        factor,
        parent,
        witness_index: witness.index,
        witness_diagonal_contraction: witness.diagonal_contraction,
        score_fraction,
        separating_gap: best_gap,
        left_members,
        right_members,
    })
}

fn apply_split(
    aggregation: &FactorAggregation,
    split: &AggregateSplit,
) -> Result<FactorAggregation, MultiwayError> {
    let fine_counts = aggregation.fine_counts();
    let mut parents: [Vec<u32>; 3] =
        core::array::from_fn(|factor| aggregation.parents(factor).to_vec());
    let selected_parent = split.parent;
    let mut right = vec![false; fine_counts[split.factor]];
    for &level in &split.right_members {
        right[level as usize] = true;
    }
    for level in 0..fine_counts[split.factor] {
        let old_parent = aggregation.parents(split.factor)[level];
        parents[split.factor][level] = if old_parent < selected_parent {
            old_parent
        } else if old_parent > selected_parent {
            old_parent + 1
        } else if right[level] {
            selected_parent + 1
        } else {
            selected_parent
        };
    }
    FactorAggregation::new(fine_counts, parents).map_err(Into::into)
}

#[derive(Debug, Clone, Copy)]
struct StructuralMetrics {
    coarse_dimension_ratio: f64,
    tuple_reduction: f64,
    two_level_tuple_complexity: f64,
}

fn structural_metrics(
    problem: &ThreeWayProblem,
    coarse_dimension: usize,
    coarse_tuple_count: usize,
) -> StructuralMetrics {
    let coarse_dimension_ratio = coarse_dimension as f64 / problem.dimension() as f64;
    let tuple_ratio = coarse_tuple_count as f64 / problem.tuple_count() as f64;
    StructuralMetrics {
        coarse_dimension_ratio,
        tuple_reduction: 1.0 - tuple_ratio,
        two_level_tuple_complexity: 1.0 + tuple_ratio,
    }
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
        return Err(MultiwayError::InvalidOption {
            name,
            message: format!("must be finite and lie in the admitted unit interval, got {value}"),
        });
    }
    Ok(())
}
