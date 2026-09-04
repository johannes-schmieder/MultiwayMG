//! Automatic-to-oracle gap matrix for issue #3.

#[path = "support/issue2_fixtures.rs"]
mod fixtures;

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use fixtures::{DynError, OracleCase, deterministic_rhs, one_level_cases};
use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, DenseRangeDecomposition, DiagonalPreconditioner,
    FactorAggregation, PairNeighborhoodAggregationOptions, PcgTraceOptions,
    SpectralAnalysisOptions, SymmetricMapPreconditioner, SymmetricTwoGridPreconditioner,
    ThreeWayProblem, analyze_compatible_relaxation, build_bootstrap_aggregation,
    build_pair_neighborhood_aggregation, evaluate_compatible_relaxation,
    repair_aggregation_by_splitting, solve_projected_pcg_traced,
};

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let mut summary = writer(&output_directory.join("issue3-automatic-gap-matrix.tsv"))?;
    writeln!(
        summary,
        "case\tfamily\tmethod\taccepted\texact_oracle_partition\toracle_pair_recall\twrong_pair_count\tfine_dimension\tfine_tuples\tcoarse_dimension\tcoarse_tuples\ttuple_reduction\ttwo_level_tuple_complexity\tcompatible_factor_per_sweep\tbaseline_map_condition\toracle_two_grid_condition\tcandidate_two_grid_condition\toracle_improvement_recovered\tpcg_iterations\tpcg_final_relative_residual\tbootstrap_rounds\tbootstrap_witnesses\trepair_splits\tcandidate_pairs_generated\tcandidate_pairs_retained\tretained_test_vector_bytes\tretained_report_bytes_estimate\tstop_reason"
    )?;

    for case in one_level_cases()? {
        run_case(&case, &mut summary)?;
    }
    println!(
        "wrote {}",
        output_directory
            .join("issue3-automatic-gap-matrix.tsv")
            .display()
    );
    Ok(())
}

