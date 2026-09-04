//! Deterministic bootstrap aggregation for weighted three-way incidence Gramians.
//!
//! The builder combines a bounded sparse structural candidate graph with
//! algebraic affinities measured on relaxed range test vectors. When a proposed
//! map fails projected compatible relaxation, its slowest measured witness is
//! range-filtered, appended to the test set, and used to rebuild the matching.
//! An optional monotone split-repair stage can enrich the final map without
//! ever weakening factor or component boundaries.

use std::collections::BTreeMap;

use crate::{
    AggregationRepairOptions, AggregationRepairResult, CompatibleRelaxationCriteria,
    CompatibleRelaxationDecision, CompatibleRelaxationOptions, CompatibleRelaxationReport,
    DiagonalPreconditioner, FactorAggregation, MultiwayError, Preconditioner, ThreeWayProblem,
    analyze_compatible_relaxation, evaluate_compatible_relaxation, repair_aggregation_by_splitting,
};

/// Controls sparse test-vector aggregation and bounded witness enrichment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootstrapAggregationOptions {
    /// Number of deterministic range test vectors used before compatible
    /// witnesses are added.
    pub setup_test_vectors: usize,
    /// Homogeneous weighted-Jacobi sweeps used to expose slow setup modes.
    pub setup_sweeps: usize,
    /// Safe weighted-Jacobi damping used during setup-vector relaxation.
    pub setup_jacobi_omega: f64,
    /// Maximum incident levels retained at one pair-marginal neighbor while
    /// constructing structural candidates.
    pub maximum_neighbor_degree: usize,
    /// Number of adjacent levels connected on each side in every sorted
    /// test-vector signature ordering.
    pub signature_window: usize,
    /// Maximum retained candidate degree per same-factor level after scoring.
    pub maximum_candidate_degree: usize,
    /// Minimum normalized combined affinity admitted to greedy matching.
    pub minimum_combined_affinity: f64,
    /// Weight on relaxed-vector value similarity.
    pub algebraic_affinity_weight: f64,
    /// Weight on exact shared pair-neighborhood mass.
    pub structural_affinity_weight: f64,
    /// Weight on weighted-degree similarity.
    pub degree_affinity_weight: f64,
    /// Weight on repeated adjacency in test-vector orderings.
    pub signature_hit_weight: f64,
    /// Compatible-relaxation experiment used to screen each proposed map.
    pub compatible_relaxation: CompatibleRelaxationOptions,
    /// Explicit compatible-relaxation acceptance policy.
    pub compatible_criteria: CompatibleRelaxationCriteria,
    /// Maximum number of slow witnesses appended after the initial map.
    pub maximum_bootstrap_witnesses: usize,
    /// Maximum accepted coarse dimension divided by fine dimension.
    pub maximum_coarse_dimension_ratio: f64,
    /// Minimum required unique-tuple reduction.
    pub minimum_tuple_reduction: f64,
    /// Maximum two-level tuple complexity `(fine + coarse) / fine`.
    pub maximum_two_level_tuple_complexity: f64,
    /// Optional final monotone split-repair stage.
    pub split_repair: Option<AggregationRepairOptions>,
    /// Deterministic seed used by setup vectors.
    pub seed: u64,
}

impl Default for BootstrapAggregationOptions {
    fn default() -> Self {
        Self {
            setup_test_vectors: 8,
            setup_sweeps: 8,
            setup_jacobi_omega: 0.5,
            maximum_neighbor_degree: 16,
            signature_window: 3,
            maximum_candidate_degree: 16,
            minimum_combined_affinity: 0.35,
            algebraic_affinity_weight: 0.55,
            structural_affinity_weight: 0.25,
            degree_affinity_weight: 0.10,
            signature_hit_weight: 0.10,
            compatible_relaxation: CompatibleRelaxationOptions::default(),
            compatible_criteria: CompatibleRelaxationCriteria {
                maximum_diagonal_factor_per_sweep: 0.80,
                maximum_energy_factor_per_sweep: Some(0.80),
                maximum_final_coarse_defect: 1.0e-10,
                maximum_final_structural_defect: 1.0e-10,
            },
            maximum_bootstrap_witnesses: 4,
            maximum_coarse_dimension_ratio: 0.75,
            minimum_tuple_reduction: 0.02,
            maximum_two_level_tuple_complexity: 1.98,
            split_repair: None,
            seed: 0x4d57_4d47_4253_3031,
        }
    }
}

