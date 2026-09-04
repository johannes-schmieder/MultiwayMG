//! Frozen v2 holdout for complete-cycle automatic aggregation screening.

#[path = "support/issue2_fixtures.rs"]
mod issue2_fixtures;
#[path = "support/issue3_cycle_fixtures.rs"]
mod issue3_cycle_fixtures;

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use issue2_fixtures::{DynError, deterministic_rhs};
use issue3_cycle_fixtures::{CycleHoldoutFixture, cycle_holdout_fixtures};
use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, CyclePortfolioCandidateSource, CycleQualityCriteria,
    CycleQualityDecision, CycleQualityOptions, CycleQualityReport, CycleScreenedBootstrapResult,
    DenseRangeDecomposition, DiagonalPreconditioner, FactorAggregation,
    PairNeighborhoodAggregationOptions, PcgTraceOptions, SpectralAnalysisOptions,
    SymmetricMapPreconditioner, SymmetricTwoGridPreconditioner, ThreeWayProblem,
    analyze_cycle_quality, build_cycle_screened_bootstrap_aggregation,
    build_pair_neighborhood_aggregation, evaluate_cycle_quality, solve_projected_pcg_traced,
};

const MAXIMUM_COARSE_DIMENSION_RATIO: f64 = 0.80;
const MINIMUM_TUPLE_REDUCTION: f64 = 0.05;
const MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY: f64 = 1.95;

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let mut matrix = writer(&output_directory.join("issue3-cycle-holdout.tsv"))?;
    let mut traces = writer(&output_directory.join("issue3-cycle-traces.tsv"))?;
    write_headers(&mut matrix, &mut traces)?;

    for fixture in cycle_holdout_fixtures()? {
        run_fixture(&fixture, &mut matrix, &mut traces)?;
    }
    println!(
        "wrote {} and {}",
        output_directory.join("issue3-cycle-holdout.tsv").display(),
        output_directory.join("issue3-cycle-traces.tsv").display(),
    );
    Ok(())
}

fn write_headers(
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    writeln!(
        matrix,
        "set\tcase\tfamily\trequested_seed\tactual_seed\tstructural_skips\tmethod\taccepted\tstructural_admissible\tselected_source\texact_oracle_partition\toracle_pair_recall\twrong_pair_count\tfine_dimension\tfine_tuples\tcomponents\tcoarse_dimension\tcoarse_tuples\tcoarse_dimension_ratio\ttuple_reduction\ttwo_level_tuple_complexity\tbaseline_map_condition\toracle_two_grid_condition\tcandidate_condition\texact_cycle_error_radius\tprobe_estimated_energy_factor\tprobe_underestimate\tprobe_maximum_observed_factor\tprobe_maximum_absolute_rayleigh\tprobe_maximum_structural_defect\tcycle_probe_accepted\toracle_improvement_recovered\tpcg_iterations\tpcg_converged\tpcg_final_relative_residual\tpcg_gramian_applications\tpcg_preconditioner_applications\tbootstrap_primary_accepted\tbootstrap_rounds\tbootstrap_witnesses\trepair_splits\tcandidate_pairs_generated\tcandidate_pairs_retained\tretained_test_vector_bytes\tretained_primary_report_bytes_estimate\tportfolio_candidates_considered\tportfolio_cycle_builds_attempted\tportfolio_cycle_build_failures\tportfolio_probe_gramian_applications\tportfolio_probe_preconditioner_applications\tportfolio_probe_energy_evaluations\tretained_portfolio_probe_bytes_estimate\tstop_reason"
    )?;
    writeln!(
        traces,
        "set\tcase\tfamily\tmethod\titeration\trelative_true_residual"
    )?;
    Ok(())
}

