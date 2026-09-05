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

fn disconnected_problem() -> ThreeWayProblem {
    ThreeWayProblem::from_observations([2, 2, 2], &[[0, 0, 0], [1, 1, 1]], &[1.0, 2.0])
        .expect("valid disconnected problem")
}

fn assert_close(left: f64, right: f64, tolerance: f64) {
    let scale = left.abs().max(right.abs()).max(1.0);
    assert!(
        (left - right).abs() <= tolerance * scale,
        "{left:.16e} differs from {right:.16e}"
    );
}

fn assert_slices_close(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (&left_value, &right_value) in left.iter().zip(right) {
        assert_close(left_value, right_value, tolerance);
    }
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
fn reusable_projection_workspace_matches_allocating_projection() {
    let problem = disconnected_problem();
    let components = problem.components();
    let base = [1.0, -2.0, 3.0, -4.0, 5.0, -6.0];
    let mut workspace = components.projection_workspace();
    let retained_bytes = workspace.retained_bytes();

    assert_eq!(workspace.dimension(), problem.dimension());
    assert_eq!(workspace.component_count(), components.count());
    assert!(retained_bytes > 0);

    for scale in [1.0, -2.5, 0.125] {
        let mut expected: Vec<f64> = base.iter().map(|value| scale * value).collect();
        let expected_removed = components
            .project_structural_range(&mut expected)
            .expect("allocating projection");

        let mut actual: Vec<f64> = base.iter().map(|value| scale * value).collect();
        let actual_removed = components
            .project_structural_range_with_workspace(&mut actual, &mut workspace)
            .expect("workspace projection");

        assert_close(actual_removed, expected_removed, 1.0e-13);
        assert_slices_close(&actual, &expected, 1.0e-13);
        assert_eq!(workspace.retained_bytes(), retained_bytes);

        let defect = components
            .maximum_structural_defect_with_workspace(&actual, &mut workspace)
            .expect("workspace defect");
        assert!(defect < 1.0e-12);
        assert_eq!(workspace.retained_bytes(), retained_bytes);
    }
}

#[test]
fn projection_workspace_mismatch_fails_before_mutating_output() {
    let connected = sample_problem();
    let disconnected = disconnected_problem();
    let mut workspace = connected.components().projection_workspace();
    let mut values = vec![1.0, -2.0, 3.0, -4.0, 5.0, -6.0];
    let original = values.clone();

    let error = disconnected
        .components()
        .project_structural_range_with_workspace(&mut values, &mut workspace)
        .expect_err("component-count mismatch must fail");
    assert!(matches!(error, IncidenceError::DimensionMismatch { .. }));
    assert_eq!(values, original);

    connected
        .components()
        .project_structural_range_with_workspace(&mut values, &mut workspace)
        .expect("workspace remains reusable after rejection");
    let defect = connected
        .components()
        .maximum_structural_defect_with_workspace(&values, &mut workspace)
        .expect("defect after valid reuse");
    assert!(defect < 1.0e-12);
}

#[test]
fn rhs_and_residual_into_match_allocating_convenience_methods() {
    let problem = sample_problem();
    let targets = [0.5, -1.0, 0.25, 2.0, -0.75];
    let expected_rhs = problem
        .rhs_from_targets(&targets)
        .expect("allocating right-hand side");
    let mut actual_rhs = vec![f64::NAN; problem.dimension()];
    let rhs_pointer = actual_rhs.as_ptr();
    let rhs_capacity = actual_rhs.capacity();
    problem
        .rhs_from_targets_into(&targets, &mut actual_rhs)
        .expect("caller-owned right-hand side");
    assert_slices_close(&actual_rhs, &expected_rhs, 1.0e-14);
    assert_eq!(actual_rhs.as_ptr(), rhs_pointer);
    assert_eq!(actual_rhs.capacity(), rhs_capacity);

    let second_targets = [-0.25, 0.75, 1.5, -2.0, 0.125];
    let second_expected = problem
        .rhs_from_targets(&second_targets)
        .expect("second allocating right-hand side");
    problem
        .rhs_from_targets_into(&second_targets, &mut actual_rhs)
        .expect("reused caller-owned right-hand side");
    assert_slices_close(&actual_rhs, &second_expected, 1.0e-14);
    assert_eq!(actual_rhs.as_ptr(), rhs_pointer);
    assert_eq!(actual_rhs.capacity(), rhs_capacity);

    let x = [0.25, -0.5, 1.0, -0.75, 0.1, 0.6];
    let expected_residual = problem
        .residual(&actual_rhs, &x)
        .expect("allocating residual");
    let mut actual_residual = vec![f64::NAN; problem.dimension()];
    let residual_pointer = actual_residual.as_ptr();
    let residual_capacity = actual_residual.capacity();
    problem
        .residual_into(&actual_rhs, &x, &mut actual_residual)
        .expect("caller-owned residual");
    assert_slices_close(&actual_residual, &expected_residual, 1.0e-14);
    assert_eq!(actual_residual.as_ptr(), residual_pointer);
    assert_eq!(actual_residual.capacity(), residual_capacity);
}

#[test]
fn into_kernels_reject_dimensions_before_mutating_output() {
    let problem = sample_problem();
    let targets = [0.5, -1.0, 0.25, 2.0];
    let mut rhs = vec![7.0; problem.dimension()];
    let rhs_before = rhs.clone();
    let error = problem
        .rhs_from_targets_into(&targets, &mut rhs)
        .expect_err("short targets must fail");
    assert!(matches!(error, IncidenceError::DimensionMismatch { .. }));
    assert_eq!(rhs, rhs_before);

    let valid_rhs = vec![1.0; problem.dimension()];
    let short_x = vec![0.0; problem.dimension() - 1];
    let mut residual = vec![9.0; problem.dimension()];
    let residual_before = residual.clone();
    let error = problem
        .residual_into(&valid_rhs, &short_x, &mut residual)
        .expect_err("short iterate must fail");
    assert!(matches!(error, IncidenceError::DimensionMismatch { .. }));
    assert_eq!(residual, residual_before);
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
    let aggregation = FactorAggregation::consecutive_halving([4, 4, 4]).expect("valid aggregation");
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
