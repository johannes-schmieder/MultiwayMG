use multiway_mg::{
    AffinityAggregationOptions, AggregationStrategy, DiagonalPreconditioner, HierarchyOptions,
    PcgOptions, Preconditioner, ThreeWayHierarchy, ThreeWayProblem, solve_projected_pcg,
};

#[cfg(feature = "cmg")]
use multiway_mg::{HybridPairVcycle, PairCmgOptions, PairCmgPreconditioner};
#[cfg(feature = "lsmr")]
use multiway_mg::{LeastSquaresOptions, solve_weighted_least_squares};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let problem = manufactured_problem(12, 2)?;
    let targets = exact_targets(&problem)?;
    let rhs = problem.rhs_from_targets(&targets)?;
    let hierarchy_options = HierarchyOptions {
        terminal_dimension: 40,
        minimum_dimension_reduction: 0.01,
        minimum_tuple_reduction: 0.0,
        aggregation: AggregationStrategy::Affinity(AffinityAggregationOptions {
            minimum_affinity: 0.5,
            maximum_context_degree: 16,
        }),
        ..HierarchyOptions::default()
    };
    let hierarchy = ThreeWayHierarchy::build(problem.clone(), hierarchy_options.clone())?;
    let diagonal = DiagonalPreconditioner::new(&problem, 0.5)?;

    println!("MultiwayMG feasibility probe");
    println!("  finest dimensions: {:?}", problem.topology().level_counts());
    println!("  finest unique tuples: {}", problem.tuple_count());
    println!("  hierarchy dimensions: {:?}", hierarchy.report().dimensions());
    println!("  hierarchy tuple counts: {:?}", hierarchy.report().tuple_counts());
    println!("  tuple complexity: {:.3}", hierarchy.report().tuple_complexity());

    run_pcg("diagonal", &problem, &rhs, &diagonal)?;
    run_pcg("three-way-vcycle", &problem, &rhs, &hierarchy)?;

    #[cfg(feature = "cmg")]
    {
        let pair = PairCmgPreconditioner::build(problem.clone(), PairCmgOptions::default())?;
        let hybrid = HybridPairVcycle::build(
            problem.clone(),
            hierarchy_options,
            PairCmgOptions::default(),
        )?;
        run_pcg("pair-cmg", &problem, &rhs, &pair)?;
        run_pcg("pair-cmg-plus-vcycle", &problem, &rhs, &hybrid)?;

        #[cfg(feature = "lsmr")]
        {
            let result = solve_weighted_least_squares(
                &problem,
                &targets,
                &hybrid,
                LeastSquaresOptions {
                    tolerance: 1.0e-9,
                    max_iterations: 500,
                    local_size: Some(8),
                },
            )?;
            println!(
                "  {:<24} iterations={:<4} solver_converged={} certified_normal_residual={:.3e}",
                "hybrid-mlsmr",
                result.iterations(),
                result.converged(),
                result.certified_normal_equation_residual()
            );
        }
    }
    Ok(())
}

fn run_pcg(
    name: &str,
    problem: &ThreeWayProblem,
    rhs: &[f64],
    preconditioner: &dyn Preconditioner,
) -> Result<(), Box<dyn std::error::Error>> {
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
    println!(
        "  {:<24} iterations={:<4} converged={} relative_residual={:.3e}",
        name,
        result.iterations(),
        result.converged(),
        result.relative_residual()
    );
    Ok(())
}

fn manufactured_problem(
    groups: usize,
    clones: usize,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let counts = [groups * clones; 3];
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for a in 0..groups {
        for b in 0..groups {
            let c = (a + b) % groups;
            for da in 0..clones {
                for db in 0..clones {
                    for dc in 0..clones {
                        tuples.push([
                            (a * clones + da) as u32,
                            (b * clones + db) as u32,
                            (c * clones + dc) as u32,
                        ]);
                        weights.push(1.0 + ((a + 2 * b + da + db + dc) % 7) as f64 / 10.0);
                    }
                }
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(counts, &tuples, &weights)?)
}

fn exact_targets(
    problem: &ThreeWayProblem,
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
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
