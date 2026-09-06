//! Full current-frame LSMR equivalence, independent acceptance and bounded RHS reuse.
#![cfg(feature = "lsmr")]
#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;
use multiway_incidence::{
    HierarchyWeightFrames, PreparedHierarchyTopology, PreparedThreeWayTopology,
    ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    CycleScreenedMapHierarchy, LeastSquaresOptions, PreparedLsmrOptions, PreparedLsmrWorkspace,
    PreparedMapHierarchy, solve_prepared_least_squares, solve_prepared_least_squares_batch_into,
    solve_weighted_least_squares,
};

fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(
        a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
    );
}
#[test]
fn complete_prepared_driver_matches_ordinary_coefficients_diagnostics_and_certificate()
-> Result<(), Box<dyn std::error::Error>> {
    let options = PreparedLsmrOptions {
        tolerance: 1e-10,
        ..Default::default()
    };
    for fixture in fixtures::recursive_holdout_fixtures()? {
        let topology = PreparedThreeWayTopology::try_from_collapsed(
            fixture.problem.topology().level_counts(),
            fixture.problem.topology().tuples(),
        )?;
        let structural =
            PreparedHierarchyTopology::try_new(&topology, fixture.oracle_maps.clone())?;
        let fine = ThreeWayWeightFrame::try_new(
            &topology,
            WeightFrameInput::Tuples(fixture.problem.weights()),
        )?;
        let frames = HierarchyWeightFrames::try_new(&structural, &fine)?;
        let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
        let mut workspace = PreparedLsmrWorkspace::try_new(&hierarchy, options)?;
        let ordinary = CycleScreenedMapHierarchy::from_maps(
            fixture.problem.clone(),
            fixture.oracle_maps.clone(),
            1e-12,
        )?;
        for column in 0..3 {
            let targets: Vec<_> = (0..fine.weights().len())
                .map(|i| ((i + column) as f64 * 0.31).sin())
                .collect();
            let actual = solve_prepared_least_squares(&hierarchy, &targets, &mut workspace)?;
            let expected = solve_weighted_least_squares(
                &fixture.problem,
                &targets,
                &ordinary,
                LeastSquaresOptions {
                    tolerance: options.tolerance,
                    max_iterations: options.max_iterations,
                    local_size: options.local_size,
                },
            )?;
            bits(actual.coefficients, expected.coefficients());
            assert_eq!(actual.report.iterations, expected.iterations());
            assert_eq!(actual.report.native_converged, expected.converged());
            assert_eq!(actual.report.native_stop_reason, expected.stop_reason());
            assert_eq!(
                actual.report.certified_normal_equation_residual.to_bits(),
                expected.certified_normal_equation_residual().to_bits()
            );
            assert_eq!(
                actual.report.accepted,
                expected.is_certified(options.certificate_tolerance)
            );
            assert_eq!(
                actual.report.work.weighted_incidence_applications,
                expected.work().solver_weighted_incidence_applications()
            );
            assert_eq!(
                actual.report.work.weighted_adjoint_applications,
                expected.work().solver_weighted_adjoint_applications()
            );
            assert_eq!(
                actual.report.work.hierarchy_applications,
                expected.work().preconditioner_applications()
            );
            assert_eq!(actual.report.work.certificate.incidence_applications, 1);
            assert_eq!(actual.report.work.certificate.adjoint_applications, 2);
        }
        let report = workspace.payload_report(123)?;
        assert_eq!(
            report.total_payload_bytes,
            PreparedLsmrWorkspace::setup_payload_bound(&hierarchy, options, 123)?
        );
    }
    Ok(())
}

#[test]
fn all_declared_rhs_counts_match_independent_scalar_reuse() {
    let tuples: Vec<_> = (0..2)
        .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let topology = PreparedThreeWayTopology::try_from_collapsed([2; 3], &tuples).unwrap();
    let structural = PreparedHierarchyTopology::try_new(&topology, vec![]).unwrap();
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let options = PreparedLsmrOptions {
        tolerance: 1e-10,
        ..Default::default()
    };
    let mut workspace = PreparedLsmrWorkspace::try_new(&hierarchy, options).unwrap();
    for count in [1, 2, 4, 8, 16, 17, 32] {
        let targets: Vec<_> = (0..count * 8).map(|i| (i as f64 * 0.19).sin()).collect();
        let mut output = vec![f64::NAN; count * 6];
        let mut reports = vec![None; count];
        solve_prepared_least_squares_batch_into(
            &hierarchy,
            &targets,
            count,
            &mut output,
            &mut reports,
            &mut workspace,
        )
        .unwrap();
        for column in 0..count {
            let result = solve_prepared_least_squares(
                &hierarchy,
                &targets[column * 8..(column + 1) * 8],
                &mut workspace,
            )
            .unwrap();
            assert!(result.report.accepted);
            assert_eq!(reports[column], Some(result.report));
            bits(&output[column * 6..(column + 1) * 6], result.coefficients);
        }
    }
}