impl BootstrapAggregationOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if self.setup_test_vectors == 0 {
            return Err(invalid("setup_test_vectors", "must be positive"));
        }
        if self.setup_sweeps == 0 {
            return Err(invalid("setup_sweeps", "must be positive"));
        }
        if !self.setup_jacobi_omega.is_finite()
            || !(0.0..(2.0 / 3.0)).contains(&self.setup_jacobi_omega)
        {
            return Err(invalid(
                "setup_jacobi_omega",
                format!(
                    "must be finite and lie in (0, 2/3), got {}",
                    self.setup_jacobi_omega
                ),
            ));
        }
        if self.maximum_neighbor_degree < 2 {
            return Err(invalid("maximum_neighbor_degree", "must be at least two"));
        }
        if self.signature_window == 0 {
            return Err(invalid("signature_window", "must be positive"));
        }
        if self.maximum_candidate_degree == 0 {
            return Err(invalid("maximum_candidate_degree", "must be positive"));
        }
        validate_unit_interval(
            "minimum_combined_affinity",
            self.minimum_combined_affinity,
            true,
        )?;
        let affinity_weights = [
            self.algebraic_affinity_weight,
            self.structural_affinity_weight,
            self.degree_affinity_weight,
            self.signature_hit_weight,
        ];
        if affinity_weights
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
            || affinity_weights.iter().sum::<f64>() <= 0.0
        {
            return Err(invalid(
                "affinity_weights",
                "must be finite, nonnegative, and have positive sum",
            ));
        }
        validate_unit_interval(
            "maximum_coarse_dimension_ratio",
            self.maximum_coarse_dimension_ratio,
            false,
        )?;
        validate_unit_interval(
            "minimum_tuple_reduction",
            self.minimum_tuple_reduction,
            true,
        )?;
        if !self.maximum_two_level_tuple_complexity.is_finite()
            || !(1.0..=2.0).contains(&self.maximum_two_level_tuple_complexity)
        {
            return Err(invalid(
                "maximum_two_level_tuple_complexity",
                format!(
                    "must be finite and lie in [1, 2], got {}",
                    self.maximum_two_level_tuple_complexity
                ),
            ));
        }
        Ok(self)
    }
}

/// Why bootstrap aggregation stopped.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum BootstrapAggregationStopReason {
    /// The first algebraic/structural matching passed all gates.
    AcceptedInitial,
    /// A matching rebuilt after adding compatible witnesses passed all gates.
    AcceptedAfterBootstrap {
        /// Number of appended witnesses.
        witnesses: usize,
    },
    /// Monotone split repair accepted the final bootstrap map.
    AcceptedAfterSplitRepair {
        /// Number of appended bootstrap witnesses.
        witnesses: usize,
        /// Number of admitted aggregate splits.
        splits: usize,
    },
    /// Candidate matching produced no compatible complement.
    NoCompatibleComplement,
    /// The proposed map exceeded the coarse-dimension budget.
    CoarseDimensionBudget {
        /// Observed coarse dimension.
        observed: usize,
        /// Maximum admitted coarse dimension.
        maximum: usize,
    },
    /// The proposed map did not reduce enough unique tuples.
    TupleReductionBudget {
        /// Observed fractional tuple reduction.
        observed: f64,
        /// Required fractional tuple reduction.
        minimum: f64,
    },
    /// The proposed map exceeded the two-level tuple-complexity budget.
    TupleComplexityBudget {
        /// Observed complexity.
        observed: f64,
        /// Maximum admitted complexity.
        maximum: f64,
    },
    /// Adding slow witnesses no longer changed the matching.
    MatchingStagnated {
        /// Number of witnesses available at stagnation.
        witnesses: usize,
    },
    /// The configured witness budget was exhausted.
    WitnessBudget {
        /// Maximum number of appended witnesses.
        maximum: usize,
    },
    /// The optional monotone split repair also rejected the map.
    SplitRepairRejected,
}

