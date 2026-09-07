//! Complete grouped solves retain exactly one optional shared image, with no action allocation.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyGrouping, PreparedHierarchyTopology,
    PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    GroupedGramianMode, PreparedMapHierarchy, PreparedPcgWorkspace,
    solve_prepared_pcg_least_squares,
};
#[cfg(feature = "lsmr")]
use multiway_mg::{PreparedLsmrWorkspace, solve_prepared_least_squares_with_certificate_gate};
use std::hint::black_box;

pub fn run() -> Result<()> {
    let tuples: Vec<_> = (0..4)
        .flat_map(|i| (0..4).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([4; 3], &tuples)?;
    let make = || {
        PreparedHierarchyTopology::try_new(
            &t,
            vec![
                FactorAggregation::consecutive_halving([4; 3]).unwrap(),
                FactorAggregation::consecutive_halving([2; 3]).unwrap(),
            ],
        )
        .unwrap()
    };
    let h = make();
    let foreign_h = make();
    let foreign_groups = PreparedHierarchyGrouping::try_new(&foreign_h, 2)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let frames = HierarchyWeightFrames::try_new(&h, &f)?;
    let scalar = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
    let scalar_cycle_bytes = scalar.workspace_required_bytes()?;
    let mut scalar_pcg = PreparedPcgWorkspace::try_new(&scalar, Default::default())?;
    let target: [f64; 64] = std::array::from_fn(|i| (i as f64 * 0.19).sin());
    let large_target: [f64; 64] = std::array::from_fn(|i| target[i] * 1e155);
    let mut expected = [0.0; 12];
    let r = solve_prepared_pcg_least_squares(&scalar, &target, &mut scalar_pcg)?;
    expected.copy_from_slice(r.coefficients);
    let expected_report = r.report;
    #[cfg(feature = "lsmr")]
    let (expected_lsmr, expected_lsmr_report) = {
        let mut w = PreparedLsmrWorkspace::try_new(&scalar, Default::default())?;
        let r = solve_prepared_least_squares_with_certificate_gate(&scalar, &target, &mut w)?;
        let mut x = [0.0; 12];
        x.copy_from_slice(r.coefficients);
        (x, r.report)
    };
    let before = GLOBAL.stats();
    assert!(
        PreparedMapHierarchy::try_new_with_grouping(
            &frames,
            &foreign_groups,
            GroupedGramianMode::TupleImage,
            1e-12
        )
        .is_err()
    );
    no_events(GLOBAL.stats() - before);
    for prefix in [0, 1, 2] {
        let before = GLOBAL.stats();
        let groups = PreparedHierarchyGrouping::try_new(&h, prefix)?;
        let setup = GLOBAL.stats() - before;
        let retained = groups.retained_payload_bytes()?;
        assert_eq!(
            setup.allocations,
            if prefix == 0 { 0 } else { 1 + 3 * prefix }
        );
        assert_eq!(setup.deallocations, prefix);
        assert_eq!(setup.reallocations, 0);
        assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, retained);
        assert_eq!(
            retained,
            PreparedHierarchyGrouping::required_payload_bytes(&h, prefix)?
        );
        for mode in [
            GroupedGramianMode::RowGather,
            GroupedGramianMode::TupleImage,
        ] {
            let before = GLOBAL.stats();
            let numerical =
                PreparedMapHierarchy::try_new_with_grouping(&frames, &groups, mode, 1e-12)?;
            let setup = GLOBAL.stats() - before;
            let terminal = numerical.retained_payload_bytes()?;
            assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, terminal);
            let image = usize::from(prefix > 0 && mode == GroupedGramianMode::TupleImage);
            assert_eq!(
                numerical.workspace_required_bytes()?,
                scalar_cycle_bytes + image * 64 * 8
            );
            let required =
                PreparedPcgWorkspace::setup_payload_bound(&numerical, Default::default(), 17)?;
            let before = GLOBAL.stats();
            assert!(
                PreparedPcgWorkspace::try_new_with_payload_budget(
                    &numerical,
                    Default::default(),
                    required - 1,
                    17
                )
                .is_err()
            );
            no_events(GLOBAL.stats() - before);
            let before = GLOBAL.stats();
            let mut workspace = PreparedPcgWorkspace::try_new_with_payload_budget(
                &numerical,
                Default::default(),
                required,
                17,
            )?;
            let setup = GLOBAL.stats() - before;
            let payload = workspace.payload_report(17)?;
            let workspace_bytes =
                payload.hierarchy.workspace_payload_bytes + payload.outer_workspace_payload_bytes;
            assert_eq!(setup.allocations, 24);
            assert_eq!(setup.deallocations, 0);
            assert_eq!(setup.reallocations, 0);
            assert_eq!(setup.bytes_allocated, workspace_bytes);
            assert_eq!(payload.hierarchy.grouping_payload_bytes, retained);
            assert_eq!(payload.total_payload_bytes, required);
            let before = GLOBAL.stats();
            for _ in 0..32 {
                let r = solve_prepared_pcg_least_squares(
                    &numerical,
                    black_box(&target),
                    &mut workspace,
                )?;
                assert_eq!(r.report, expected_report);
                for i in 0..12 {
                    assert_eq!(r.coefficients[i].to_bits(), expected[i].to_bits());
                }
            }
            assert!(solve_prepared_pcg_least_squares(&scalar, &target, &mut workspace).is_err());
            assert!(
                solve_prepared_pcg_least_squares(&numerical, &[f64::NAN; 64], &mut workspace)
                    .is_err()
            );
            assert!(
                solve_prepared_pcg_least_squares(&numerical, &[f64::MAX; 64], &mut workspace)
                    .is_err()
            );
            assert!(matches!(
                solve_prepared_pcg_least_squares(&numerical, &large_target, &mut workspace),
                Err(multiway_mg::MultiwayError::PcgMetricBreakdown {
                    iteration: 0,
                    context: "initial preconditioned metric",
                    ..
                })
            ));
            solve_prepared_pcg_least_squares(&numerical, &target, &mut workspace)?;
            no_events(GLOBAL.stats() - before);
            let before = GLOBAL.stats();
            drop(workspace);
            let released = GLOBAL.stats() - before;
            assert_eq!(released.bytes_deallocated, workspace_bytes);
            assert_eq!(released.deallocations, 24);
            assert_eq!(released.allocations, 0);
            #[cfg(feature = "lsmr")]
            {
                let required =
                    PreparedLsmrWorkspace::setup_payload_bound(&numerical, Default::default(), 17)?;
                let before = GLOBAL.stats();
                assert!(
                    PreparedLsmrWorkspace::try_new_with_payload_budget(
                        &numerical,
                        Default::default(),
                        required - 1,
                        17
                    )
                    .is_err()
                );
                no_events(GLOBAL.stats() - before);
                let before = GLOBAL.stats();
                let mut workspace = PreparedLsmrWorkspace::try_new_with_payload_budget(
                    &numerical,
                    Default::default(),
                    required,
                    17,
                )?;
                let setup = GLOBAL.stats() - before;
                let payload = workspace.payload_report(17)?;
                let workspace_bytes = payload.hierarchy.workspace_payload_bytes
                    + payload.outer_workspace_payload_bytes;
                assert_eq!(setup.allocations, 30);
                assert_eq!(setup.deallocations, 0);
                assert_eq!(setup.reallocations, 0);
                assert_eq!(setup.bytes_allocated, workspace_bytes);
                assert_eq!(payload.hierarchy.grouping_payload_bytes, retained);
                assert_eq!(payload.total_payload_bytes, required);
                let before = GLOBAL.stats();
                for _ in 0..32 {
                    let r = solve_prepared_least_squares_with_certificate_gate(
                        &numerical,
                        black_box(&target),
                        &mut workspace,
                    )?;
                    assert_eq!(r.report, expected_lsmr_report);
                    for i in 0..12 {
                        assert_eq!(r.coefficients[i].to_bits(), expected_lsmr[i].to_bits());
                    }
                }
                assert!(
                    solve_prepared_least_squares_with_certificate_gate(
                        &scalar,
                        &target,
                        &mut workspace
                    )
                    .is_err()
                );
                assert!(
                    solve_prepared_least_squares_with_certificate_gate(
                        &numerical,
                        &[f64::NAN; 64],
                        &mut workspace
                    )
                    .is_err()
                );
                assert!(
                    solve_prepared_least_squares_with_certificate_gate(
                        &numerical,
                        &[f64::MAX; 64],
                        &mut workspace
                    )
                    .is_err()
                );
                solve_prepared_least_squares_with_certificate_gate(
                    &numerical,
                    &target,
                    &mut workspace,
                )?;
                no_events(GLOBAL.stats() - before);
                let before = GLOBAL.stats();
                drop(workspace);
                let released = GLOBAL.stats() - before;
                assert_eq!(released.bytes_deallocated, workspace_bytes);
                assert_eq!(released.deallocations, 30);
                assert_eq!(released.allocations, 0);
            }
            let before = GLOBAL.stats();
            drop(numerical);
            let released = GLOBAL.stats() - before;
            assert_eq!(released.bytes_deallocated, terminal);
            assert_eq!(released.allocations, 0);
        }
        let before = GLOBAL.stats();
        drop(groups);
        let released = GLOBAL.stats() - before;
        assert_eq!(released.bytes_deallocated, retained);
        assert_eq!(
            released.deallocations,
            if prefix == 0 { 0 } else { 1 + 2 * prefix }
        );
        assert_eq!(released.allocations, 0);
    }
    println!(
        "complete grouped hierarchy: all prefixes/modes first/repeat32/errors allocate zero; one optional image, exact grouping/PCG/LSMR capacities and release"
    );
    Ok(())
}