#[test]
fn batch_failure_preserves_completed_prefix_and_recovers() {
    let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let structural = PreparedHierarchyTopology::try_new(&topology, vec![]).unwrap();
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[2.0])).unwrap();
    let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let equal = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let mut workspace = PreparedLsmrWorkspace::try_new(&hierarchy, Default::default()).unwrap();
    let mut output = [7.0; 9];
    let mut reports = [None; 3];
    let before = format!("{workspace:?}");
    assert!(
        solve_prepared_least_squares_batch_into(
            &equal,
            &[1.0; 3],
            3,
            &mut output,
            &mut reports,
            &mut workspace
        )
        .is_err()
    );
    assert!(
        solve_prepared_least_squares_batch_into(
            &hierarchy,
            &[1.0, f64::NAN, 3.0],
            3,
            &mut output,
            &mut reports,
            &mut workspace
        )
        .is_err()
    );
    assert!(
        solve_prepared_least_squares_batch_into(
            &hierarchy,
            &[1.0; 3],
            0,
            &mut output,
            &mut reports,
            &mut workspace
        )
        .is_err()
    );
    assert_eq!(format!("{workspace:?}"), before);
    assert_eq!(output, [7.0; 9]);
    assert_eq!(reports, [None; 3]);
    assert!(
        solve_prepared_least_squares_batch_into(
            &hierarchy,
            &[1.0, f64::MAX, 3.0],
            3,
            &mut output,
            &mut reports,
            &mut workspace
        )
        .is_err()
    );
    assert!(reports[0].unwrap().accepted);
    assert!(reports[1].is_none() && reports[2].is_none());
    assert_eq!(&output[3..], &[7.0; 6]);
    assert_eq!(workspace.last_work().weighted_incidence_applications, 0);
    let result = solve_prepared_least_squares(&hierarchy, &[0.0], &mut workspace).unwrap();
    assert!(result.report.accepted);
    assert_eq!(result.report.certified_normal_equation_residual, 0.0);
}

#[test]
fn native_stopping_never_overrides_a_stricter_independent_tolerance() {
    let tuples: Vec<_> = (0..2)
        .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let topology = PreparedThreeWayTopology::try_from_collapsed([2; 3], &tuples).unwrap();
    let structural = PreparedHierarchyTopology::try_new(&topology, vec![]).unwrap();
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let options = PreparedLsmrOptions {
        certificate_tolerance: f64::from_bits(1),
        ..Default::default()
    };
    let mut workspace = PreparedLsmrWorkspace::try_new(&hierarchy, options).unwrap();
    let targets: Vec<_> = (0..8).map(|i| (i as f64 * 0.31).sin()).collect();
    let result = solve_prepared_least_squares(&hierarchy, &targets, &mut workspace).unwrap();
    assert!(result.report.native_converged);
    assert!(result.report.certified_normal_equation_residual > options.certificate_tolerance);
    assert!(!result.report.accepted);
}

use multiway_mg::{
    PreparedLsmrGateWorkReport, solve_prepared_least_squares_with_certificate_gate,
    solve_prepared_least_squares_with_certificate_gate_batch_into,
};