/// Structural size and work diagnostics for one proposed aggregation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootstrapStructuralMetrics {
    coarse_dimension: usize,
    coarse_tuple_count: usize,
    coarse_dimension_ratio: f64,
    tuple_reduction: f64,
    two_level_tuple_complexity: f64,
}

impl BootstrapStructuralMetrics {
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

    /// Fractional reduction in unique tuple count.
    #[must_use]
    pub const fn tuple_reduction(self) -> f64 {
        self.tuple_reduction
    }

    /// `(fine tuples + coarse tuples) / fine tuples`.
    #[must_use]
    pub const fn two_level_tuple_complexity(self) -> f64 {
        self.two_level_tuple_complexity
    }
}

/// One bootstrap matching and compatible-relaxation evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapAggregationRound {
    index: usize,
    bootstrap_witnesses: usize,
    candidate_pairs_generated: usize,
    candidate_pairs_retained: usize,
    maximum_retained_candidate_degree: usize,
    matching_changed: bool,
    structural_metrics: BootstrapStructuralMetrics,
    compatible_report: CompatibleRelaxationReport,
    compatible_decision: CompatibleRelaxationDecision,
}

impl BootstrapAggregationRound {
    /// Zero-based round index.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Number of previously appended slow witnesses used to build this map.
    #[must_use]
    pub const fn bootstrap_witnesses(&self) -> usize {
        self.bootstrap_witnesses
    }

    /// Number of unique sparse candidate pairs generated before degree pruning.
    #[must_use]
    pub const fn candidate_pairs_generated(&self) -> usize {
        self.candidate_pairs_generated
    }

    /// Number of scored candidate pairs retained after threshold and degree pruning.
    #[must_use]
    pub const fn candidate_pairs_retained(&self) -> usize {
        self.candidate_pairs_retained
    }

    /// Largest retained same-factor candidate degree.
    #[must_use]
    pub const fn maximum_retained_candidate_degree(&self) -> usize {
        self.maximum_retained_candidate_degree
    }

    /// Whether this round's matching differs from the preceding round.
    #[must_use]
    pub const fn matching_changed(&self) -> bool {
        self.matching_changed
    }

    /// Structural size metrics.
    #[must_use]
    pub const fn structural_metrics(&self) -> BootstrapStructuralMetrics {
        self.structural_metrics
    }

    /// Full compatible-relaxation report.
    #[must_use]
    pub const fn compatible_report(&self) -> &CompatibleRelaxationReport {
        &self.compatible_report
    }

    /// Explicit compatible-relaxation decision.
    #[must_use]
    pub const fn compatible_decision(&self) -> &CompatibleRelaxationDecision {
        &self.compatible_decision
    }
}

/// Work and retained-state diagnostics for a complete bootstrap build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapAggregationWorkReport {
    setup_gramian_applications: usize,
    setup_smoother_applications: usize,
    compatible_gramian_applications: usize,
    compatible_smoother_applications: usize,
    candidate_pairs_generated: usize,
    candidate_pairs_retained: usize,
    retained_test_vector_bytes: usize,
    retained_round_report_bytes_estimate: usize,
}

impl BootstrapAggregationWorkReport {
    /// Gramian applications used to generate initial range test vectors.
    #[must_use]
    pub const fn setup_gramian_applications(self) -> usize {
        self.setup_gramian_applications
    }

    /// Weighted-Jacobi applications used for setup-vector relaxation.
    #[must_use]
    pub const fn setup_smoother_applications(self) -> usize {
        self.setup_smoother_applications
    }

    /// Gramian applications used by compatible-relaxation screens.
    #[must_use]
    pub const fn compatible_gramian_applications(self) -> usize {
        self.compatible_gramian_applications
    }

    /// Smoother applications used by compatible-relaxation screens.
    #[must_use]
    pub const fn compatible_smoother_applications(self) -> usize {
        self.compatible_smoother_applications
    }

    /// Sum of generated candidate-pair counts across rounds.
    #[must_use]
    pub const fn candidate_pairs_generated(self) -> usize {
        self.candidate_pairs_generated
    }

    /// Sum of retained candidate-pair counts across rounds.
    #[must_use]
    pub const fn candidate_pairs_retained(self) -> usize {
        self.candidate_pairs_retained
    }

    /// Bytes retained by setup and appended witness vectors.
    #[must_use]
    pub const fn retained_test_vector_bytes(self) -> usize {
        self.retained_test_vector_bytes
    }

