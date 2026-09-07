//! Complete explicit layouts preserve scalar arithmetic, owners and memory scopes.
#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyGrouping, PreparedHierarchyTopology,
    PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    GroupedGramianMode, PreparedMapHierarchy, PreparedPcgOptions, PreparedPcgWorkspace,
    solve_prepared_pcg_batch_into, solve_prepared_pcg_least_squares,
};
#[cfg(feature = "lsmr")]
use multiway_mg::{
    PreparedLsmrOptions, PreparedLsmrWorkspace, solve_prepared_least_squares,
    solve_prepared_least_squares_with_certificate_gate,
    solve_prepared_least_squares_with_certificate_gate_batch_into,
};
fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(
        a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
    );
}

#[test]
fn all_explicit_prefixes_and_image_modes_match_scalar_recursive_solvers()
-> Result<(), Box<dyn std::error::Error>> {
    // Existing historical regression seeds, not the campaign's untouched holdout.
    for fixture in fixtures::recursive_holdout_fixtures()? {
        let t = PreparedThreeWayTopology::try_from_collapsed(
            fixture.problem.topology().level_counts(),
            fixture.problem.topology().tuples(),
        )?;
        let h = PreparedHierarchyTopology::try_new(&t, fixture.oracle_maps.clone())?;
        let f =
            ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(fixture.problem.weights()))?;
        let frames = HierarchyWeightFrames::try_new(&h, &f)?;
        let scalar = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
        let options = PreparedPcgOptions::default();
        let mut scalar_workspace = PreparedPcgWorkspace::try_new(&scalar, options)?;
        #[cfg(feature = "lsmr")]
        let mut lsmr_scalar =
            PreparedLsmrWorkspace::try_new(&scalar, PreparedLsmrOptions::default())?;
        for prefix in 0..h.level_count() {
            let groups = PreparedHierarchyGrouping::try_new(&h, prefix)?;
            for mode in [
                GroupedGramianMode::RowGather,
                GroupedGramianMode::TupleImage,
            ] {
                let grouped =
                    PreparedMapHierarchy::try_new_with_grouping(&frames, &groups, mode, 1e-12)?;
                assert_eq!(
                    grouped.tuple_image_len(),
                    if prefix > 0 && mode == GroupedGramianMode::TupleImage {
                        f.weights().len()
                    } else {
                        0
                    }
                );
                let mut workspace = PreparedPcgWorkspace::try_new(&grouped, options)?;
                let rhs: Vec<_> = (0..scalar.dimension())
                    .map(|i| (i as f64 * 0.73).cos())
                    .collect();
                let mut old_cycle = scalar.application_workspace()?;
                let mut new_cycle = grouped.application_workspace()?;
                let mut a = vec![0.0; scalar.dimension()];
                let mut b = vec![f64::NAN; scalar.dimension()];
                scalar.apply_with_workspace(&rhs, &mut a, &mut old_cycle)?;
                grouped.apply_with_workspace(&rhs, &mut b, &mut new_cycle)?;
                bits(&a, &b);

                let payload = workspace.payload_report(123)?;
                assert_eq!(
                    payload.hierarchy.grouping_payload_bytes,
                    groups.retained_payload_bytes()?
                );
                assert_eq!(
                    payload.total_payload_bytes,
                    PreparedPcgWorkspace::setup_payload_bound(&grouped, options, 123)?
                );
                let old = scalar_workspace.payload_report(123)?;
                let image_bytes = if grouped.tuple_image_len() > 0 {
                    8 * grouped.tuple_image_len()
                } else {
                    0
                };
                assert_eq!(
                    payload.outer_workspace_payload_bytes,
                    old.outer_workspace_payload_bytes
                );
                assert_eq!(
                    payload.hierarchy.workspace_payload_bytes,
                    old.hierarchy.workspace_payload_bytes + image_bytes
                );
                assert_eq!(
                    payload.total_payload_bytes,
                    old.total_payload_bytes + groups.retained_payload_bytes()? + image_bytes
                );
                #[cfg(feature = "lsmr")]
                let mut lsmr_grouped =
                    PreparedLsmrWorkspace::try_new(&grouped, PreparedLsmrOptions::default())?;
                for column in 0..3 {
                    let y: Vec<_> = (0..f.weights().len())
                        .map(|i| {
                            if column == 1 {
                                0.0
                            } else {
                                ((i + column) as f64 * 0.31).sin()
                            }
                        })
                        .collect();
                    let expected =
                        solve_prepared_pcg_least_squares(&scalar, &y, &mut scalar_workspace)?;
                    let actual = solve_prepared_pcg_least_squares(&grouped, &y, &mut workspace)?;
                    bits(actual.coefficients, expected.coefficients);
                    assert_eq!(
                        format!("{:?}", actual.report),
                        format!("{:?}", expected.report),
                        "{} prefix {prefix} {mode:?}",
                        fixture.name
                    );
                    #[cfg(feature = "lsmr")]
                    {
                        let expected = solve_prepared_least_squares(&scalar, &y, &mut lsmr_scalar)?;
                        let actual = solve_prepared_least_squares(&grouped, &y, &mut lsmr_grouped)?;
                        bits(actual.coefficients, expected.coefficients);
                        assert_eq!(
                            format!("{:?}", actual.report),
                            format!("{:?}", expected.report)
                        );
                        let expected = solve_prepared_least_squares_with_certificate_gate(
                            &scalar,
                            &y,
                            &mut lsmr_scalar,
                        )?;
                        let actual = solve_prepared_least_squares_with_certificate_gate(
                            &grouped,
                            &y,
                            &mut lsmr_grouped,
                        )?;
                        bits(actual.coefficients, expected.coefficients);
                        assert_eq!(
                            format!("{:?}", actual.report),
                            format!("{:?}", expected.report)
                        );
                    }
                }
                #[cfg(feature = "lsmr")]
                {
                    let p = lsmr_grouped.payload_report(123)?;
                    let old = lsmr_scalar.payload_report(123)?;
                    assert_eq!(
                        p.total_payload_bytes,
                        PreparedLsmrWorkspace::setup_payload_bound(
                            &grouped,
                            PreparedLsmrOptions::default(),
                            123
                        )?
                    );
                    assert_eq!(
                        p.outer_workspace_payload_bytes,
                        old.outer_workspace_payload_bytes
                    );
                    assert_eq!(
                        p.total_payload_bytes,
                        old.total_payload_bytes + groups.retained_payload_bytes()? + image_bytes
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn all_rhs_widths_and_new_weight_frames_reuse_the_same_structural_groups()
-> Result<(), Box<dyn std::error::Error>> {
    let tuples: Vec<_> = (0..4)
        .flat_map(|i| (0..4).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([4; 3], &tuples)?;
    let h = PreparedHierarchyTopology::try_new(
        &t,
        vec![
            FactorAggregation::consecutive_halving([4; 3])?,
            FactorAggregation::consecutive_halving([2; 3])?,
        ],
    )?;
    for prefix in [0, 1, 2] {
        let groups = PreparedHierarchyGrouping::try_new(&h, prefix)?;
        for change in 0..2 {
            let weights: Vec<_> = (0..64)
                .map(|i| {
                    if change == 0 {
                        1.0
                    } else {
                        2.0_f64.powi((i % 9) - 4)
                    }
                })
                .collect();
            let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
            let frames = HierarchyWeightFrames::try_new(&h, &f)?;
            let scalar = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
            let mut scalar_workspace = PreparedPcgWorkspace::try_new(&scalar, Default::default())?;
            #[cfg(feature = "lsmr")]
            let mut lsmr_scalar = PreparedLsmrWorkspace::try_new(&scalar, Default::default())?;
            for mode in [
                GroupedGramianMode::RowGather,
                GroupedGramianMode::TupleImage,
            ] {
                let grouped =
                    PreparedMapHierarchy::try_new_with_grouping(&frames, &groups, mode, 1e-12)?;
                let mut workspace = PreparedPcgWorkspace::try_new(&grouped, Default::default())?;
                #[cfg(feature = "lsmr")]
                let mut lsmr = PreparedLsmrWorkspace::try_new(&grouped, Default::default())?;
                for width in [1, 2, 4, 8, 16, 17, 32] {
                    let y: Vec<_> = (0..64 * width)
                        .map(|i| {
                            if (i / 64) % 4 == 1 {
                                0.0
                            } else {
                                (i as f64 * 0.19).sin()
                            }
                        })
                        .collect();
                    let mut coefficients = vec![f64::NAN; 12 * width];
                    let mut reports = vec![None; width];
                    solve_prepared_pcg_batch_into(
                        &grouped,
                        &y,
                        width,
                        &mut coefficients,
                        &mut reports,
                        &mut workspace,
                    )?;
                    for column in 0..width {
                        let expected = solve_prepared_pcg_least_squares(
                            &scalar,
                            &y[column * 64..(column + 1) * 64],
                            &mut scalar_workspace,
                        )?;
                        bits(
                            &coefficients[column * 12..(column + 1) * 12],
                            expected.coefficients,
                        );
                        assert_eq!(reports[column], Some(expected.report));
                    }
                    #[cfg(feature = "lsmr")]
                    {
                        let mut reports = vec![None; width];
                        solve_prepared_least_squares_with_certificate_gate_batch_into(
                            &grouped,
                            &y,
                            width,
                            &mut coefficients,
                            &mut reports,
                            &mut lsmr,
                        )?;
                        for column in 0..width {
                            let expected = solve_prepared_least_squares_with_certificate_gate(
                                &scalar,
                                &y[column * 64..(column + 1) * 64],
                                &mut lsmr_scalar,
                            )?;
                            bits(
                                &coefficients[column * 12..(column + 1) * 12],
                                expected.coefficients,
                            );
                            assert_eq!(reports[column], Some(expected.report));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn foreign_structure_layout_workspace_and_failed_rhs_never_publish_outputs()
-> Result<(), Box<dyn std::error::Error>> {
    let tuples: Vec<_> = (0..2)
        .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &tuples)?;
    let make = || {
        PreparedHierarchyTopology::try_new(
            &t,
            vec![FactorAggregation::consecutive_halving([2; 3]).unwrap()],
        )
        .unwrap()
    };
    let h = make();
    let other = make();
    let groups = PreparedHierarchyGrouping::try_new(&h, 1)?;
    let foreign = PreparedHierarchyGrouping::try_new(&other, 1)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[2.0; 8]))?;
    let frames = HierarchyWeightFrames::try_new(&h, &f)?;
    assert!(
        PreparedMapHierarchy::try_new_with_grouping(
            &frames,
            &foreign,
            GroupedGramianMode::TupleImage,
            1e-12
        )
        .is_err()
    );
    for mode in [
        GroupedGramianMode::RowGather,
        GroupedGramianMode::TupleImage,
    ] {
        let grouped = PreparedMapHierarchy::try_new_with_grouping(&frames, &groups, mode, 1e-12)?;
        let scalar = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
        let mut workspace = PreparedPcgWorkspace::try_new(&grouped, Default::default())?;
        let required = PreparedPcgWorkspace::setup_payload_bound(&grouped, Default::default(), 17)?;
        assert!(
            PreparedPcgWorkspace::try_new_with_payload_budget(
                &grouped,
                Default::default(),
                required - 1,
                17
            )
            .is_err()
        );
        let mut output = [7.0; 18];
        let mut reports = [None; 3];
        let valid = [0.25; 24];
        let before = format!("{workspace:?}");
        assert!(
            solve_prepared_pcg_batch_into(
                &scalar,
                &valid,
                3,
                &mut output,
                &mut reports,
                &mut workspace
            )
            .is_err()
        );
        let mut y = valid;
        y[8] = f64::NAN;
        assert!(
            solve_prepared_pcg_batch_into(
                &grouped,
                &y,
                3,
                &mut output,
                &mut reports,
                &mut workspace
            )
            .is_err()
        );
        assert!(
            solve_prepared_pcg_batch_into(
                &grouped,
                &valid[..23],
                3,
                &mut output,
                &mut reports,
                &mut workspace
            )
            .is_err()
        );
        assert_eq!(before, format!("{workspace:?}"));
        assert_eq!(output, [7.0; 18]);
        assert_eq!(reports, [None; 3]);
        y[8..16].fill(f64::MAX);
        assert!(
            solve_prepared_pcg_batch_into(
                &grouped,
                &y,
                3,
                &mut output,
                &mut reports,
                &mut workspace
            )
            .is_err()
        );
        assert!(reports[0].is_some());
        assert_eq!(reports[1..], [None; 2]);
        assert_eq!(output[6..], [7.0; 12]);
        solve_prepared_pcg_batch_into(
            &grouped,
            &valid,
            3,
            &mut output,
            &mut reports,
            &mut workspace,
        )?;
        #[cfg(feature = "lsmr")]
        {
            let mut workspace = PreparedLsmrWorkspace::try_new(&grouped, Default::default())?;
            let required =
                PreparedLsmrWorkspace::setup_payload_bound(&grouped, Default::default(), 17)?;
            assert!(
                PreparedLsmrWorkspace::try_new_with_payload_budget(
                    &grouped,
                    Default::default(),
                    required - 1,
                    17
                )
                .is_err()
            );
            let mut output = [7.0; 18];
            let mut reports = [None; 3];
            let before = format!("{workspace:?}");
            assert!(
                solve_prepared_least_squares_with_certificate_gate_batch_into(
                    &scalar,
                    &valid,
                    3,
                    &mut output,
                    &mut reports,
                    &mut workspace
                )
                .is_err()
            );
            let mut y = valid;
            y[8] = f64::NAN;
            assert!(
                solve_prepared_least_squares_with_certificate_gate_batch_into(
                    &grouped,
                    &y,
                    3,
                    &mut output,
                    &mut reports,
                    &mut workspace
                )
                .is_err()
            );
            assert_eq!(before, format!("{workspace:?}"));
            assert_eq!(output, [7.0; 18]);
            assert_eq!(reports, [None; 3]);
            y[8..16].fill(f64::MAX);
            assert!(
                solve_prepared_least_squares_with_certificate_gate_batch_into(
                    &grouped,
                    &y,
                    3,
                    &mut output,
                    &mut reports,
                    &mut workspace
                )
                .is_err()
            );
            assert!(reports[0].is_some());
            assert_eq!(reports[1..], [None; 2]);
            assert_eq!(output[6..], [7.0; 12]);
            solve_prepared_least_squares_with_certificate_gate_batch_into(
                &grouped,
                &valid,
                3,
                &mut output,
                &mut reports,
                &mut workspace,
            )?;
        }
    }
    Ok(())
}
