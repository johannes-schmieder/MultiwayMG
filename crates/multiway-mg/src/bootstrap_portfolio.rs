//! Two-stage acceptance portfolio for bootstrap aggregation.
//!
//! Bootstrap matching and witness repair use a deliberately conservative
//! primary smoother, typically weighted Jacobi. A structurally admissible map
//! can nevertheless be highly effective with the smoother intended for the
//! final cycle, such as symmetric MAP. This module keeps those questions
//! separate: the primary builder remains the witness authority, while a
//! secondary screen may rescue only an already-constructed, structurally valid
//! candidate. Dimension and tuple budgets are never relaxed.

use std::time::{Duration, Instant};

use crate::{
    BootstrapAggregationBuildTiming, BootstrapAggregationOptions, BootstrapAggregationResult,
    CompatibleRelaxationCriteria, CompatibleRelaxationDecision, CompatibleRelaxationReport,
    FactorAggregation, MultiwayError, PairNeighborhoodAggregationOptions, Preconditioner,
    ThreeWayProblem, analyze_compatible_relaxation, build_bootstrap_aggregation_with_timing,
    build_pair_neighborhood_aggregation, evaluate_compatible_relaxation,
};

/// Candidate evaluated by the secondary compatible-relaxation screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum SecondaryScreenCandidateSource {
    /// Final map returned by the primary bootstrap/repair procedure.
    BootstrapFinal,
    /// Protected bounded pair-neighborhood map.
    StructuralBaseline,
}

/// How the final portfolio map was accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BootstrapAcceptanceScreen {
    /// The ordinary conservative bootstrap path accepted the map.
    Primary,
    /// The secondary smoother accepted the primary bootstrap final map.
    SecondaryBootstrapFinal,
    /// The secondary smoother accepted the protected structural baseline.
    SecondaryStructuralBaseline,
    /// No structurally admissible candidate passed either screen.
    Rejected,
}

/// Why a secondary candidate failed a structural gate before relaxation.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SecondaryScreenStructuralRejection {
    /// The map spans the complete fine coefficient space.
    NoCompatibleComplement,
    /// Coarse coefficient dimension exceeded the declared budget.
    CoarseDimension {
        /// Observed coarse coefficient dimension.
        observed: usize,
        /// Maximum admitted coarse coefficient dimension.
        maximum: usize,
    },
    /// Unique coarse tuples did not contract enough.
    TupleReduction {
        /// Observed fractional reduction.
        observed: f64,
        /// Minimum required reduction.
        minimum: f64,
    },
    /// Two-level tuple complexity exceeded the declared budget.
    TupleComplexity {
        /// Observed two-level tuple complexity.
        observed: f64,
        /// Maximum admitted complexity.
        maximum: f64,
    },
}

/// Structural diagnostics for one secondary-screen candidate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SecondaryScreenStructuralMetrics {
    coarse_dimension: usize,
    coarse_tuple_count: usize,
    coarse_dimension_ratio: f64,
    tuple_reduction: f64,
    two_level_tuple_complexity: f64,
}

impl SecondaryScreenStructuralMetrics {
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

/// Result of screening one candidate under the production-smoother proxy.
#[derive(Debug, Clone, PartialEq)]
pub struct SecondaryScreenEvaluation {
    source: SecondaryScreenCandidateSource,
    structural_metrics: SecondaryScreenStructuralMetrics,
    structural_rejection: Option<SecondaryScreenStructuralRejection>,
    compatible_report: Option<CompatibleRelaxationReport>,
    compatible_decision: Option<CompatibleRelaxationDecision>,
}

impl SecondaryScreenEvaluation {
    /// Candidate source.
    #[must_use]
    pub const fn source(&self) -> SecondaryScreenCandidateSource {
        self.source
    }

    /// Structural metrics measured before secondary relaxation.
    #[must_use]
    pub const fn structural_metrics(&self) -> SecondaryScreenStructuralMetrics {
        self.structural_metrics
    }

    /// Structural rejection, when the candidate was not eligible for screening.
    #[must_use]
    pub const fn structural_rejection(&self) -> Option<&SecondaryScreenStructuralRejection> {
        self.structural_rejection.as_ref()
    }

