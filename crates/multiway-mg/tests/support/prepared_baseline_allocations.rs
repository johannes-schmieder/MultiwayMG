//! Baseline and complete shared-solver array lifetimes and allocation-free reuse.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    PreparedHierarchyBudget, PreparedThreeWayTopology, PreparedTupleGrouping, ThreeWayWeightFrame,
    WeightFrameInput,
};
use multiway_mg::{
    GroupedGramianMode, PreparedBaseline, PreparedBaselineKind, PreparedPcgWorkspace,
    PreparedSolverAction, solve_prepared_pcg_batch_into, solve_prepared_pcg_least_squares,
};
fn setup(s: stats_alloc::Stats, count: usize, retained: usize) {
    assert_eq!(s.allocations, count);
    assert_eq!(s.deallocations, 0);
    assert_eq!(s.reallocations, 0);
    assert_eq!(s.bytes_allocated, retained);
}
fn release(s: stats_alloc::Stats, count: usize, retained: usize) {
    assert_eq!(s.allocations, 0);
    assert_eq!(s.deallocations, count);
    assert_eq!(s.reallocations, 0);
    assert_eq!(s.bytes_deallocated, retained);
}
pub fn run() -> Result<()> {
    let keys: Vec<_> = (0..2)
        .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &keys)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let g = PreparedTupleGrouping::try_new(&t)?;
    for kind in [
        PreparedBaselineKind::Identity,
        PreparedBaselineKind::InverseDiagonal,
        PreparedBaselineKind::SymmetricMap,
    ] {
        for mode in [
            None,
            Some(GroupedGramianMode::RowGather),
            Some(GroupedGramianMode::TupleImage),
        ] {
            let mut owner = PreparedBaseline::new(&f, kind);
            if let Some(mode) = mode {
                owner = owner.with_grouping(&g, mode)?;
            }
            let count = (if kind == PreparedBaselineKind::Identity {
                0
            } else if kind == PreparedBaselineKind::SymmetricMap {
                4
            } else {
                2
            }) + usize::from(mode == Some(GroupedGramianMode::TupleImage));
            let required = owner.workspace_setup_payload_bound(37)?;
            let before = GLOBAL.stats();
            assert!(
                owner
                    .application_workspace_with_budget(PreparedHierarchyBudget {
                        maximum_payload_bytes: required - 1,
                        additional_live_payload_bytes: 37
                    })
                    .is_err()
            );
            no_events(GLOBAL.stats() - before);
            let before = GLOBAL.stats();
            let mut w = owner.application_workspace_with_budget(PreparedHierarchyBudget {
                maximum_payload_bytes: required,
                additional_live_payload_bytes: 37,
            })?;
            let delta = GLOBAL.stats() - before;
            let retained = w.retained_payload_bytes()?;
            setup(delta, count, retained);
            let other = owner;
            let rhs = [0.25, -1., 2., 0.75, 0., 1.];
            let mut out = [0.; 6];
            let before = GLOBAL.stats();
            for _ in 0..32 {
                owner.apply_with_workspace(&rhs, &mut out, &mut w)?;
                owner.fine_gramian_with_workspace(&rhs, &mut out, &mut w)?;
            }
            assert!(other.apply_with_workspace(&rhs, &mut out, &mut w).is_err());
            assert!(
                other
                    .fine_gramian_with_workspace(&rhs, &mut out, &mut w)
                    .is_err()
            );
            assert!(
                owner
                    .apply_with_workspace(&rhs[..5], &mut out, &mut w)
                    .is_err()
            );
            assert!(
                owner
                    .apply_with_workspace(&[f64::NAN; 6], &mut out, &mut w)
                    .is_err()
            );
            owner.apply_with_workspace(&rhs, &mut out, &mut w)?;
            no_events(GLOBAL.stats() - before);
            let before = GLOBAL.stats();
            drop(w);
            release(GLOBAL.stats() - before, count, retained);

            let options = Default::default();
            let required = PreparedPcgWorkspace::setup_payload_bound(&owner, options, 37)?;
            let before = GLOBAL.stats();
            assert!(
                PreparedPcgWorkspace::try_new_with_payload_budget(
                    &owner,
                    options,
                    required - 1,
                    37
                )
                .is_err()
            );
            no_events(GLOBAL.stats() - before);
            let before = GLOBAL.stats();
            let mut pcg =
                PreparedPcgWorkspace::try_new_with_payload_budget(&owner, options, required, 37)?;
            let delta = GLOBAL.stats() - before;
            let retained = pcg.retained_payload_bytes()?;
            setup(delta, count + 10, retained);
            assert_eq!(pcg.payload_report(37)?.total_payload_bytes, required);
            let targets: Vec<_> = (0..8 * 32)
                .map(|i| {
                    if (i / 8) % 3 == 0 {
                        0.
                    } else {
                        (i as f64 * 0.31).sin()
                    }
                })
                .collect();
            let mut output = vec![0.; 6 * 32];
            let mut reports = vec![None; 32];
            let before = GLOBAL.stats();
            assert!(
                solve_prepared_pcg_least_squares(&owner, &targets[..8], &mut pcg)?
                    .report
                    .accepted
            );
            for count in [1, 2, 4, 8, 16, 17, 32] {
                solve_prepared_pcg_batch_into(
                    &owner,
                    &targets[..8 * count],
                    count,
                    &mut output[..6 * count],
                    &mut reports[..count],
                    &mut pcg,
                )?;
                assert!(reports[..count].iter().all(|x| x.unwrap().accepted));
            }
            assert!(solve_prepared_pcg_least_squares(&other, &targets[..8], &mut pcg).is_err());
            assert!(solve_prepared_pcg_least_squares(&owner, &targets[..7], &mut pcg).is_err());
            assert!(solve_prepared_pcg_least_squares(&owner, &[f64::NAN; 8], &mut pcg).is_err());
            assert!(
                solve_prepared_pcg_least_squares(&owner, &targets[8..16], &mut pcg)?
                    .report
                    .accepted
            );
            no_events(GLOBAL.stats() - before);
            let before = GLOBAL.stats();
            drop(pcg);
            release(GLOBAL.stats() - before, count + 10, retained);
            #[cfg(feature = "lsmr")]
            {
                use multiway_mg::{
                    PreparedLsmrWorkspace, solve_prepared_least_squares_with_certificate_gate,
                    solve_prepared_least_squares_with_certificate_gate_batch_into,
                };
                let options = Default::default();
                let required = PreparedLsmrWorkspace::setup_payload_bound(&owner, options, 37)?;
                let before = GLOBAL.stats();
                assert!(
                    PreparedLsmrWorkspace::try_new_with_payload_budget(
                        &owner,
                        options,
                        required - 1,
                        37
                    )
                    .is_err()
                );
                no_events(GLOBAL.stats() - before);
                let before = GLOBAL.stats();
                let mut lsmr = PreparedLsmrWorkspace::try_new_with_payload_budget(
                    &owner, options, required, 37,
                )?;
                let delta = GLOBAL.stats() - before;
                let retained = lsmr.retained_payload_bytes()?;
                setup(delta, count + 16, retained);
                assert_eq!(lsmr.payload_report(37)?.total_payload_bytes, required);
                let mut reports = vec![None; 32];
                let before = GLOBAL.stats();
                assert!(
                    solve_prepared_least_squares_with_certificate_gate(
                        &owner,
                        &targets[..8],
                        &mut lsmr
                    )?
                    .report
                    .solve
                    .accepted
                );
                for count in [1, 2, 4, 8, 16, 17, 32] {
                    solve_prepared_least_squares_with_certificate_gate_batch_into(
                        &owner,
                        &targets[..8 * count],
                        count,
                        &mut output[..6 * count],
                        &mut reports[..count],
                        &mut lsmr,
                    )?;
                    assert!(reports[..count].iter().all(|x| x.unwrap().solve.accepted));
                }
                assert!(
                    solve_prepared_least_squares_with_certificate_gate(
                        &other,
                        &targets[..8],
                        &mut lsmr
                    )
                    .is_err()
                );
                assert!(
                    solve_prepared_least_squares_with_certificate_gate(
                        &owner,
                        &targets[..7],
                        &mut lsmr
                    )
                    .is_err()
                );
                assert!(
                    solve_prepared_least_squares_with_certificate_gate(
                        &owner,
                        &[f64::NAN; 8],
                        &mut lsmr
                    )
                    .is_err()
                );
                assert!(
                    solve_prepared_least_squares_with_certificate_gate(
                        &owner,
                        &targets[8..16],
                        &mut lsmr
                    )?
                    .report
                    .solve
                    .accepted
                );
                no_events(GLOBAL.stats() - before);
                let before = GLOBAL.stats();
                drop(lsmr);
                release(GLOBAL.stats() - before, count + 16, retained);
            }
        }
    }
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[1e-300; 8]))?;
    for kind in [
        PreparedBaselineKind::Identity,
        PreparedBaselineKind::InverseDiagonal,
        PreparedBaselineKind::SymmetricMap,
    ] {
        let owner = PreparedBaseline::new(&f, kind);
        let mut w = owner.application_workspace()?;
        let mut out = [17.; 6];
        let bad = if kind == PreparedBaselineKind::Identity {
            f64::NAN
        } else {
            1e20
        };
        let before = GLOBAL.stats();
        assert!(
            owner
                .apply_with_workspace(&[bad; 6], &mut out, &mut w)
                .is_err()
        );
        assert_eq!(out, [17.; 6]);
        owner.apply_with_workspace(&[0.; 6], &mut out, &mut w)?;
        assert_eq!(out, [0.; 6]);
        no_events(GLOBAL.stats() - before);
    }
    println!(
        "explicit baselines: identity/diagonal/MAP have zero/two/four action arrays plus optional image; complete PCG adds ten, LSMR sixteen; first/repeated/RHS32/rejection/failure recovery allocate zero, exact setup/admission/drop"
    );
    Ok(())
}
