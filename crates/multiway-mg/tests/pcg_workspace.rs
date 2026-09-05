//! Exact traced-PCG equivalence with caller-owned hierarchy scratch.

use multiway_mg::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, FactorAggregation,
    MultiwayError, PcgTraceOptions, PcgTraceResult, ThreeWayProblem, solve_projected_pcg_traced,
    solve_projected_pcg_traced_with_hierarchy_workspace,
};

fn problem(levels: usize) -> ThreeWayProblem {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for i in 0..levels {
        for j in 0..levels {
            for k in 0..levels {
                tuples.push([i as u32, j as u32, k as u32]);
                weights.push((1 + (7 * i + 11 * j + 13 * k + i * j) % 17) as f64);
            }
        }
    }
    ThreeWayProblem::from_observations([levels; 3], &tuples, &weights).unwrap()
}

fn hierarchy(problem: &ThreeWayProblem) -> CycleScreenedMapHierarchy {
    let mut counts = problem.topology().level_counts();
    let mut maps = Vec::new();
    while counts[0] > 1 {
        let map = FactorAggregation::consecutive_halving(counts).unwrap();
        counts = map.coarse_counts();
        maps.push(map);
    }
    CycleScreenedMapHierarchy::from_maps(problem.clone(), maps, 1.0e-12).unwrap()
}

fn rhs(problem: &ThreeWayProblem, scale: f64) -> Vec<f64> {
    let coefficients: Vec<_> = (0..problem.dimension())
        .map(|i| scale * ((i as f64 + 0.5) * 0.71).sin())
        .collect();
    let mut rhs = vec![0.0; coefficients.len()];
    problem.apply_gramian(&coefficients, &mut rhs).unwrap();
    rhs
}

fn assert_bits_equal(actual: &PcgTraceResult, expected: &PcgTraceResult) {
    assert_eq!(actual.iterations(), expected.iterations());
    assert_eq!(actual.converged(), expected.converged());
    assert_eq!(
        actual.gramian_applications(),
        expected.gramian_applications()
    );
    assert_eq!(
        actual.preconditioner_applications(),
        expected.preconditioner_applications()
    );
    assert_eq!(
        actual.rhs_projection_norm().to_bits(),
        expected.rhs_projection_norm().to_bits()
    );
    assert_eq!(
        actual.final_relative_residual().to_bits(),
        expected.final_relative_residual().to_bits()
    );
    assert_eq!(actual.solution().len(), expected.solution().len());
    for (&actual, &expected) in actual.solution().iter().zip(expected.solution()) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
    assert_eq!(actual.samples().len(), expected.samples().len());
    for (actual, expected) in actual.samples().iter().zip(expected.samples()) {
        assert_eq!(actual.iteration(), expected.iteration());
        assert_eq!(
            actual.residual_norm().to_bits(),
            expected.residual_norm().to_bits()
        );
        assert_eq!(
            actual.relative_residual().to_bits(),
            expected.relative_residual().to_bits()
        );
    }
}

#[test]
fn repeated_solves_preserve_every_trace_bit_counter_and_workspace_capacity() {
    let problem = problem(5);
    let hierarchy = hierarchy(&problem);
    let mut workspace = hierarchy.application_workspace().unwrap();
    let bytes = workspace.retained_bytes().unwrap();
    let buffers = workspace.retained_buffer_count();
    for scale in [1.0, -2.0, 0.5, 1.25] {
        let rhs = rhs(&problem, scale);
        let options = PcgTraceOptions::default();
        let expected = solve_projected_pcg_traced(&problem, &rhs, &hierarchy, options).unwrap();
        let actual = solve_projected_pcg_traced_with_hierarchy_workspace(
            &problem,
            &rhs,
            &hierarchy,
            options,
            &mut workspace,
        )
        .unwrap();
        assert_bits_equal(&actual, &expected);
        assert!(actual.converged());
        assert_eq!(actual.samples().len(), actual.iterations() + 1);
        assert_eq!(workspace.retained_bytes().unwrap(), bytes);
        assert_eq!(workspace.retained_buffer_count(), buffers);
        // Independently certify the result against the submitted operator.
        let residual = problem.residual(&rhs, actual.solution()).unwrap();
        let residual_norm = residual.iter().map(|x| x * x).sum::<f64>().sqrt();
        let rhs_norm = rhs.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(residual_norm <= 1.0e-9 * rhs_norm);
    }
}

