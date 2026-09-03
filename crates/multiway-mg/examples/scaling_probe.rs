//! Larger recursive-scaling probe for the first MultiwayMG research version.

use std::time::{Duration, Instant};

use multiway_mg::{
    DiagonalPreconditioner, HierarchyOptions, HybridPairVcycle, LeastSquaresOptions,
    LeastSquaresResult, PairCmgOptions, PcgOptions, PcgResult, Preconditioner, ThreeWayProblem,
    solve_projected_pcg, solve_weighted_least_squares,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "groups\tlevels\ttuples\tmethod\tsetup_ms\tmedian_solve_ms\titerations\tconverged\tcertified_residual\thierarchy_depth\ttuple_complexity\taggregation_kinds"
    );
    for groups in [16_usize, 32, 64, 128] {
        run_case(groups, 2)?;
    }
    Ok(())
}

fn run_case(groups: usize, clones: usize) -> Result<(), Box<dyn std::error::Error>> {
    let problem = manufactured_problem(groups, clones)?;
    let targets = exact_targets(&problem)?;
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

    let hierarchy_options = HierarchyOptions {
        terminal_dimension: 60,
        minimum_dimension_reduction: 0.01,
        minimum_tuple_reduction: 0.0,
        ..HierarchyOptions::default()
    };
    let setup_start = Instant::now();
    let hybrid = HybridPairVcycle::build(
        problem.clone(),
        hierarchy_options,
        PairCmgOptions::default(),
    )?;
    let hybrid_setup = setup_start.elapsed();
    let report = hybrid.hierarchy().report();
    let aggregation_kinds = format!("{:?}", report.aggregation_kinds());

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
        report.tuple_complexity(),
        &aggregation_kinds,
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
        report.tuple_complexity(),
        &aggregation_kinds,
    );
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
                max_iterations: 500,
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
                max_iterations: 500,
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

fn manufactured_problem(
    groups: usize,
    clones: usize,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let counts = [groups * clones; 3];
    let mut tuples = Vec::with_capacity(groups * groups * clones * clones * clones);
    let mut weights = Vec::with_capacity(tuples.capacity());
    for first in 0..groups {
        for second in 0..groups {
            let third = (first + second) % groups;
            for first_clone in 0..clones {
                for second_clone in 0..clones {
                    for third_clone in 0..clones {
                        tuples.push([
                            (first * clones + first_clone) as u32,
                            (second * clones + second_clone) as u32,
                            (third * clones + third_clone) as u32,
                        ]);
                        weights.push(
                            0.5 + ((11 * first
                                + 7 * second
                                + 5 * first_clone
                                + 3 * second_clone
                                + third_clone)
                                % 17) as f64
                                / 10.0,
                        );
                    }
                }
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        counts, &tuples, &weights,
    )?)
}

fn exact_targets(problem: &ThreeWayProblem) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let counts = problem.topology().level_counts();
    let mut coefficients = Vec::with_capacity(problem.dimension());
    for factor in 0..3 {
        for level in 0..counts[factor] {
            coefficients.push(
                ((factor + 1) as f64 * 0.37 + level as f64 * 0.11).sin()
                    + (level as f64 * 0.07).cos(),
            );
        }
    }
    problem
        .components()
        .project_structural_range(&mut coefficients)?;
    let mut targets = vec![0.0; problem.tuple_count()];
    problem.apply_incidence(&coefficients, &mut targets)?;
    Ok(targets)
}