    /// Principal bytes retained by round reports and compatible witnesses.
    #[must_use]
    pub const fn retained_round_report_bytes_estimate(self) -> usize {
        self.retained_round_report_bytes_estimate
    }
}

/// Complete deterministic bootstrap aggregation result.
#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapAggregationResult {
    initial_aggregation: FactorAggregation,
    final_aggregation: FactorAggregation,
    accepted: bool,
    stop_reason: BootstrapAggregationStopReason,
    rounds: Vec<BootstrapAggregationRound>,
    split_repair: Option<AggregationRepairResult>,
    work: BootstrapAggregationWorkReport,
}

impl BootstrapAggregationResult {
    /// Initial matching before compatible witnesses were added.
    #[must_use]
    pub const fn initial_aggregation(&self) -> &FactorAggregation {
        &self.initial_aggregation
    }

    /// Final aggregation returned by bootstrap and optional split repair.
    #[must_use]
    pub const fn final_aggregation(&self) -> &FactorAggregation {
        &self.final_aggregation
    }

    /// Whether the final aggregation passed all declared compatible and structural gates.
    #[must_use]
    pub const fn accepted(&self) -> bool {
        self.accepted
    }

    /// Deterministic stop reason.
    #[must_use]
    pub const fn stop_reason(&self) -> &BootstrapAggregationStopReason {
        &self.stop_reason
    }

    /// Matching and compatible-relaxation rounds.
    #[must_use]
    pub fn rounds(&self) -> &[BootstrapAggregationRound] {
        &self.rounds
    }

    /// Optional monotone repair result.
    #[must_use]
    pub const fn split_repair(&self) -> Option<&AggregationRepairResult> {
        self.split_repair.as_ref()
    }

    /// Deterministic structural-work and retained-state report.
    #[must_use]
    pub const fn work_report(&self) -> BootstrapAggregationWorkReport {
        self.work
    }
}