#[test]
fn iteration_limit_and_zero_rhs_preserve_exact_outcomes() {
    let problem = problem(5);
    let hierarchy = hierarchy(&problem);
    let mut workspace = CycleScreenedMapHierarchyWorkspace::new();
    let zero = vec![0.0; problem.dimension()];
    let options = PcgTraceOptions::default();
    let expected = solve_projected_pcg_traced(&problem, &zero, &hierarchy, options).unwrap();
    let actual = solve_projected_pcg_traced_with_hierarchy_workspace(
        &problem,
        &zero,
        &hierarchy,
        options,
        &mut workspace,
    )
    .unwrap();
    assert_bits_equal(&actual, &expected);
    assert_eq!(actual.gramian_applications(), 0);
    assert_eq!(actual.preconditioner_applications(), 0);
    assert_eq!(workspace.retained_bytes().unwrap(), 0);
    assert_eq!(workspace.retained_buffer_count(), 0);

    let rhs = rhs(&problem, 1.0);
    let options = PcgTraceOptions {
        relative_tolerance: 1.0e-14,
        max_iterations: 1,
        ..PcgTraceOptions::default()
    };
    let expected = solve_projected_pcg_traced(&problem, &rhs, &hierarchy, options).unwrap();
    let actual = solve_projected_pcg_traced_with_hierarchy_workspace(
        &problem,
        &rhs,
        &hierarchy,
        options,
        &mut workspace,
    )
    .unwrap();
    assert_bits_equal(&actual, &expected);
    assert_eq!(actual.iterations(), 1);
    assert!(!actual.converged());
    assert_eq!(actual.gramian_applications(), 2);
    // Preserve the original driver's last nonconverged preconditioner call.
    assert_eq!(actual.preconditioner_applications(), 2);
}

#[test]
fn invalid_inputs_fail_in_the_same_order_without_touching_workspace() {
    let problem = problem(5);
    let hierarchy = hierarchy(&problem);
    let rhs = rhs(&problem, 1.0);
    let mut workspace = CycleScreenedMapHierarchyWorkspace::new();
    for options in [
        PcgTraceOptions {
            relative_tolerance: f64::NAN,
            ..PcgTraceOptions::default()
        },
        PcgTraceOptions {
            absolute_tolerance: -1.0,
            ..PcgTraceOptions::default()
        },
        PcgTraceOptions {
            relative_tolerance: 0.0,
            absolute_tolerance: 0.0,
            ..PcgTraceOptions::default()
        },
        PcgTraceOptions {
            max_iterations: 0,
            ..PcgTraceOptions::default()
        },
    ] {
        let expected = solve_projected_pcg_traced(&problem, &rhs, &hierarchy, options).unwrap_err();
        let actual = solve_projected_pcg_traced_with_hierarchy_workspace(
            &problem,
            &rhs,
            &hierarchy,
            options,
            &mut workspace,
        )
        .unwrap_err();
        assert_eq!(actual.to_string(), expected.to_string());
    }
    let options = PcgTraceOptions::default();
    for length in [rhs.len() - 1, rhs.len() + 1] {
        let bad_rhs = vec![1.0; length];
        let expected =
            solve_projected_pcg_traced(&problem, &bad_rhs, &hierarchy, options).unwrap_err();
        let actual = solve_projected_pcg_traced_with_hierarchy_workspace(
            &problem,
            &bad_rhs,
            &hierarchy,
            options,
            &mut workspace,
        )
        .unwrap_err();
        assert_eq!(actual.to_string(), expected.to_string());
    }
    let other_problem = self::problem(2);
    let other_hierarchy = self::hierarchy(&other_problem);
    let expected =
        solve_projected_pcg_traced(&problem, &rhs, &other_hierarchy, options).unwrap_err();
    let actual = solve_projected_pcg_traced_with_hierarchy_workspace(
        &problem,
        &rhs,
        &other_hierarchy,
        options,
        &mut workspace,
    )
    .unwrap_err();
    assert_eq!(actual.to_string(), expected.to_string());
    assert_eq!(workspace.retained_bytes().unwrap(), 0);
    assert_eq!(workspace.retained_buffer_count(), 0);
}

#[test]
fn numerical_breakdown_propagates_and_scratch_remains_reusable() {
    let problem = problem(5);
    let hierarchy = hierarchy(&problem);
    let rhs = rhs(&problem, 1.0);
    let huge: Vec<_> = rhs.iter().map(|value| value * 1.0e160).collect();
    assert!(huge.iter().all(|value| value.is_finite()));
    let options = PcgTraceOptions::default();
    let mut workspace = CycleScreenedMapHierarchyWorkspace::new();
    let expected = solve_projected_pcg_traced(&problem, &huge, &hierarchy, options).unwrap_err();
    let actual = solve_projected_pcg_traced_with_hierarchy_workspace(
        &problem,
        &huge,
        &hierarchy,
        options,
        &mut workspace,
    )
    .unwrap_err();
    assert!(matches!(actual, MultiwayError::PcgBreakdown { .. }));
    assert_eq!(actual.to_string(), expected.to_string());
    let bytes = workspace.retained_bytes().unwrap();
    let buffers = workspace.retained_buffer_count();
    let expected = solve_projected_pcg_traced(&problem, &rhs, &hierarchy, options).unwrap();
    let actual = solve_projected_pcg_traced_with_hierarchy_workspace(
        &problem,
        &rhs,
        &hierarchy,
        options,
        &mut workspace,
    )
    .unwrap();
    assert_bits_equal(&actual, &expected);
    assert_eq!(workspace.retained_bytes().unwrap(), bytes);
    assert_eq!(workspace.retained_buffer_count(), buffers);
}
