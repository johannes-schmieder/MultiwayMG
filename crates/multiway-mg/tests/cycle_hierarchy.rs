//! Tests for recursively cycle-screened automatic MAP hierarchies.

use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, CycleQualityCriteria, CycleQualityOptions,
    CycleScreenedHierarchyOptions, CycleScreenedHierarchyPlan, CycleScreenedHierarchyStopReason,
    DenseRangeDecomposition, FactorAggregation, PcgTraceOptions, SpectralAnalysisOptions,
    ThreeWayProblem, solve_projected_pcg_traced,
};

#[test]
fn automatic_recursive_plan_reaches_oracle_terminal_and_builds_a_strong_cycle() {
    let (problem, oracle_maps, _tuples, _weights) = two_level_refined_weak_chain(6, 0.01);
    let plan = CycleScreenedHierarchyPlan::build(problem.clone(), hierarchy_options(18, 3.0))
        .expect("recursive planning succeeds");

    assert!(plan.accepted(), "stop reason: {:?}", plan.stop_reason());
    assert_eq!(plan.depth(), 2);
    assert_eq!(plan.aggregations().len(), oracle_maps.len());
    let mut oracle_terminal = problem.clone();
    for aggregation in &oracle_maps {
        oracle_terminal = aggregation
            .coarsen(&oracle_terminal)
            .expect("oracle hierarchy coarsens exactly");
    }
    // Complete-tensor refinement leaves several equally valid sibling
    // pairings at the first level. Require the same terminal operator,
    // not one arbitrary intermediate parent labeling.
    assert_eq!(plan.terminal_problem(), &oracle_terminal);
    assert!(plan.dimension_complexity() < 2.0);
    assert!(plan.tuple_complexity() < 1.40);
    assert!(plan.level_reports().iter().all(|level| level.admitted()));

    let hierarchy = plan
        .build_preconditioner()
        .expect("accepted plan builds a hierarchy");
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 128,
        ..SpectralAnalysisOptions::default()
    };
    let spectral = DenseRangeDecomposition::from_problem(&problem, spectral_options)
        .expect("dense range decomposition succeeds")
        .analyze(&hierarchy, spectral_options)
        .expect("hierarchy spectral analysis succeeds");
    assert!(spectral.numerically_symmetric());
    assert!(spectral.positive_definite_on_range());
    assert!(spectral.preconditioned_condition_number() < 1.20);

    let rhs = deterministic_rhs(&problem);
    let solve = solve_projected_pcg_traced(
        &problem,
        &rhs,
        &hierarchy,
        PcgTraceOptions {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 0.0,
            max_iterations: 500,
        },
    )
    .expect("traced PCG succeeds");
    assert!(solve.converged());
    assert!(solve.final_relative_residual() < 1.0e-9);
    assert!(solve.iterations() <= 6);
}

#[test]
fn recursive_planning_is_observation_order_invariant() {
    let (problem, _oracle_maps, mut tuples, mut weights) = two_level_refined_weak_chain(6, 0.01);
    tuples.reverse();
    weights.reverse();
    let reversed =
        ThreeWayProblem::from_observations(problem.topology().level_counts(), &tuples, &weights)
            .expect("reversed problem is valid");
    assert_eq!(problem, reversed);

    let first = CycleScreenedHierarchyPlan::build(problem, hierarchy_options(18, 3.0))
        .expect("first plan succeeds");
    let second = CycleScreenedHierarchyPlan::build(reversed, hierarchy_options(18, 3.0))
        .expect("second plan succeeds");
    assert_eq!(first, second);
}

#[test]
fn cumulative_tuple_budget_rejects_before_admitting_the_candidate() {
    let (problem, _oracle_maps, _tuples, _weights) = two_level_refined_weak_chain(6, 0.01);
    let plan = CycleScreenedHierarchyPlan::build(problem, hierarchy_options(18, 1.05))
        .expect("bounded plan returns a decision");

    assert!(!plan.accepted());
    assert_eq!(plan.depth(), 0);
    assert_eq!(plan.tuple_complexity(), 1.0);
    assert!(matches!(
        plan.stop_reason(),
        CycleScreenedHierarchyStopReason::TupleComplexityBudget {
            level: 0,
            attempted,
            maximum,
        } if *attempted > *maximum && (*maximum - 1.05).abs() < f64::EPSILON
    ));
    assert_eq!(plan.level_reports().len(), 1);
    assert!(!plan.level_reports()[0].admitted());
    assert!(plan.build_preconditioner().is_err());
}

fn hierarchy_options(
    terminal_dimension: usize,
    maximum_tuple_complexity: f64,
) -> CycleScreenedHierarchyOptions {
    CycleScreenedHierarchyOptions {
        maximum_levels: 4,
        terminal_dimension,
        maximum_dimension_complexity: 2.5,
        maximum_tuple_complexity,
        bootstrap: bootstrap_options(),
        cycle_probe: CycleQualityOptions {
            test_vectors: 10,
            power_iterations: 20,
            tail_iterations: 5,
            ..CycleQualityOptions::default()
        },
        cycle_criteria: CycleQualityCriteria {
            maximum_estimated_energy_factor: 0.40,
            maximum_observed_energy_factor: Some(1.05),
            maximum_structural_defect: 1.0e-10,
        },
        terminal_relative_tolerance: 1.0e-12,
    }
}