/// Build and screen one hard factor-respecting aggregation.
pub fn build_bootstrap_aggregation<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    screen_smoother: &P,
    options: BootstrapAggregationOptions,
) -> Result<BootstrapAggregationResult, MultiwayError> {
    let options = options.validate()?;
    if screen_smoother.dimension() != problem.dimension() {
        return Err(crate::error::dimension(
            "build_bootstrap_aggregation screen smoother",
            problem.dimension(),
            screen_smoother.dimension(),
        ));
    }

    let mut test_vectors = relaxed_range_test_vectors(problem, options)?;
    let initial_vector_count = test_vectors.len();
    let mut rounds = Vec::with_capacity(options.maximum_bootstrap_witnesses + 1);
    let mut previous: Option<FactorAggregation> = None;
    let mut initial_aggregation: Option<FactorAggregation> = None;
    let mut final_aggregation: Option<FactorAggregation> = None;
    let mut stop_reason = BootstrapAggregationStopReason::WitnessBudget {
        maximum: options.maximum_bootstrap_witnesses,
    };
    let mut accepted = false;
    let mut total_generated = 0_usize;
    let mut total_retained = 0_usize;

    for round_index in 0..=options.maximum_bootstrap_witnesses {
        let matching = build_matching(problem, &test_vectors, options)?;
        total_generated = total_generated.saturating_add(matching.generated);
        total_retained = total_retained.saturating_add(matching.retained);
        if initial_aggregation.is_none() {
            initial_aggregation = Some(matching.aggregation.clone());
        }
        let matching_changed = previous
            .as_ref()
            .is_none_or(|prior| prior != &matching.aggregation);
        let structural = structural_metrics(problem, &matching.aggregation)?;
        if let Some(reason) = structural_rejection(problem, structural, options) {
            final_aggregation = Some(matching.aggregation);
            stop_reason = reason;
            break;
        }
        if structural.coarse_dimension >= problem.dimension() {
            final_aggregation = Some(matching.aggregation);
            stop_reason = BootstrapAggregationStopReason::NoCompatibleComplement;
            break;
        }

        let compatible_report = analyze_compatible_relaxation(
            problem,
            &matching.aggregation,
            screen_smoother,
            options.compatible_relaxation,
        )?;
        let compatible_decision =
            evaluate_compatible_relaxation(&compatible_report, options.compatible_criteria)?;
        let decision_accepted = compatible_decision.accepted();
        rounds.push(BootstrapAggregationRound {
            index: round_index,
            bootstrap_witnesses: test_vectors.len() - initial_vector_count,
            candidate_pairs_generated: matching.generated,
            candidate_pairs_retained: matching.retained,
            maximum_retained_candidate_degree: matching.maximum_degree,
            matching_changed,
            structural_metrics: structural,
            compatible_report,
            compatible_decision,
        });
        final_aggregation = Some(matching.aggregation.clone());
        if decision_accepted {
            accepted = true;
            stop_reason = if round_index == 0 {
                BootstrapAggregationStopReason::AcceptedInitial
            } else {
                BootstrapAggregationStopReason::AcceptedAfterBootstrap {
                    witnesses: round_index,
                }
            };
            break;
        }
        if round_index == options.maximum_bootstrap_witnesses {
            stop_reason = BootstrapAggregationStopReason::WitnessBudget {
                maximum: options.maximum_bootstrap_witnesses,
            };
            break;
        }
        if !matching_changed && round_index > 0 {
            stop_reason = BootstrapAggregationStopReason::MatchingStagnated {
                witnesses: round_index,
            };
            break;
        }

        let report = &rounds
            .last()
            .expect("current compatible-relaxation round was just appended")
            .compatible_report;
        let slowest_index =
            report
                .slowest_vector_index()
                .ok_or_else(|| MultiwayError::CompatibleRelaxation {
                    message: "bootstrap compatible report contained no witness".to_owned(),
                })?;
        let mut witness = report.vectors()[slowest_index].final_error().to_vec();
        range_filter_and_normalize(problem, &mut witness)?;
        orient_deterministically(&mut witness);
        test_vectors.push(witness);
        previous = Some(matching.aggregation);
    }

    let mut final_aggregation =
        final_aggregation.ok_or_else(|| MultiwayError::CompatibleRelaxation {
            message: "bootstrap builder produced no aggregation".to_owned(),
        })?;
    let mut split_repair = None;
    if !accepted {
        if let Some(repair_options) = options.split_repair {
            let repair = repair_aggregation_by_splitting(
                problem,
                &final_aggregation,
                screen_smoother,
                repair_options,
            )?;
            if repair.accepted() {
                final_aggregation = repair.final_aggregation().clone();
                accepted = true;
                stop_reason = BootstrapAggregationStopReason::AcceptedAfterSplitRepair {
                    witnesses: test_vectors.len() - initial_vector_count,
                    splits: repair.accepted_splits(),
                };
            } else {
                stop_reason = BootstrapAggregationStopReason::SplitRepairRejected;
            }
            split_repair = Some(repair);
        }
    }

    let retained_test_vector_bytes = test_vectors
        .iter()
        .map(|vector| {
            vector
                .capacity()
                .saturating_mul(core::mem::size_of::<f64>())
        })
        .sum();
    let retained_round_report_bytes_estimate = rounds
        .iter()
        .map(|round| round.compatible_report.retained_bytes_estimate())
        .sum();
    let work = BootstrapAggregationWorkReport {
        setup_gramian_applications: options
            .setup_test_vectors
            .saturating_mul(options.setup_sweeps.saturating_add(1)),
        setup_smoother_applications: options
            .setup_test_vectors
            .saturating_mul(options.setup_sweeps),
        compatible_gramian_applications: rounds
            .iter()
            .map(|round| round.compatible_report.gramian_applications())
            .sum(),
        compatible_smoother_applications: rounds
            .iter()
            .map(|round| round.compatible_report.smoother_applications())
            .sum(),
        candidate_pairs_generated: total_generated,
        candidate_pairs_retained: total_retained,
        retained_test_vector_bytes,
        retained_round_report_bytes_estimate,
    };

    Ok(BootstrapAggregationResult {
        initial_aggregation: initial_aggregation.expect("first matching is always retained"),
        final_aggregation,
        accepted,
        stop_reason,
        rounds,
        split_repair,
        work,
    })
}

