//! Tests for the two-stage bootstrap acceptance portfolio.

use multiway_mg::{
    BootstrapAcceptanceScreen, BootstrapAggregationOptions, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, DiagonalPreconditioner, FactorAggregation,
    SecondaryScreenStructuralRejection, SymmetricMapPreconditioner, ThreeWayProblem,
    build_screened_bootstrap_aggregation,
};

#[test]
fn secondary_map_screen_can_rescue_a_structurally_valid_primary_rejection() {
    let (problem, _oracle) = refined_weak_chain(8, 2, 0.01);
    let primary = DiagonalPreconditioner::new(&problem, 0.5).expect("Jacobi screen succeeds");
    let secondary = SymmetricMapPreconditioner::new(problem.clone());
    let result = build_screened_bootstrap_aggregation(
        &problem,
        &primary,
        &secondary,
        strict_primary_options(0.80),
        secondary_criteria(),
    )
    .expect("screened bootstrap succeeds");

    assert!(!result.primary_result().accepted());
    assert!(result.accepted());
    assert!(matches!(
        result.acceptance_screen(),
        BootstrapAcceptanceScreen::SecondaryBootstrapFinal
            | BootstrapAcceptanceScreen::SecondaryStructuralBaseline
    ));
    let selected = result
        .selected_secondary_evaluation()
        .expect("secondary evaluation selected");
    assert!(selected.accepted());
    assert!(selected.structural_rejection().is_none());
    assert!(selected.compatible_report().is_some());
    assert!(selected.compatible_decision().is_some());
    let work = result.secondary_work_report();
    assert!(work.candidate_maps_considered() >= 1);
    assert!(work.compatible_gramian_applications() > 0);
    assert!(work.compatible_smoother_applications() > 0);
    assert!(work.retained_report_bytes_estimate() > 0);
}

#[test]
fn secondary_screen_never_bypasses_the_coarse_dimension_budget() {
    let (problem, _oracle) = refined_weak_chain(8, 2, 0.01);
    let primary = DiagonalPreconditioner::new(&problem, 0.5).expect("Jacobi screen succeeds");
    let secondary = SymmetricMapPreconditioner::new(problem.clone());
    let result = build_screened_bootstrap_aggregation(
        &problem,
        &primary,
        &secondary,
        strict_primary_options(0.49),
        secondary_criteria(),
    )
    .expect("screened bootstrap returns a decision");

    assert!(!result.accepted());
    assert_eq!(
        result.acceptance_screen(),
        BootstrapAcceptanceScreen::Rejected
    );
    assert!(!result.secondary_evaluations().is_empty());
    assert!(result.secondary_evaluations().iter().all(|evaluation| {
        matches!(
            evaluation.structural_rejection(),
            Some(SecondaryScreenStructuralRejection::CoarseDimension { .. })
                | Some(SecondaryScreenStructuralRejection::NoCompatibleComplement)
        )
    }));
    assert_eq!(
        result
            .secondary_work_report()
            .compatible_gramian_applications(),
        0
    );
}

#[test]
fn screened_bootstrap_is_invariant_to_observation_order() {
    let (problem, oracle, tuples, weights) = refined_weak_chain_parts(8, 2, 0.01);
    let mut reversed_tuples = tuples;
    let mut reversed_weights = weights;
    reversed_tuples.reverse();
    reversed_weights.reverse();
    let reversed = ThreeWayProblem::from_observations(
        problem.topology().level_counts(),
        &reversed_tuples,
        &reversed_weights,
    )
    .expect("reversed problem is valid");
    assert_eq!(problem, reversed);

    let options = strict_primary_options(0.80);
    let first = build_screened_bootstrap_aggregation(
        &problem,
        &DiagonalPreconditioner::new(&problem, 0.5).expect("first Jacobi screen succeeds"),
        &SymmetricMapPreconditioner::new(problem.clone()),
        options,
        secondary_criteria(),
    )
    .expect("first screened build succeeds");
    let second = build_screened_bootstrap_aggregation(
        &reversed,
        &DiagonalPreconditioner::new(&reversed, 0.5)
            .expect("second Jacobi screen succeeds"),
        &SymmetricMapPreconditioner::new(reversed.clone()),
        options,
        secondary_criteria(),
    )
    .expect("second screened build succeeds");

    assert_eq!(first, second);
    assert_eq!(first.final_aggregation(), &oracle);
}

fn strict_primary_options(maximum_coarse_dimension_ratio: f64) -> BootstrapAggregationOptions {
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
        split_repair: None,
        seed: 0x4d57_4d47_5054_4631,
    }
}

fn secondary_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.95,
        maximum_energy_factor_per_sweep: Some(0.95),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
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
