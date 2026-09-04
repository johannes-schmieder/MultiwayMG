//! Tests for complete-cycle screening of bootstrap aggregation candidates.

use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, CyclePortfolioStructuralRejection, CycleQualityCriteria,
    CycleQualityOptions, DiagonalPreconditioner, FactorAggregation,
    SymmetricMapPreconditioner, SymmetricTwoGridPreconditioner, ThreeWayProblem,
    build_cycle_screened_bootstrap_aggregation,
};

#[test]
fn complete_cycle_screen_accepts_the_protected_oracle_quality_map() {
    let (problem, oracle) = refined_weak_chain(10, 2, 0.005);
    let primary = DiagonalPreconditioner::new(&problem, 0.5).expect("Jacobi succeeds");
    let result = build_cycle_screened_bootstrap_aggregation(
        &problem,
        &primary,
        bootstrap_options(0.80),
        probe_options(),
        probe_criteria(),
        |aggregation| map_cycle(&problem, aggregation),
    )
    .expect("cycle-screened bootstrap succeeds");

    assert!(result.accepted());
    assert_eq!(result.final_aggregation(), &oracle);
    let selected = result
        .selected_evaluation()
        .expect("accepted evaluation exists");
    assert!(selected.accepted());
    assert!(selected.cycle_build_error().is_none());
    assert!(
        selected
            .cycle_report()
            .expect("cycle report exists")
            .maximum_estimated_energy_factor()
            < 0.40
    );
    let work = result.work_report();
    assert!(work.candidate_maps_considered() >= 1);
    assert!(work.cycle_builds_attempted() >= 1);
    assert_eq!(work.cycle_build_failures(), 0);
    assert!(work.probe_gramian_applications() > 0);
    assert!(work.probe_preconditioner_applications() > 0);
}

#[test]
fn complete_cycle_screen_does_not_bypass_structural_budgets() {
    let (problem, _oracle) = refined_weak_chain(10, 2, 0.005);
    let primary = DiagonalPreconditioner::new(&problem, 0.5).expect("Jacobi succeeds");
    let result = build_cycle_screened_bootstrap_aggregation(
        &problem,
        &primary,
        bootstrap_options(0.49),
        probe_options(),
        probe_criteria(),
        |aggregation| map_cycle(&problem, aggregation),
    )
    .expect("cycle-screened bootstrap returns a decision");

    assert!(!result.accepted());
    assert!(result.selected_source().is_none());
    assert!(result.evaluations().iter().all(|evaluation| {
        matches!(
            evaluation.structural_rejection(),
            Some(CyclePortfolioStructuralRejection::CoarseDimension { .. })
                | Some(CyclePortfolioStructuralRejection::NoCoarseReduction)
        )
    }));
    assert_eq!(result.work_report().cycle_builds_attempted(), 0);
    assert_eq!(result.work_report().probe_gramian_applications(), 0);
}

#[test]
fn cycle_screened_selection_is_observation_order_invariant() {
    let (problem, oracle, mut tuples, mut weights) =
        refined_weak_chain_parts(8, 2, 0.01);
    tuples.reverse();
    weights.reverse();
    let reversed = ThreeWayProblem::from_observations(
        problem.topology().level_counts(),
        &tuples,
        &weights,
    )
    .expect("reversed problem is valid");
    assert_eq!(problem, reversed);

    let first = build_cycle_screened_bootstrap_aggregation(
        &problem,
        &DiagonalPreconditioner::new(&problem, 0.5).expect("first Jacobi succeeds"),
        bootstrap_options(0.80),
        probe_options(),
        probe_criteria(),
        |aggregation| map_cycle(&problem, aggregation),
    )
    .expect("first cycle portfolio succeeds");
    let second = build_cycle_screened_bootstrap_aggregation(
        &reversed,
        &DiagonalPreconditioner::new(&reversed, 0.5).expect("second Jacobi succeeds"),
        bootstrap_options(0.80),
        probe_options(),
        probe_criteria(),
        |aggregation| map_cycle(&reversed, aggregation),
    )
    .expect("second cycle portfolio succeeds");

    assert_eq!(first, second);
    assert_eq!(first.final_aggregation(), &oracle);
}

