//! Matrix-free complete-cycle quality probe tests.

use multiway_mg::{
    CycleQualityCriteria, CycleQualityOptions, DensePseudoinverse, FactorAggregation,
    SymmetricMapPreconditioner, SymmetricTwoGridPreconditioner, ThreeWayProblem,
    analyze_cycle_quality, evaluate_cycle_quality,
};

#[test]
fn exact_pseudoinverse_annihilates_range_error() {
    let problem = complete_problem(3);
    let inverse =
        DensePseudoinverse::from_problem(&problem, 1.0e-12).expect("exact pseudoinverse succeeds");
    let report = analyze_cycle_quality(
        &problem,
        &inverse,
        CycleQualityOptions {
            test_vectors: 6,
            power_iterations: 12,
            tail_iterations: 4,
            ..CycleQualityOptions::default()
        },
    )
    .expect("cycle probe succeeds");

    assert!(report.maximum_estimated_energy_factor() < 1.0e-8);
    assert!(report.vectors().iter().all(|vector| vector.annihilated()));
    assert!(report.maximum_structural_defect() < 1.0e-10);
    assert!(report.preconditioner_applications() <= 6);
}

#[test]
fn oracle_two_grid_has_a_better_complete_cycle_than_an_overmerged_map() {
    let (problem, oracle) = refined_weak_chain(10, 2, 0.005);
    let overmerged = overmerged_pairs(&oracle);
    let oracle_cycle = map_cycle(&problem, &oracle);
    let overmerged_cycle = map_cycle(&problem, &overmerged);
    let options = CycleQualityOptions {
        test_vectors: 12,
        power_iterations: 24,
        tail_iterations: 6,
        ..CycleQualityOptions::default()
    };
    let oracle_report = analyze_cycle_quality(&problem, &oracle_cycle, options)
        .expect("oracle cycle probe succeeds");
    let overmerged_report = analyze_cycle_quality(&problem, &overmerged_cycle, options)
        .expect("overmerged cycle probe succeeds");

    assert!(oracle_report.maximum_estimated_energy_factor() < 0.25);
    assert!(
        oracle_report.maximum_estimated_energy_factor()
            < overmerged_report.maximum_estimated_energy_factor()
    );
    assert!(overmerged_report.maximum_estimated_energy_factor() > 0.50);
}

#[test]
fn report_and_selected_witness_are_observation_order_invariant() {
    let (problem, oracle, mut tuples, mut weights) = refined_weak_chain_parts(8, 2, 0.01);
    tuples.reverse();
    weights.reverse();
    let reversed =
        ThreeWayProblem::from_observations(problem.topology().level_counts(), &tuples, &weights)
            .expect("reversed problem succeeds");
    assert_eq!(problem, reversed);

    let options = CycleQualityOptions {
        test_vectors: 8,
        power_iterations: 18,
        tail_iterations: 5,
        ..CycleQualityOptions::default()
    };
    let first = analyze_cycle_quality(&problem, &map_cycle(&problem, &oracle), options)
        .expect("first probe succeeds");
    let second = analyze_cycle_quality(&reversed, &map_cycle(&reversed, &oracle), options)
        .expect("second probe succeeds");
    assert_eq!(first, second);
    assert_eq!(first.slowest_vector_index(), second.slowest_vector_index());
}

#[test]
fn explicit_cycle_gate_accepts_oracle_and_rejects_overmerged_map() {
    let (problem, oracle) = refined_weak_chain(10, 2, 0.005);
    let overmerged = overmerged_pairs(&oracle);
    let options = CycleQualityOptions {
        test_vectors: 12,
        power_iterations: 24,
        tail_iterations: 6,
        ..CycleQualityOptions::default()
    };
    let criteria = CycleQualityCriteria {
        maximum_estimated_energy_factor: 0.40,
        maximum_observed_energy_factor: Some(1.05),
        maximum_structural_defect: 1.0e-10,
    };
    let oracle = evaluate_cycle_quality(
        &analyze_cycle_quality(&problem, &map_cycle(&problem, &oracle), options)
            .expect("oracle probe succeeds"),
        criteria,
    )
    .expect("oracle decision succeeds");
    let overmerged = evaluate_cycle_quality(
        &analyze_cycle_quality(&problem, &map_cycle(&problem, &overmerged), options)
            .expect("overmerged probe succeeds"),
        criteria,
    )
    .expect("overmerged decision succeeds");

    assert!(oracle.accepted());
    assert!(!overmerged.accepted());
    assert!(!overmerged.rejections().is_empty());
}

fn map_cycle(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> SymmetricTwoGridPreconditioner<SymmetricMapPreconditioner> {
    SymmetricTwoGridPreconditioner::build(
        problem.clone(),
        aggregation.clone(),
        SymmetricMapPreconditioner::new(problem.clone()),
        1,
        1.0,
        1.0e-12,
    )
    .expect("two-grid cycle succeeds")
}

fn complete_problem(levels: u32) -> ThreeWayProblem {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            for third in 0..levels {
                tuples.push([first, second, third]);
                weights.push(0.75 + ((3 * first + 5 * second + 7 * third) % 11) as f64 / 10.0);
            }
        }
    }
    ThreeWayProblem::from_observations([levels as usize; 3], &tuples, &weights)
        .expect("complete problem is valid")
}

fn overmerged_pairs(oracle: &FactorAggregation) -> FactorAggregation {
    let fine_counts = oracle.fine_counts();
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| oracle.parents(factor)[level] / 2)
            .collect()
    });
    FactorAggregation::new(fine_counts, parents).expect("overmerged map is valid")
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
) -> (ThreeWayProblem, FactorAggregation, Vec<[u32; 3]>, Vec<f64>) {
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
