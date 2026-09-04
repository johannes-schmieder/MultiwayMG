//! Frozen recursive holdout for cycle-screened automatic hierarchies.

#[path = "support/issue2_fixtures.rs"]
mod issue2_fixtures;
#[path = "support/issue3_recursive_fixtures.rs"]
mod issue3_recursive_fixtures;

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use issue2_fixtures::{DynError, deterministic_rhs};
use issue3_recursive_fixtures::{RecursiveHoldoutFixture, recursive_holdout_fixtures};
use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, CycleQualityCriteria, CycleQualityOptions,
    CycleScreenedHierarchyOptions, CycleScreenedHierarchyPlan, CycleScreenedMapHierarchy,
    DenseRangeDecomposition, FactorAggregation, PairNeighborhoodAggregationOptions,
    PcgTraceOptions, Preconditioner, SpectralAnalysisOptions, SymmetricMapPreconditioner,
    ThreeWayProblem, build_pair_neighborhood_aggregation, solve_projected_pcg_traced,
};

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let mut matrix = writer(&output_directory.join("issue3-recursive-holdout.tsv"))?;
    let mut traces = writer(&output_directory.join("issue3-recursive-traces.tsv"))?;
    write_headers(&mut matrix, &mut traces)?;

    for fixture in recursive_holdout_fixtures()? {
        run_fixture(&fixture, &mut matrix, &mut traces)?;
    }
    println!(
        "wrote {} and {}",
        output_directory
            .join("issue3-recursive-holdout.tsv")
            .display(),
        output_directory
            .join("issue3-recursive-traces.tsv")
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
        "set\tcase\tfamily\trequested_seed\tactual_seed\tstructural_skips\trequested_depth\tmethod\taccepted\tachieved_depth\tstop_reason\tdimension_complexity\ttuple_complexity\tbaseline_condition\toracle_condition\tcandidate_condition\toracle_improvement_recovered\tpcg_iterations\tpcg_converged\tpcg_final_relative_residual\tpcg_gramian_applications\tpcg_preconditioner_applications\tlevel_sources\tlevel_cycle_factors\tlevel_coarse_dimensions\tlevel_coarse_tuples\tlevel_probe_gramian_applications\tlevel_probe_preconditioner_applications\tlevel_bootstrap_rounds\tlevel_bootstrap_witnesses\tlevel_repair_splits"
    )?;
    writeln!(
        traces,
        "set\tcase\tfamily\tmethod\titeration\trelative_true_residual"
    )?;
    Ok(())
}