    /// Secondary compatible-relaxation report for an eligible candidate.
    #[must_use]
    pub const fn compatible_report(&self) -> Option<&CompatibleRelaxationReport> {
        self.compatible_report.as_ref()
    }

    /// Secondary acceptance decision for an eligible candidate.
    #[must_use]
    pub const fn compatible_decision(&self) -> Option<&CompatibleRelaxationDecision> {
        self.compatible_decision.as_ref()
    }

    /// Whether the candidate passed structural and secondary-compatible gates.
    #[must_use]
    pub fn accepted(&self) -> bool {
        self.structural_rejection.is_none()
            && self
                .compatible_decision
                .as_ref()
                .is_some_and(CompatibleRelaxationDecision::accepted)
    }
}

/// Additional deterministic work charged to the secondary portfolio screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondaryScreenWorkReport {
    candidate_maps_considered: usize,
    compatible_gramian_applications: usize,
    compatible_smoother_applications: usize,
    retained_report_bytes_estimate: usize,
}

impl SecondaryScreenWorkReport {
    /// Distinct candidate maps considered.
    #[must_use]
    pub const fn candidate_maps_considered(self) -> usize {
        self.candidate_maps_considered
    }

    /// Additional Gramian applications.
    #[must_use]
    pub const fn compatible_gramian_applications(self) -> usize {
        self.compatible_gramian_applications
    }

    /// Additional secondary-smoother applications.
    #[must_use]
    pub const fn compatible_smoother_applications(self) -> usize {
        self.compatible_smoother_applications
    }

    /// Principal retained bytes in secondary reports and witnesses.
    #[must_use]
    pub const fn retained_report_bytes_estimate(self) -> usize {
        self.retained_report_bytes_estimate
    }
}

/// Phase-separated timing for the complete two-stage builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenedBootstrapBuildTiming {
    primary: BootstrapAggregationBuildTiming,
    secondary_screen: Duration,
    total: Duration,
}

impl ScreenedBootstrapBuildTiming {
    /// Primary bootstrap, repair, and protected-baseline timing.
    #[must_use]
    pub const fn primary(self) -> BootstrapAggregationBuildTiming {
        self.primary
    }

    /// Secondary candidate construction and compatible screening.
    #[must_use]
    pub const fn secondary_screen(self) -> Duration {
        self.secondary_screen
    }

    /// Complete portfolio construction time.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

/// Complete two-stage bootstrap selection.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenedBootstrapAggregationResult {
    primary: BootstrapAggregationResult,
    final_aggregation: FactorAggregation,
    accepted: bool,
    acceptance_screen: BootstrapAcceptanceScreen,
    evaluations: Vec<SecondaryScreenEvaluation>,
    selected_evaluation_index: Option<usize>,
    secondary_work: SecondaryScreenWorkReport,
}

impl ScreenedBootstrapAggregationResult {
    /// Full primary bootstrap result.
    #[must_use]
    pub const fn primary_result(&self) -> &BootstrapAggregationResult {
        &self.primary
    }

    /// Final map selected by the complete portfolio.
    #[must_use]
    pub const fn final_aggregation(&self) -> &FactorAggregation {
        &self.final_aggregation
    }

    /// Whether the final map passed a declared acceptance screen and every
    /// structural budget.
    #[must_use]
    pub const fn accepted(&self) -> bool {
        self.accepted
    }

    /// Screen that accepted the final map.
    #[must_use]
    pub const fn acceptance_screen(&self) -> BootstrapAcceptanceScreen {
        self.acceptance_screen
    }

    /// All distinct secondary candidate evaluations.
    #[must_use]
    pub fn secondary_evaluations(&self) -> &[SecondaryScreenEvaluation] {
        &self.evaluations
    }

    /// Selected secondary evaluation when the primary path was rescued.
    #[must_use]
    pub fn selected_secondary_evaluation(&self) -> Option<&SecondaryScreenEvaluation> {
        self.selected_evaluation_index
            .map(|index| &self.evaluations[index])
    }

    /// Additional deterministic work charged to the secondary screen.
    #[must_use]
    pub const fn secondary_work_report(&self) -> SecondaryScreenWorkReport {
        self.secondary_work
    }
}