#[test]
fn certificate_gate_continues_same_recurrence_on_all_recursive_cases()
-> Result<(), Box<dyn std::error::Error>> {
    let mut total_vetoes = 0;
    for fixture in fixtures::recursive_holdout_fixtures()? {
        let topology = PreparedThreeWayTopology::try_from_collapsed(
            fixture.problem.topology().level_counts(),
            fixture.problem.topology().tuples(),
        )?;
        let structural =
            PreparedHierarchyTopology::try_new(&topology, fixture.oracle_maps.clone())?;
        let fine = ThreeWayWeightFrame::try_new(
            &topology,
            WeightFrameInput::Tuples(fixture.problem.weights()),
        )?;
        let frames = HierarchyWeightFrames::try_new(&structural, &fine)?;
        let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
        // Force native proposals early; acceptance still uses the fixed original tolerance.
        let options = PreparedLsmrOptions {
            tolerance: 1e-2,
            ..Default::default()
        };
        let mut workspace = PreparedLsmrWorkspace::try_new(&hierarchy, options)?;
        let payload = workspace.payload_report(0)?;
        for column in 0..4 {
            let targets: Vec<_> = (0..fine.weights().len())
                .map(|i| {
                    if column == 3 {
                        0.0
                    } else {
                        ((i + column) as f64 * 0.31).sin()
                    }
                })
                .collect();
            let result = solve_prepared_least_squares_with_certificate_gate(
                &hierarchy,
                &targets,
                &mut workspace,
            )?;
            let actual = result.coefficients.to_vec();
            let report = result.report;
            assert!(
                report.solve.accepted,
                "case {} column {column}: {report:?}",
                fixture.name
            );
            total_vetoes += report.gate.candidate_vetoes;
            assert!(report.gate.candidate_vetoes <= report.gate.candidate_checks);
            assert_eq!(
                report.gate.projection_applications,
                report.gate.candidate_checks + 1
            );
            assert_eq!(
                report.gate.candidate_certificate.incidence_applications,
                report.gate.candidate_checks
            );
            assert_eq!(
                report.gate.candidate_certificate.adjoint_applications,
                2 * report.gate.candidate_checks
            );
            assert_eq!(report.solve.work.certificate.incidence_applications, 1);
            assert_eq!(report.solve.work.certificate.adjoint_applications, 2);
            assert_eq!(workspace.last_work(), report.solve.work);
            assert_eq!(workspace.last_gate_work(), report.gate);
            assert_eq!(workspace.payload_report(0)?, payload);
            if report.solve.iterations > 0 {
                // Disable tolerance stopping to compare the same uninterrupted number of steps.
                let mut reference = PreparedLsmrWorkspace::try_new(
                    &hierarchy,
                    PreparedLsmrOptions {
                        tolerance: 1e-300,
                        max_iterations: report.solve.iterations,
                        ..Default::default()
                    },
                )?;
                let expected = solve_prepared_least_squares(&hierarchy, &targets, &mut reference)?;
                assert_eq!(expected.report.iterations, report.solve.iterations);
                bits(&actual, expected.coefficients);
                assert_eq!(
                    expected.report.certified_normal_equation_residual.to_bits(),
                    report.solve.certified_normal_equation_residual.to_bits()
                );
            } else {
                assert_eq!(
                    report.gate,
                    PreparedLsmrGateWorkReport {
                        projection_applications: 1,
                        ..Default::default()
                    }
                );
            }
            let repeated = solve_prepared_least_squares_with_certificate_gate(
                &hierarchy,
                &targets,
                &mut workspace,
            )?;
            bits(&actual, repeated.coefficients);
            assert_eq!(report, repeated.report);
            // A subsequent native solve explicitly resets the gate counters, reusing all storage.
            solve_prepared_least_squares(&hierarchy, &targets, &mut workspace)?;
            assert_eq!(
                workspace.last_gate_work(),
                PreparedLsmrGateWorkReport {
                    projection_applications: 1,
                    ..Default::default()
                }
            );
        }
    }
    assert!(total_vetoes > 0);
    Ok(())
}

