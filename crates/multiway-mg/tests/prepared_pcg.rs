//! Complete prepared PCG, original-operator acceptance and scalar RHS reuse.
#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;
use multiway_incidence::{
    HierarchyWeightFrames, PreparedHierarchyTopology, PreparedThreeWayTopology,
    ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    CycleScreenedMapHierarchy, PcgOptions, PreparedMapHierarchy, PreparedPcgOptions,
    PreparedPcgWorkspace, solve_prepared_pcg_batch_into, solve_prepared_pcg_least_squares,
    solve_projected_pcg,
};
fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(
        a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
    );
}
#[test]
fn recursive_prepared_pcg_matches_ordinary_coefficients_and_native_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let options = PreparedPcgOptions {
        pcg: PcgOptions {
            relative_tolerance: 1e-10,
            ..Default::default()
        },
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
        let ordinary = CycleScreenedMapHierarchy::from_maps(
            fixture.problem.clone(),
            fixture.oracle_maps.clone(),
            1e-12,
        )?;
        let mut workspace = PreparedPcgWorkspace::try_new(&hierarchy, options)?;
        for column in 0..3 {
            let targets: Vec<_> = (0..fine.weights().len())
                .map(|i| ((i + column) as f64 * 0.31).sin())
                .collect();
            let rhs = fixture.problem.rhs_from_targets(&targets)?;
            let expected = solve_projected_pcg(&fixture.problem, &rhs, &ordinary, options.pcg)?;
            let actual = solve_prepared_pcg_least_squares(&hierarchy, &targets, &mut workspace)?;
            bits(actual.coefficients, expected.solution());
            assert_eq!(actual.report.iterations, expected.iterations());
            assert_eq!(actual.report.native_converged, expected.converged());
            assert_eq!(actual.report.native_stop_reason, expected.stop_reason());
            assert_eq!(
                actual.report.native_residual_norm.to_bits(),
                expected.residual_norm().to_bits()
            );
            assert_eq!(
                actual.report.native_relative_residual.to_bits(),
                expected.relative_residual().to_bits()
            );
            assert_eq!(
                actual.report.rhs_projection_norm.to_bits(),
                expected.rhs_projection_norm().to_bits()
            );
            assert!(
                actual.report.accepted,
                "{}: {:?}",
                fixture.name, actual.report
            );
            assert_eq!(actual.report.work.rhs_adjoint_applications, 1);
            assert_eq!(actual.report.work.certificate.incidence_applications, 1);
            assert_eq!(actual.report.work.certificate.adjoint_applications, 2);
        }
        assert_eq!(
            workspace.payload_report(123)?.total_payload_bytes,
            PreparedPcgWorkspace::setup_payload_bound(&hierarchy, options, 123)?
        );
    }
    Ok(())
}
#[test]
fn batch_counts_failure_prefix_static_rejection_and_recovery() {
    let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let structural = PreparedHierarchyTopology::try_new(&topology, vec![]).unwrap();
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[2.0])).unwrap();
    let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let other = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let mut workspace = PreparedPcgWorkspace::try_new(&hierarchy, Default::default()).unwrap();
    for count in [1, 2, 4, 8, 16, 17, 32] {
        let targets: Vec<_> = (0..count).map(|i| (i as f64 * 0.19).sin()).collect();
        let mut output = vec![0.0; 3 * count];
        let mut reports = vec![None; count];
        solve_prepared_pcg_batch_into(
            &hierarchy,
            &targets,
            count,
            &mut output,
            &mut reports,
            &mut workspace,
        )
        .unwrap();
        for column in 0..count {
            let result = solve_prepared_pcg_least_squares(
                &hierarchy,
                &targets[column..column + 1],
                &mut workspace,
            )
            .unwrap();
            assert!(result.report.accepted);
            assert_eq!(reports[column], Some(result.report));
            bits(&output[column * 3..(column + 1) * 3], result.coefficients);
        }
    }
    let mut output = [7.0; 9];
    let mut reports = [None; 3];
    let before = format!("{workspace:?}");
    assert!(
        solve_prepared_pcg_batch_into(
            &other,
            &[1.0; 3],
            3,
            &mut output,
            &mut reports,
            &mut workspace
        )
        .is_err()
    );
    assert!(
        solve_prepared_pcg_batch_into(
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
        solve_prepared_pcg_batch_into(
            &hierarchy,
            &[1.0; 3],
            33,
            &mut output,
            &mut reports,
            &mut workspace
        )
        .is_err()
    );
    assert_eq!(format!("{workspace:?}"), before);
    assert_eq!(output, [7.0; 9]);
    assert!(
        solve_prepared_pcg_batch_into(
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
    assert_eq!(workspace.last_work().rhs_adjoint_applications, 1);
    assert_eq!(workspace.last_work().gramian_applications, 0);
    let result = solve_prepared_pcg_least_squares(&hierarchy, &[0.0], &mut workspace).unwrap();
    assert!(result.report.accepted);
    assert_eq!(result.report.iterations, 0);
}
#[test]
fn iteration_cap_remains_uncertified_and_extra_nullity_is_allowed()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixtures::recursive_holdout_fixtures()?.remove(0);
    let topology = PreparedThreeWayTopology::try_from_collapsed(
        fixture.problem.topology().level_counts(),
        fixture.problem.topology().tuples(),
    )?;
    let structural = PreparedHierarchyTopology::try_new(&topology, fixture.oracle_maps)?;
    let fine = ThreeWayWeightFrame::try_new(
        &topology,
        WeightFrameInput::Tuples(fixture.problem.weights()),
    )?;
    let frames = HierarchyWeightFrames::try_new(&structural, &fine)?;
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
    let mut workspace = PreparedPcgWorkspace::try_new(
        &hierarchy,
        PreparedPcgOptions {
            pcg: PcgOptions {
                max_iterations: 1,
                relative_tolerance: 1e-12,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    let targets: Vec<_> = (0..fine.weights().len())
        .map(|i| (i as f64 * 0.31).sin())
        .collect();
    let result = solve_prepared_pcg_least_squares(&hierarchy, &targets, &mut workspace)?;
    assert!(!result.report.native_converged);
    assert!(!result.report.accepted);
    assert!(result.report.work.gramian_applications >= 2);
    let topology =
        PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0, 0, 0], [0, 1, 1], [1, 0, 1]])?;
    let structural = PreparedHierarchyTopology::try_new(&topology, vec![])?;
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples)?;
    let frames = HierarchyWeightFrames::try_new(&structural, &fine)?;
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
    assert_eq!(hierarchy.terminal_rank(), 3);
    let mut workspace = PreparedPcgWorkspace::try_new(&hierarchy, Default::default())?;
    assert!(
        solve_prepared_pcg_least_squares(&hierarchy, &[1.0, -2.0, 3.0], &mut workspace)?
            .report
            .accepted
    );
    Ok(())
}
