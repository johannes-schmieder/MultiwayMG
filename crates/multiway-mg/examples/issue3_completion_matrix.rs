//! Frozen calibration and holdout evidence for issue #3.

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
use issue3_fixtures::{Issue3Fixture, calibration_fixtures, holdout_fixtures};
use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, BootstrapAggregationResult,
    CompatibleRelaxationCriteria, CompatibleRelaxationOptions, DenseRangeDecomposition,
    DiagonalPreconditioner, FactorAggregation, PairNeighborhoodAggregationOptions, PcgTraceOptions,
    SpectralAnalysisOptions, SymmetricMapPreconditioner, SymmetricTwoGridPreconditioner,
    ThreeWayProblem, analyze_compatible_relaxation, build_bootstrap_aggregation,
    build_pair_neighborhood_aggregation, evaluate_compatible_relaxation,
    solve_projected_pcg_traced,
};

const MAXIMUM_COARSE_DIMENSION_RATIO: f64 = 0.80;
const MINIMUM_TUPLE_REDUCTION: f64 = 0.02;
const MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY: f64 = 1.98;

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let mut matrix = writer(&output_directory.join("issue3-completion-matrix.tsv"))?;
    let mut traces = writer(&output_directory.join("issue3-pcg-traces.tsv"))?;
    let mut policy = writer(&output_directory.join("issue3-policy.tsv"))?;
    write_headers(&mut matrix, &mut traces, &mut policy)?;

    let mut fixtures = calibration_fixtures()?;
    fixtures.extend(holdout_fixtures()?);
    for fixture in &fixtures {
        run_fixture(fixture, &mut matrix, &mut traces)?;
    }
    println!(
        "wrote {}, {}, and {}",
        output_directory
            .join("issue3-completion-matrix.tsv")
            .display(),
        output_directory.join("issue3-pcg-traces.tsv").display(),
        output_directory.join("issue3-policy.tsv").display(),
    );
    Ok(())
}

fn write_headers(
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
    policy: &mut BufWriter<File>,
) -> Result<(), DynError> {
    writeln!(
        matrix,
        "set\tcase\tfamily\trequested_seed\tactual_seed\tstructural_skips\tmethod\taccepted\tstructural_admissible\tselected_source\tstructural_baseline_selected\texact_oracle_partition\toracle_pair_recall\twrong_pair_count\tfine_dimension\tfine_tuples\tcoarse_dimension\tcoarse_tuples\tcoarse_dimension_ratio\ttuple_reduction\ttwo_level_tuple_complexity\tcompatible_diagonal_factor\tcompatible_energy_factor\tbaseline_map_condition\toracle_two_grid_condition\tcandidate_condition\toracle_improvement_recovered\tpcg_iterations\tpcg_converged\tpcg_final_relative_residual\tpcg_gramian_applications\tpcg_preconditioner_applications\tbootstrap_rounds\tbootstrap_witnesses\trepair_splits\tcandidate_pairs_generated\tcandidate_pairs_retained\tretained_test_vector_bytes\tretained_report_bytes_estimate\tstop_reason"
    )?;
    writeln!(
        traces,
        "set\tcase\tfamily\tmethod\titeration\trelative_true_residual"
    )?;
    writeln!(policy, "name\tvalue")?;
    let options = bootstrap_options();
    for (name, value) in [
        ("setup_test_vectors", options.setup_test_vectors.to_string()),
        ("setup_sweeps", options.setup_sweeps.to_string()),
        ("setup_jacobi_omega", options.setup_jacobi_omega.to_string()),
        (
            "maximum_neighbor_degree",
            options.maximum_neighbor_degree.to_string(),
        ),
        ("signature_window", options.signature_window.to_string()),
        (
            "maximum_candidate_degree",
            options.maximum_candidate_degree.to_string(),
        ),
        (
            "minimum_combined_affinity",
            options.minimum_combined_affinity.to_string(),
        ),
        (
            "algebraic_affinity_weight",
            options.algebraic_affinity_weight.to_string(),
        ),
        (
            "structural_affinity_weight",
            options.structural_affinity_weight.to_string(),
        ),
        (
            "degree_affinity_weight",
            options.degree_affinity_weight.to_string(),
        ),
        (
            "signature_hit_weight",
            options.signature_hit_weight.to_string(),
        ),
        (
            "maximum_bootstrap_witnesses",
            options.maximum_bootstrap_witnesses.to_string(),
        ),
        (
            "maximum_coarse_dimension_ratio",
            MAXIMUM_COARSE_DIMENSION_RATIO.to_string(),
        ),
        (
            "minimum_tuple_reduction",
            MINIMUM_TUPLE_REDUCTION.to_string(),
        ),
        (
            "maximum_two_level_tuple_complexity",
            MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY.to_string(),
        ),
        (
            "compatible_test_vectors",
            options.compatible_relaxation.test_vectors.to_string(),
        ),
        (
            "compatible_sweeps",
            options.compatible_relaxation.sweeps.to_string(),
        ),
        (
            "compatible_diagonal_limit",
            options
                .compatible_criteria
                .maximum_diagonal_factor_per_sweep
                .to_string(),
        ),
        (
            "compatible_energy_limit",
            options
                .compatible_criteria
                .maximum_energy_factor_per_sweep
                .expect("completion policy requires energy")
                .to_string(),
        ),
    ] {
        writeln!(policy, "{name}\t{value}")?;
    }
    Ok(())
}