#[derive(Debug)]
struct MatchingResult {
    aggregation: FactorAggregation,
    generated: usize,
    retained: usize,
    maximum_degree: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct CandidateAccumulator {
    structural_overlap: f64,
    signature_hits: usize,
}

#[derive(Debug, Clone, Copy)]
struct ScoredCandidate {
    factor: usize,
    left: u32,
    right: u32,
    score: f64,
}

fn build_matching(
    problem: &ThreeWayProblem,
    test_vectors: &[Vec<f64>],
    options: BootstrapAggregationOptions,
) -> Result<MatchingResult, MultiwayError> {
    let mut candidates: BTreeMap<(usize, u32, u32), CandidateAccumulator> = BTreeMap::new();
    add_structural_candidates(problem, options.maximum_neighbor_degree, &mut candidates);
    add_signature_candidates(
        problem,
        test_vectors,
        options.signature_window,
        &mut candidates,
    )?;
    let generated = candidates.len();
    let weight_sum = options.algebraic_affinity_weight
        + options.structural_affinity_weight
        + options.degree_affinity_weight
        + options.signature_hit_weight;
    let offsets = problem.topology().offsets();
    let mut scored = Vec::with_capacity(generated);
    for ((factor, left, right), candidate) in candidates {
        let left_index = offsets[factor] + left as usize;
        let right_index = offsets[factor] + right as usize;
        let left_degree = problem.diagonal()[left_index];
        let right_degree = problem.diagonal()[right_index];
        let algebraic = algebraic_affinity(test_vectors, left_index, right_index);
        let structural = (candidate.structural_overlap
            / (left_degree.sqrt() * right_degree.sqrt()))
        .clamp(0.0, 1.0);
        let degree = left_degree.min(right_degree) / left_degree.max(right_degree);
        let signature = candidate.signature_hits as f64 / test_vectors.len() as f64;
        let score = (options.algebraic_affinity_weight * algebraic
            + options.structural_affinity_weight * structural
            + options.degree_affinity_weight * degree
            + options.signature_hit_weight * signature)
            / weight_sum;
        if score >= options.minimum_combined_affinity {
            scored.push(ScoredCandidate {
                factor,
                left,
                right,
                score,
            });
        }
    }
    scored.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.factor.cmp(&right.factor))
            .then_with(|| left.left.cmp(&right.left))
            .then_with(|| left.right.cmp(&right.right))
    });

    let counts = problem.topology().level_counts();
    let mut candidate_degrees: [Vec<usize>; 3] =
        core::array::from_fn(|factor| vec![0; counts[factor]]);
    let mut retained_candidates = Vec::with_capacity(scored.len());
    for candidate in scored {
        let left = candidate.left as usize;
        let right = candidate.right as usize;
        if candidate_degrees[candidate.factor][left] >= options.maximum_candidate_degree
            || candidate_degrees[candidate.factor][right] >= options.maximum_candidate_degree
        {
            continue;
        }
        candidate_degrees[candidate.factor][left] += 1;
        candidate_degrees[candidate.factor][right] += 1;
        retained_candidates.push(candidate);
    }
    let maximum_degree = candidate_degrees
        .iter()
        .flat_map(|degrees| degrees.iter().copied())
        .max()
        .unwrap_or(0);

    let mut mates: [Vec<Option<usize>>; 3] =
        core::array::from_fn(|factor| vec![None; counts[factor]]);
    for candidate in &retained_candidates {
        let left = candidate.left as usize;
        let right = candidate.right as usize;
        if mates[candidate.factor][left].is_none() && mates[candidate.factor][right].is_none() {
            mates[candidate.factor][left] = Some(right);
            mates[candidate.factor][right] = Some(left);
        }
    }
    let parents = core::array::from_fn(|factor| canonical_parents(&mates[factor]));
    Ok(MatchingResult {
        aggregation: FactorAggregation::new(counts, parents)?,
        generated,
        retained: retained_candidates.len(),
        maximum_degree,
    })
}