/// Build bootstrap aggregation and, only after primary rejection, screen
/// structurally admissible candidates with a second fixed smoother.
pub fn build_screened_bootstrap_aggregation<P, S>(
    problem: &ThreeWayProblem,
    primary_smoother: &P,
    secondary_smoother: &S,
    options: BootstrapAggregationOptions,
    secondary_criteria: CompatibleRelaxationCriteria,
) -> Result<ScreenedBootstrapAggregationResult, MultiwayError>
where
    P: Preconditioner + ?Sized,
    S: Preconditioner + ?Sized,
{
    build_screened_bootstrap_aggregation_with_timing(
        problem,
        primary_smoother,
        secondary_smoother,
        options,
        secondary_criteria,
    )
    .map(|(result, _timing)| result)
}

/// Build the two-stage portfolio and return descriptive phase timing.
pub fn build_screened_bootstrap_aggregation_with_timing<P, S>(
    problem: &ThreeWayProblem,
    primary_smoother: &P,
    secondary_smoother: &S,
    options: BootstrapAggregationOptions,
    secondary_criteria: CompatibleRelaxationCriteria,
) -> Result<
    (
        ScreenedBootstrapAggregationResult,
        ScreenedBootstrapBuildTiming,
    ),
    MultiwayError,
>
where
    P: Preconditioner + ?Sized,
    S: Preconditioner + ?Sized,
{
    let total_start = Instant::now();
    if secondary_smoother.dimension() != problem.dimension() {
        return Err(crate::error::dimension(
            "build_screened_bootstrap_aggregation secondary smoother",
            problem.dimension(),
            secondary_smoother.dimension(),
        ));
    }
    let (primary, primary_timing) =
        build_bootstrap_aggregation_with_timing(problem, primary_smoother, options)?;
    if primary.accepted() {
        let result = ScreenedBootstrapAggregationResult {
            final_aggregation: primary.final_aggregation().clone(),
            primary,
            accepted: true,
            acceptance_screen: BootstrapAcceptanceScreen::Primary,
            evaluations: Vec::new(),
            selected_evaluation_index: None,
            secondary_work: SecondaryScreenWorkReport {
                candidate_maps_considered: 0,
                compatible_gramian_applications: 0,
                compatible_smoother_applications: 0,
                retained_report_bytes_estimate: 0,
            },
        };
        return Ok((
            result,
            ScreenedBootstrapBuildTiming {
                primary: primary_timing,
                secondary_screen: Duration::ZERO,
                total: total_start.elapsed(),
            },
        ));
    }

    let secondary_start = Instant::now();
    let structural_baseline = build_pair_neighborhood_aggregation(
        problem,
        PairNeighborhoodAggregationOptions {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: options.maximum_neighbor_degree,
        },
    )?;
    let mut candidates = vec![(
        SecondaryScreenCandidateSource::BootstrapFinal,
        primary.final_aggregation().clone(),
    )];
    if structural_baseline != candidates[0].1 {
        candidates.push((
            SecondaryScreenCandidateSource::StructuralBaseline,
            structural_baseline,
        ));
    }

    let mut evaluations = Vec::with_capacity(candidates.len());
    let mut candidate_maps = Vec::with_capacity(candidates.len());
    let mut compatible_gramian_applications = 0_usize;
    let mut compatible_smoother_applications = 0_usize;
    let mut retained_report_bytes_estimate = 0_usize;

    for (source, aggregation) in candidates {
        let metrics = structural_metrics(problem, &aggregation)?;
        let structural_rejection = structural_rejection(problem, metrics, options);
        let (compatible_report, compatible_decision) = if structural_rejection.is_none() {
            let report = analyze_compatible_relaxation(
                problem,
                &aggregation,
                secondary_smoother,
                options.compatible_relaxation,
            )?;
            let decision = evaluate_compatible_relaxation(&report, secondary_criteria)?;
            compatible_gramian_applications = compatible_gramian_applications
                .saturating_add(report.gramian_applications());
            compatible_smoother_applications = compatible_smoother_applications
                .saturating_add(report.smoother_applications());
            retained_report_bytes_estimate = retained_report_bytes_estimate
                .saturating_add(report.retained_bytes_estimate());
            (Some(report), Some(decision))
        } else {
            (None, None)
        };
        candidate_maps.push(aggregation);
        evaluations.push(SecondaryScreenEvaluation {
            source,
            structural_metrics: metrics,
            structural_rejection,
            compatible_report,
            compatible_decision,
        });
    }

    let selected_evaluation_index = evaluations
        .iter()
        .enumerate()
        .filter(|(_, evaluation)| evaluation.accepted())
        .min_by(|(_, left), (_, right)| compare_accepted_candidates(left, right))
        .map(|(index, _)| index);
    let (final_aggregation, accepted, acceptance_screen) =
        if let Some(index) = selected_evaluation_index {
            let screen = match evaluations[index].source {
                SecondaryScreenCandidateSource::BootstrapFinal => {
                    BootstrapAcceptanceScreen::SecondaryBootstrapFinal
                }
                SecondaryScreenCandidateSource::StructuralBaseline => {
                    BootstrapAcceptanceScreen::SecondaryStructuralBaseline
                }
            };
            (candidate_maps[index].clone(), true, screen)
        } else {
            (
                primary.final_aggregation().clone(),
                false,
                BootstrapAcceptanceScreen::Rejected,
            )
        };
    let secondary_screen = secondary_start.elapsed();
    let result = ScreenedBootstrapAggregationResult {
        primary,
        final_aggregation,
        accepted,
        acceptance_screen,
        evaluations,
        selected_evaluation_index,
        secondary_work: SecondaryScreenWorkReport {
            candidate_maps_considered: candidate_maps.len(),
            compatible_gramian_applications,
            compatible_smoother_applications,
            retained_report_bytes_estimate,
        },
    };
    Ok((
        result,
        ScreenedBootstrapBuildTiming {
            primary: primary_timing,
            secondary_screen,
            total: total_start.elapsed(),
        },
    ))
}

