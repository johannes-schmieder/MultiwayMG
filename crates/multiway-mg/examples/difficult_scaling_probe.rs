//! Difficult weak-chain scaling probe for the first MultiwayMG research version.

use std::time::{Duration, Instant};

use multiway_mg::{
    AggregationStrategy, DiagonalPreconditioner, HierarchyOptions, HybridPairVcycle,
    LeastSquaresOptions, LeastSquaresResult, PairCmgOptions, PairCmgPreconditioner, PcgOptions,
    PcgResult, Preconditioner, ThreeWayHierarchy, ThreeWayProblem, solve_projected_pcg,
    solve_weighted_least_squares,
};

const PCG_MAX_ITERATIONS: usize = 10_000;
const LSMR_MAX_ITERATIONS: usize = 10_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "groups\tlevels\ttuples\tmethod\tsetup_ms\tmedian_solve_ms\titerations\tconverged\tcertified_residual\thierarchy_depth\ttuple_complexity\taggregation_kinds"
    );
    for groups in [16_usize, 32, 64, 128, 256, 512] {
        run_case(groups, 2, 0.02)?;
    }
    Ok(())
}

fn run_case(
    groups: usize,
    clones: usize,
    bridge_weight: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let problem = weak_chain_problem(groups, clones, bridge_weight)?;
    let targets = slow_chain_targets(&problem, groups, clones)?;
    let rhs = problem.rhs_from_targets(&targets)?;
    let levels = problem.dimension();

    let setup_start = Instant::now();
    let diagonal = DiagonalPreconditioner::new(&problem, 0.5)?;
    let diagonal_setup = setup_start.elapsed();
    let (diagonal_result, diagonal_solve) = median_pcg(&problem, &rhs, &diagonal)?;
    print_pcg(
        groups,
        levels,
        problem.tuple_count(),
        "diagonal-pcg",
        diagonal_setup,
        diagonal_solve,
        &diagonal_result,
        0,
        0.0,
        "-",
    );

    let setup_start = Instant::now();
    let pair = PairCmgPreconditioner::build(problem.clone(), PairCmgOptions::default())?;
    let pair_setup = setup_start.elapsed();
    let (pair_result, pair_solve) = median_pcg(&problem, &rhs, &pair)?;
    print_pcg(
        groups,
        levels,
        problem.tuple_count(),
        "pair-cmg-pcg",
        pair_setup,
        pair_solve,
        &pair_result,
        0,
        0.0,
        "-",
    );

    let hierarchy_options = HierarchyOptions {
        terminal_dimension: 48,
        minimum_dimension_reduction: 0.01,
        minimum_tuple_reduction: 0.0,
        aggregation: AggregationStrategy::Consecutive,
        ..HierarchyOptions::default()
    };
    let setup_start = Instant::now();
    let hierarchy = ThreeWayHierarchy::build(problem.clone(), hierarchy_options.clone())?;
    let hierarchy_setup = setup_start.elapsed();
    let hierarchy_kinds = format!("{:?}", hierarchy.report().aggregation_kinds());
    let (hierarchy_result, hierarchy_solve) = median_pcg(&problem, &rhs, &hierarchy)?;
    print_pcg(
        groups,
        levels,
        problem.tuple_count(),
        "three-way-vcycle-pcg",
        hierarchy_setup,
        hierarchy_solve,
        &hierarchy_result,
        hierarchy.depth(),
        hierarchy.report().tuple_complexity(),
        &hierarchy_kinds,
    );

    let setup_start = Instant::now();
    let hybrid = HybridPairVcycle::build(
        problem.clone(),
        hierarchy_options,
        PairCmgOptions::default(),
    )?;
    let hybrid_setup = setup_start.elapsed();
    let hybrid_report = hybrid.hierarchy().report();
    let hybrid_kinds = format!("{:?}", hybrid_report.aggregation_kinds());
    let (hybrid_result, hybrid_solve) = median_pcg(&problem, &rhs, &hybrid)?;
    print_pcg(
        groups,
        levels,
        problem.tuple_count(),
        "hybrid-pcg",
        hybrid_setup,
        hybrid_solve,
        &hybrid_result,
        hybrid.hierarchy().depth(),
        hybrid_report.tuple_complexity(),
        &hybrid_kinds,
    );

    let (lsmr_result, lsmr_solve) = median_lsmr(&problem, &targets, &hybrid)?;
    print_lsmr(
        groups,
        levels,
        problem.tuple_count(),
        hybrid_setup,
        lsmr_solve,
        &lsmr_result,
        hybrid.hierarchy().depth(),
        hybrid_report.tuple_complexity(),
        &hybrid_kinds,
    );

    if !hybrid_result.converged() || !lsmr_result.converged() {
        return Err(
            format!("hybrid solver did not converge for {groups} weak-chain groups").into(),
        );
    }
    Ok(())
}