fn add_structural_candidates(
    problem: &ThreeWayProblem,
    maximum_neighbor_degree: usize,
    candidates: &mut BTreeMap<(usize, u32, u32), CandidateAccumulator>,
) {
    let counts = problem.topology().level_counts();
    for factor in 0..3 {
        for neighbor_factor in 0..3 {
            if neighbor_factor == factor {
                continue;
            }
            let mut neighborhoods: Vec<BTreeMap<u32, f64>> = (0..counts[neighbor_factor])
                .map(|_| BTreeMap::new())
                .collect();
            for (&tuple, &weight) in problem.topology().tuples().iter().zip(problem.weights()) {
                *neighborhoods[tuple[neighbor_factor] as usize]
                    .entry(tuple[factor])
                    .or_insert(0.0) += weight;
            }
            for neighborhood in neighborhoods {
                let mut entries: Vec<(u32, f64)> = neighborhood.into_iter().collect();
                entries.sort_by(|left, right| {
                    right
                        .1
                        .total_cmp(&left.1)
                        .then_with(|| left.0.cmp(&right.0))
                });
                entries.truncate(maximum_neighbor_degree);
                entries.sort_by_key(|entry| entry.0);
                for left in 0..entries.len() {
                    for right in (left + 1)..entries.len() {
                        let a = entries[left].0;
                        let b = entries[right].0;
                        if problem.components().component_of(factor, a as usize)
                            != problem.components().component_of(factor, b as usize)
                        {
                            continue;
                        }
                        let key = (factor, a.min(b), a.max(b));
                        candidates.entry(key).or_default().structural_overlap +=
                            entries[left].1.min(entries[right].1);
                    }
                }
            }
        }
    }
}

fn add_signature_candidates(
    problem: &ThreeWayProblem,
    test_vectors: &[Vec<f64>],
    signature_window: usize,
    candidates: &mut BTreeMap<(usize, u32, u32), CandidateAccumulator>,
) -> Result<(), MultiwayError> {
    let counts = problem.topology().level_counts();
    let offsets = problem.topology().offsets();
    for (vector_index, vector) in test_vectors.iter().enumerate() {
        if vector.len() != problem.dimension() {
            return Err(crate::error::dimension(
                "add_signature_candidates test vector",
                problem.dimension(),
                vector.len(),
            ));
        }
        for factor in 0..3 {
            let mut components: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            for level in 0..counts[factor] {
                components
                    .entry(problem.components().component_of(factor, level))
                    .or_default()
                    .push(level);
            }
            for levels in components.values_mut() {
                levels.sort_by(|&left, &right| {
                    vector[offsets[factor] + left]
                        .total_cmp(&vector[offsets[factor] + right])
                        .then_with(|| left.cmp(&right))
                });
                for position in 0..levels.len() {
                    let end = (position + signature_window + 1).min(levels.len());
                    for neighbor in (position + 1)..end {
                        let left = levels[position] as u32;
                        let right = levels[neighbor] as u32;
                        let entry = candidates
                            .entry((factor, left.min(right), left.max(right)))
                            .or_default();
                        entry.signature_hits = entry.signature_hits.saturating_add(1);
                    }
                }
            }
        }
        debug_assert!(vector_index < test_vectors.len());
    }
    Ok(())
}

fn algebraic_affinity(test_vectors: &[Vec<f64>], left: usize, right: usize) -> f64 {
    let mut difference = 0.0;
    let mut scale = 0.0;
    for vector in test_vectors {
        let left_value = vector[left];
        let right_value = vector[right];
        difference = (left_value - right_value).mul_add(left_value - right_value, difference);
        scale = left_value.mul_add(left_value, right_value.mul_add(right_value, scale));
    }
    if scale <= f64::MIN_POSITIVE {
        0.0
    } else {
        1.0 / (1.0 + difference / scale)
    }
}

fn canonical_parents(mates: &[Option<usize>]) -> Vec<u32> {
    let mut parents = vec![u32::MAX; mates.len()];
    let mut next_parent = 0_u32;
    for level in 0..mates.len() {
        if parents[level] != u32::MAX {
            continue;
        }
        parents[level] = next_parent;
        if let Some(other) = mates[level] {
            parents[other] = next_parent;
        }
        next_parent += 1;
    }
    parents
}

