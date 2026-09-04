//! Tests for sparse deterministic bootstrap aggregation.

use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, BootstrapAggregationStopReason,
    CompatibleRelaxationCriteria, CompatibleRelaxationOptions, DiagonalPreconditioner,
    FactorAggregation, ThreeWayProblem, build_bootstrap_aggregation,
};

#[test]
fn bootstrap_is_deterministic_and_recovers_a_planted_weak_chain_partition() {
    let (problem, oracle) = refined_weak_chain(8, 2, 0.01, false);
    let smoother = DiagonalPreconditioner::new(&problem, 0.5).expect("smoother succeeds");
    let options = bootstrap_options();
    let first = build_bootstrap_aggregation(&problem, &smoother, options)
        .expect("first bootstrap build succeeds");
    let second = build_bootstrap_aggregation(&problem, &smoother, options)
        .expect("second bootstrap build succeeds");

    assert_eq!(first, second);
    assert!(first.accepted());
    assert_eq!(first.final_aggregation(), &oracle);
    assert!(matches!(
        first.stop_reason(),
        BootstrapAggregationStopReason::AcceptedInitial
            | BootstrapAggregationStopReason::AcceptedAfterBootstrap { .. }
            | BootstrapAggregationStopReason::AcceptedAfterSplitRepair { .. }
    ));
    assert!(!first.rounds().is_empty());
    assert!(first.rounds().iter().all(|round| {
        round.maximum_retained_candidate_degree() <= options.maximum_candidate_degree
    }));
    let work = first.work_report();
    assert_eq!(
        work.setup_gramian_applications(),
        options.setup_test_vectors * (options.setup_sweeps + 1)
    );
    assert_eq!(
        work.setup_smoother_applications(),
        options.setup_test_vectors * options.setup_sweeps
    );
    assert!(work.retained_test_vector_bytes() > 0);
    assert!(work.retained_round_report_bytes_estimate() > 0);
}

#[test]
fn parity_sparse_refinement_is_recovered_without_exact_shared_contexts() {
    let (problem, oracle) = refined_weak_chain(8, 2, 0.01, true);
    let smoother = DiagonalPreconditioner::new(&problem, 0.5).expect("smoother succeeds");
    let result = build_bootstrap_aggregation(&problem, &smoother, bootstrap_options())
        .expect("bootstrap build succeeds");

    assert!(result.accepted());
    assert_eq!(result.final_aggregation(), &oracle);
    let coarse = result
        .final_aggregation()
        .coarsen(&problem)
        .expect("coarsening succeeds");
    assert_eq!(coarse.dimension(), 24);
    assert_eq!(coarse.tuple_count(), 29);
}

#[test]
fn structural_dimension_budget_rejects_before_compatible_acceptance() {
    let (problem, _) = refined_weak_chain(8, 2, 0.01, false);
    let smoother = DiagonalPreconditioner::new(&problem, 0.5).expect("smoother succeeds");
    let mut options = bootstrap_options();
    options.maximum_coarse_dimension_ratio = 0.49;
    options.split_repair = None;
    let result = build_bootstrap_aggregation(&problem, &smoother, options)
        .expect("bootstrap returns a fail-closed decision");

    assert!(!result.accepted());
    assert!(matches!(
        result.stop_reason(),
        BootstrapAggregationStopReason::CoarseDimensionBudget { .. }
    ));
    assert!(result.rounds().is_empty());
}

#[test]
fn additional_nesting_nullity_does_not_create_nonfinite_bootstrap_diagnostics() {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..6_u32 {
        for second in 0..6_u32 {
            tuples.push([first, second, first]);
            weights.push(0.75 + ((3 * first + 5 * second) % 11) as f64 / 10.0);
        }
    }
    let problem = ThreeWayProblem::from_observations([6, 6, 6], &tuples, &weights)
        .expect("nested problem is valid");
    let smoother = DiagonalPreconditioner::new(&problem, 0.5).expect("smoother succeeds");
    let mut options = bootstrap_options();
    options.compatible_criteria = CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 1.05,
        maximum_energy_factor_per_sweep: Some(0.90),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    };
    options.minimum_tuple_reduction = 0.0;
    options.maximum_bootstrap_witnesses = 2;
    options.split_repair = None;
    let result = build_bootstrap_aggregation(&problem, &smoother, options)
        .expect("nested bootstrap produces a finite decision");

    for round in result.rounds() {
        assert!(
            round
                .compatible_decision()
                .maximum_diagonal_factor_per_sweep()
                .is_finite()
        );
        assert!(
            round
                .compatible_decision()
                .maximum_energy_factor_per_sweep()
                .is_none_or(f64::is_finite)
        );
        assert!(
            round
                .compatible_report()
                .maximum_final_coarse_defect()
                .is_finite()
        );
        assert!(
            round
                .compatible_report()
                .maximum_final_structural_defect()
                .is_finite()
        );
    }
}

