//! Non-finite input must never masquerade as a zero traced-PCG residual.

use multiway_mg::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, FactorAggregation,
    MultiwayError, PcgTraceOptions, ThreeWayProblem, solve_projected_pcg_traced,
    solve_projected_pcg_traced_with_hierarchy_workspace,
};

fn problem() -> ThreeWayProblem {
    let mut tuples = Vec::new();
    for i in 0..2 {
        for j in 0..2 {
            for k in 0..2 {
                tuples.push([i, j, k]);
            }
        }
    }
    ThreeWayProblem::from_observations([2; 3], &tuples, &[1.0; 8]).unwrap()
}

fn hierarchy(problem: &ThreeWayProblem) -> CycleScreenedMapHierarchy {
    let map = FactorAggregation::consecutive_halving([2; 3]).unwrap();
    CycleScreenedMapHierarchy::from_maps(problem.clone(), vec![map], 1.0e-12).unwrap()
}

#[test]
fn ordinary_traced_pcg_rejects_nonfinite_rhs_instead_of_zero_convergence() {
    let problem = problem();
    let hierarchy = hierarchy(&problem);
    let options = PcgTraceOptions::default();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for all_entries in [true, false] {
            let mut rhs = vec![1.0; problem.dimension()];
            if all_entries {
                rhs.fill(bad);
            } else {
                rhs[0] = bad;
            }
            let result = solve_projected_pcg_traced(&problem, &rhs, &hierarchy, options);
            assert!(
                matches!(result, Err(MultiwayError::PcgBreakdown { .. })),
                "non-finite RHS was not rejected: {result:?}"
            );
        }
    }
}

#[test]
fn workspace_traced_pcg_rejects_nonfinite_rhs_without_preparing_scratch() {
    let problem = problem();
    let hierarchy = hierarchy(&problem);
    let options = PcgTraceOptions::default();
    let mut workspace = CycleScreenedMapHierarchyWorkspace::new();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for all_entries in [true, false] {
            let mut rhs = vec![1.0; problem.dimension()];
            if all_entries {
                rhs.fill(bad);
            } else {
                rhs[0] = bad;
            }
            let result = solve_projected_pcg_traced_with_hierarchy_workspace(
                &problem,
                &rhs,
                &hierarchy,
                options,
                &mut workspace,
            );
            assert!(
                matches!(result, Err(MultiwayError::PcgBreakdown { .. })),
                "non-finite RHS was not rejected: {result:?}"
            );
            assert_eq!(workspace.retained_bytes().unwrap(), 0);
            assert_eq!(workspace.retained_buffer_count(), 0);
        }
    }
    let mut rhs = vec![0.0; problem.dimension()];
    let coefficients = [1.0, -2.0, 3.0, -4.0, 5.0, -6.0];
    problem.apply_gramian(&coefficients, &mut rhs).unwrap();
    let expected = solve_projected_pcg_traced(&problem, &rhs, &hierarchy, options).unwrap();
    let actual = solve_projected_pcg_traced_with_hierarchy_workspace(
        &problem,
        &rhs,
        &hierarchy,
        options,
        &mut workspace,
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert!(actual.converged());
}

#[test]
fn unrepresentable_initial_diagnostics_fail_before_preconditioner_application() {
    let problem = problem();
    let hierarchy = hierarchy(&problem);
    let options = PcgTraceOptions::default();
    let half_max = f64::MAX / 2.0;
    let overflowing_norm = vec![
        half_max, -half_max, half_max, -half_max, half_max, -half_max,
    ];
    let overflowing_projection = vec![1.0e160, 1.0e160, -1.0e160, -1.0e160, 0.0, 0.0];
    let overflowing_tolerance = PcgTraceOptions {
        relative_tolerance: f64::MAX,
        ..options
    };
    for (rhs, options) in [
        (overflowing_norm, options),
        (overflowing_projection, options),
        (vec![3.0; 6], overflowing_tolerance),
    ] {
        let mut workspace = CycleScreenedMapHierarchyWorkspace::new();
        let expected = solve_projected_pcg_traced(&problem, &rhs, &hierarchy, options).unwrap_err();
        let actual = solve_projected_pcg_traced_with_hierarchy_workspace(
            &problem,
            &rhs,
            &hierarchy,
            options,
            &mut workspace,
        )
        .unwrap_err();
        assert!(matches!(
            actual,
            MultiwayError::PcgBreakdown { iteration: 0, .. }
        ));
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(workspace.retained_bytes().unwrap(), 0);
        assert_eq!(workspace.retained_buffer_count(), 0);
    }
}
