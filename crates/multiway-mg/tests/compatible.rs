//! Compatible-projection and relaxation tests.

use multiway_mg::{
    CompatibleRelaxationOptions, DiagonalAggregationProjector, DiagonalPreconditioner,
    FactorAggregation, MultiwayError, ThreeWayProblem, analyze_compatible_relaxation,
};

#[test]
fn diagonal_projector_is_idempotent_and_enforces_weighted_coarse_orthogonality() {
    let problem = heterogeneous_problem();
    let aggregation = FactorAggregation::new([4, 2, 2], [vec![0, 0, 1, 1], vec![0, 0], vec![0, 1]])
        .expect("aggregation is valid");
    let projector = DiagonalAggregationProjector::new(problem.clone(), aggregation)
        .expect("projector construction succeeds");
    let mut values = vec![0.7, -1.3, 2.1, -0.4, 1.2, -0.8, 0.5, -1.7];
    let original = values.clone();
    let original_norm = projector
        .diagonal_norm(&original)
        .expect("original norm succeeds");
    let removed_norm = projector
        .project_complement_in_place(&mut values)
        .expect("projection succeeds");
    let retained_norm = projector
        .diagonal_norm(&values)
        .expect("retained norm succeeds");
    let second_removed = projector
        .project_complement_in_place(&mut values)
        .expect("second projection succeeds");

    assert!(second_removed <= 1.0e-12 * retained_norm.max(1.0));
    assert!(
        projector
            .relative_coarse_defect(&values)
            .expect("coarse defect succeeds")
            < 1.0e-12
    );
    assert!(
        projector
            .relative_structural_defect(&values)
            .expect("structural defect succeeds")
            < 1.0e-12
    );
    let pythagorean_defect = (original_norm * original_norm
        - removed_norm * removed_norm
        - retained_norm * retained_norm)
        .abs();
    assert!(pythagorean_defect <= 1.0e-11 * original_norm.powi(2).max(1.0));
}

#[test]
fn compatible_relaxation_is_bitwise_deterministic_for_fixed_inputs() {
    let problem = refined_weak_chain(6, 2, 0.02).0;
    let aggregation = refined_weak_chain(6, 2, 0.02).1;
    let smoother = DiagonalPreconditioner::new(&problem, 0.5).expect("diagonal smoother succeeds");
    let options = CompatibleRelaxationOptions {
        test_vectors: 6,
        sweeps: 7,
        ..CompatibleRelaxationOptions::default()
    };
    let first = analyze_compatible_relaxation(&problem, &aggregation, &smoother, options)
        .expect("first compatible analysis succeeds");
    let second = analyze_compatible_relaxation(&problem, &aggregation, &smoother, options)
        .expect("second compatible analysis succeeds");
    assert_eq!(first, second);
    assert_eq!(first.smoother_applications(), 42);
    assert_eq!(first.gramian_applications(), 42);
    assert!(first.maximum_final_coarse_defect() < 1.0e-12);
    assert!(first.maximum_final_structural_defect() < 1.0e-12);
}

#[test]
fn oracle_map_leaves_more_rapidly_damped_error_than_a_misaligned_map() {
    let (problem, oracle) = refined_weak_chain(8, 2, 0.01);
    let bad_parents =
        core::array::from_fn(|_| vec![0, 1, 0, 1, 2, 3, 2, 3, 4, 5, 4, 5, 6, 7, 6, 7]);
    let bad = FactorAggregation::new([16, 16, 16], bad_parents)
        .expect("misaligned aggregation remains structurally valid");
    let smoother = DiagonalPreconditioner::new(&problem, 0.5).expect("diagonal smoother succeeds");
    let options = CompatibleRelaxationOptions {
        test_vectors: 12,
        sweeps: 10,
        ..CompatibleRelaxationOptions::default()
    };
    let oracle_report = analyze_compatible_relaxation(&problem, &oracle, &smoother, options)
        .expect("oracle compatible analysis succeeds");
    let bad_report = analyze_compatible_relaxation(&problem, &bad, &smoother, options)
        .expect("bad-map compatible analysis succeeds");

    assert!(
        oracle_report.maximum_diagonal_contraction() < bad_report.maximum_diagonal_contraction()
    );
    assert!(
        oracle_report.geometric_mean_diagonal_contraction()
            < bad_report.geometric_mean_diagonal_contraction()
    );
}

#[test]
fn aggregation_crossing_incidence_components_is_rejected() {
    let problem =
        ThreeWayProblem::from_observations([2, 2, 2], &[[0, 0, 0], [1, 1, 1]], &[1.0, 2.0])
            .expect("disconnected problem is valid");
    let aggregation = FactorAggregation::new([2, 2, 2], [vec![0, 0], vec![0, 0], vec![0, 0]])
        .expect("parent labels themselves are valid");
    let error = DiagonalAggregationProjector::new(problem, aggregation)
        .expect_err("cross-component aggregation must fail");
    assert!(matches!(error, MultiwayError::InvalidAggregation { .. }));
}

fn heterogeneous_problem() -> ThreeWayProblem {
    let tuples = [
        [0, 0, 0],
        [0, 1, 1],
        [1, 0, 1],
        [1, 1, 0],
        [2, 0, 0],
        [2, 1, 1],
        [3, 0, 1],
        [3, 1, 0],
    ];
    let weights = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];
    ThreeWayProblem::from_observations([4, 2, 2], &tuples, &weights)
        .expect("heterogeneous problem is valid")
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
