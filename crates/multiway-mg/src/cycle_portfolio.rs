//! Complete-cycle screening and selection for bootstrap aggregation candidates.
//!
//! Compatible relaxation remains the conservative witness generator used by
//! bootstrap matching and monotone repair. Final map acceptance, however, is
//! based on the actual fixed cycle that a caller intends to use. This avoids
//! rejecting a compact map merely because smoother-only compatible contraction
//! is pessimistic even though coarse correction and smoothing compose into a
//! strong complete cycle.

use std::time::{Duration, Instant};

use crate::{
    BootstrapAggregationBuildTiming, BootstrapAggregationOptions, BootstrapAggregationResult,
    CycleQualityCriteria, CycleQualityDecision, CycleQualityOptions, CycleQualityReport,
    FactorAggregation, MultiwayError, PairNeighborhoodAggregationOptions, Preconditioner,
    ThreeWayProblem, analyze_cycle_quality, build_bootstrap_aggregation_with_timing,
    build_pair_neighborhood_aggregation, evaluate_cycle_quality,
};

/// Candidate source in the complete-cycle portfolio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum CyclePortfolioCandidateSource {
    /// Final map returned by bootstrap and optional monotone repair.
    BootstrapFinal,
    /// Protected bounded pair-neighborhood structural baseline.
    StructuralBaseline,
}

/// Why a candidate could not enter complete-cycle probing.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CyclePortfolioStructuralRejection {
    /// The map leaves no nontrivial reduction in coefficient dimension.
    NoCoarseReduction,
    /// Coarse coefficient dimension exceeded its declared budget.
    CoarseDimension {
        /// Observed coarse coefficient dimension.
        observed: usize,
        /// Largest admitted coarse coefficient dimension.
        maximum: usize,
    },
    /// Unique tuple reduction was too small.
    TupleReduction {
        /// Observed fractional tuple reduction.
        observed: f64,
        /// Minimum admitted reduction.
        minimum: f64,
    },
    /// Two-level tuple complexity exceeded its declared budget.
    TupleComplexity {
        /// Observed two-level tuple complexity.
        observed: f64,
        /// Maximum admitted complexity.
        maximum: f64,
    },
}

/// Structural measurements for one candidate map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CyclePortfolioStructuralMetrics {
    coarse_dimension: usize,
    coarse_tuple_count: usize,
    coarse_dimension_ratio: f64,
    tuple_reduction: f64,
    two_level_tuple_complexity: f64,
}

impl CyclePortfolioStructuralMetrics {
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

    /// `(fine tuples + coarse tuples) / fine tuples`.
    #[must_use]
    pub const fn two_level_tuple_complexity(self) -> f64 {
        self.two_level_tuple_complexity
    }
}

/// Complete-cycle evaluation of one distinct candidate map.
#[derive(Debug, Clone, PartialEq)]
pub struct CyclePortfolioEvaluation {
    source: CyclePortfolioCandidateSource,
    aggregation: FactorAggregation,
    structural_metrics: CyclePortfolioStructuralMetrics,
    structural_rejection: Option<CyclePortfolioStructuralRejection>,
    cycle_build_error: Option<String>,
    cycle_report: Option<CycleQualityReport>,
    cycle_decision: Option<CycleQualityDecision>,
}

impl CyclePortfolioEvaluation {
    /// Candidate source.
    #[must_use]
    pub const fn source(&self) -> CyclePortfolioCandidateSource {
        self.source
    }

    /// Candidate hard factor map.
    #[must_use]
    pub const fn aggregation(&self) -> &FactorAggregation {
        &self.aggregation
    }

    /// Structural metrics measured before constructing a cycle.
    #[must_use]
    pub const fn structural_metrics(&self) -> CyclePortfolioStructuralMetrics {
        self.structural_metrics
    }

    /// Structural rejection, when present.
    #[must_use]
    pub const fn structural_rejection(&self) -> Option<&CyclePortfolioStructuralRejection> {
        self.structural_rejection.as_ref()
    }

    /// Cycle construction failure, retained as a fail-closed candidate result.
    #[must_use]
    pub fn cycle_build_error(&self) -> Option<&str> {
        self.cycle_build_error.as_deref()
    }

    /// Matrix-free complete-cycle probe for an eligible, constructible map.
    #[must_use]
    pub const fn cycle_report(&self) -> Option<&CycleQualityReport> {
        self.cycle_report.as_ref()
    }