fn map_cycle(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<SymmetricTwoGridPreconditioner<SymmetricMapPreconditioner>, multiway_mg::MultiwayError>
{
    SymmetricTwoGridPreconditioner::build(
        problem.clone(),
        aggregation.clone(),
        SymmetricMapPreconditioner::new(problem.clone()),
        1,
        1.0,
        1.0e-12,
    )
}

fn probe_options() -> CycleQualityOptions {
    CycleQualityOptions {
        test_vectors: 10,
        power_iterations: 20,
        tail_iterations: 5,
        ..CycleQualityOptions::default()
    }
}

fn probe_criteria() -> CycleQualityCriteria {
    CycleQualityCriteria {
        maximum_estimated_energy_factor: 0.40,
        maximum_observed_energy_factor: Some(1.05),
        maximum_structural_defect: 1.0e-10,
    }
}

fn bootstrap_options(maximum_coarse_dimension_ratio: f64) -> BootstrapAggregationOptions {
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
        maximum_coarse_dimension_ratio,
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
            maximum_coarse_dimension_ratio,
            minimum_tuple_reduction: 0.02,
            maximum_two_level_tuple_complexity: 1.98,
            minimum_split_score_fraction: 0.001,
        }),
        seed: 0x4d57_4d47_4350_4631,
    }
}

fn refined_weak_chain(
    levels: usize,
    clones: usize,
    bridge_weight: f64,
) -> (ThreeWayProblem, FactorAggregation) {
    let (problem, oracle, _tuples, _weights) =
        refined_weak_chain_parts(levels, clones, bridge_weight);
    (problem, oracle)
}

fn refined_weak_chain_parts(
    levels: usize,
    clones: usize,
    bridge_weight: f64,
) -> (
    ThreeWayProblem,
    FactorAggregation,
    Vec<[u32; 3]>,
    Vec<f64>,
) {
    let mut coarse_tuples = Vec::new();
    let mut coarse_weights = Vec::new();
    for level in 0..levels {
        coarse_tuples.push([level as u32, level as u32, level as u32]);
        coarse_weights.push(1.0 + (level % 5) as f64 / 10.0);
        if level + 1 < levels {
            coarse_tuples.push([level as u32, (level + 1) as u32, (level + 1) as u32]);
            coarse_weights.push(bridge_weight);
            coarse_tuples.push([(level + 1) as u32, level as u32, (level + 1) as u32]);
            coarse_weights.push(bridge_weight * 1.1);
            coarse_tuples.push([(level + 1) as u32, (level + 1) as u32, level as u32]);
            coarse_weights.push(bridge_weight * 0.9);
        }
    }
    let coarse = ThreeWayProblem::from_observations([levels; 3], &coarse_tuples, &coarse_weights)
        .expect("coarse weak chain is valid");
    let fine_counts = [levels * clones; 3];
    let parents = core::array::from_fn(|_| {
        (0..levels * clones)
            .map(|level| (level / clones) as u32)
            .collect()
    });
    let oracle = FactorAggregation::new(fine_counts, parents).expect("oracle map is valid");
    let mut fine_tuples = Vec::new();
    let mut fine_weights = Vec::new();
    for (&tuple, &weight) in coarse.topology().tuples().iter().zip(coarse.weights()) {
        let child_weight = weight / (clones * clones * clones) as f64;
        for first_child in 0..clones {
            for second_child in 0..clones {
                for third_child in 0..clones {
                    fine_tuples.push([
                        (tuple[0] as usize * clones + first_child) as u32,
                        (tuple[1] as usize * clones + second_child) as u32,
                        (tuple[2] as usize * clones + third_child) as u32,
                    ]);
                    fine_weights.push(child_weight);
                }
            }
        }
    }
    let fine = ThreeWayProblem::from_observations(fine_counts, &fine_tuples, &fine_weights)
        .expect("refined weak chain is valid");
    (fine, oracle, fine_tuples, fine_weights)
}
