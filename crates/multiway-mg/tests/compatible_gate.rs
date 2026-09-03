//! Acceptance-policy tests for projected compatible relaxation.

use multiway_mg::{
    CompatibleRelaxationCriteria, CompatibleRelaxationOptions,
    CompatibleRelaxationRejection, DiagonalPreconditioner, FactorAggregation,
    ThreeWayProblem, analyze_compatible_relaxation, evaluate_compatible_relaxation,
};

#[test]
fn conservative_gate_accepts_oracle_map_and_rejects_misaligned_weak_chain() {
    let (problem, oracle) = refined_weak_chain(8, 2, 0.01);
    let bad_parents =
        core::array::from_fn(|_| vec![0, 1, 0, 1, 2, 3, 2, 3, 4, 5, 4, 5, 6, 7, 6, 7]);
    let bad = FactorAggregation::new([16, 16, 16], bad_parents)
        .expect("misaligned aggregation remains structurally valid");
    let smoother = DiagonalPreconditioner::new(&problem, 0.5)
        .expect("diagonal smoother succeeds");
    let options = CompatibleRelaxationOptions {
        test_vectors: 12,
        sweeps: 10,
        ..CompatibleRelaxationOptions::default()
    };
    let criteria = CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.75,
        maximum_energy_factor_per_sweep: Some(0.75),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    };

    let oracle_report = analyze_compatible_relaxation(&problem, &oracle, &smoother, options)
        .expect("oracle analysis succeeds");
    let bad_report = analyze_compatible_relaxation(&problem, &bad, &smoother, options)
        .expect("bad-map analysis succeeds");
    let oracle_decision = evaluate_compatible_relaxation(&oracle_report, criteria)
        .expect("oracle decision succeeds");
    let bad_decision = evaluate_compatible_relaxation(&bad_report, criteria)
        .expect("bad-map decision succeeds");

    assert!(oracle_decision.accepted());
    assert!(!bad_decision.accepted());
    assert!(oracle_decision.maximum_diagonal_factor_per_sweep() < 0.75);
    assert!(bad_decision.maximum_diagonal_factor_per_sweep() > 0.9);
    assert!(bad_decision.rejections().iter().any(|reason| matches!(
        reason,
        CompatibleRelaxationRejection::DiagonalContraction { .. }
    )));
}

#[test]
fn criteria_are_explicit_and_invalid_thresholds_fail_closed() {
    let (problem, oracle) = refined_weak_chain(4, 2, 0.02);
    let smoother = DiagonalPreconditioner::new(&problem, 0.5)
        .expect("diagonal smoother succeeds");
    let report = analyze_compatible_relaxation(
        &problem,
        &oracle,
        &smoother,
        CompatibleRelaxationOptions {
            test_vectors: 4,
            sweeps: 4,
            ..CompatibleRelaxationOptions::default()
        },
    )
    .expect("compatible analysis succeeds");
    let error = evaluate_compatible_relaxation(
        &report,
        CompatibleRelaxationCriteria {
            maximum_diagonal_factor_per_sweep: f64::NAN,
            maximum_energy_factor_per_sweep: None,
            maximum_final_coarse_defect: 1.0e-10,
            maximum_final_structural_defect: 1.0e-10,
        },
    )
    .expect_err("nonfinite criteria must fail");
    assert!(error.to_string().contains("maximum_diagonal_factor_per_sweep"));
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