fn structural_metrics(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<SecondaryScreenStructuralMetrics, MultiwayError> {
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension: usize = aggregation.coarse_counts().iter().sum();
    let tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    Ok(SecondaryScreenStructuralMetrics {
        coarse_dimension,
        coarse_tuple_count: coarse.tuple_count(),
        coarse_dimension_ratio: coarse_dimension as f64 / problem.dimension() as f64,
        tuple_reduction: 1.0 - tuple_ratio,
        two_level_tuple_complexity: 1.0 + tuple_ratio,
    })
}

fn structural_rejection(
    problem: &ThreeWayProblem,
    metrics: SecondaryScreenStructuralMetrics,
    options: BootstrapAggregationOptions,
) -> Option<SecondaryScreenStructuralRejection> {
    if metrics.coarse_dimension >= problem.dimension() {
        return Some(SecondaryScreenStructuralRejection::NoCompatibleComplement);
    }
    let maximum_dimension =
        (options.maximum_coarse_dimension_ratio * problem.dimension() as f64).floor() as usize;
    if metrics.coarse_dimension > maximum_dimension {
        return Some(SecondaryScreenStructuralRejection::CoarseDimension {
            observed: metrics.coarse_dimension,
            maximum: maximum_dimension,
        });
    }
    if metrics.tuple_reduction < options.minimum_tuple_reduction {
        return Some(SecondaryScreenStructuralRejection::TupleReduction {
            observed: metrics.tuple_reduction,
            minimum: options.minimum_tuple_reduction,
        });
    }
    if metrics.two_level_tuple_complexity > options.maximum_two_level_tuple_complexity {
        return Some(SecondaryScreenStructuralRejection::TupleComplexity {
            observed: metrics.two_level_tuple_complexity,
            maximum: options.maximum_two_level_tuple_complexity,
        });
    }
    None
}

fn compare_accepted_candidates(
    left: &SecondaryScreenEvaluation,
    right: &SecondaryScreenEvaluation,
) -> core::cmp::Ordering {
    left.structural_metrics
        .coarse_tuple_count
        .cmp(&right.structural_metrics.coarse_tuple_count)
        .then_with(|| {
            left.structural_metrics
                .coarse_dimension
                .cmp(&right.structural_metrics.coarse_dimension)
        })
        .then_with(|| {
            left.compatible_decision
                .as_ref()
                .expect("accepted candidate has a decision")
                .maximum_diagonal_factor_per_sweep()
                .total_cmp(
                    &right
                        .compatible_decision
                        .as_ref()
                        .expect("accepted candidate has a decision")
                        .maximum_diagonal_factor_per_sweep(),
                )
        })
        .then_with(|| left.source.cmp(&right.source))
}
