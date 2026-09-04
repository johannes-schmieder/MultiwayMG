//! Recursive automatic hierarchy planning tests.

use multiway_mg::{
    BootstrapAggregationOptions, BootstrapHierarchyOptions, BootstrapHierarchyPlan,
    BootstrapHierarchyStopReason, CompatibleRelaxationCriteria, ThreeWayProblem,
};

#[test]
fn planted_recursive_problem_reaches_the_declared_terminal() {
    let (problem, _, _) = recursive_clone_problem(2, 2);
    let plan = BootstrapHierarchyPlan::build(problem, hierarchy_options(2.5, 3.0))
        .expect("recursive bootstrap planning succeeds");

    assert!(plan.completed());
    assert!(matches!(
        plan.stop_reason(),
        BootstrapHierarchyStopReason::ReachedTerminal
    ));
    assert_eq!(plan.aggregations().len(), 2);
    assert_eq!(plan.level_reports().len(), 2);
    assert_eq!(plan.problems().len(), 3);
    assert_eq!(plan.terminal_candidate().dimension(), 6);
    assert!(plan.dimension_complexity() < 2.0);
    assert!(plan.tuple_complexity() < 1.2);
    assert!(plan
        .level_reports()
        .iter()
        .all(|level| level.aggregation_result().accepted()));
}

#[test]
fn cumulative_tuple_budget_stops_before_admitting_the_candidate_level() {
    let (problem, _, _) = recursive_clone_problem(2, 2);
    let plan = BootstrapHierarchyPlan::build(problem, hierarchy_options(2.5, 1.01))
        .expect("budgeted planning returns a decision");

    assert!(!plan.completed());
    assert!(matches!(
        plan.stop_reason(),
        BootstrapHierarchyStopReason::TupleComplexityBudget { level: 0, .. }
    ));
    assert!(plan.aggregations().is_empty());
    assert!(plan.level_reports().is_empty());
    assert_eq!(plan.problems().len(), 1);
    assert_eq!(plan.dimension_complexity(), 1.0);
    assert_eq!(plan.tuple_complexity(), 1.0);
}

#[test]
fn tuple_order_permutation_leaves_the_complete_plan_unchanged() {
    let (problem, mut tuples, mut weights) = recursive_clone_problem(2, 2);
    tuples.reverse();
    weights.reverse();
    let permuted = ThreeWayProblem::from_observations(
        problem.topology().level_counts(),
        &tuples,
        &weights,
    )
    .expect("permuted problem is valid");
    let options = hierarchy_options(2.5, 3.0);
    let first = BootstrapHierarchyPlan::build(problem, options)
        .expect("first recursive bootstrap plan succeeds");
    let second = BootstrapHierarchyPlan::build(permuted, options)
        .expect("permuted recursive bootstrap plan succeeds");

    assert_eq!(first, second);
}

#[test]
fn an_impossibly_strict_compatible_gate_rejects_without_a_partial_hierarchy() {
    let (problem, _, _) = recursive_clone_problem(2, 2);
    let mut options = hierarchy_options(2.5, 3.0);
    options.aggregation.compatible_criteria = CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 1.0e-6,
        maximum_energy_factor_per_sweep: Some(1.0e-6),
        maximum_final_coarse_defect: 1.0e-12,
        maximum_final_structural_defect: 1.0e-12,
    };
    options.aggregation.maximum_bootstrap_witnesses = 1;
    options.aggregation.split_repair = None;
    let plan = BootstrapHierarchyPlan::build(problem, options)
        .expect("strict compatible gate returns a fail-closed plan");

    assert!(!plan.completed());
    assert!(matches!(
        plan.stop_reason(),
        BootstrapHierarchyStopReason::AggregationRejected { level: 0, .. }
    ));
    assert!(plan.aggregations().is_empty());
    assert_eq!(plan.problems().len(), 1);
}

fn hierarchy_options(
    maximum_dimension_complexity: f64,
    maximum_tuple_complexity: f64,
) -> BootstrapHierarchyOptions {
    let aggregation = BootstrapAggregationOptions {
        setup_test_vectors: 6,
        setup_sweeps: 6,
        maximum_bootstrap_witnesses: 2,
        maximum_coarse_dimension_ratio: 0.80,
        minimum_tuple_reduction: 0.02,
        maximum_two_level_tuple_complexity: 1.98,
        split_repair: None,
        ..BootstrapAggregationOptions::default()
    };
    BootstrapHierarchyOptions {
        maximum_levels: 2,
        terminal_dimension: 6,
        minimum_dimension_reduction: 0.20,
        minimum_tuple_reduction: 0.05,
        maximum_dimension_complexity,
        maximum_tuple_complexity,
        screen_jacobi_omega: 0.5,
        aggregation,
    }
}

fn recursive_clone_problem(
    base_levels: usize,
    depth: usize,
) -> (ThreeWayProblem, Vec<[u32; 3]>, Vec<f64>) {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..base_levels {
        for second in 0..base_levels {
            for third in 0..base_levels {
                tuples.push([first as u32, second as u32, third as u32]);
                weights.push(0.8 + ((3 * first + 5 * second + 7 * third) % 11) as f64 / 10.0);
            }
        }
    }
    let mut level_counts = [base_levels; 3];
    for refinement in 0..depth {
        let mut fine_tuples = Vec::with_capacity(tuples.len() * 8);
        let mut fine_weights = Vec::with_capacity(weights.len() * 8);
        for (tuple_index, (&tuple, &weight)) in tuples.iter().zip(&weights).enumerate() {
            let mut scores = [0.0; 8];
            let mut score_sum = 0.0;
            for child in 0..8 {
                let score = 0.75
                    + ((tuple_index * 13 + refinement * 17 + child * 5) % 19) as f64 / 20.0;
                scores[child] = score;
                score_sum += score;
            }
            for first_child in 0..2_u32 {
                for second_child in 0..2_u32 {
                    for third_child in 0..2_u32 {
                        let child = (4 * first_child + 2 * second_child + third_child) as usize;
                        fine_tuples.push([
                            2 * tuple[0] + first_child,
                            2 * tuple[1] + second_child,
                            2 * tuple[2] + third_child,
                        ]);
                        fine_weights.push(weight * scores[child] / score_sum);
                    }
                }
            }
        }
        tuples = fine_tuples;
        weights = fine_weights;
        level_counts = level_counts.map(|count| count * 2);
    }
    let problem = ThreeWayProblem::from_observations(level_counts, &tuples, &weights)
        .expect("recursive clone problem is valid");
    (problem, tuples, weights)
}