fn bootstrap_options() -> BootstrapAggregationOptions {
    let repair = AggregationRepairOptions {
        relaxation: CompatibleRelaxationOptions {
            test_vectors: 12,
            sweeps: 10,
            ..CompatibleRelaxationOptions::default()
        },
        criteria: CompatibleRelaxationCriteria {
            maximum_diagonal_factor_per_sweep: 0.80,
            maximum_energy_factor_per_sweep: Some(0.80),
            maximum_final_coarse_defect: 1.0e-10,
            maximum_final_structural_defect: 1.0e-10,
        },
        maximum_rounds: 12,
        maximum_coarse_dimension_ratio: 0.75,
        minimum_tuple_reduction: 0.02,
        maximum_two_level_tuple_complexity: 1.98,
        minimum_split_score_fraction: 0.005,
    };
    BootstrapAggregationOptions {
        setup_test_vectors: 8,
        setup_sweeps: 8,
        setup_jacobi_omega: 0.5,
        maximum_neighbor_degree: 16,
        signature_window: 3,
        maximum_candidate_degree: 12,
        minimum_combined_affinity: 0.35,
        algebraic_affinity_weight: 0.55,
        structural_affinity_weight: 0.25,
        degree_affinity_weight: 0.10,
        signature_hit_weight: 0.10,
        compatible_relaxation: CompatibleRelaxationOptions {
            test_vectors: 12,
            sweeps: 10,
            ..CompatibleRelaxationOptions::default()
        },
        compatible_criteria: repair.criteria,
        maximum_bootstrap_witnesses: 3,
        maximum_coarse_dimension_ratio: 0.75,
        minimum_tuple_reduction: 0.02,
        maximum_two_level_tuple_complexity: 1.98,
        split_repair: Some(repair),
        seed: 0x4d57_4d47_4253_3031,
    }
}

fn refined_weak_chain(
    levels: usize,
    clones: usize,
    bridge_weight: f64,
    parity_sparse: bool,
) -> (ThreeWayProblem, FactorAggregation) {
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
    for (tuple_index, (&tuple, &weight)) in coarse
        .topology()
        .tuples()
        .iter()
        .zip(coarse.weights())
        .enumerate()
    {
        let parity = tuple_index % 2;
        let retained_children = if parity_sparse {
            clones * clones * clones / 2
        } else {
            clones * clones * clones
        };
        for first_child in 0..clones {
            for second_child in 0..clones {
                for third_child in 0..clones {
                    if parity_sparse && (first_child + second_child + third_child) % 2 != parity {
                        continue;
                    }
                    fine_tuples.push([
                        (tuple[0] as usize * clones + first_child) as u32,
                        (tuple[1] as usize * clones + second_child) as u32,
                        (tuple[2] as usize * clones + third_child) as u32,
                    ]);
                    fine_weights.push(weight / retained_children as f64);
                }
            }
        }
    }
    let fine = ThreeWayProblem::from_observations(fine_counts, &fine_tuples, &fine_weights)
        .expect("refined weak chain is valid");
    let reconstructed = oracle.coarsen(&fine).expect("oracle coarsening succeeds");
    assert_eq!(
        reconstructed.topology().tuples(),
        coarse.topology().tuples()
    );
    for (&expected, &actual) in coarse.weights().iter().zip(reconstructed.weights()) {
        assert!((expected - actual).abs() <= 1.0e-12 * expected.abs().max(1.0));
    }
    (fine, oracle)
}
