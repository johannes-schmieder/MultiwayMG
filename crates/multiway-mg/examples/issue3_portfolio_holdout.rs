//! Frozen second-holdout evaluation of the two-stage issue #3 portfolio.

#[path = "support/issue2_fixtures.rs"]
mod issue2_fixtures;
#[path = "support/issue3_fixtures.rs"]
mod issue3_fixtures;

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use issue2_fixtures::{DynError, deterministic_rhs};
use issue3_fixtures::{Issue3Fixture, portfolio_holdout_fixtures};
use multiway_mg::{
    AggregationRepairOptions, BootstrapAcceptanceScreen, BootstrapAggregationOptions,
    CompatibleRelaxationCriteria, CompatibleRelaxationOptions, DenseRangeDecomposition,
    DiagonalPreconditioner, FactorAggregation, PairNeighborhoodAggregationOptions, PcgTraceOptions,
    ScreenedBootstrapAggregationResult, SpectralAnalysisOptions, SymmetricMapPreconditioner,
    SymmetricTwoGridPreconditioner, ThreeWayProblem, analyze_compatible_relaxation,
    build_pair_neighborhood_aggregation, build_screened_bootstrap_aggregation,
    evaluate_compatible_relaxation, solve_projected_pcg_traced,
};

const MAXIMUM_COARSE_DIMENSION_RATIO: f64 = 0.80;
const MINIMUM_TUPLE_REDUCTION: f64 = 0.05;
const MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY: f64 = 1.95;

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let mut matrix = writer(&output_directory.join("issue3-portfolio-holdout.tsv"))?;
    let mut traces = writer(&output_directory.join("issue3-portfolio-traces.tsv"))?;
    write_headers(&mut matrix, &mut traces)?;

    for fixture in portfolio_holdout_fixtures()? {
        run_fixture(&fixture, &mut matrix, &mut traces)?;
    }
    println!(
        "wrote {} and {}",
        output_directory
            .join("issue3-portfolio-holdout.tsv")
            .display(),
        output_directory
            .join("issue3-portfolio-traces.tsv")
            .display(),
    );
    Ok(())
}

fn write_headers(
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    writeln!(
        matrix,
        "set\tcase\tfamily\trequested_seed\tactual_seed\tstructural_skips\tmethod\taccepted\tstructural_admissible\tacceptance_screen\tselected_source\texact_oracle_partition\toracle_pair_recall\twrong_pair_count\tfine_dimension\tfine_tuples\tcoarse_dimension\tcoarse_tuples\tcoarse_dimension_ratio\ttuple_reduction\ttwo_level_tuple_complexity\tprimary_compatible_diagonal_factor\tprimary_compatible_energy_factor\tsecondary_compatible_diagonal_factor\tsecondary_compatible_energy_factor\tbaseline_map_condition\toracle_two_grid_condition\tcandidate_condition\toracle_improvement_recovered\tpcg_iterations\tpcg_converged\tpcg_final_relative_residual\tpcg_gramian_applications\tpcg_preconditioner_applications\tbootstrap_rounds\tbootstrap_witnesses\trepair_splits\tcandidate_pairs_generated\tcandidate_pairs_retained\tretained_test_vector_bytes\tretained_primary_report_bytes_estimate\tsecondary_candidates_considered\tsecondary_gramian_applications\tsecondary_smoother_applications\tretained_secondary_report_bytes_estimate\tstop_reason"
    )?;
    writeln!(
        traces,
        "set\tcase\tfamily\tmethod\titeration\trelative_true_residual"
    )?;
    Ok(())
}