fn run_case(case: &OracleCase, summary: &mut BufWriter<File>) -> Result<(), DynError> {
    let oracle = case
        .maps
        .first()
        .cloned()
        .ok_or("issue #3 fixture has no oracle map")?;
    let problem = &case.problem;
    let screen = DiagonalPreconditioner::new(problem, 0.5)?;
    let map_smoother = SymmetricMapPreconditioner::new(problem.clone());
    let range = DenseRangeDecomposition::from_problem(
        problem,
        SpectralAnalysisOptions {
            maximum_dimension: 256,
            ..SpectralAnalysisOptions::default()
        },
    )?;
    let baseline_condition = range
        .analyze(&map_smoother, SpectralAnalysisOptions::default())?
        .preconditioned_condition_number();
    let oracle_cycle = SymmetricTwoGridPreconditioner::build(
        problem.clone(),
        oracle.clone(),
        SymmetricMapPreconditioner::new(problem.clone()),
        1,
        1.0,
        1.0e-12,
    )?;
    let oracle_condition = range
        .analyze(&oracle_cycle, SpectralAnalysisOptions::default())?
        .preconditioned_condition_number();

    record_map(
        case,
        "oracle",
        &oracle,
        true,
        "oracle-reference",
        &screen,
        &range,
        baseline_condition,
        oracle_condition,
        WorkFields::default(),
        summary,
    )?;

    let one_shot = build_pair_neighborhood_aggregation(
        problem,
        PairNeighborhoodAggregationOptions::default(),
    )?;
    let one_shot_accepted = compatible_accepts(problem, &one_shot, &screen)?;
    record_map(
        case,
        "one-shot-pair-neighborhood",
        &one_shot,
        one_shot_accepted,
        if one_shot_accepted {
            "compatible-accepted"
        } else {
            "compatible-rejected"
        },
        &screen,
        &range,
        baseline_condition,
        oracle_condition,
        WorkFields::default(),
        summary,
    )?;

    let bootstrap = build_bootstrap_aggregation(problem, &screen, bootstrap_options())?;
    let bootstrap_work = bootstrap.work_report();
    let bootstrap_witnesses = bootstrap
        .rounds()
        .last()
        .map_or(0, |round| round.bootstrap_witnesses());
    let repair_splits = bootstrap
        .split_repair()
        .map_or(0, |repair| repair.accepted_splits());
    record_map(
        case,
        "bootstrap-final",
        bootstrap.final_aggregation(),
        bootstrap.accepted(),
        &format!("{:?}", bootstrap.stop_reason()),
        &screen,
        &range,
        baseline_condition,
        oracle_condition,
        WorkFields {
            bootstrap_rounds: bootstrap.rounds().len(),
            bootstrap_witnesses,
            repair_splits,
            candidate_pairs_generated: bootstrap_work.candidate_pairs_generated(),
            candidate_pairs_retained: bootstrap_work.candidate_pairs_retained(),
            retained_test_vector_bytes: bootstrap_work.retained_test_vector_bytes(),
            retained_report_bytes_estimate: bootstrap_work.retained_round_report_bytes_estimate(),
        },
        summary,
    )?;

    let overmerged = overmerge_oracle(&oracle)?;
    let overmerged_accepted = compatible_accepts(problem, &overmerged, &screen)?;
    record_map(
        case,
        "overmerged-control",
        &overmerged,
        overmerged_accepted,
        if overmerged_accepted {
            "compatible-accepted"
        } else {
            "compatible-rejected"
        },
        &screen,
        &range,
        baseline_condition,
        oracle_condition,
        WorkFields::default(),
        summary,
    )?;

    if overmerged.coarse_counts() != oracle.coarse_counts() {
        let repair =
            repair_aggregation_by_splitting(problem, &overmerged, &screen, repair_options())?;
        record_map(
            case,
            "repaired-overmerged-control",
            repair.final_aggregation(),
            repair.accepted(),
            &format!("{:?}", repair.stop_reason()),
            &screen,
            &range,
            baseline_condition,
            oracle_condition,
            WorkFields {
                repair_splits: repair.accepted_splits(),
                ..WorkFields::default()
            },
            summary,
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct WorkFields {
    bootstrap_rounds: usize,
    bootstrap_witnesses: usize,
    repair_splits: usize,
    candidate_pairs_generated: usize,
    candidate_pairs_retained: usize,
    retained_test_vector_bytes: usize,
    retained_report_bytes_estimate: usize,
}

#[allow(clippy::too_many_arguments)]
fn record_map(
    case: &OracleCase,
    method: &str,
    aggregation: &FactorAggregation,
    accepted: bool,
    stop_reason: &str,
    screen: &DiagonalPreconditioner,
    range: &DenseRangeDecomposition,
    baseline_condition: f64,
    oracle_condition: f64,
    work: WorkFields,
    summary: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let problem = &case.problem;
    let oracle = case.maps.first().ok_or("missing oracle map")?;
    let coarse = aggregation.coarsen(problem)?;
    let compatible_factor = compatible_factor(problem, aggregation, screen)?;
    let cycle = SymmetricTwoGridPreconditioner::build(
        problem.clone(),
        aggregation.clone(),
        SymmetricMapPreconditioner::new(problem.clone()),
        1,
        1.0,
        1.0e-12,
    )?;
    let spectral = range.analyze(
        &cycle,
        SpectralAnalysisOptions {
            maximum_dimension: 256,
            ..SpectralAnalysisOptions::default()
        },
    )?;
    let condition = spectral.preconditioned_condition_number();
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
    let recovery = recovery_fraction(baseline_condition, oracle_condition, condition);
    let partition = partition_metrics(oracle, aggregation);
    let tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    let coarse_dimension: usize = aggregation.coarse_counts().iter().sum();

    writeln!(
        summary,
        "{}\t{}\t{}\t{}\t{}\t{:.12e}\t{}\t{}\t{}\t{}\t{}\t{:.12e}\t{:.12e}\t{}\t{:.12e}\t{:.12e}\t{:.12e}\t{}\t{}\t{:.12e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        case.name,
        case.family,
        method,
        accepted,
        partition.exact,
        partition.oracle_pair_recall,
        partition.wrong_pair_count,
        problem.dimension(),
        problem.tuple_count(),
        coarse_dimension,
        coarse.tuple_count(),
        1.0 - tuple_ratio,
        1.0 + tuple_ratio,
        optional(compatible_factor),
        baseline_condition,
        oracle_condition,
        condition,
        optional(recovery),
        pcg.iterations(),
        pcg.final_relative_residual(),
        work.bootstrap_rounds,
        work.bootstrap_witnesses,
        work.repair_splits,
        work.candidate_pairs_generated,
        work.candidate_pairs_retained,
        work.retained_test_vector_bytes,
        work.retained_report_bytes_estimate,
        stop_reason.replace(['\t', '\n'], " "),
    )?;
    Ok(())
}

fn compatible_accepts(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    screen: &DiagonalPreconditioner,
) -> Result<bool, DynError> {
    if aggregation.coarse_counts().iter().sum::<usize>() >= problem.dimension() {
        return Ok(false);
    }
    let report = analyze_compatible_relaxation(problem, aggregation, screen, compatible_options())?;
    Ok(evaluate_compatible_relaxation(&report, compatible_criteria())?.accepted())
}

fn compatible_factor(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    screen: &DiagonalPreconditioner,
) -> Result<Option<f64>, DynError> {
    if aggregation.coarse_counts().iter().sum::<usize>() >= problem.dimension() {
        return Ok(None);
    }
    let report = analyze_compatible_relaxation(problem, aggregation, screen, compatible_options())?;
    Ok(Some(
        report
            .maximum_diagonal_contraction()
            .powf(1.0 / report.sweeps() as f64),
    ))
}

fn bootstrap_options() -> BootstrapAggregationOptions {
    BootstrapAggregationOptions {
        setup_test_vectors: 12,
        setup_sweeps: 10,
        setup_jacobi_omega: 0.5,
        maximum_neighbor_degree: 16,
        signature_window: 4,
        maximum_candidate_degree: 20,
        minimum_combined_affinity: 0.30,
        algebraic_affinity_weight: 0.60,
        structural_affinity_weight: 0.20,
        degree_affinity_weight: 0.10,
        signature_hit_weight: 0.10,
        compatible_relaxation: compatible_options(),
        compatible_criteria: compatible_criteria(),
        maximum_bootstrap_witnesses: 4,
        maximum_coarse_dimension_ratio: 0.75,
        minimum_tuple_reduction: 0.02,
        maximum_two_level_tuple_complexity: 1.98,
        split_repair: Some(repair_options()),
        seed: 0x4d57_4d47_4253_3031,
    }
}

fn repair_options() -> AggregationRepairOptions {
    AggregationRepairOptions {
        relaxation: compatible_options(),
        criteria: compatible_criteria(),
        maximum_rounds: 16,
        maximum_coarse_dimension_ratio: 0.75,
        minimum_tuple_reduction: 0.02,
        maximum_two_level_tuple_complexity: 1.98,
        minimum_split_score_fraction: 0.0025,
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

fn compatible_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.80,
        maximum_energy_factor_per_sweep: Some(0.80),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    }
}

fn overmerge_oracle(oracle: &FactorAggregation) -> Result<FactorAggregation, DynError> {
    let counts = oracle.fine_counts();
    let parents = core::array::from_fn(|factor| {
        (0..counts[factor])
            .map(|level| oracle.parents(factor)[level] / 2)
            .collect()
    });
    Ok(FactorAggregation::new(counts, parents)?)
}

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
                    if candidate_same {
                        recovered_pairs += 1;
                    }
                } else if candidate_same {
                    wrong_pairs += 1;
                }
            }
        }
    }
    PartitionMetrics {
        exact: oracle_pairs == recovered_pairs && wrong_pairs == 0,
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

fn writer(path: &Path) -> Result<BufWriter<File>, DynError> {
    Ok(BufWriter::new(File::create(path)?))
}