fn run_fixture(
    fixture: &RecursiveHoldoutFixture,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let problem = &fixture.problem;
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(problem, spectral_options)?;
    let baseline = SymmetricMapPreconditioner::new(problem.clone());
    let baseline_condition = range
        .analyze(&baseline, spectral_options)?
        .preconditioned_condition_number();
    trace_method(fixture, "baseline-symmetric-map", &baseline, traces)?;

    let oracle = CycleScreenedMapHierarchy::from_maps(
        problem.clone(),
        fixture.oracle_maps.clone(),
        1.0e-12,
    )?;
    let oracle_condition = range
        .analyze(&oracle, spectral_options)?
        .preconditioned_condition_number();
    let oracle_solve = trace_method(fixture, "oracle-map-hierarchy", &oracle, traces)?;
    write_row(
        fixture,
        "oracle-map-hierarchy",
        true,
        fixture.depth,
        "oracle",
        complexity(problem, &fixture.oracle_maps)?,
        baseline_condition,
        oracle_condition,
        Some(oracle_condition),
        Some(1.0),
        Some(&oracle_solve),
        LevelFields::oracle(&fixture.oracle_maps, problem)?,
        matrix,
    )?;

    let one_shot_maps = structural_maps(problem, fixture.depth)?;
    let one_shot_depth = one_shot_maps.len();
    let one_shot_accepted = one_shot_depth == fixture.depth;
    let one_shot_complexity = complexity(problem, &one_shot_maps)?;
    let (one_shot_condition, one_shot_solve) = if one_shot_accepted {
        let hierarchy = CycleScreenedMapHierarchy::from_maps(
            problem.clone(),
            one_shot_maps.clone(),
            1.0e-12,
        )?;
        let condition = range
            .analyze(&hierarchy, spectral_options)?
            .preconditioned_condition_number();
        let solve = trace_method(fixture, "recursive-one-shot", &hierarchy, traces)?;
        (Some(condition), Some(solve))
    } else {
        (None, None)
    };
    write_row(
        fixture,
        "recursive-one-shot",
        one_shot_accepted,
        one_shot_depth,
        if one_shot_accepted {
            "reached-requested-depth"
        } else {
            "one-shot-stagnated"
        },
        one_shot_complexity,
        baseline_condition,
        oracle_condition,
        one_shot_condition,
        one_shot_condition.and_then(|value| {
            recovery_fraction(baseline_condition, oracle_condition, value)
        }),
        one_shot_solve.as_ref(),
        LevelFields::structural(&one_shot_maps, problem)?,
        matrix,
    )?;

    let plan = CycleScreenedHierarchyPlan::build(
        problem.clone(),
        hierarchy_options(fixture.terminal_dimension),
    )?;
    let automatic_accepted = plan.accepted() && plan.depth() == fixture.depth;
    let (automatic_condition, automatic_solve) = if automatic_accepted {
        let hierarchy = plan.build_preconditioner()?;
        let condition = range
            .analyze(&hierarchy, spectral_options)?
            .preconditioned_condition_number();
        let solve = trace_method(fixture, "cycle-screened-automatic", &hierarchy, traces)?;
        (Some(condition), Some(solve))
    } else {
        (None, None)
    };
    write_row(
        fixture,
        "cycle-screened-automatic",
        automatic_accepted,
        plan.depth(),
        &format!("{:?}", plan.stop_reason()),
        (plan.dimension_complexity(), plan.tuple_complexity()),
        baseline_condition,
        oracle_condition,
        automatic_condition,
        automatic_condition.and_then(|value| {
            recovery_fraction(baseline_condition, oracle_condition, value)
        }),
        automatic_solve.as_ref(),
        LevelFields::automatic(&plan),
        matrix,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_row(
    fixture: &RecursiveHoldoutFixture,
    method: &str,
    accepted: bool,
    achieved_depth: usize,
    stop_reason: &str,
    complexities: (f64, f64),
    baseline_condition: f64,
    oracle_condition: f64,
    candidate_condition: Option<f64>,
    recovery: Option<f64>,
    solve: Option<&multiway_mg::PcgTraceResult>,
    levels: LevelFields,
    matrix: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let row = vec![
        fixture.set.to_owned(),
        fixture.name.clone(),
        fixture.family.to_owned(),
        fixture.requested_seed.to_string(),
        fixture.actual_seed.to_string(),
        fixture.structural_skips.to_string(),
        fixture.depth.to_string(),
        method.to_owned(),
        accepted.to_string(),
        achieved_depth.to_string(),
        stop_reason.replace(['\t', '\n'], " "),
        format!("{:.12e}", complexities.0),
        format!("{:.12e}", complexities.1),
        format!("{baseline_condition:.12e}"),
        format!("{oracle_condition:.12e}"),
        optional(candidate_condition),
        optional(recovery),
        optional_usize(solve.map(multiway_mg::PcgTraceResult::iterations)),
        optional_bool(solve.map(multiway_mg::PcgTraceResult::converged)),
        optional(solve.map(multiway_mg::PcgTraceResult::final_relative_residual)),
        optional_usize(solve.map(multiway_mg::PcgTraceResult::gramian_applications)),
        optional_usize(solve.map(multiway_mg::PcgTraceResult::preconditioner_applications)),
        levels.sources,
        levels.cycle_factors,
        levels.coarse_dimensions,
        levels.coarse_tuples,
        levels.probe_gramian_applications,
        levels.probe_preconditioner_applications,
        levels.bootstrap_rounds,
        levels.bootstrap_witnesses,
        levels.repair_splits,
    ];
    writeln!(matrix, "{}", row.join("\t"))?;
    Ok(())
}

struct LevelFields {
    sources: String,
    cycle_factors: String,
    coarse_dimensions: String,
    coarse_tuples: String,
    probe_gramian_applications: String,
    probe_preconditioner_applications: String,
    bootstrap_rounds: String,
    bootstrap_witnesses: String,
    repair_splits: String,
}

impl LevelFields {
    fn oracle(
        maps: &[FactorAggregation],
        problem: &ThreeWayProblem,
    ) -> Result<Self, DynError> {
        let mut current = problem.clone();
        let mut dimensions = Vec::new();
        let mut tuples = Vec::new();
        for map in maps {
            current = map.coarsen(&current)?;
            dimensions.push(current.dimension());
            tuples.push(current.tuple_count());
        }
        Ok(Self {
            sources: repeat_field("oracle", maps.len()),
            cycle_factors: repeat_field("NA", maps.len()),
            coarse_dimensions: join_usize(&dimensions),
            coarse_tuples: join_usize(&tuples),
            probe_gramian_applications: repeat_field("0", maps.len()),
            probe_preconditioner_applications: repeat_field("0", maps.len()),
            bootstrap_rounds: repeat_field("0", maps.len()),
            bootstrap_witnesses: repeat_field("0", maps.len()),
            repair_splits: repeat_field("0", maps.len()),
        })
    }

    fn structural(
        maps: &[FactorAggregation],
        problem: &ThreeWayProblem,
    ) -> Result<Self, DynError> {
        let mut current = problem.clone();
        let mut dimensions = Vec::new();
        let mut tuples = Vec::new();
        for map in maps {
            current = map.coarsen(&current)?;
            dimensions.push(current.dimension());
            tuples.push(current.tuple_count());
        }
        Ok(Self {
            sources: repeat_field("pair-neighborhood", maps.len()),
            cycle_factors: repeat_field("NA", maps.len()),
            coarse_dimensions: join_usize(&dimensions),
            coarse_tuples: join_usize(&tuples),
            probe_gramian_applications: repeat_field("0", maps.len()),
            probe_preconditioner_applications: repeat_field("0", maps.len()),
            bootstrap_rounds: repeat_field("0", maps.len()),
            bootstrap_witnesses: repeat_field("0", maps.len()),
            repair_splits: repeat_field("0", maps.len()),
        })
    }

    fn automatic(plan: &CycleScreenedHierarchyPlan) -> Self {
        let reports = plan.level_reports();
        Self {
            sources: reports
                .iter()
                .map(|level| format!("{:?}", level.selected_source()))
                .collect::<Vec<_>>()
                .join(";"),
            cycle_factors: reports
                .iter()
                .map(|level| optional(level.selected_cycle_factor()))
                .collect::<Vec<_>>()
                .join(";"),
            coarse_dimensions: reports
                .iter()
                .map(|level| level.coarse_dimension().to_string())
                .collect::<Vec<_>>()
                .join(";"),
            coarse_tuples: reports
                .iter()
                .map(|level| level.coarse_tuple_count().to_string())
                .collect::<Vec<_>>()
                .join(";"),
            probe_gramian_applications: reports
                .iter()
                .map(|level| {
                    level
                        .portfolio()
                        .work_report()
                        .probe_gramian_applications()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(";"),
            probe_preconditioner_applications: reports
                .iter()
                .map(|level| {
                    level
                        .portfolio()
                        .work_report()
                        .probe_preconditioner_applications()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(";"),
            bootstrap_rounds: reports
                .iter()
                .map(|level| level.portfolio().primary_result().rounds().len().to_string())
                .collect::<Vec<_>>()
                .join(";"),
            bootstrap_witnesses: reports
                .iter()
                .map(|level| {
                    level
                        .portfolio()
                        .primary_result()
                        .rounds()
                        .last()
                        .map_or(0, |round| round.bootstrap_witnesses())
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(";"),
            repair_splits: reports
                .iter()
                .map(|level| {
                    level
                        .portfolio()
                        .primary_result()
                        .split_repair()
                        .map_or(0, |repair| repair.accepted_splits())
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(";"),
        }
    }
}

fn structural_maps(
    problem: &ThreeWayProblem,
    maximum_depth: usize,
) -> Result<Vec<FactorAggregation>, DynError> {
    let mut current = problem.clone();
    let mut maps = Vec::with_capacity(maximum_depth);
    for _ in 0..maximum_depth {
        let map = build_pair_neighborhood_aggregation(
            &current,
            PairNeighborhoodAggregationOptions {
                minimum_affinity: 0.02,
                maximum_neighbor_degree: 12,
            },
        )?;
        let coarse = map.coarsen(&current)?;
        if coarse.dimension() >= current.dimension()
            || coarse.tuple_count() >= current.tuple_count()
        {
            break;
        }
        maps.push(map);
        current = coarse;
    }
    Ok(maps)
}

fn hierarchy_options(terminal_dimension: usize) -> CycleScreenedHierarchyOptions {
    CycleScreenedHierarchyOptions {
        maximum_levels: 4,
        terminal_dimension,
        maximum_dimension_complexity: 2.25,
        maximum_tuple_complexity: 2.25,
        bootstrap: bootstrap_options(),
        cycle_probe: CycleQualityOptions {
            test_vectors: 12,
            power_iterations: 24,
            tail_iterations: 6,
            correction_damping: 1.0,
            seed: 0x4d57_4d47_4359_4331,
            relative_zero_tolerance: 1.0e-13,
        },
        cycle_criteria: CycleQualityCriteria {
            maximum_estimated_energy_factor: 0.50,
            maximum_observed_energy_factor: Some(1.05),
            maximum_structural_defect: 1.0e-10,
        },
        terminal_relative_tolerance: 1.0e-12,
    }
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
        maximum_coarse_dimension_ratio: 0.80,
        minimum_tuple_reduction: 0.05,
        maximum_two_level_tuple_complexity: 1.95,
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
            maximum_coarse_dimension_ratio: 0.80,
            minimum_tuple_reduction: 0.05,
            maximum_two_level_tuple_complexity: 1.95,
            minimum_split_score_fraction: 0.001,
        }),
        seed: 0x4d57_4d47_434f_5645,
    }
}

fn complexity(
    problem: &ThreeWayProblem,
    maps: &[FactorAggregation],
) -> Result<(f64, f64), DynError> {
    let finest_dimension = problem.dimension();
    let finest_tuples = problem.tuple_count();
    let mut dimension_sum = finest_dimension;
    let mut tuple_sum = finest_tuples;
    let mut current = problem.clone();
    for map in maps {
        current = map.coarsen(&current)?;
        dimension_sum += current.dimension();
        tuple_sum += current.tuple_count();
    }
    Ok((
        dimension_sum as f64 / finest_dimension as f64,
        tuple_sum as f64 / finest_tuples as f64,
    ))
}

fn trace_method<P: Preconditioner + ?Sized>(
    fixture: &RecursiveHoldoutFixture,
    method: &str,
    preconditioner: &P,
    traces: &mut BufWriter<File>,
) -> Result<multiway_mg::PcgTraceResult, DynError> {
    let rhs = deterministic_rhs(&fixture.problem)?;
    let solve = solve_projected_pcg_traced(
        &fixture.problem,
        &rhs,
        preconditioner,
        PcgTraceOptions {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 0.0,
            max_iterations: 2_000,
        },
    )?;
    for sample in solve.samples() {
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
    Ok(solve)
}

fn recovery_fraction(baseline: f64, oracle: f64, candidate: f64) -> Option<f64> {
    let denominator = baseline - oracle;
    (denominator > 1.0e-12 * baseline.abs().max(1.0))
        .then_some((baseline - candidate) / denominator)
}

fn repeat_field(value: &str, count: usize) -> String {
    std::iter::repeat_n(value, count)
        .collect::<Vec<_>>()
        .join(";")
}

fn join_usize(values: &[usize]) -> String {
    values
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(";")
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
