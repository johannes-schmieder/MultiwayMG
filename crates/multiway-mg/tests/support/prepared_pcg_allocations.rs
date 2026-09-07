//! Complete prepared PCG/certificate/RHS allocation and release accounting.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyTopology, PreparedThreeWayTopology,
    ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    PcgOptions, PreparedMapHierarchy, PreparedPcgOptions, PreparedPcgWorkspace,
    solve_prepared_pcg_batch_into, solve_prepared_pcg_least_squares,
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
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples)?;
    let frames = HierarchyWeightFrames::try_new(&structural, &fine)?;
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
    let options = PreparedPcgOptions {
        pcg: PcgOptions {
            relative_tolerance: 1e-10,
            ..Default::default()
        },
        ..Default::default()
    };
    let required = PreparedPcgWorkspace::setup_payload_bound(&hierarchy, options, 123)?;
    let before = GLOBAL.stats();
    assert!(
        PreparedPcgWorkspace::try_new_with_payload_budget(&hierarchy, options, required - 1, 123)
            .is_err()
    );
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let mut workspace =
        PreparedPcgWorkspace::try_new_with_payload_budget(&hierarchy, options, required, 123)?;
    let setup = GLOBAL.stats() - before;
    let retained = workspace.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 25);
    assert_eq!(setup.deallocations, 0);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.bytes_allocated, retained);
    assert_eq!(workspace.payload_report(123)?.total_payload_bytes, required);
    let targets: Vec<_> = (0..64 * 32).map(|i| (i as f64 * 0.19).sin()).collect();
    let mut output = vec![0.0; 12 * 32];
    let mut reports = vec![None; 32];
    let before = GLOBAL.stats();
    assert!(
        solve_prepared_pcg_least_squares(&hierarchy, black_box(&targets[..64]), &mut workspace)?
            .report
            .accepted
    );
    for count in [1, 2, 4, 8, 16, 17, 32] {
        solve_prepared_pcg_batch_into(
            &hierarchy,
            black_box(&targets[..64 * count]),
            count,
            &mut output[..12 * count],
            &mut reports[..count],
            &mut workspace,
        )?;
        assert!(reports[..count].iter().all(|r| r.unwrap().accepted));
    }
    assert!(solve_prepared_pcg_least_squares(&hierarchy, &targets[..63], &mut workspace).is_err());
    assert!(solve_prepared_pcg_least_squares(&hierarchy, &[f64::NAN; 64], &mut workspace).is_err());
    no_events(GLOBAL.stats() - before);
    #[cfg(feature = "profiling")]
    {
        use multiway_incidence::profiling::{Phase, collect};
        let mut expected = [0.0; 12];
        let reference =
            solve_prepared_pcg_least_squares(&hierarchy, &targets[..64], &mut workspace)?;
        expected.copy_from_slice(reference.coefficients);
        let expected_report = reference.report;
        let mut actual = [0.0; 12];
        let before = GLOBAL.stats();
        for _ in 0..4 {
            let (result, profile) = collect(|| {
                solve_prepared_pcg_least_squares(&hierarchy, &targets[..64], &mut workspace).map(
                    |r| {
                        actual.copy_from_slice(r.coefficients);
                        r.report
                    },
                )
            })?;
            let report = result?;
            assert_eq!(report, expected_report);
            super::equal_bits(&actual, &expected);
            assert!(profile.valid);
            let calls = |p: Phase| profile.phases[p as usize].calls as usize;
            assert_eq!(calls(Phase::PreparedPcg), 1);
            assert_eq!(calls(Phase::PcgRecurrence), 1);
            assert_eq!(calls(Phase::Certificate), 1);
            assert_eq!(calls(Phase::Incidence), 1);
            assert_eq!(calls(Phase::Rhs), 2 + report.work.rhs_adjoint_applications);
            assert_eq!(
                calls(Phase::Projection),
                report.work.projection_applications + 15 * report.work.hierarchy_applications
            );
            assert_eq!(
                calls(Phase::MapSweep),
                4 * report.work.hierarchy_applications
            );
            assert_eq!(
                calls(Phase::Gramian),
                report.work.gramian_applications + 4 * report.work.hierarchy_applications
            );
            assert_eq!(
                calls(Phase::DenseTerminal),
                report.work.hierarchy_applications
            );
            assert!(
                profile.phases.iter().map(|p| p.exclusive_ns).sum::<u128>() <= profile.elapsed_ns
            );
        }
        no_events(GLOBAL.stats() - before);
    }
    let before = GLOBAL.stats();
    drop(workspace);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, retained);
    assert_eq!(released.allocations, 0);
    println!(
        "complete prepared PCG: first/scalar and RHS1,2,4,8,16,17,32 allocations=0; exact 25-array release"
    );
    Ok(())
}
