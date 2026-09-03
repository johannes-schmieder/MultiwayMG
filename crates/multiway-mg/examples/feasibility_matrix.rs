//! Broader deterministic feasibility matrix for the first MultiwayMG prototype.

use std::time::Instant;

use multiway_mg::{
    AffinityAggregationOptions, AggregationStrategy, DiagonalPreconditioner, FactorAggregation,
    HierarchyOptions, HybridPairVcycle, LeastSquaresOptions, PairCmgOptions, PairCmgPreconditioner,
    PairNeighborhoodAggregationOptions, PcgOptions, Preconditioner, ThreeWayHierarchy,
    ThreeWayProblem, build_affinity_aggregation, build_pair_neighborhood_aggregation,
    solve_projected_pcg, solve_weighted_least_squares,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "case\tlevels\ttuples\tcomponents\taggregation\tcoarse_levels\tcoarse_tuples\tmethod\tsetup_ms\tsolve_ms\titerations\tconverged\tcertified_residual\tnote"
    );
    for case in cases()? {
        run_case(&case)?;
    }
    Ok(())
}

struct Case {
    name: &'static str,
    problem: ThreeWayProblem,
    targets: Vec<f64>,
}

fn run_case(case: &Case) -> Result<(), Box<dyn std::error::Error>> {
    let problem = &case.problem;
    let rhs = problem.rhs_from_targets(&case.targets)?;
    let levels = problem.topology().level_counts().iter().sum::<usize>();

    let exact = build_affinity_aggregation(problem, AffinityAggregationOptions::default())?;
    let exact_coarse = exact.coarsen(problem)?;
    let (aggregation_name, aggregation, coarse) = if exact_coarse.dimension() < problem.dimension()
        && exact_coarse.tuple_count() < problem.tuple_count()
    {
        ("exact-context", exact, exact_coarse)
    } else {
        let neighborhood = build_pair_neighborhood_aggregation(
            problem,
            PairNeighborhoodAggregationOptions::default(),
        )?;
        let coarse = neighborhood.coarsen(problem)?;
        ("pair-neighborhood", neighborhood, coarse)
    };
    let coarse_levels = coarse.dimension();
    let coarse_tuples = coarse.tuple_count();

    let start = Instant::now();
    let diagonal = DiagonalPreconditioner::new(problem, 0.5)?;
    let setup = start.elapsed();
    run_pcg(
        case,
        levels,
        aggregation_name,
        coarse_levels,
        coarse_tuples,
        "diagonal-pcg",
        setup,
        &rhs,
        &diagonal,
    );

    let start = Instant::now();
    let pair = PairCmgPreconditioner::build(problem.clone(), PairCmgOptions::default())?;
    let setup = start.elapsed();
    run_pcg(
        case,
        levels,
        aggregation_name,
        coarse_levels,
        coarse_tuples,
        "pair-cmg-pcg",
        setup,
        &rhs,
        &pair,
    );

    let hierarchy_options = hierarchy_options(aggregation.clone());
    let start = Instant::now();
    let hierarchy = ThreeWayHierarchy::build(problem.clone(), hierarchy_options.clone())?;
    let setup = start.elapsed();
    run_pcg(
        case,
        levels,
        aggregation_name,
        coarse_levels,
        coarse_tuples,
        "three-way-vcycle-pcg",
        setup,
        &rhs,
        &hierarchy,
    );

    let start = Instant::now();
    let hybrid = HybridPairVcycle::build(
        problem.clone(),
        hierarchy_options,
        PairCmgOptions::default(),
    )?;
    let setup = start.elapsed();
    run_pcg(
        case,
        levels,
        aggregation_name,
        coarse_levels,
        coarse_tuples,
        "hybrid-pcg",
        setup,
        &rhs,
        &hybrid,
    );

    let start = Instant::now();
    let result = solve_weighted_least_squares(
        problem,
        &case.targets,
        &hybrid,
        LeastSquaresOptions {
            tolerance: 1.0e-9,
            max_iterations: 500,
            local_size: Some(8),
        },
    );
    let solve = start.elapsed();
    match result {
        Ok(result) => print_row(
            case,
            levels,
            aggregation_name,
            coarse_levels,
            coarse_tuples,
            "hybrid-mlsmr",
            setup,
            solve,
            result.iterations(),
            result.converged(),
            result.certified_normal_equation_residual(),
            "rectangular-rank-robust",
        ),
        Err(error) => print_error(
            case,
            levels,
            aggregation_name,
            coarse_levels,
            coarse_tuples,
            "hybrid-mlsmr",
            setup,
            solve,
            &error,
        ),
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_pcg(
    case: &Case,
    levels: usize,
    aggregation: &str,
    coarse_levels: usize,
    coarse_tuples: usize,
    method: &str,
    setup: std::time::Duration,
    rhs: &[f64],
    preconditioner: &dyn Preconditioner,
) {
    let start = Instant::now();
    let result = solve_projected_pcg(
        &case.problem,
        rhs,
        preconditioner,
        PcgOptions {
            relative_tolerance: 1.0e-9,
            max_iterations: 500,
            ..PcgOptions::default()
        },
    );
    let solve = start.elapsed();
    match result {
        Ok(result) => print_row(
            case,
            levels,
            aggregation,
            coarse_levels,
            coarse_tuples,
            method,
            setup,
            solve,
            result.iterations(),
            result.converged(),
            result.relative_residual(),
            "projected-gramian",
        ),
        Err(error) => print_error(
            case,
            levels,
            aggregation,
            coarse_levels,
            coarse_tuples,
            method,
            setup,
            solve,
            &error,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn print_row(
    case: &Case,
    levels: usize,
    aggregation: &str,
    coarse_levels: usize,
    coarse_tuples: usize,
    method: &str,
    setup: std::time::Duration,
    solve: std::time::Duration,
    iterations: usize,
    converged: bool,
    residual: f64,
    note: &str,
) {
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{}\t{}\t{:.6e}\t{}",
        case.name,
        levels,
        case.problem.tuple_count(),
        case.problem.components().count(),
        aggregation,
        coarse_levels,
        coarse_tuples,
        method,
        setup.as_secs_f64() * 1_000.0,
        solve.as_secs_f64() * 1_000.0,
        iterations,
        converged,
        residual,
        note
    );
}

#[allow(clippy::too_many_arguments)]
fn print_error(
    case: &Case,
    levels: usize,
    aggregation: &str,
    coarse_levels: usize,
    coarse_tuples: usize,
    method: &str,
    setup: std::time::Duration,
    solve: std::time::Duration,
    error: &dyn std::error::Error,
) {
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t0\tfalse\tNaN\t{}",
        case.name,
        levels,
        case.problem.tuple_count(),
        case.problem.components().count(),
        aggregation,
        coarse_levels,
        coarse_tuples,
        method,
        setup.as_secs_f64() * 1_000.0,
        solve.as_secs_f64() * 1_000.0,
        error.to_string().replace('\t', " ")
    );
}

fn hierarchy_options(aggregation: FactorAggregation) -> HierarchyOptions {
    HierarchyOptions {
        terminal_dimension: 60,
        minimum_dimension_reduction: 0.01,
        minimum_tuple_reduction: 0.0,
        aggregation: AggregationStrategy::Supplied(vec![aggregation]),
        ..HierarchyOptions::default()
    }
}

fn cases() -> Result<Vec<Case>, Box<dyn std::error::Error>> {
    let problems = [
        ("planted-clones", planted_clones(12, 2)?),
        ("noisy-clones", noisy_clones(8, 3)?),
        ("latin-square", latin_square(24, 0)?),
        ("weak-chain", weak_chain(12, 2)?),
        ("nested-third-factor", nested_third_factor(24)?),
        ("disconnected-latin", disconnected_latin(12)?),
    ];
    problems
        .into_iter()
        .map(|(name, problem)| {
            let targets = exact_targets(&problem)?;
            Ok(Case {
                name,
                problem,
                targets,
            })
        })
        .collect()
}

fn planted_clones(
    groups: usize,
    clones: usize,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
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
                            1.0 + ((first + 2 * second + first_clone + second_clone + third_clone)
                                % 7) as f64
                                / 10.0,
                        );
                    }
                }
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [groups * clones; 3],
        &tuples,
        &weights,
    )?)
}