fn run_fixture(
    fixture: &Issue3Fixture,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let problem = &fixture.problem;
    let screen = DiagonalPreconditioner::new(problem, 0.5)?;
    let range_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(problem, range_options)?;
    let baseline = SymmetricMapPreconditioner::new(problem.clone());
    let baseline_report = range.analyze(&baseline, range_options)?;
    let baseline_condition = baseline_report.preconditioned_condition_number();
    let oracle_metrics = evaluate_map(
        fixture,
        "oracle",
        "oracle-reference",
        &fixture.oracle,
        true,
        WorkFields::default(),
        &screen,
        &range,
        range_options,
        baseline_condition,
        None,
        matrix,
        traces,
    )?;
    let oracle_condition = oracle_metrics.condition.ok_or_else(|| {
        format!(
            "oracle map for {} was not structurally admissible",
            fixture.name
        )
    })?;

    record_baseline(
        fixture,
        &baseline,
        baseline_condition,
        oracle_condition,
        matrix,
        traces,
    )?;

    let one_shot = build_pair_neighborhood_aggregation(
        problem,
        PairNeighborhoodAggregationOptions::default(),
    )?;
    let one_shot_accepted = map_acceptance(problem, &one_shot, &screen)?.accepted;
    evaluate_map(
        fixture,
        "one-shot-pair-neighborhood",
        "structural-baseline",
        &one_shot,
        one_shot_accepted,
        WorkFields::default(),
        &screen,
        &range,
        range_options,
        baseline_condition,
        Some(oracle_condition),
        matrix,
        traces,
    )?;

    let bootstrap = build_bootstrap_aggregation(problem, &screen, bootstrap_options())?;
    let initial_acceptance = map_acceptance(problem, bootstrap.initial_aggregation(), &screen)?;
    evaluate_map(
        fixture,
        "bootstrap-initial",
        "relaxed-signature-initial",
        bootstrap.initial_aggregation(),
        initial_acceptance.accepted,
        WorkFields::default(),
        &screen,
        &range,
        range_options,
        baseline_condition,
        Some(oracle_condition),
        matrix,
        traces,
    )?;

    let work = bootstrap.work_report();
    let bootstrap_witnesses = bootstrap
        .rounds()
        .last()
        .map_or(0, |round| round.bootstrap_witnesses());
    let repair_splits = bootstrap
        .split_repair()
        .map_or(0, |repair| repair.accepted_splits());
    let selected_source = selected_source(&bootstrap);
    evaluate_map(
        fixture,
        "bootstrap-final",
        selected_source,
        bootstrap.final_aggregation(),
        bootstrap.accepted(),
        WorkFields {
            structural_baseline_selected: bootstrap.structural_baseline_selected(),
            bootstrap_rounds: bootstrap.rounds().len(),
            bootstrap_witnesses,
            repair_splits,
            candidate_pairs_generated: work.candidate_pairs_generated(),
            candidate_pairs_retained: work.candidate_pairs_retained(),
            retained_test_vector_bytes: work.retained_test_vector_bytes(),
            retained_report_bytes_estimate: work.retained_round_report_bytes_estimate(),
            stop_reason: format!("{:?}", bootstrap.stop_reason()),
        },
        &screen,
        &range,
        range_options,
        baseline_condition,
        Some(oracle_condition),
        matrix,
        traces,
    )?;
    Ok(())
}