fn relaxed_range_test_vectors(
    problem: &ThreeWayProblem,
    options: BootstrapAggregationOptions,
) -> Result<Vec<Vec<f64>>, MultiwayError> {
    let smoother = DiagonalPreconditioner::new(problem, options.setup_jacobi_omega)?;
    let mut vectors = Vec::with_capacity(options.setup_test_vectors);
    for vector_index in 0..options.setup_test_vectors {
        let mut random = vec![0.0; problem.dimension()];
        fill_deterministic(
            &mut random,
            options.seed ^ (vector_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15),
        );
        let mut vector = vec![0.0; problem.dimension()];
        problem.apply_gramian(&random, &mut vector)?;
        normalize_range_vector(problem, &mut vector)?;
        let mut gradient = vec![0.0; problem.dimension()];
        let mut correction = vec![0.0; problem.dimension()];
        for _ in 0..options.setup_sweeps {
            problem.apply_gramian(&vector, &mut gradient)?;
            smoother.apply(&gradient, &mut correction)?;
            for (value, &step) in vector.iter_mut().zip(&correction) {
                *value -= step;
            }
            normalize_range_vector(problem, &mut vector)?;
        }
        orient_deterministically(&mut vector);
        vectors.push(vector);
    }
    Ok(vectors)
}

fn range_filter_and_normalize(
    problem: &ThreeWayProblem,
    vector: &mut Vec<f64>,
) -> Result<(), MultiwayError> {
    let mut filtered = vec![0.0; problem.dimension()];
    problem.apply_gramian(vector, &mut filtered)?;
    normalize_range_vector(problem, &mut filtered)?;
    *vector = filtered;
    Ok(())
}

fn normalize_range_vector(
    problem: &ThreeWayProblem,
    vector: &mut [f64],
) -> Result<(), MultiwayError> {
    problem.components().project_structural_range(vector)?;
    let norm = diagonal_norm(problem, vector);
    if !norm.is_finite() || norm <= f64::MIN_POSITIVE {
        return Err(MultiwayError::CompatibleRelaxation {
            message: format!("unable to normalize range test vector with D norm {norm}"),
        });
    }
    for value in vector {
        *value /= norm;
    }
    Ok(())
}

fn diagonal_norm(problem: &ThreeWayProblem, vector: &[f64]) -> f64 {
    vector
        .iter()
        .zip(problem.diagonal())
        .fold(0.0, |norm, (&value, &degree)| {
            norm.hypot(value.abs() * degree.sqrt())
        })
}

fn orient_deterministically(vector: &mut [f64]) {
    if let Some(&first) = vector.iter().find(|value| value.abs() > 1.0e-15) {
        if first < 0.0 {
            for value in vector {
                *value = -*value;
            }
        }
    }
}

fn structural_metrics(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<BootstrapStructuralMetrics, MultiwayError> {
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension: usize = aggregation.coarse_counts().iter().sum();
    let tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    Ok(BootstrapStructuralMetrics {
        coarse_dimension,
        coarse_tuple_count: coarse.tuple_count(),
        coarse_dimension_ratio: coarse_dimension as f64 / problem.dimension() as f64,
        tuple_reduction: 1.0 - tuple_ratio,
        two_level_tuple_complexity: 1.0 + tuple_ratio,
    })
}

fn structural_rejection(
    problem: &ThreeWayProblem,
    metrics: BootstrapStructuralMetrics,
    options: BootstrapAggregationOptions,
) -> Option<BootstrapAggregationStopReason> {
    let maximum_dimension =
        (options.maximum_coarse_dimension_ratio * problem.dimension() as f64).floor() as usize;
    if metrics.coarse_dimension > maximum_dimension {
        return Some(BootstrapAggregationStopReason::CoarseDimensionBudget {
            observed: metrics.coarse_dimension,
            maximum: maximum_dimension,
        });
    }
    if metrics.tuple_reduction < options.minimum_tuple_reduction {
        return Some(BootstrapAggregationStopReason::TupleReductionBudget {
            observed: metrics.tuple_reduction,
            minimum: options.minimum_tuple_reduction,
        });
    }
    if metrics.two_level_tuple_complexity > options.maximum_two_level_tuple_complexity {
        return Some(BootstrapAggregationStopReason::TupleComplexityBudget {
            observed: metrics.two_level_tuple_complexity,
            maximum: options.maximum_two_level_tuple_complexity,
        });
    }
    None
}

fn fill_deterministic(values: &mut [f64], mut state: u64) {
    for value in values {
        state = splitmix64(state);
        let unit = (state >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64));
        *value = 2.0 * unit - 1.0;
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
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
