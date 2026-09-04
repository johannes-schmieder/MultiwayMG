//! Witness-driven bounded aggregate-repair tests.

use multiway_mg::{
    AggregationRepairOptions, AggregationRepairStopReason, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, DiagonalPreconditioner, FactorAggregation, ThreeWayProblem,
    repair_aggregation_by_splitting,
};

#[test]
fn oracle_weak_chain_map_is_accepted_without_repair() {
    let (problem, oracle) = refined_weak_chain(8, 2, 0.01);
    let smoother =
        DiagonalPreconditioner::new(&problem, 0.5).expect("diagonal smoother succeeds");
    let result = repair_aggregation_by_splitting(
        &problem,
        &oracle,
        &smoother,
        repair_options(12, 0.75),
    )
    .expect("repair evaluation succeeds");

    assert!(result.accepted());
    assert_eq!(result.accepted_splits(), 0);
    assert_eq!(result.final_aggregation(), &oracle);
    assert!(matches!(
        result.stop_reason(),
        AggregationRepairStopReason::AlreadyAccepted
    ));
    assert_eq!(result.rounds().len(), 1);
    assert!(result.rounds()[0].decision().accepted());
}

#[test]
fn overmerged_weak_chain_is_repaired_under_explicit_budgets() {
    let (problem, oracle) = refined_weak_chain(8, 2, 0.01);
    let overmerged = overmerged_pairs(&oracle);
    let smoother =
        DiagonalPreconditioner::new(&problem, 0.5).expect("diagonal smoother succeeds");
    let first = repair_aggregation_by_splitting(
        &problem,
        &overmerged,
        &smoother,
        repair_options(12, 0.75),
    )
    .expect("first repair succeeds");
    let second = repair_aggregation_by_splitting(
        &problem,
        &overmerged,
        &smoother,
        repair_options(12, 0.75),
    )
    .expect("second repair succeeds");

    assert_eq!(first, second);
    assert!(first.accepted());
    assert!(first.accepted_splits() > 0);
    assert!(first.accepted_splits() <= 12);
    assert!(matches!(
        first.stop_reason(),
        AggregationRepairStopReason::Accepted
    ));
    let initial_dimension: usize = overmerged.coarse_counts().iter().sum();
    let final_dimension: usize = first.final_aggregation().coarse_counts().iter().sum();
    assert!(final_dimension > initial_dimension);
    assert!(final_dimension <= (0.75 * problem.dimension() as f64) as usize);
    let first_factor = first.rounds()[0]
        .decision()
        .maximum_diagonal_factor_per_sweep();
    let final_factor = first
        .rounds()
        .last()
        .expect("accepted round exists")
        .decision()
        .maximum_diagonal_factor_per_sweep();
    assert!(first_factor > 0.75);
    assert!(final_factor <= 0.75);
    assert!(first
        .rounds()
        .iter()
        .take(first.rounds().len() - 1)
        .all(|round| round.proposed_split().is_some()));
}

#[test]
fn coarse_dimension_budget_rejects_a_split_before_mutation() {
    let (problem, oracle) = refined_weak_chain(8, 2, 0.01);
    let overmerged = overmerged_pairs(&oracle);
    let initial_dimension: usize = overmerged.coarse_counts().iter().sum();
    let exact_ratio = initial_dimension as f64 / problem.dimension() as f64;
    let smoother =
        DiagonalPreconditioner::new(&problem, 0.5).expect("diagonal smoother succeeds");
    let result = repair_aggregation_by_splitting(
        &problem,
        &overmerged,
        &smoother,
        repair_options(12, exact_ratio),
    )
    .expect("bounded repair returns a decision");

    assert!(!result.accepted());
    assert_eq!(result.accepted_splits(), 0);
    assert_eq!(result.final_aggregation(), &overmerged);
    assert!(matches!(
        result.stop_reason(),
        AggregationRepairStopReason::CoarseDimensionBudget {
            attempted_dimension,
            maximum_dimension,
        } if *attempted_dimension == initial_dimension + 1
            && *maximum_dimension == initial_dimension
    ));
    assert!(result.rounds()[0].proposed_split().is_some());
}

fn repair_options(maximum_rounds: usize, maximum_coarse_dimension_ratio: f64) -> AggregationRepairOptions {
    AggregationRepairOptions {
        relaxation: CompatibleRelaxationOptions {
            test_vectors: 12,
            sweeps: 10,
            ..CompatibleRelaxationOptions::default()
        },
        criteria: CompatibleRelaxationCriteria {
            maximum_diagonal_factor_per_sweep: 0.75,
            maximum_energy_factor_per_sweep: Some(0.75),
            maximum_final_coarse_defect: 1.0e-10,
            maximum_final_structural_defect: 1.0e-10,
        },
        maximum_rounds,
        maximum_coarse_dimension_ratio,
        minimum_tuple_reduction: 0.05,
        maximum_two_level_tuple_complexity: 1.95,
        minimum_split_score_fraction: 0.01,
    }
}

fn overmerged_pairs(oracle: &FactorAggregation) -> FactorAggregation {
    let fine_counts = oracle.fine_counts();
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (oracle.parents(factor)[level] / 2) as u32)
            .collect()
    });
    FactorAggregation::new(fine_counts, parents).expect("overmerged map is valid")
}

fn refined_weak_chain(
    levels: usize,
    clones: usize,
    bridge_weight: f64,
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
    let aggregation =
        FactorAggregation::new(fine_counts, parents).expect("oracle aggregation is valid");
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
    (fine, aggregation)
}