fn selected_source(result: &BootstrapAggregationResult) -> &'static str {
    if result.structural_baseline_selected() {
        "protected-structural-baseline"
    } else if result
        .split_repair()
        .is_some_and(|repair| repair.accepted())
    {
        "witness-split-repair"
    } else if result.rounds().len() > 1 {
        "bootstrap-witness-rematching"
    } else {
        "relaxed-signature-initial"
    }
}

#[derive(Debug, Clone, Default)]
struct WorkFields {
    structural_baseline_selected: bool,
    bootstrap_rounds: usize,
    bootstrap_witnesses: usize,
    repair_splits: usize,
    candidate_pairs_generated: usize,
    candidate_pairs_retained: usize,
    retained_test_vector_bytes: usize,
    retained_report_bytes_estimate: usize,
    stop_reason: String,
}

#[derive(Debug, Clone, Copy)]
struct MapAcceptance {
    structural_admissible: bool,
    accepted: bool,
    compatible_diagonal_factor: Option<f64>,
    compatible_energy_factor: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct EvaluatedMap {
    condition: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
fn evaluate_map(
    fixture: &Issue3Fixture,
    method: &str,
    selected_source: &str,
    aggregation: &FactorAggregation,
    requested_acceptance: bool,
    work: WorkFields,
    screen: &DiagonalPreconditioner,
    range: &DenseRangeDecomposition,
    range_options: SpectralAnalysisOptions,
    baseline_condition: f64,
    oracle_condition: Option<f64>,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<EvaluatedMap, DynError> {
    let problem = &fixture.problem;
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension: usize = aggregation.coarse_counts().iter().sum();
    let coarse_dimension_ratio = coarse_dimension as f64 / problem.dimension() as f64;
    let coarse_tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    let tuple_reduction = 1.0 - coarse_tuple_ratio;
    let two_level_tuple_complexity = 1.0 + coarse_tuple_ratio;
    let acceptance = map_acceptance(problem, aggregation, screen)?;
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
            .analyze(&cycle, range_options)?
            .preconditioned_condition_number();
        let rhs = deterministic_rhs(problem)?;
        let pcg = solve_projected_pcg_traced(problem, &rhs, &cycle, PcgTraceOptions::default())?;
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

    writeln!(
        matrix,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.12e}\t{}\t{}\t{}\t{}\t{}\t{:.12e}\t{:.12e}\t{:.12e}\t{}\t{}\t{:.12e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        fixture.set,
        fixture.name,
        fixture.family,
        fixture.requested_seed,
        fixture.actual_seed,
        fixture.structural_skips,
        method,
        accepted,
        acceptance.structural_admissible,
        selected_source,
        work.structural_baseline_selected,
        partition.exact,
        partition.oracle_pair_recall,
        partition.wrong_pair_count,
        problem.dimension(),
        problem.tuple_count(),
        coarse_dimension,
        coarse.tuple_count(),
        coarse_dimension_ratio,
        tuple_reduction,
        two_level_tuple_complexity,
        optional(acceptance.compatible_diagonal_factor),
        optional(acceptance.compatible_energy_factor),
        baseline_condition,
        optional(oracle_condition),
        optional(condition),
        optional(recovery),
        optional_usize(pcg.as_ref().map(|result| result.iterations())),
        optional_bool(pcg.as_ref().map(|result| result.converged())),
        optional(pcg.as_ref().map(|result| result.final_relative_residual())),
        optional_usize(pcg.as_ref().map(|result| result.gramian_applications())),
        optional_usize(
            pcg.as_ref()
                .map(|result| result.preconditioner_applications())
        ),
        work.bootstrap_rounds,
        work.bootstrap_witnesses,
        work.repair_splits,
        work.candidate_pairs_generated,
        work.candidate_pairs_retained,
        work.retained_test_vector_bytes,
        work.retained_report_bytes_estimate,
        work.stop_reason.replace(['\t', '\n'], " "),
    )?;
    Ok(EvaluatedMap { condition })
}

fn record_baseline(
    fixture: &Issue3Fixture,
    baseline: &SymmetricMapPreconditioner,
    baseline_condition: f64,
    oracle_condition: f64,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let problem = &fixture.problem;
    let rhs = deterministic_rhs(problem)?;
    let pcg = solve_projected_pcg_traced(problem, &rhs, baseline, PcgTraceOptions::default())?;
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
    writeln!(
        matrix,
        "{}\t{}\t{}\t{}\t{}\t{}\tbaseline-symmetric-map\ttrue\ttrue\tbaseline\tfalse\tfalse\t0.000000000000e0\t0\t{}\t{}\t0\t0\t0.000000000000e0\t0.000000000000e0\t1.000000000000e0\tNA\tNA\t{:.12e}\t{:.12e}\t{:.12e}\t0.000000000000e0\t{}\t{}\t{:.12e}\t{}\t{}\t0\t0\t0\t0\t0\t0\t0\tbaseline",
        fixture.set,
        fixture.name,
        fixture.family,
        fixture.requested_seed,
        fixture.actual_seed,
        fixture.structural_skips,
        problem.dimension(),
        problem.tuple_count(),
        baseline_condition,
        oracle_condition,
        baseline_condition,
        pcg.iterations(),
        pcg.converged(),
        pcg.final_relative_residual(),
        pcg.gramian_applications(),
        pcg.preconditioner_applications(),
    )?;
    Ok(())
}

fn map_acceptance(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    screen: &DiagonalPreconditioner,
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
            compatible_diagonal_factor: None,
            compatible_energy_factor: None,
        });
    }
    let report = analyze_compatible_relaxation(problem, aggregation, screen, compatible_options())?;
    let decision = evaluate_compatible_relaxation(&report, compatible_criteria())?;
    Ok(MapAcceptance {
        structural_admissible,
        accepted: decision.accepted(),
        compatible_diagonal_factor: Some(decision.maximum_diagonal_factor_per_sweep()),
        compatible_energy_factor: decision.maximum_energy_factor_per_sweep(),
    })
}