fn noisy_clones(
    groups: usize,
    clones: usize,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..groups {
        for second in 0..groups {
            let third = (first + second) % groups;
            for first_clone in 0..clones {
                for second_clone in 0..clones {
                    for third_clone in 0..clones {
                        if (first_clone + 2 * second_clone + 3 * third_clone + first + 2 * second)
                            % 4
                            == 0
                        {
                            continue;
                        }
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
                                % 13) as f64
                                / 10.0,
                        );
                    }
                }
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [groups * clones; 3],
        &tuples,
        &weights,
    )?)
}

fn latin_square(levels: usize, offset: u32) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let tuples = latin_square_tuples(levels as u32, offset);
    let weights: Vec<f64> = (0..tuples.len())
        .map(|index| 0.8 + (index % 11) as f64 / 10.0)
        .collect();
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn weak_chain(groups: usize, clones: usize) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
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
                        1.0 + ((group + first_clone + 2 * second_clone + third_clone) % 7) as f64
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
                weights.push(0.05);
                tuples.push([
                    ((group + 1) * clones + clone) as u32,
                    (group * clones + clone) as u32,
                    ((group + 1) * clones + (clone + 1) % clones) as u32,
                ]);
                weights.push(0.05);
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [groups * clones; 3],
        &tuples,
        &weights,
    )?)
}

fn nested_third_factor(levels: usize) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([first as u32, second as u32, first as u32]);
            weights.push(1.0 + ((7 * first + 3 * second) % 13) as f64 / 10.0);
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn disconnected_latin(
    levels_per_component: usize,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = latin_square_tuples(levels_per_component as u32, 0);
    tuples.extend(latin_square_tuples(
        levels_per_component as u32,
        levels_per_component as u32,
    ));
    let weights: Vec<f64> = (0..tuples.len())
        .map(|index| 0.9 + (index % 9) as f64 / 10.0)
        .collect();
    Ok(ThreeWayProblem::from_observations(
        [2 * levels_per_component; 3],
        &tuples,
        &weights,
    )?)
}

fn latin_square_tuples(levels: u32, offset: u32) -> Vec<[u32; 3]> {
    let mut tuples = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([
                first + offset,
                second + offset,
                (first + second) % levels + offset,
            ]);
        }
    }
    tuples
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