#[test]
fn gated_batches_all_widths_match_scalar_and_preserve_failure_suffix() {
    let fixture = fixtures::recursive_holdout_fixtures().unwrap().remove(0);
    let topology = PreparedThreeWayTopology::try_from_collapsed(
        fixture.problem.topology().level_counts(),
        fixture.problem.topology().tuples(),
    )
    .unwrap();
    let structural =
        PreparedHierarchyTopology::try_new(&topology, fixture.oracle_maps.clone()).unwrap();
    let fine = ThreeWayWeightFrame::try_new(
        &topology,
        WeightFrameInput::Tuples(fixture.problem.weights()),
    )
    .unwrap();
    let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let mut workspace = PreparedLsmrWorkspace::try_new(
        &hierarchy,
        PreparedLsmrOptions {
            tolerance: 1e-2,
            ..Default::default()
        },
    )
    .unwrap();
    let e = fine.weights().len();
    let n = hierarchy.dimension();
    for columns in [1, 2, 4, 8, 16, 17, 32] {
        let targets: Vec<_> = (0..columns * e)
            .map(|i| {
                if i / e == 16 || i / e == 31 {
                    0.0
                } else {
                    (i as f64 * 0.19).sin()
                }
            })
            .collect();
        let mut coefficients = vec![7.0; columns * n];
        let mut reports = vec![None; columns];
        solve_prepared_least_squares_with_certificate_gate_batch_into(
            &hierarchy,
            &targets,
            columns,
            &mut coefficients,
            &mut reports,
            &mut workspace,
        )
        .unwrap();
        for column in 0..columns {
            let scalar = solve_prepared_least_squares_with_certificate_gate(
                &hierarchy,
                &targets[column * e..(column + 1) * e],
                &mut workspace,
            )
            .unwrap();
            assert!(scalar.report.solve.accepted);
            assert_eq!(reports[column], Some(scalar.report));
            bits(
                &coefficients[column * n..(column + 1) * n],
                scalar.coefficients,
            );
        }
        let before = coefficients.clone();
        let before_reports = reports.clone();
        let work = (workspace.last_work(), workspace.last_gate_work());
        let equal = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
        assert!(
            solve_prepared_least_squares_with_certificate_gate_batch_into(
                &equal,
                &targets,
                columns,
                &mut coefficients,
                &mut reports,
                &mut workspace
            )
            .is_err()
        );
        assert_eq!(coefficients, before);
        assert_eq!(reports, before_reports);
        assert_eq!((workspace.last_work(), workspace.last_gate_work()), work);
        let mut invalid = targets;
        invalid[0] = f64::NAN;
        assert!(
            solve_prepared_least_squares_with_certificate_gate_batch_into(
                &hierarchy,
                &invalid,
                columns,
                &mut coefficients,
                &mut reports,
                &mut workspace
            )
            .is_err()
        );
        assert_eq!(coefficients, before);
        assert_eq!(reports, before_reports);
        assert_eq!((workspace.last_work(), workspace.last_gate_work()), work);
    }
    let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let structure = PreparedHierarchyTopology::try_new(&topology, vec![]).unwrap();
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[2.0])).unwrap();
    let frames = HierarchyWeightFrames::try_new(&structure, &fine).unwrap();
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let mut workspace = PreparedLsmrWorkspace::try_new(&hierarchy, Default::default()).unwrap();
    let mut coefficients = [7.0; 9];
    let mut reports = [None; 3];
    assert!(
        solve_prepared_least_squares_with_certificate_gate_batch_into(
            &hierarchy,
            &[1.0, f64::MAX, 2.0],
            3,
            &mut coefficients,
            &mut reports,
            &mut workspace
        )
        .is_err()
    );
    assert!(reports[0].unwrap().solve.accepted);
    assert_eq!(&reports[1..], &[None, None]);
    assert_eq!(&coefficients[3..], &[7.0; 6]);
    assert_eq!(workspace.last_gate_work(), Default::default());
    solve_prepared_least_squares_with_certificate_gate_batch_into(
        &hierarchy,
        &[1.0, 0.0, 2.0],
        3,
        &mut coefficients,
        &mut reports,
        &mut workspace,
    )
    .unwrap();
    assert!(reports.iter().all(|r| r.unwrap().solve.accepted));
}

#[test]
fn certificate_gate_does_not_promise_acceptance_at_iteration_limit() {
    let fixture = fixtures::recursive_holdout_fixtures().unwrap().remove(0);
    let topology = PreparedThreeWayTopology::try_from_collapsed(
        fixture.problem.topology().level_counts(),
        fixture.problem.topology().tuples(),
    )
    .unwrap();
    let structural =
        PreparedHierarchyTopology::try_new(&topology, fixture.oracle_maps.clone()).unwrap();
    let fine = ThreeWayWeightFrame::try_new(
        &topology,
        WeightFrameInput::Tuples(fixture.problem.weights()),
    )
    .unwrap();
    let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let mut workspace = PreparedLsmrWorkspace::try_new(
        &hierarchy,
        PreparedLsmrOptions {
            tolerance: 1.0,
            certificate_tolerance: 1e-15,
            max_iterations: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let targets: Vec<_> = (0..fine.weights().len())
        .map(|i| (i as f64 * 0.31).sin())
        .collect();
    let result =
        solve_prepared_least_squares_with_certificate_gate(&hierarchy, &targets, &mut workspace)
            .unwrap();
    assert_eq!(result.report.solve.iterations, 1);
    assert!(!result.report.solve.accepted);
    assert_eq!(result.report.gate.candidate_checks, 0);
    assert_eq!(
        result.report.solve.work.certificate.incidence_applications,
        1
    );
}