    /// Explicit complete-cycle acceptance decision.
    #[must_use]
    pub const fn cycle_decision(&self) -> Option<&CycleQualityDecision> {
        self.cycle_decision.as_ref()
    }

    /// Whether every structural and complete-cycle gate passed.
    #[must_use]
    pub fn accepted(&self) -> bool {
        self.structural_rejection.is_none()
            && self.cycle_build_error.is_none()
            && self
                .cycle_decision
                .as_ref()
                .is_some_and(CycleQualityDecision::accepted)
    }
}

/// Additional deterministic work charged to candidate cycle screening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CyclePortfolioWorkReport {
    candidate_maps_considered: usize,
    cycle_builds_attempted: usize,
    cycle_build_failures: usize,
    probe_gramian_applications: usize,
    probe_preconditioner_applications: usize,
    probe_energy_evaluations: usize,
    retained_probe_bytes_estimate: usize,
}

impl CyclePortfolioWorkReport {
    /// Distinct maps considered.
    #[must_use]
    pub const fn candidate_maps_considered(self) -> usize {
        self.candidate_maps_considered
    }

    /// Eligible candidate cycles whose construction was attempted.
    #[must_use]
    pub const fn cycle_builds_attempted(self) -> usize {
        self.cycle_builds_attempted
    }

    /// Candidate cycle construction failures.
    #[must_use]
    pub const fn cycle_build_failures(self) -> usize {
        self.cycle_build_failures
    }

    /// Gramian applications in complete-cycle probes.
    #[must_use]
    pub const fn probe_gramian_applications(self) -> usize {
        self.probe_gramian_applications
    }

    /// Complete preconditioner applications in probes.
    #[must_use]
    pub const fn probe_preconditioner_applications(self) -> usize {
        self.probe_preconditioner_applications
    }

    /// Direct energy evaluations in probes.
    #[must_use]
    pub const fn probe_energy_evaluations(self) -> usize {
        self.probe_energy_evaluations
    }

    /// Principal retained bytes in complete-cycle reports and final witnesses.
    #[must_use]
    pub const fn retained_probe_bytes_estimate(self) -> usize {
        self.retained_probe_bytes_estimate
    }
}

/// Phase-separated descriptive timing for complete-cycle portfolio construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CyclePortfolioBuildTiming {
    primary: BootstrapAggregationBuildTiming,
    cycle_screen: Duration,
    total: Duration,
}

impl CyclePortfolioBuildTiming {
    /// Primary bootstrap and repair timing.
    #[must_use]
    pub const fn primary(self) -> BootstrapAggregationBuildTiming {
        self.primary
    }

    /// Candidate construction and complete-cycle probing time.
    #[must_use]
    pub const fn cycle_screen(self) -> Duration {
        self.cycle_screen
    }

    /// Complete portfolio build time.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

/// Final cycle-screened automatic aggregation result.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleScreenedBootstrapResult {
    primary: BootstrapAggregationResult,
    final_aggregation: FactorAggregation,
    accepted: bool,
    selected_source: Option<CyclePortfolioCandidateSource>,
    selected_evaluation_index: Option<usize>,
    evaluations: Vec<CyclePortfolioEvaluation>,
    work: CyclePortfolioWorkReport,
}

impl CycleScreenedBootstrapResult {
    /// Full conservative bootstrap and repair result.
    #[must_use]
    pub const fn primary_result(&self) -> &BootstrapAggregationResult {
        &self.primary
    }

    /// Final selected map, or the primary final map when every candidate failed.
    #[must_use]
    pub const fn final_aggregation(&self) -> &FactorAggregation {
        &self.final_aggregation
    }

    /// Whether one candidate passed every declared structural and cycle gate.
    #[must_use]
    pub const fn accepted(&self) -> bool {
        self.accepted
    }

    /// Source of the selected accepted map.
    #[must_use]
    pub const fn selected_source(&self) -> Option<CyclePortfolioCandidateSource> {
        self.selected_source
    }

    /// Selected evaluation when the portfolio accepted a candidate.
    #[must_use]
    pub fn selected_evaluation(&self) -> Option<&CyclePortfolioEvaluation> {
        self.selected_evaluation_index
            .map(|index| &self.evaluations[index])
    }

