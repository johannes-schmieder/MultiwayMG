//! Full prepared LSMR/certificate/bounded-RHS allocation and lifetime accounting.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyTopology, PreparedThreeWayTopology,
    ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    PreparedLsmrOptions, PreparedLsmrWorkspace, PreparedMapHierarchy, solve_prepared_least_squares,
    solve_prepared_least_squares_batch_into, solve_prepared_least_squares_with_certificate_gate,
    solve_prepared_least_squares_with_certificate_gate_batch_into,
};
use std::hint::black_box;
pub fn run() -> Result<()> {
    let tuples: Vec<_> = (0..4)
        .flat_map(|i| (0..4).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
        .collect();
    let topology = PreparedThreeWayTopology::try_from_collapsed([4; 3], &tuples)?;
    let structural = PreparedHierarchyTopology::try_new(
        &topology,
        vec![
            FactorAggregation::consecutive_halving([4; 3])?,
            FactorAggregation::consecutive_halving([2; 3])?,
        ],
    )?;
    let weights: Vec<_> = (0..64).map(|i| 2.0_f64.powi((i * 5 % 13) - 6)).collect();
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&weights))?;
    let frames = HierarchyWeightFrames::try_new(&structural, &fine)?;
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
    let options = PreparedLsmrOptions {
        tolerance: 1e-10,
        ..Default::default()
    };
    let required = PreparedLsmrWorkspace::setup_payload_bound(&hierarchy, options, 123)?;
    let before = GLOBAL.stats();
    assert!(
        PreparedLsmrWorkspace::try_new_with_payload_budget(&hierarchy, options, required - 1, 123)
            .is_err()
    );
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let mut workspace =
        PreparedLsmrWorkspace::try_new_with_payload_budget(&hierarchy, options, required, 123)?;
    let setup = GLOBAL.stats() - before;
    let retained = workspace.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 40);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.deallocations, 0);
    assert_eq!(setup.bytes_allocated, retained);
    assert_eq!(workspace.payload_report(123)?.total_payload_bytes, required);
    let targets: Vec<_> = (0..64 * 32)
        .map(|i| {
            if i / 64 == 16 || i / 64 == 31 {
                0.0
            } else {
                (i as f64 * 0.19).sin()
            }
        })
        .collect();
    let mut output = vec![0.0; 12 * 32];
    let mut reports = vec![None; 32];
    let before = GLOBAL.stats();
    let first =
        solve_prepared_least_squares(&hierarchy, black_box(&targets[..64]), &mut workspace)?;
    assert!(first.report.accepted);
    for count in [1, 2, 4, 8, 16, 17, 32] {
        solve_prepared_least_squares_batch_into(
            &hierarchy,
            black_box(&targets[..64 * count]),
            count,
            &mut output[..12 * count],
            &mut reports[..count],
            &mut workspace,
        )?;
        assert!(reports[..count].iter().all(|r| r.unwrap().accepted));
    }
    assert!(solve_prepared_least_squares(&hierarchy, &targets[..63], &mut workspace).is_err());
    assert!(solve_prepared_least_squares(&hierarchy, &[f64::NAN; 64], &mut workspace).is_err());
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    drop(workspace);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, retained);
    assert_eq!(released.allocations, 0);
    let gated_options = PreparedLsmrOptions {
        tolerance: 1e-2,
        ..Default::default()
    };
    let required = PreparedLsmrWorkspace::setup_payload_bound(&hierarchy, gated_options, 123)?;
    let before = GLOBAL.stats();
    let mut gated = PreparedLsmrWorkspace::try_new_with_payload_budget(
        &hierarchy,
        gated_options,
        required,
        123,
    )?;
    let setup = GLOBAL.stats() - before;
    assert_eq!(setup.allocations, 40);
    assert_eq!(setup.bytes_allocated, retained);
    assert_eq!(gated.retained_payload_bytes()?, retained);
    assert_eq!((setup.deallocations, setup.reallocations), (0, 0));
    let mut gate_reports = vec![None; 32];
    let before = GLOBAL.stats();
    let first = solve_prepared_least_squares_with_certificate_gate(
        &hierarchy,
        black_box(&targets[..64]),
        &mut gated,
    )?;
    assert!(first.report.solve.accepted);
    let mut vetoes = first.report.gate.candidate_vetoes;
    for count in [1, 2, 4, 8, 16, 17, 32] {
        solve_prepared_least_squares_with_certificate_gate_batch_into(
            &hierarchy,
            black_box(&targets[..64 * count]),
            count,
            &mut output[..12 * count],
            &mut gate_reports[..count],
            &mut gated,
        )?;
        for report in &gate_reports[..count] {
            let report = report.unwrap();
            assert!(report.solve.accepted);
            assert_eq!(
                report.gate.projection_applications,
                report.gate.candidate_checks + 1
            );
            vetoes += report.gate.candidate_vetoes;
        }
    }
    assert!(vetoes > 0);
    assert!(
        solve_prepared_least_squares_with_certificate_gate(&hierarchy, &targets[..63], &mut gated)
            .is_err()
    );
    assert!(
        solve_prepared_least_squares_with_certificate_gate(&hierarchy, &[f64::NAN; 64], &mut gated)
            .is_err()
    );
    assert!(
        solve_prepared_least_squares_with_certificate_gate(&hierarchy, &[f64::MAX; 64], &mut gated)
            .is_err()
    );
    assert!(
        solve_prepared_least_squares_with_certificate_gate(&hierarchy, &targets[..64], &mut gated)?
            .report
            .solve
            .accepted
    );
    no_events(GLOBAL.stats() - before);
    #[cfg(feature = "profiling")]
    {
        use multiway_incidence::profiling::{Phase, collect};
        let mut expected = [0.0; 12];
        let reference = solve_prepared_least_squares_with_certificate_gate(
            &hierarchy,
            &targets[..64],
            &mut gated,
        )?;
        expected.copy_from_slice(reference.coefficients);
        let expected_report = reference.report;
        let mut actual = [0.0; 12];
        let before = GLOBAL.stats();
        for _ in 0..4 {
            let (result, profile) = collect(|| {
                solve_prepared_least_squares_with_certificate_gate(
                    &hierarchy,
                    &targets[..64],
                    &mut gated,
                )
                .map(|r| {
                    actual.copy_from_slice(r.coefficients);
                    r.report
                })
            })?;
            let report = result?;
            assert_eq!(report, expected_report);
            super::equal_bits(&actual, &expected);
            assert!(profile.valid);
            let calls = |p: Phase| profile.phases[p as usize].calls as usize;
            assert_eq!(calls(Phase::PreparedLsmr), 1);
            assert_eq!(calls(Phase::Certificate), report.gate.candidate_checks + 1);
            assert_eq!(
                calls(Phase::WeightedIncidence),
                report.solve.work.weighted_incidence_applications
            );
            assert_eq!(
                calls(Phase::WeightedAdjoint),
                report.solve.work.weighted_adjoint_applications
            );
            assert_eq!(
                calls(Phase::Incidence),
                report.solve.work.weighted_incidence_applications
                    + report.gate.candidate_checks
                    + 1
            );
            assert_eq!(calls(Phase::Rhs), 2 * (report.gate.candidate_checks + 1));
            assert_eq!(
                calls(Phase::Projection),
                report.gate.projection_applications + 15 * report.solve.work.hierarchy_applications
            );
            assert_eq!(
                calls(Phase::MapSweep),
                4 * report.solve.work.hierarchy_applications
            );
            assert_eq!(
                calls(Phase::Gramian),
                4 * report.solve.work.hierarchy_applications
            );
            assert_eq!(
                calls(Phase::DenseTerminal),
                report.solve.work.hierarchy_applications
            );
            assert!(
                profile.phases.iter().map(|p| p.exclusive_ns).sum::<u128>() <= profile.elapsed_ns
            );
        }
        let (failed, profile) = collect(|| {
            solve_prepared_least_squares_with_certificate_gate(
                &hierarchy,
                &[f64::NAN; 64],
                &mut gated,
            )
        })?;
        assert!(failed.is_err());
        assert!(profile.valid);
        no_events(GLOBAL.stats() - before);
    }
    let before = GLOBAL.stats();
    drop(gated);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, retained);
    assert_eq!(released.deallocations, 40);
    assert_eq!((released.allocations, released.reallocations), (0, 0));
    println!(
        "complete prepared native/gated LSMR: first/scalar and RHS1,2,4,8,16,17,32, vetoes and error/recovery allocations=0; all 40 arrays released per workspace"
    );
    Ok(())
}