fn run_fixture(
    fixture: &Issue3Fixture,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let problem = &fixture.problem;
    let primary_screen = DiagonalPreconditioner::new(problem, 0.5)?;
    let secondary_screen = SymmetricMapPreconditioner::new(problem.clone());
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(problem, spectral_options)?;
    let baseline_condition = range
        .analyze(&secondary_screen, spectral_options)?
        .preconditioned_condition_number();
    trace_baseline(fixture, &secondary_screen, traces)?;

    let oracle_evaluation = evaluate_map(
        fixture,
        "oracle",
        &fixture.oracle,
        true,
        "oracle-reference",
        "oracle",
        &primary_screen,
        &secondary_screen,
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
        .ok_or("oracle map was unexpectedly structurally inadmissible")?;

    let one_shot = build_pair_neighborhood_aggregation(
        problem,
        PairNeighborhoodAggregationOptions {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: 12,
        },
    )?;
    let one_shot_acceptance =
        map_acceptance(problem, &one_shot, &primary_screen, &secondary_screen)?;
    evaluate_map(
        fixture,
        "one-shot-pair-neighborhood",
        &one_shot,
        one_shot_acceptance.accepted,
        one_shot_acceptance.acceptance_screen,
        "protected-structural-baseline",
        &primary_screen,
        &secondary_screen,
        &range,
        spectral_options,
        baseline_condition,
        Some(oracle_condition),
        WorkFields::default(),
        matrix,
        traces,
    )?;

    let portfolio = build_screened_bootstrap_aggregation(
        problem,
        &primary_screen,
        &secondary_screen,
        bootstrap_options(),
        secondary_criteria(),
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
    let primary_acceptance = map_acceptance(
        problem,
        primary.final_aggregation(),
        &primary_screen,
        &secondary_screen,
    )?;
    evaluate_map(
        fixture,
        "primary-bootstrap-final",
        primary.final_aggregation(),
        primary.accepted(),
        primary_acceptance.acceptance_screen,
        primary_selected_source(primary),
        &primary_screen,
        &secondary_screen,
        &range,
        spectral_options,
        baseline_condition,
        Some(oracle_condition),
        WorkFields {
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
        },
        matrix,
        traces,
    )?;

    let secondary_work = portfolio.secondary_work_report();
    let portfolio_acceptance = map_acceptance(
        problem,
        portfolio.final_aggregation(),
        &primary_screen,
        &secondary_screen,
    )?;
    let portfolio_evaluation = evaluate_map(
        fixture,
        "portfolio-final",
        portfolio.final_aggregation(),
        portfolio.accepted(),
        portfolio_acceptance.acceptance_screen,
        portfolio_selected_source(&portfolio),
        &primary_screen,
        &secondary_screen,
        &range,
        spectral_options,
        baseline_condition,
        Some(oracle_condition),
        WorkFields {
            bootstrap_rounds: primary.rounds().len(),
            bootstrap_witnesses: primary_witnesses,
            repair_splits,
            candidate_pairs_generated: primary_work.candidate_pairs_generated(),
            candidate_pairs_retained: primary_work.candidate_pairs_retained(),
            retained_test_vector_bytes: primary_work.retained_test_vector_bytes(),
            retained_primary_report_bytes_estimate: primary_work
                .retained_round_report_bytes_estimate(),
            secondary_candidates_considered: secondary_work.candidate_maps_considered(),
            secondary_gramian_applications: secondary_work.compatible_gramian_applications(),
            secondary_smoother_applications: secondary_work.compatible_smoother_applications(),
            retained_secondary_report_bytes_estimate: secondary_work
                .retained_report_bytes_estimate(),
            stop_reason: format!(
                "screen={:?}; primary={:?}",
                portfolio.acceptance_screen(),
                primary.stop_reason(),
            ),
        },
        matrix,
        traces,
    )?;
    if portfolio.accepted() != portfolio_evaluation.accepted {
        return Err(format!(
            "portfolio acceptance mismatch on {}: builder={} evaluation={}",
            fixture.name,
            portfolio.accepted(),
            portfolio_evaluation.accepted
        )
        .into());
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
struct WorkFields {
    bootstrap_rounds: usize,
    bootstrap_witnesses: usize,
    repair_splits: usize,
    candidate_pairs_generated: usize,
    candidate_pairs_retained: usize,
    retained_test_vector_bytes: usize,
    retained_primary_report_bytes_estimate: usize,
    secondary_candidates_considered: usize,
    secondary_gramian_applications: usize,
    secondary_smoother_applications: usize,
    retained_secondary_report_bytes_estimate: usize,
    stop_reason: String,
}

#[derive(Debug, Clone, Copy)]
struct MapAcceptance {
    structural_admissible: bool,
    accepted: bool,
    acceptance_screen: &'static str,
    primary_diagonal_factor: Option<f64>,
    primary_energy_factor: Option<f64>,
    secondary_diagonal_factor: Option<f64>,
    secondary_energy_factor: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct MapEvaluation {
    accepted: bool,
    condition: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
fn evaluate_map(
    fixture: &Issue3Fixture,
    method: &str,
    aggregation: &FactorAggregation,
    requested_acceptance: bool,
    acceptance_screen: &str,
    selected_source: &str,
    primary_screen: &DiagonalPreconditioner,
    secondary_screen: &SymmetricMapPreconditioner,
    range: &DenseRangeDecomposition,
    spectral_options: SpectralAnalysisOptions,
    baseline_condition: f64,
    oracle_condition: Option<f64>,
    work: WorkFields,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<MapEvaluation, DynError> {
    let problem = &fixture.problem;
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension: usize = aggregation.coarse_counts().iter().sum();
    let coarse_dimension_ratio = coarse_dimension as f64 / problem.dimension() as f64;
    let tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    let tuple_reduction = 1.0 - tuple_ratio;
    let two_level_tuple_complexity = 1.0 + tuple_ratio;
    let acceptance = map_acceptance(problem, aggregation, primary_screen, secondary_screen)?;
    let accepted = requested_acceptance && acceptance.accepted;
    let partition = partition_metrics(&fixture.oracle, aggregation);

    let (condition, pcg) = if acceptance.structural_admissible {
        let cycle = SymmetricTwoGridPreconditioner::build(
            problem.clone(),
            aggregation.clone(),
            SymmetricMapPreconditioner::new(problem.clone()),
            1,
            1.0,
            1.0e-12,
        )?;
        let condition = range
            .analyze(&cycle, spectral_options)?
            .preconditioned_condition_number();
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
        (Some(condition), Some(pcg))
    } else {
        (None, None)
    };
    let recovery = oracle_condition.and_then(|oracle| {
        condition.and_then(|candidate| recovery_fraction(baseline_condition, oracle, candidate))
    });

    let row = vec![
        fixture.set.to_owned(),
        fixture.name.clone(),
        fixture.family.to_owned(),
        fixture.requested_seed.to_string(),
        fixture.actual_seed.to_string(),
        fixture.structural_skips.to_string(),
        method.to_owned(),
        accepted.to_string(),
        acceptance.structural_admissible.to_string(),
        acceptance_screen.to_owned(),
        selected_source.to_owned(),
        partition.exact.to_string(),
        format!("{:.12e}", partition.oracle_pair_recall),
        partition.wrong_pair_count.to_string(),
        problem.dimension().to_string(),
        problem.tuple_count().to_string(),
        coarse_dimension.to_string(),
        coarse.tuple_count().to_string(),
        format!("{coarse_dimension_ratio:.12e}"),
        format!("{tuple_reduction:.12e}"),
        format!("{two_level_tuple_complexity:.12e}"),
        optional(acceptance.primary_diagonal_factor),
        optional(acceptance.primary_energy_factor),
        optional(acceptance.secondary_diagonal_factor),
        optional(acceptance.secondary_energy_factor),
        format!("{baseline_condition:.12e}"),
        optional(oracle_condition),
        optional(condition),
        optional(recovery),
        optional_usize(pcg.as_ref().map(|result| result.iterations())),
        optional_bool(pcg.as_ref().map(|result| result.converged())),
        optional(pcg.as_ref().map(|result| result.final_relative_residual())),
        optional_usize(pcg.as_ref().map(|result| result.gramian_applications())),
        optional_usize(
            pcg.as_ref()
                .map(|result| result.preconditioner_applications()),
        ),
        work.bootstrap_rounds.to_string(),
        work.bootstrap_witnesses.to_string(),
        work.repair_splits.to_string(),
        work.candidate_pairs_generated.to_string(),
        work.candidate_pairs_retained.to_string(),
        work.retained_test_vector_bytes.to_string(),
        work.retained_primary_report_bytes_estimate.to_string(),
        work.secondary_candidates_considered.to_string(),
        work.secondary_gramian_applications.to_string(),
        work.secondary_smoother_applications.to_string(),
        work.retained_secondary_report_bytes_estimate.to_string(),
        work.stop_reason.replace(['\t', '\n'], " "),
    ];
    writeln!(matrix, "{}", row.join("\t"))?;
    Ok(MapEvaluation {
        accepted,
        condition,
    })
}

fn trace_baseline(
    fixture: &Issue3Fixture,
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

fn map_acceptance(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    primary_screen: &DiagonalPreconditioner,
    secondary_screen: &SymmetricMapPreconditioner,
) -> Result<MapAcceptance, DynError> {
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension: usize = aggregation.coarse_counts().iter().sum();
    let dimension_ratio = coarse_dimension as f64 / problem.dimension() as f64;
    let tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    let tuple_reduction = 1.0 - tuple_ratio;
    let complexity = 1.0 + tuple_ratio;
    let structural_admissible = coarse_dimension < problem.dimension()
        && dimension_ratio <= MAXIMUM_COARSE_DIMENSION_RATIO
        && tuple_reduction >= MINIMUM_TUPLE_REDUCTION
        && complexity <= MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY;
    if !structural_admissible {
        return Ok(MapAcceptance {
            structural_admissible,
            accepted: false,
            acceptance_screen: "structural-rejected",
            primary_diagonal_factor: None,
            primary_energy_factor: None,
            secondary_diagonal_factor: None,
            secondary_energy_factor: None,
        });
    }

    let primary_report =
        analyze_compatible_relaxation(problem, aggregation, primary_screen, compatible_options())?;
    let primary_decision = evaluate_compatible_relaxation(&primary_report, primary_criteria())?;
    if primary_decision.accepted() {
        return Ok(MapAcceptance {
            structural_admissible,
            accepted: true,
            acceptance_screen: "weighted-jacobi",
            primary_diagonal_factor: Some(primary_decision.maximum_diagonal_factor_per_sweep()),
            primary_energy_factor: primary_decision.maximum_energy_factor_per_sweep(),
            secondary_diagonal_factor: None,
            secondary_energy_factor: None,
        });
    }

    let secondary_report = analyze_compatible_relaxation(
        problem,
        aggregation,
        secondary_screen,
        compatible_options(),
    )?;
    let secondary_decision =
        evaluate_compatible_relaxation(&secondary_report, secondary_criteria())?;
    Ok(MapAcceptance {
        structural_admissible,
        accepted: secondary_decision.accepted(),
        acceptance_screen: if secondary_decision.accepted() {
            "symmetric-map"
        } else {
            "rejected"
        },
        primary_diagonal_factor: Some(primary_decision.maximum_diagonal_factor_per_sweep()),
        primary_energy_factor: primary_decision.maximum_energy_factor_per_sweep(),
        secondary_diagonal_factor: Some(secondary_decision.maximum_diagonal_factor_per_sweep()),
        secondary_energy_factor: secondary_decision.maximum_energy_factor_per_sweep(),
    })
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
        compatible_relaxation: compatible_options(),
        compatible_criteria: primary_criteria(),
        maximum_bootstrap_witnesses: 6,
        maximum_coarse_dimension_ratio: MAXIMUM_COARSE_DIMENSION_RATIO,
        minimum_tuple_reduction: MINIMUM_TUPLE_REDUCTION,
        maximum_two_level_tuple_complexity: MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
        split_repair: Some(repair_options()),
        seed: 0x4d57_4d47_434f_5645,
    }
}

fn repair_options() -> AggregationRepairOptions {
    AggregationRepairOptions {
        relaxation: compatible_options(),
        criteria: primary_criteria(),
        maximum_rounds: 18,
        maximum_coarse_dimension_ratio: MAXIMUM_COARSE_DIMENSION_RATIO,
        minimum_tuple_reduction: MINIMUM_TUPLE_REDUCTION,
        maximum_two_level_tuple_complexity: MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
        minimum_split_score_fraction: 0.001,
    }
}

fn compatible_options() -> CompatibleRelaxationOptions {
    CompatibleRelaxationOptions {
        test_vectors: 16,
        sweeps: 12,
        relaxation_damping: 1.0,
        seed: 0x4d57_4d47_4352_3031,
        relative_zero_tolerance: 1.0e-13,
    }
}

fn primary_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.85,
        maximum_energy_factor_per_sweep: Some(0.85),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    }
}

fn secondary_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.85,
        maximum_energy_factor_per_sweep: Some(0.85),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    }
}

fn primary_selected_source(primary: &multiway_mg::BootstrapAggregationResult) -> &'static str {
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

fn portfolio_selected_source(portfolio: &ScreenedBootstrapAggregationResult) -> &'static str {
    match portfolio.acceptance_screen() {
        BootstrapAcceptanceScreen::Primary => primary_selected_source(portfolio.primary_result()),
        BootstrapAcceptanceScreen::SecondaryBootstrapFinal => "secondary-bootstrap-final",
        BootstrapAcceptanceScreen::SecondaryStructuralBaseline => {
            "secondary-protected-structural-baseline"
        }
        BootstrapAcceptanceScreen::Rejected => "rejected-primary-final",
        _ => "rejected-unknown-screen",
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
