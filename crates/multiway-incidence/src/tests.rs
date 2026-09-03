use super::*;

fn sample_problem() -> ThreeWayProblem {
    ThreeWayProblem::from_observations(
        [2, 2, 2],
        &[
            [0, 0, 0],
            [0, 0, 1],
            [0, 1, 0],
            [1, 0, 0],
            [1, 1, 1],
            [1, 1, 1],
        ],
        &[1.0, 2.0, 1.5, 0.5, 3.0, 4.0],
    )
    .expect("valid sample problem")
}

#[test]
fn duplicate_tuples_are_collapsed_deterministically() {
    let problem = sample_problem();
    assert_eq!(problem.tuple_count(), 5);
    assert_eq!(problem.topology().tuples().last(), Some(&[1, 1, 1]));
    assert!((problem.weights().last().copied().unwrap_or_default() - 7.0).abs() < 1.0e-14);
}

#[test]
fn gramian_kernel_matches_dense_materialization() {
    let problem = sample_problem();
    let x = [0.25, -0.5, 1.0, -0.75, 0.1, 0.6];
    let mut actual = vec![0.0; problem.dimension()];
    problem
        .apply_gramian(&x, &mut actual)
        .expect("gramian application");
    let dense = problem.dense_gramian();
    let expected: Vec<f64> = dense
        .iter()
        .map(|row| row.iter().zip(x).map(|(a, b)| a * b).sum())
        .collect();
    for (left, right) in actual.iter().zip(expected) {
        assert!((left - right).abs() < 1.0e-12);
    }
}

#[test]
fn structural_projection_removes_known_shift_modes() {
    let problem = sample_problem();
    let mut values = vec![1.0, 1.0, -2.0, -2.0, 0.5, 0.5];
    let removed = problem
        .components()
        .project_structural_range(&mut values)
        .expect("projection succeeds");
    assert!(removed > 0.0);
    let defect = problem
        .components()
        .maximum_structural_defect(&values)
        .expect("defect computation succeeds");
    assert!(defect < 1.0e-12);
}

#[test]
fn mapped_tuples_equal_dense_galerkin_product() {
    let fine = ThreeWayProblem::from_observations(
        [4, 4, 4],
        &[
            [0, 0, 0],
            [1, 1, 1],
            [0, 1, 1],
            [1, 0, 0],
            [2, 2, 2],
            [3, 3, 3],
            [2, 3, 3],
            [3, 2, 2],
        ],
        &[1.0; 8],
    )
    .expect("valid fine problem");
    let aggregation = FactorAggregation::consecutive_halving([4, 4, 4])
        .expect("valid aggregation");
    let coarse = aggregation.coarsen(&fine).expect("coarsening succeeds");

    let fine_matrix = fine.dense_gramian();
    let coarse_matrix = coarse.dense_gramian();
    let coarse_dimension = coarse.dimension();
    for column in 0..coarse_dimension {
        let mut coarse_basis = vec![0.0; coarse_dimension];
        coarse_basis[column] = 1.0;
        let mut prolonged = vec![0.0; fine.dimension()];
        aggregation
            .prolong(&coarse_basis, &mut prolonged)
            .expect("prolongation succeeds");
        let fine_product: Vec<f64> = fine_matrix
            .iter()
            .map(|row| row.iter().zip(&prolonged).map(|(a, b)| a * b).sum())
            .collect();
        let mut restricted = vec![0.0; coarse_dimension];
        aggregation
            .restrict(&fine_product, &mut restricted)
            .expect("restriction succeeds");
        for row in 0..coarse_dimension {
            assert!((restricted[row] - coarse_matrix[row][column]).abs() < 1.0e-12);
        }
    }
}