    /// Every distinct candidate evaluation in stable source order.
    #[must_use]
    pub fn evaluations(&self) -> &[CyclePortfolioEvaluation] {
        &self.evaluations
    }

    /// Deterministic probe work report.
    #[must_use]
    pub const fn work_report(&self) -> CyclePortfolioWorkReport {
        self.work
    }
}

/// Build a conservative bootstrap map and select among it and the protected
/// structural baseline using the caller's actual complete-cycle constructor.
pub fn build_cycle_screened_bootstrap_aggregation<P, C, F>(
    problem: &ThreeWayProblem,
    primary_smoother: &P,
    bootstrap_options: BootstrapAggregationOptions,
    probe_options: CycleQualityOptions,
    probe_criteria: CycleQualityCriteria,
    cycle_builder: F,
) -> Result<CycleScreenedBootstrapResult, MultiwayError>
where
    P: Preconditioner + ?Sized,
    C: Preconditioner,
    F: FnMut(&FactorAggregation) -> Result<C, MultiwayError>,
{
    build_cycle_screened_bootstrap_aggregation_with_timing(
        problem,
        primary_smoother,
        bootstrap_options,
        probe_options,
        probe_criteria,
        cycle_builder,
    )
    .map(|(result, _timing)| result)
}

/// Build and screen the portfolio while returning descriptive phase timing.
pub fn build_cycle_screened_bootstrap_aggregation_with_timing<P, C, F>(
    problem: &ThreeWayProblem,
    primary_smoother: &P,
    bootstrap_options: BootstrapAggregationOptions,
    probe_options: CycleQualityOptions,
    probe_criteria: CycleQualityCriteria,
    mut cycle_builder: F,
) -> Result<(CycleScreenedBootstrapResult, CyclePortfolioBuildTiming), MultiwayError>
where
    P: Preconditioner + ?Sized,
    C: Preconditioner,
    F: FnMut(&FactorAggregation) -> Result<C, MultiwayError>,
{
    let total_start = Instant::now();
    let (primary, primary_timing) =
        build_bootstrap_aggregation_with_timing(problem, primary_smoother, bootstrap_options)?;
    let structural_baseline = build_pair_neighborhood_aggregation(
        problem,
        PairNeighborhoodAggregationOptions {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: bootstrap_options.maximum_neighbor_degree,
        },
    )?;
    let mut candidates = vec![(
        CyclePortfolioCandidateSource::BootstrapFinal,
        primary.final_aggregation().clone(),
    )];
    if structural_baseline != candidates[0].1 {
        candidates.push((
            CyclePortfolioCandidateSource::StructuralBaseline,
            structural_baseline,
        ));
    }

    let cycle_start = Instant::now();
    let mut evaluations = Vec::with_capacity(candidates.len());
    let mut cycle_builds_attempted = 0_usize;
    let mut cycle_build_failures = 0_usize;
    let mut probe_gramian_applications = 0_usize;
    let mut probe_preconditioner_applications = 0_usize;
    let mut probe_energy_evaluations = 0_usize;
    let mut retained_probe_bytes_estimate = 0_usize;

    for (source, aggregation) in candidates {
        let metrics = structural_metrics(problem, &aggregation)?;
        let structural_rejection = structural_rejection(problem, metrics, bootstrap_options);
        let mut cycle_build_error = None;
        let mut cycle_report = None;
        let mut cycle_decision = None;
        if structural_rejection.is_none() {
            cycle_builds_attempted = cycle_builds_attempted.saturating_add(1);
            match cycle_builder(&aggregation) {
                Ok(cycle) => {
                    let report = analyze_cycle_quality(problem, &cycle, probe_options)?;
                    let decision = evaluate_cycle_quality(&report, probe_criteria)?;
                    probe_gramian_applications =
                        probe_gramian_applications.saturating_add(report.gramian_applications());
                    probe_preconditioner_applications = probe_preconditioner_applications
                        .saturating_add(report.preconditioner_applications());
                    probe_energy_evaluations =
                        probe_energy_evaluations.saturating_add(report.energy_evaluations());
                    retained_probe_bytes_estimate = retained_probe_bytes_estimate
                        .saturating_add(report.retained_bytes_estimate());
                    cycle_report = Some(report);
                    cycle_decision = Some(decision);
                }
                Err(error) => {
                    cycle_build_failures = cycle_build_failures.saturating_add(1);
                    cycle_build_error = Some(error.to_string());
                }
            }
        }
        evaluations.push(CyclePortfolioEvaluation {
            source,
            aggregation,
            structural_metrics: metrics,
            structural_rejection,
            cycle_build_error,
            cycle_report,
            cycle_decision,
        });
    }

    let selected_evaluation_index = evaluations
        .iter()
        .enumerate()
        .filter(|(_, evaluation)| evaluation.accepted())
        .min_by(|(_, left), (_, right)| compare_accepted_candidates(left, right))
        .map(|(index, _)| index);
    let (final_aggregation, accepted, selected_source) = selected_evaluation_index.map_or_else(
        || (primary.final_aggregation().clone(), false, None),
        |index| {
            (
                evaluations[index].aggregation.clone(),
                true,
                Some(evaluations[index].source),
            )
        },
    );
    let cycle_screen = cycle_start.elapsed();
    let candidate_maps_considered = evaluations.len();
    let result = CycleScreenedBootstrapResult {
        primary,
        final_aggregation,
        accepted,
        selected_source,
        selected_evaluation_index,
        evaluations,
        work: CyclePortfolioWorkReport {
            candidate_maps_considered,
            cycle_builds_attempted,
            cycle_build_failures,
            probe_gramian_applications,
            probe_preconditioner_applications,
            probe_energy_evaluations,
            retained_probe_bytes_estimate,
        },
    };
    Ok((
        result,
        CyclePortfolioBuildTiming {
            primary: primary_timing,
            cycle_screen,
            total: total_start.elapsed(),
        },
    ))
}