fn run_fixture(
    fixture: &CycleHoldoutFixture,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let problem = &fixture.problem;
    let primary_smoother = DiagonalPreconditioner::new(problem, 0.5)?;
    let baseline = SymmetricMapPreconditioner::new(problem.clone());
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(problem, spectral_options)?;
    let baseline_condition = range
        .analyze(&baseline, spectral_options)?
        .preconditioned_condition_number();
    trace_baseline(fixture, &baseline, traces)?;

    let oracle_evaluation = evaluate_map(
        fixture,
        "oracle",
        &fixture.oracle,
        true,
        "oracle-reference",
        &range,
        spectral_options,
        baseline_condition,
        None,
        WorkFields::default(),
        matrix,
        traces,
    )?;
    let oracle_condition = oracle_evaluation
        .condition
        .ok_or("oracle map did not produce a complete-cycle spectrum")?;

    let one_shot = build_pair_neighborhood_aggregation(
        problem,
        PairNeighborhoodAggregationOptions {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: 12,
        },
    )?;
    let one_shot_evaluation = evaluate_map(
        fixture,
        "one-shot-pair-neighborhood",
        &one_shot,
        true,
        "protected-structural-baseline",
        &range,
        spectral_options,
        baseline_condition,
        Some(oracle_condition),
        WorkFields::default(),
        matrix,
        traces,
    )?;

    let portfolio = build_cycle_screened_bootstrap_aggregation(
        problem,
        &primary_smoother,
        bootstrap_options(),
        cycle_probe_options(),
        cycle_probe_criteria(),
        |aggregation| map_cycle(problem, aggregation),
    )?;
    let primary = portfolio.primary_result();
    let primary_work = primary.work_report();
    let primary_witnesses = primary
        .rounds()
        .last()
        .map_or(0, |round| round.bootstrap_witnesses());
    let repair_splits = primary
        .split_repair()
        .map_or(0, |repair| repair.accepted_splits());
    let common_work = WorkFields {
        bootstrap_primary_accepted: primary.accepted(),
        bootstrap_rounds: primary.rounds().len(),
        bootstrap_witnesses: primary_witnesses,
        repair_splits,
        candidate_pairs_generated: primary_work.candidate_pairs_generated(),
        candidate_pairs_retained: primary_work.candidate_pairs_retained(),
        retained_test_vector_bytes: primary_work.retained_test_vector_bytes(),
        retained_primary_report_bytes_estimate: primary_work
            .retained_round_report_bytes_estimate(),
        stop_reason: format!("{:?}", primary.stop_reason()),
        ..WorkFields::default()
    };
    evaluate_map(
        fixture,
        "primary-bootstrap-final",
        primary.final_aggregation(),
        primary.accepted(),
        primary_source(primary),
        &range,
        spectral_options,
        baseline_condition,
        Some(oracle_condition),
        common_work.clone(),
        matrix,
        traces,
    )?;

    let portfolio_work = portfolio.work_report();
    let portfolio_evaluation = evaluate_map(
        fixture,
        "cycle-portfolio-final",
        portfolio.final_aggregation(),
        portfolio.accepted(),
        portfolio_source(&portfolio),
        &range,
        spectral_options,
        baseline_condition,
        Some(oracle_condition),
        WorkFields {
            portfolio_candidates_considered: portfolio_work.candidate_maps_considered(),
            portfolio_cycle_builds_attempted: portfolio_work.cycle_builds_attempted(),
            portfolio_cycle_build_failures: portfolio_work.cycle_build_failures(),
            portfolio_probe_gramian_applications: portfolio_work.probe_gramian_applications(),
            portfolio_probe_preconditioner_applications: portfolio_work
                .probe_preconditioner_applications(),
            portfolio_probe_energy_evaluations: portfolio_work.probe_energy_evaluations(),
            retained_portfolio_probe_bytes_estimate: portfolio_work
                .retained_probe_bytes_estimate(),
            stop_reason: format!(
                "selected={:?}; primary={:?}",
                portfolio.selected_source(),
                primary.stop_reason(),
            ),
            ..common_work
        },
        matrix,
        traces,
    )?;
    if portfolio.accepted() != portfolio_evaluation.accepted {
        return Err(format!(
            "portfolio acceptance mismatch on {}: builder={} independent={}",
            fixture.name,
            portfolio.accepted(),
            portfolio_evaluation.accepted
        )
        .into());
    }
    if let (Some(one_shot_condition), Some(portfolio_condition)) =
        (one_shot_evaluation.condition, portfolio_evaluation.condition)
    {
        if portfolio.accepted()
            && portfolio_condition > one_shot_condition * 1.10
            && one_shot_evaluation.accepted
        {
            return Err(format!(
                "accepted portfolio materially regressed against one-shot on {}: {} versus {}",
                fixture.name, portfolio_condition, one_shot_condition
            )
            .into());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
struct WorkFields {
    bootstrap_primary_accepted: bool,
    bootstrap_rounds: usize,
    bootstrap_witnesses: usize,
    repair_splits: usize,
    candidate_pairs_generated: usize,
    candidate_pairs_retained: usize,
    retained_test_vector_bytes: usize,
    retained_primary_report_bytes_estimate: usize,
    portfolio_candidates_considered: usize,
    portfolio_cycle_builds_attempted: usize,
    portfolio_cycle_build_failures: usize,
    portfolio_probe_gramian_applications: usize,
    portfolio_probe_preconditioner_applications: usize,
    portfolio_probe_energy_evaluations: usize,
    retained_portfolio_probe_bytes_estimate: usize,
    stop_reason: String,
}

#[derive(Debug, Clone, Copy)]
struct MapEvaluation {
    accepted: bool,
    condition: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
fn evaluate_map(
    fixture: &CycleHoldoutFixture,
    method: &str,
    aggregation: &FactorAggregation,
    requested_acceptance: bool,
    selected_source: &str,
    range: &DenseRangeDecomposition,
    spectral_options: SpectralAnalysisOptions,
    baseline_condition: f64,
    oracle_condition: Option<f64>,
    work: WorkFields,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<MapEvaluation, DynError> {
    let problem = &fixture.problem;
    let metrics = structural_metrics(problem, aggregation)?;
    let partition = partition_metrics(&fixture.oracle, aggregation);
    let structural_admissible = metrics.structural_admissible;

    let (condition, exact_radius, probe, decision, pcg) = if structural_admissible {
        let cycle = map_cycle(problem, aggregation)?;
        let spectral = range.analyze(&cycle, spectral_options)?;
        let condition = spectral.preconditioned_condition_number();
        let exact_radius = spectral
            .preconditioned_eigenvalues()
            .iter()
            .map(|&eigenvalue| (1.0 - eigenvalue).abs())
            .fold(0.0, f64::max);
        let probe = analyze_cycle_quality(problem, &cycle, cycle_probe_options())?;
        let decision = evaluate_cycle_quality(&probe, cycle_probe_criteria())?;
        let rhs = deterministic_rhs(problem)?;
        let pcg = solve_projected_pcg_traced(
            problem,
            &rhs,
            &cycle,
            PcgTraceOptions {
                relative_tolerance: 1.0e-10,
                absolute_tolerance: 0.0,
                max_iterations: 2_000,
            },
        )?;
        for sample in pcg.samples() {
            writeln!(
                traces,
                "{}\t{}\t{}\t{}\t{}\t{:.12e}",
                fixture.set,
                fixture.name,
                fixture.family,
                method,
                sample.iteration(),
                sample.relative_residual(),
            )?;
        }
        (
            Some(condition),
            Some(exact_radius),
            Some(probe),
            Some(decision),
            Some(pcg),
        )
    } else {
        (None, None, None, None, None)
    };
    let cycle_accepted = decision.as_ref().is_some_and(CycleQualityDecision::accepted);
    let accepted = requested_acceptance && structural_admissible && cycle_accepted;
    let recovery = oracle_condition.and_then(|oracle| {
        condition.and_then(|candidate| recovery_fraction(baseline_condition, oracle, candidate))
    });
    let probe_estimate = probe
        .as_ref()
        .map(CycleQualityReport::maximum_estimated_energy_factor);
    let probe_underestimate = exact_radius.zip(probe_estimate).map(|(exact, estimate)| exact - estimate);

    let row = vec![
        fixture.set.to_owned(),
        fixture.name.clone(),
        fixture.family.to_owned(),
        fixture.requested_seed.to_string(),
        fixture.actual_seed.to_string(),
        fixture.structural_skips.to_string(),
        method.to_owned(),
        accepted.to_string(),
        structural_admissible.to_string(),
        selected_source.to_owned(),
        partition.exact.to_string(),
        format!("{:.12e}", partition.oracle_pair_recall),
        partition.wrong_pair_count.to_string(),
        problem.dimension().to_string(),
        problem.tuple_count().to_string(),
        problem.components().count().to_string(),
        metrics.coarse_dimension.to_string(),
        metrics.coarse_tuple_count.to_string(),
        format!("{:.12e}", metrics.coarse_dimension_ratio),
        format!("{:.12e}", metrics.tuple_reduction),
        format!("{:.12e}", metrics.two_level_tuple_complexity),
        format!("{baseline_condition:.12e}"),
        optional(oracle_condition),
        optional(condition),
        optional(exact_radius),
        optional(probe_estimate),
        optional(probe_underestimate),
        optional(
            probe
                .as_ref()
                .map(CycleQualityReport::maximum_observed_energy_factor),
        ),
        optional(
            probe
                .as_ref()
                .map(CycleQualityReport::maximum_absolute_final_rayleigh),
        ),
        optional(
            probe
                .as_ref()
                .map(CycleQualityReport::maximum_structural_defect),
        ),
        cycle_accepted.to_string(),
        optional(recovery),
        optional_usize(pcg.as_ref().map(|result| result.iterations())),
        optional_bool(pcg.as_ref().map(|result| result.converged())),
        optional(pcg.as_ref().map(|result| result.final_relative_residual())),
        optional_usize(pcg.as_ref().map(|result| result.gramian_applications())),
        optional_usize(
            pcg.as_ref()
                .map(|result| result.preconditioner_applications()),
        ),
        work.bootstrap_primary_accepted.to_string(),
        work.bootstrap_rounds.to_string(),
        work.bootstrap_witnesses.to_string(),
        work.repair_splits.to_string(),
        work.candidate_pairs_generated.to_string(),
        work.candidate_pairs_retained.to_string(),
        work.retained_test_vector_bytes.to_string(),
        work.retained_primary_report_bytes_estimate.to_string(),
        work.portfolio_candidates_considered.to_string(),
        work.portfolio_cycle_builds_attempted.to_string(),
        work.portfolio_cycle_build_failures.to_string(),
        work.portfolio_probe_gramian_applications.to_string(),
        work.portfolio_probe_preconditioner_applications.to_string(),
        work.portfolio_probe_energy_evaluations.to_string(),
        work.retained_portfolio_probe_bytes_estimate.to_string(),
        work.stop_reason.replace(['\t', '\n'], " "),
    ];
    writeln!(matrix, "{}", row.join("\t"))?;
    Ok(MapEvaluation { accepted, condition })
}

#[derive(Debug, Clone, Copy)]
struct StructuralMetrics {
    structural_admissible: bool,
    coarse_dimension: usize,
    coarse_tuple_count: usize,
    coarse_dimension_ratio: f64,
    tuple_reduction: f64,
    two_level_tuple_complexity: f64,
}

fn structural_metrics(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<StructuralMetrics, DynError> {
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension: usize = aggregation.coarse_counts().iter().sum();
    let coarse_dimension_ratio = coarse_dimension as f64 / problem.dimension() as f64;
    let tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    let tuple_reduction = 1.0 - tuple_ratio;
    let two_level_tuple_complexity = 1.0 + tuple_ratio;
    Ok(StructuralMetrics {
        structural_admissible: coarse_dimension < problem.dimension()
            && coarse_dimension_ratio <= MAXIMUM_COARSE_DIMENSION_RATIO
            && tuple_reduction >= MINIMUM_TUPLE_REDUCTION
            && two_level_tuple_complexity <= MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
        coarse_dimension,
        coarse_tuple_count: coarse.tuple_count(),
        coarse_dimension_ratio,
        tuple_reduction,
        two_level_tuple_complexity,
    })
}

fn map_cycle(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<SymmetricTwoGridPreconditioner<SymmetricMapPreconditioner>, multiway_mg::MultiwayError>
{
    SymmetricTwoGridPreconditioner::build(
        problem.clone(),
        aggregation.clone(),
        SymmetricMapPreconditioner::new(problem.clone()),
        1,
        1.0,
        1.0e-12,
    )
}

fn trace_baseline(
    fixture: &CycleHoldoutFixture,
    baseline: &SymmetricMapPreconditioner,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let rhs = deterministic_rhs(&fixture.problem)?;
    let pcg = solve_projected_pcg_traced(
        &fixture.problem,
        &rhs,
        baseline,
        PcgTraceOptions {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 0.0,
            max_iterations: 2_000,
        },
    )?;
    for sample in pcg.samples() {
        writeln!(
            traces,
            "{}\t{}\t{}\tbaseline-symmetric-map\t{}\t{:.12e}",
            fixture.set,
            fixture.name,
            fixture.family,
            sample.iteration(),
            sample.relative_residual(),
        )?;
    }
    Ok(())
}

fn bootstrap_options() -> BootstrapAggregationOptions {
    BootstrapAggregationOptions {
        setup_test_vectors: 5,
        setup_sweeps: 5,
        setup_jacobi_omega: 0.5,
        maximum_neighbor_degree: 12,
        signature_window: 3,
        maximum_candidate_degree: 12,
        minimum_combined_affinity: 0.40,
        algebraic_affinity_weight: 0.75,
        structural_affinity_weight: 0.05,
        degree_affinity_weight: 0.10,
        signature_hit_weight: 0.10,
        structural_baseline_required_factor_ratio: 0.90,
        structural_baseline_maximum_dimension_overhead_ratio: 0.05,
        structural_baseline_maximum_tuple_overhead_ratio: 0.05,
        compatible_relaxation: CompatibleRelaxationOptions {
            test_vectors: 16,
            sweeps: 12,
            relaxation_damping: 1.0,
            seed: 0x4d57_4d47_4352_3031,
            relative_zero_tolerance: 1.0e-13,
        },
        compatible_criteria: CompatibleRelaxationCriteria {
            maximum_diagonal_factor_per_sweep: 0.85,
            maximum_energy_factor_per_sweep: Some(0.85),
            maximum_final_coarse_defect: 1.0e-10,
            maximum_final_structural_defect: 1.0e-10,
        },
        maximum_bootstrap_witnesses: 6,
        maximum_coarse_dimension_ratio: MAXIMUM_COARSE_DIMENSION_RATIO,
        minimum_tuple_reduction: MINIMUM_TUPLE_REDUCTION,
        maximum_two_level_tuple_complexity: MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
        split_repair: Some(AggregationRepairOptions {
            relaxation: CompatibleRelaxationOptions {
                test_vectors: 16,
                sweeps: 12,
                relaxation_damping: 1.0,
                seed: 0x4d57_4d47_4352_3031,
                relative_zero_tolerance: 1.0e-13,
            },
            criteria: CompatibleRelaxationCriteria {
                maximum_diagonal_factor_per_sweep: 0.85,
                maximum_energy_factor_per_sweep: Some(0.85),
                maximum_final_coarse_defect: 1.0e-10,
                maximum_final_structural_defect: 1.0e-10,
            },
            maximum_rounds: 18,
            maximum_coarse_dimension_ratio: MAXIMUM_COARSE_DIMENSION_RATIO,
            minimum_tuple_reduction: MINIMUM_TUPLE_REDUCTION,
            maximum_two_level_tuple_complexity: MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
            minimum_split_score_fraction: 0.001,
        }),
        seed: 0x4d57_4d47_434f_5645,
    }
}

fn cycle_probe_options() -> CycleQualityOptions {
    CycleQualityOptions {
        test_vectors: 12,
        power_iterations: 24,
        tail_iterations: 6,
        correction_damping: 1.0,
        seed: 0x4d57_4d47_4359_4331,
        relative_zero_tolerance: 1.0e-13,
    }
}

fn cycle_probe_criteria() -> CycleQualityCriteria {
    CycleQualityCriteria {
        maximum_estimated_energy_factor: 0.50,
        maximum_observed_energy_factor: Some(1.05),
        maximum_structural_defect: 1.0e-10,
    }
}

fn primary_source(primary: &multiway_mg::BootstrapAggregationResult) -> &'static str {
    if primary.structural_baseline_selected() {
        "protected-structural-baseline"
    } else if primary
        .split_repair()
        .is_some_and(|repair| repair.accepted())
    {
        "witness-split-repair"
    } else if primary.rounds().len() > 1 {
        "bootstrap-witness-rematching"
    } else {
        "relaxed-signature-initial"
    }
}

fn portfolio_source(portfolio: &CycleScreenedBootstrapResult) -> &'static str {
    match portfolio.selected_source() {
        Some(CyclePortfolioCandidateSource::BootstrapFinal) => "cycle-screened-bootstrap-final",
        Some(CyclePortfolioCandidateSource::StructuralBaseline) => {
            "cycle-screened-structural-baseline"
        }
        None => "cycle-screened-rejected",
        Some(_) => "cycle-screened-unknown",
    }
}

#[derive(Debug, Clone, Copy)]
struct PartitionMetrics {
    exact: bool,
    oracle_pair_recall: f64,
    wrong_pair_count: usize,
}

fn partition_metrics(
    oracle: &FactorAggregation,
    candidate: &FactorAggregation,
) -> PartitionMetrics {
    let counts = oracle.fine_counts();
    let mut oracle_pairs = 0_usize;
    let mut recovered_pairs = 0_usize;
    let mut wrong_pairs = 0_usize;
    for factor in 0..3 {
        for left in 0..counts[factor] {
            for right in (left + 1)..counts[factor] {
                let oracle_same = oracle.parents(factor)[left] == oracle.parents(factor)[right];
                let candidate_same =
                    candidate.parents(factor)[left] == candidate.parents(factor)[right];
                if oracle_same {
                    oracle_pairs += 1;
                    recovered_pairs += usize::from(candidate_same);
                } else {
                    wrong_pairs += usize::from(candidate_same);
                }
            }
        }
    }
    PartitionMetrics {
        exact: recovered_pairs == oracle_pairs && wrong_pairs == 0,
        oracle_pair_recall: if oracle_pairs == 0 {
            1.0
        } else {
            recovered_pairs as f64 / oracle_pairs as f64
        },
        wrong_pair_count: wrong_pairs,
    }
}

fn recovery_fraction(baseline: f64, oracle: f64, candidate: f64) -> Option<f64> {
    let denominator = baseline - oracle;
    (denominator > 1.0e-12 * baseline.abs().max(1.0))
        .then_some((baseline - candidate) / denominator)
}

fn optional(value: Option<f64>) -> String {
    value.map_or_else(|| "NA".to_owned(), |number| format!("{number:.12e}"))
}

fn optional_usize(value: Option<usize>) -> String {
    value.map_or_else(|| "NA".to_owned(), |number| number.to_string())
}

fn optional_bool(value: Option<bool>) -> String {
    value.map_or_else(|| "NA".to_owned(), |flag| flag.to_string())
}

fn writer(path: &Path) -> Result<BufWriter<File>, DynError> {
    Ok(BufWriter::new(File::create(path)?))
}