fn bootstrap_options() -> BootstrapAggregationOptions {
    BootstrapAggregationOptions {
        setup_test_vectors: 4,
        setup_sweeps: 4,
        setup_jacobi_omega: 0.5,
        maximum_neighbor_degree: 12,
        signature_window: 2,
        maximum_candidate_degree: 12,
        minimum_combined_affinity: 0.40,
        algebraic_affinity_weight: 0.75,
        structural_affinity_weight: 0.05,
        degree_affinity_weight: 0.10,
        signature_hit_weight: 0.10,
        compatible_relaxation: compatible_options(),
        compatible_criteria: compatible_criteria(),
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
        criteria: compatible_criteria(),
        maximum_rounds: 18,
        maximum_coarse_dimension_ratio: MAXIMUM_COARSE_DIMENSION_RATIO,
        minimum_tuple_reduction: MINIMUM_TUPLE_REDUCTION,
        maximum_two_level_tuple_complexity: MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
        minimum_split_score_fraction: 0.001,
    }
}

fn compatible_options() -> CompatibleRelaxationOptions {
    CompatibleRelaxationOptions {
        test_vectors: 12,
        sweeps: 10,
        relaxation_damping: 1.0,
        seed: 0x4d57_4d47_4352_3031,
        relative_zero_tolerance: 1.0e-13,
    }
}

fn compatible_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.85,
        maximum_energy_factor_per_sweep: Some(0.85),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
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
