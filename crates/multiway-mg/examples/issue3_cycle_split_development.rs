//! Development matrix for complete-cycle witness-driven aggregate splitting.
//!
//! This executable deliberately reuses the observed issue #3 v3 seeds
//! `900`--`909`. Its output is calibration evidence only and cannot change the
//! frozen v3 verdict.

#[path = "support/issue2_fixtures.rs"]
mod issue2_fixtures;
#[path = "support/issue3_cycle_fixtures.rs"]
mod issue3_cycle_fixtures;

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
};

use issue2_fixtures::{DynError, deterministic_rhs};
use issue3_cycle_fixtures::{CycleHoldoutFixture, cycle_holdout_v3_fixtures};
use multiway_mg::{
    CycleQualityCriteria, CycleQualityOptions, CycleSplitRepairOptions,
    DenseRangeDecomposition, FactorAggregation, PairCmgOptions, PairCmgPreconditioner,
    PairNeighborhoodAggregationOptions, PcgTraceOptions, Preconditioner,
    SpectralAnalysisOptions, SymmetricMapPreconditioner, SymmetricTwoGridPreconditioner,
    ThreeWayProblem, build_pair_neighborhood_aggregation,
    repair_cycle_aggregation_by_splitting, solve_projected_pcg_traced,
};

const MAXIMUM_ROUNDS: usize = 8;
const MAXIMUM_COARSE_DIMENSION_RATIO: f64 = 0.65;
const MINIMUM_TUPLE_REDUCTION: f64 = 0.05;
const MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY: f64 = 1.95;
const MAXIMUM_CANDIDATE_FACTOR_RATIO: f64 = 0.98;

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let path = output_directory.join("issue3-cycle-split-development.tsv");
    let mut output = BufWriter::new(File::create(&path)?);
    writeln!(
        output,
        "case\tfamily\tsmoother\tinitial_coarse_dimension\tfinal_coarse_dimension\tinitial_coarse_tuples\tfinal_coarse_tuples\tinitial_two_level_tuple_complexity\tfinal_two_level_tuple_complexity\taccepted_splits\tinitial_probe_factor\tfinal_probe_factor\tprobe_factor_ratio\tinitial_condition\tfinal_condition\tcondition_improvement_fraction\tfinal_cycle_accepted\tpcg_iterations\tpcg_converged\tpcg_final_relative_residual\tstop_reason"
    )?;

    for fixture in cycle_holdout_v3_fixtures()? {
        run_fixture(&fixture, &mut output)?;
    }
    println!("wrote {}", path.display());
    Ok(())
}

fn run_fixture(
    fixture: &CycleHoldoutFixture,
    output: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let baseline = build_pair_neighborhood_aggregation(
        &fixture.problem,
        PairNeighborhoodAggregationOptions {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: 12,
        },
    )?;
    let map = SymmetricMapPreconditioner::new(fixture.problem.clone());
    run_smoother(fixture, &baseline, "symmetric-map", map, output)?;

    let pair = PairCmgPreconditioner::build(
        fixture.problem.clone(),
        PairCmgOptions::default(),
    )?;
    run_smoother(fixture, &baseline, "all-pairs-cmg", pair, output)?;
    Ok(())
}

fn run_smoother<S>(
    fixture: &CycleHoldoutFixture,
    baseline: &FactorAggregation,
    smoother_name: &str,
    smoother: S,
    output: &mut BufWriter<File>,
) -> Result<(), DynError>
where
    S: Preconditioner + Clone,
{
    let problem = &fixture.problem;
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(problem, spectral_options)?;
    let baseline_cycle = build_cycle(problem, baseline, smoother.clone())?;
    let initial_condition = range
        .analyze(&baseline_cycle, spectral_options)?
        .preconditioned_condition_number();

    let repair = repair_cycle_aggregation_by_splitting(
        problem,
        baseline,
        repair_options(),
        |aggregation| build_cycle(problem, aggregation, smoother.clone()),
    )?;
    let final_cycle = build_cycle(problem, repair.final_aggregation(), smoother)?;
    let final_condition = range
        .analyze(&final_cycle, spectral_options)?
        .preconditioned_condition_number();
    let rhs = deterministic_rhs(problem)?;
    let pcg = solve_projected_pcg_traced(
        problem,
        &rhs,
        &final_cycle,
        PcgTraceOptions {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 0.0,
            max_iterations: 2_000,
        },
    )?;

    let initial_factor = repair.initial_report().maximum_estimated_energy_factor();
    let final_factor = repair.final_report().maximum_estimated_energy_factor();
    let initial_metrics = repair.initial_metrics();
    let final_metrics = repair.final_metrics();
    writeln!(
        output,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.12e}\t{:.12e}\t{}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{}\t{}\t{}\t{:.12e}\t{:?}",
        fixture.name,
        fixture.family,
        smoother_name,
        initial_metrics.coarse_dimension(),
        final_metrics.coarse_dimension(),
        initial_metrics.coarse_tuple_count(),
        final_metrics.coarse_tuple_count(),
        initial_metrics.two_level_tuple_complexity(),
        final_metrics.two_level_tuple_complexity(),
        repair.accepted_splits(),
        initial_factor,
        final_factor,
        final_factor / initial_factor,
        initial_condition,
        final_condition,
        (initial_condition - final_condition) / initial_condition,
        repair.accepted(),
        pcg.iterations(),
        pcg.converged(),
        pcg.final_relative_residual(),
        repair.stop_reason(),
    )?;
    Ok(())
}

fn build_cycle<S>(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    smoother: S,
) -> Result<SymmetricTwoGridPreconditioner<S>, multiway_mg::MultiwayError>
where
    S: Preconditioner,
{
    SymmetricTwoGridPreconditioner::build(
        problem.clone(),
        aggregation.clone(),
        smoother,
        1,
        1.0,
        1.0e-12,
    )
}

fn repair_options() -> CycleSplitRepairOptions {
    CycleSplitRepairOptions {
        probe: CycleQualityOptions {
            test_vectors: 12,
            power_iterations: 24,
            tail_iterations: 6,
            correction_damping: 1.0,
            seed: 0x4d57_4d47_4359_4331,
            relative_zero_tolerance: 1.0e-13,
        },
        criteria: CycleQualityCriteria {
            maximum_estimated_energy_factor: 0.50,
            maximum_observed_energy_factor: Some(1.05),
            maximum_structural_defect: 1.0e-10,
        },
        maximum_rounds: MAXIMUM_ROUNDS,
        maximum_coarse_dimension_ratio: MAXIMUM_COARSE_DIMENSION_RATIO,
        minimum_tuple_reduction: MINIMUM_TUPLE_REDUCTION,
        maximum_two_level_tuple_complexity: MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
        minimum_split_score_fraction: 0.001,
        maximum_candidate_factor_ratio: MAXIMUM_CANDIDATE_FACTOR_RATIO,
    }
}