fn bootstrap_options() -> BootstrapAggregationOptions {
    BootstrapAggregationOptions {
        setup_test_vectors: 8,
        setup_sweeps: 8,
        setup_jacobi_omega: 0.5,
        maximum_neighbor_degree: 16,
        signature_window: 3,
        maximum_candidate_degree: 16,
        minimum_combined_affinity: 0.35,
        algebraic_affinity_weight: 0.55,
        structural_affinity_weight: 0.25,
        degree_affinity_weight: 0.10,
        signature_hit_weight: 0.10,
        structural_baseline_required_factor_ratio: 0.97,
        structural_baseline_maximum_dimension_overhead_ratio: 0.05,
        structural_baseline_maximum_tuple_overhead_ratio: 0.10,
        compatible_relaxation: CompatibleRelaxationOptions {
            test_vectors: 10,
            sweeps: 8,
            ..CompatibleRelaxationOptions::default()
        },
        compatible_criteria: CompatibleRelaxationCriteria {
            maximum_diagonal_factor_per_sweep: 0.01,
            maximum_energy_factor_per_sweep: Some(0.01),
            maximum_final_coarse_defect: 1.0e-10,
            maximum_final_structural_defect: 1.0e-10,
        },
        maximum_bootstrap_witnesses: 2,
        maximum_coarse_dimension_ratio: 0.80,
        minimum_tuple_reduction: 0.02,
        maximum_two_level_tuple_complexity: 1.98,
        split_repair: Some(AggregationRepairOptions {
            relaxation: CompatibleRelaxationOptions {
                test_vectors: 10,
                sweeps: 8,
                ..CompatibleRelaxationOptions::default()
            },
            criteria: CompatibleRelaxationCriteria {
                maximum_diagonal_factor_per_sweep: 0.01,
                maximum_energy_factor_per_sweep: Some(0.01),
                maximum_final_coarse_defect: 1.0e-10,
                maximum_final_structural_defect: 1.0e-10,
            },
            maximum_rounds: 4,
            maximum_coarse_dimension_ratio: 0.80,
            minimum_tuple_reduction: 0.02,
            maximum_two_level_tuple_complexity: 1.98,
            minimum_split_score_fraction: 0.001,
        }),
        seed: 0x4d57_4d47_4849_4631,
    }
}

fn deterministic_rhs(problem: &ThreeWayProblem) -> Vec<f64> {
    let mut coefficients: Vec<f64> = (0..problem.dimension())
        .map(|index| {
            let x = index as f64 + 1.0;
            (0.173 * x).sin() + 0.37 * (0.071 * x).cos()
        })
        .collect();
    problem
        .components()
        .project_structural_range(&mut coefficients)
        .expect("projection succeeds");
    let mut rhs = vec![0.0; problem.dimension()];
    problem
        .apply_gramian(&coefficients, &mut rhs)
        .expect("rhs construction succeeds");
    rhs
}

fn two_level_refined_weak_chain(
    levels: usize,
    bridge_weight: f64,
) -> (
    ThreeWayProblem,
    Vec<FactorAggregation>,
    Vec<[u32; 3]>,
    Vec<f64>,
) {
    let base = weak_chain_base(levels, bridge_weight);
    let (middle, middle_to_base, _middle_tuples, _middle_weights) = refine_once(&base, 2);
    let (fine, fine_to_middle, fine_tuples, fine_weights) = refine_once(&middle, 2);
    (
        fine,
        vec![fine_to_middle, middle_to_base],
        fine_tuples,
        fine_weights,
    )
}

fn weak_chain_base(levels: usize, bridge_weight: f64) -> ThreeWayProblem {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for level in 0..levels {
        tuples.push([level as u32, level as u32, level as u32]);
        weights.push(1.0 + (level % 5) as f64 / 10.0);
        if level + 1 < levels {
            tuples.push([level as u32, (level + 1) as u32, (level + 1) as u32]);
            weights.push(bridge_weight);
            tuples.push([(level + 1) as u32, level as u32, (level + 1) as u32]);
            weights.push(bridge_weight * 1.1);
            tuples.push([(level + 1) as u32, (level + 1) as u32, level as u32]);
            weights.push(bridge_weight * 0.9);
        }
    }
    ThreeWayProblem::from_observations([levels; 3], &tuples, &weights)
        .expect("base weak chain is valid")
}

fn refine_once(
    coarse: &ThreeWayProblem,
    clones: usize,
) -> (ThreeWayProblem, FactorAggregation, Vec<[u32; 3]>, Vec<f64>) {
    let coarse_counts = coarse.topology().level_counts();
    let fine_counts = coarse_counts.map(|count| count * clones);
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (level / clones) as u32)
            .collect()
    });
    let aggregation =
        FactorAggregation::new(fine_counts, parents).expect("refinement aggregation is valid");
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for (&tuple, &weight) in coarse.topology().tuples().iter().zip(coarse.weights()) {
        let child_weight = weight / (clones * clones * clones) as f64;
        for first_child in 0..clones {
            for second_child in 0..clones {
                for third_child in 0..clones {
                    tuples.push([
                        (tuple[0] as usize * clones + first_child) as u32,
                        (tuple[1] as usize * clones + second_child) as u32,
                        (tuple[2] as usize * clones + third_child) as u32,
                    ]);
                    weights.push(child_weight);
                }
            }
        }
    }
    let fine = ThreeWayProblem::from_observations(fine_counts, &tuples, &weights)
        .expect("refined problem is valid");
    let reconstructed = aggregation
        .coarsen(&fine)
        .expect("oracle recoarsening succeeds");
    assert_eq!(reconstructed, *coarse);
    (fine, aggregation, tuples, weights)
}