fn structural_metrics(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<CyclePortfolioStructuralMetrics, MultiwayError> {
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension: usize = aggregation.coarse_counts().iter().sum();
    let tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    Ok(CyclePortfolioStructuralMetrics {
        coarse_dimension,
        coarse_tuple_count: coarse.tuple_count(),
        coarse_dimension_ratio: coarse_dimension as f64 / problem.dimension() as f64,
        tuple_reduction: 1.0 - tuple_ratio,
        two_level_tuple_complexity: 1.0 + tuple_ratio,
    })
}

fn structural_rejection(
    problem: &ThreeWayProblem,
    metrics: CyclePortfolioStructuralMetrics,
    options: BootstrapAggregationOptions,
) -> Option<CyclePortfolioStructuralRejection> {
    if metrics.coarse_dimension >= problem.dimension() {
        return Some(CyclePortfolioStructuralRejection::NoCoarseReduction);
    }
    let maximum_dimension =
        (options.maximum_coarse_dimension_ratio * problem.dimension() as f64).floor() as usize;
    if metrics.coarse_dimension > maximum_dimension {
        return Some(CyclePortfolioStructuralRejection::CoarseDimension {
            observed: metrics.coarse_dimension,
            maximum: maximum_dimension,
        });
    }
    if metrics.tuple_reduction < options.minimum_tuple_reduction {
        return Some(CyclePortfolioStructuralRejection::TupleReduction {
            observed: metrics.tuple_reduction,
            minimum: options.minimum_tuple_reduction,
        });
    }
    if metrics.two_level_tuple_complexity > options.maximum_two_level_tuple_complexity {
        return Some(CyclePortfolioStructuralRejection::TupleComplexity {
            observed: metrics.two_level_tuple_complexity,
            maximum: options.maximum_two_level_tuple_complexity,
        });
    }
    None
}

fn compare_accepted_candidates(
    left: &CyclePortfolioEvaluation,
    right: &CyclePortfolioEvaluation,
) -> core::cmp::Ordering {
    let left_factor = left
        .cycle_report
        .as_ref()
        .expect("accepted candidate has a report")
        .maximum_estimated_energy_factor();
    let right_factor = right
        .cycle_report
        .as_ref()
        .expect("accepted candidate has a report")
        .maximum_estimated_energy_factor();
    left_factor
        .total_cmp(&right_factor)
        .then_with(|| {
            left.structural_metrics
                .coarse_tuple_count
                .cmp(&right.structural_metrics.coarse_tuple_count)
        })
        .then_with(|| {
            left.structural_metrics
                .coarse_dimension
                .cmp(&right.structural_metrics.coarse_dimension)
        })
        .then_with(|| left.source.cmp(&right.source))
}
