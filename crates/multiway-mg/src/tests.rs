use super::*;

fn manufactured_problem(groups: usize, clones: usize) -> ThreeWayProblem {
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
    ThreeWayProblem::from_observations(counts, &tuples, &weights)
        .expect("manufactured problem is valid")
}

fn exact_targets(problem: &ThreeWayProblem) -> Vec<f64> {
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
        .project_structural_range(&mut coefficients)
        .expect("coefficient normalization succeeds");
    let mut targets = vec![0.0; problem.tuple_count()];
    problem
        .apply_incidence(&coefficients, &mut targets)
        .expect("incidence application succeeds");
    targets
}

fn hierarchy_options() -> HierarchyOptions {
    HierarchyOptions {
        terminal_dimension: 24,
        minimum_dimension_reduction: 0.01,
        minimum_tuple_reduction: 0.0,
        aggregation: AggregationStrategy::Affinity(AffinityAggregationOptions {
            minimum_affinity: 0.5,
            maximum_context_degree: 16,
        }),
        ..HierarchyOptions::default()
    }
}

#[test]
fn affinity_matching_recovers_manufactured_clone_pairs() {
    let problem = manufactured_problem(6, 2);
    let aggregation = build_affinity_aggregation(
        &problem,
        AffinityAggregationOptions {
            minimum_affinity: 0.5,
            maximum_context_degree: 16,
        },
    )
    .expect("affinity aggregation succeeds");
    assert_eq!(aggregation.coarse_counts(), [6, 6, 6]);
    for factor in 0..3 {
        for group in 0..6 {
            assert_eq!(
                aggregation.parents(factor)[2 * group],
                aggregation.parents(factor)[2 * group + 1]
            );
        }
    }
}

#[test]
fn dense_terminal_solves_a_consistent_gramian_system() {
    let problem = manufactured_problem(3, 1);
    let targets = exact_targets(&problem);
    let rhs = problem
        .rhs_from_targets(&targets)
        .expect("normal rhs construction succeeds");
    let terminal = DensePseudoinverse::from_problem(&problem, 1.0e-12)
        .expect("terminal factorization succeeds");
    let mut solution = vec![0.0; problem.dimension()];
    terminal
        .solve_into(&rhs, &mut solution)
        .expect("terminal solve succeeds");
    let residual = problem
        .residual(&rhs, &solution)
        .expect("residual computation succeeds");
    assert!(euclidean_norm(&residual) / euclidean_norm(&rhs) < 1.0e-10);
}

#[test]
fn automatic_hierarchy_is_symmetric_to_roundoff() {
    let problem = manufactured_problem(6, 2);
    let hierarchy = ThreeWayHierarchy::build(problem.clone(), hierarchy_options())
        .expect("hierarchy construction succeeds");
    assert_eq!(hierarchy.depth(), 1);
    assert!(hierarchy.report().tuple_complexity() <= 2.0);

    let mut left: Vec<f64> = (0..problem.dimension())
        .map(|index| (index as f64 * 0.31).sin())
        .collect();
    let mut right: Vec<f64> = (0..problem.dimension())
        .map(|index| (index as f64 * 0.17).cos())
        .collect();
    problem
        .components()
        .project_structural_range(&mut left)
        .expect("left projection succeeds");
    problem
        .components()
        .project_structural_range(&mut right)
        .expect("right projection succeeds");
    let mut applied_left = vec![0.0; problem.dimension()];
    let mut applied_right = vec![0.0; problem.dimension()];
    hierarchy
        .apply(&left, &mut applied_left)
        .expect("left hierarchy application succeeds");
    hierarchy
        .apply(&right, &mut applied_right)
        .expect("right hierarchy application succeeds");
    let forward = dot(&left, &applied_right);
    let reverse = dot(&applied_left, &right);
    let scale = forward.abs().max(reverse.abs()).max(1.0);
    assert!((forward - reverse).abs() / scale < 1.0e-10);
}

#[test]
fn hierarchy_preconditioned_pcg_certifies_the_original_gramian() {
    let problem = manufactured_problem(6, 2);
    let targets = exact_targets(&problem);
    let rhs = problem
        .rhs_from_targets(&targets)
        .expect("normal rhs construction succeeds");
    let hierarchy = ThreeWayHierarchy::build(problem.clone(), hierarchy_options())
        .expect("hierarchy construction succeeds");
    let result = solve_projected_pcg(
        &problem,
        &rhs,
        &hierarchy,
        PcgOptions {
            relative_tolerance: 1.0e-9,
            max_iterations: 200,
            ..PcgOptions::default()
        },
    )
    .expect("projected PCG succeeds");
    assert!(result.converged());
    assert!(result.relative_residual() < 1.0e-9);
}

#[cfg(feature = "cmg")]
#[test]
fn pair_cmg_and_hybrid_are_symmetric_and_usable() {
    let problem = manufactured_problem(6, 2);
    let pair = PairCmgPreconditioner::build(problem.clone(), PairCmgOptions::default())
        .expect("pair CMG construction succeeds");
    let hybrid = HybridPairVcycle::build(
        problem.clone(),
        hierarchy_options(),
        PairCmgOptions::default(),
    )
    .expect("hybrid construction succeeds");

    for preconditioner in [&pair as &dyn Preconditioner, &hybrid as &dyn Preconditioner] {
        let mut left: Vec<f64> = (0..problem.dimension())
            .map(|index| (index as f64 * 0.23).sin())
            .collect();
        let mut right: Vec<f64> = (0..problem.dimension())
            .map(|index| (index as f64 * 0.41).cos())
            .collect();
        problem
            .components()
            .project_structural_range(&mut left)
            .expect("left projection succeeds");
        problem
            .components()
            .project_structural_range(&mut right)
            .expect("right projection succeeds");
        let mut applied_left = vec![0.0; problem.dimension()];
        let mut applied_right = vec![0.0; problem.dimension()];
        preconditioner
            .apply(&left, &mut applied_left)
            .expect("left application succeeds");
        preconditioner
            .apply(&right, &mut applied_right)
            .expect("right application succeeds");
        let forward = dot(&left, &applied_right);
        let reverse = dot(&applied_left, &right);
        let scale = forward.abs().max(reverse.abs()).max(1.0);
        assert!((forward - reverse).abs() / scale < 1.0e-9);
    }

    let targets = exact_targets(&problem);
    let rhs = problem
        .rhs_from_targets(&targets)
        .expect("normal rhs construction succeeds");
    let result = solve_projected_pcg(
        &problem,
        &rhs,
        &hybrid,
        PcgOptions {
            relative_tolerance: 1.0e-8,
            max_iterations: 200,
            ..PcgOptions::default()
        },
    )
    .expect("hybrid PCG succeeds");
    assert!(result.converged());
}

#[cfg(feature = "lsmr")]
#[test]
fn rectangular_lsmr_has_an_independent_certificate() {
    let problem = manufactured_problem(6, 2);
    let targets = exact_targets(&problem);
    let hierarchy = ThreeWayHierarchy::build(problem.clone(), hierarchy_options())
        .expect("hierarchy construction succeeds");
    let result = solve_weighted_least_squares(
        &problem,
        &targets,
        &hierarchy,
        LeastSquaresOptions {
            tolerance: 1.0e-9,
            max_iterations: 300,
            local_size: Some(8),
        },
    )
    .expect("modified LSMR succeeds");
    assert!(result.converged());
    assert!(result.certified_normal_equation_residual() < 1.0e-8);
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn euclidean_norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}