fn median_pcg(
    problem: &ThreeWayProblem,
    rhs: &[f64],
    preconditioner: &dyn Preconditioner,
) -> Result<(PcgResult, Duration), Box<dyn std::error::Error>> {
    let mut recorded = Vec::with_capacity(3);
    let mut last = None;
    for repetition in 0..4 {
        let start = Instant::now();
        let result = solve_projected_pcg(
            problem,
            rhs,
            preconditioner,
            PcgOptions {
                relative_tolerance: 1.0e-9,
                max_iterations: PCG_MAX_ITERATIONS,
                ..PcgOptions::default()
            },
        )?;
        let elapsed = start.elapsed();
        if repetition > 0 {
            recorded.push(elapsed);
        }
        last = Some(result);
    }
    recorded.sort();
    Ok((
        last.expect("at least one PCG solve"),
        recorded[recorded.len() / 2],
    ))
}

fn median_lsmr(
    problem: &ThreeWayProblem,
    targets: &[f64],
    preconditioner: &dyn Preconditioner,
) -> Result<(LeastSquaresResult, Duration), Box<dyn std::error::Error>> {
    let mut recorded = Vec::with_capacity(3);
    let mut last = None;
    for repetition in 0..4 {
        let start = Instant::now();
        let result = solve_weighted_least_squares(
            problem,
            targets,
            preconditioner,
            LeastSquaresOptions {
                tolerance: 1.0e-9,
                max_iterations: LSMR_MAX_ITERATIONS,
                local_size: Some(8),
            },
        )?;
        let elapsed = start.elapsed();
        if repetition > 0 {
            recorded.push(elapsed);
        }
        last = Some(result);
    }
    recorded.sort();
    Ok((
        last.expect("at least one modified-LSMR solve"),
        recorded[recorded.len() / 2],
    ))
}

#[allow(clippy::too_many_arguments)]
fn print_pcg(
    groups: usize,
    levels: usize,
    tuples: usize,
    method: &str,
    setup: Duration,
    solve: Duration,
    result: &PcgResult,
    depth: usize,
    tuple_complexity: f64,
    aggregation_kinds: &str,
) {
    println!(
        "{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{}\t{}\t{:.6e}\t{}\t{:.3}\t{}",
        groups,
        levels,
        tuples,
        method,
        setup.as_secs_f64() * 1_000.0,
        solve.as_secs_f64() * 1_000.0,
        result.iterations(),
        result.converged(),
        result.relative_residual(),
        depth,
        tuple_complexity,
        aggregation_kinds
    );
}

#[allow(clippy::too_many_arguments)]
fn print_lsmr(
    groups: usize,
    levels: usize,
    tuples: usize,
    setup: Duration,
    solve: Duration,
    result: &LeastSquaresResult,
    depth: usize,
    tuple_complexity: f64,
    aggregation_kinds: &str,
) {
    println!(
        "{}\t{}\t{}\thybrid-mlsmr\t{:.3}\t{:.3}\t{}\t{}\t{:.6e}\t{}\t{:.3}\t{}",
        groups,
        levels,
        tuples,
        setup.as_secs_f64() * 1_000.0,
        solve.as_secs_f64() * 1_000.0,
        result.iterations(),
        result.converged(),
        result.certified_normal_equation_residual(),
        depth,
        tuple_complexity,
        aggregation_kinds
    );
}

fn weak_chain_problem(
    groups: usize,
    clones: usize,
    bridge_weight: f64,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let counts = [groups * clones; 3];
    let mut tuples = Vec::with_capacity(groups * clones.pow(3) + (groups - 1) * 2 * clones);
    let mut weights = Vec::with_capacity(tuples.capacity());

    for group in 0..groups {
        for first_clone in 0..clones {
            for second_clone in 0..clones {
                for third_clone in 0..clones {
                    tuples.push([
                        (group * clones + first_clone) as u32,
                        (group * clones + second_clone) as u32,
                        (group * clones + third_clone) as u32,
                    ]);
                    weights.push(
                        0.8 + ((group + first_clone + 2 * second_clone + 3 * third_clone) % 7)
                            as f64
                            / 10.0,
                    );
                }
            }
        }
        if group + 1 < groups {
            for clone in 0..clones {
                tuples.push([
                    (group * clones + clone) as u32,
                    ((group + 1) * clones + clone) as u32,
                    ((group + 1) * clones + clone) as u32,
                ]);
                weights.push(bridge_weight);
                tuples.push([
                    ((group + 1) * clones + clone) as u32,
                    (group * clones + clone) as u32,
                    ((group + 1) * clones + (clone + 1) % clones) as u32,
                ]);
                weights.push(bridge_weight);
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        counts, &tuples, &weights,
    )?)
}

fn slow_chain_targets(
    problem: &ThreeWayProblem,
    groups: usize,
    clones: usize,
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let factor_scales = [1.0_f64, 0.7, 0.4];
    let mut coefficients = Vec::with_capacity(problem.dimension());
    for factor_scale in factor_scales {
        for group in 0..groups {
            let phase = std::f64::consts::PI * (group as f64 + 0.5) / groups as f64;
            let slow_mode = phase.sin();
            for clone in 0..clones {
                coefficients.push(factor_scale * slow_mode + 0.01 * clone as f64);
            }
        }
    }
    problem
        .components()
        .project_structural_range(&mut coefficients)?;
    let mut targets = vec![0.0; problem.tuple_count()];
    problem.apply_incidence(&coefficients, &mut targets)?;
    Ok(targets)
}
