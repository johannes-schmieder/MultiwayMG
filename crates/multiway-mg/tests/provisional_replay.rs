//! One-transition provisional weights feed proposals and match complete fresh replay.
#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;
use multiway_incidence::{
    HierarchyWeightFrames, PreparedHierarchyBudget, PreparedHierarchyBuilder,
    PreparedHierarchyLimits, PreparedProvisionalFrame, PreparedThreeWayTopology,
    ProvisionalWeightInput, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    PairNeighborhoodAggregationOptions, PreparedMapHierarchy, PreparedPairNeighborhoodCandidate,
    PreparedPcgOptions, PreparedPcgWorkspace, ThreeWayProblem, build_pair_neighborhood_aggregation,
    solve_prepared_pcg_least_squares,
};
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
}
#[test]
fn successive_owned_inputs_match_fresh_maps_frames_and_complete_replay()
-> Result<(), Box<dyn std::error::Error>> {
    for fixture in fixtures::recursive_holdout_fixtures()? {
        let t = PreparedThreeWayTopology::try_from_collapsed(
            fixture.problem.topology().level_counts(),
            fixture.problem.topology().tuples(),
        )?;
        for generation in 0..3 {
            let weights: Vec<_> = fixture
                .problem
                .weights()
                .iter()
                .enumerate()
                .map(|(i, w)| w * (1. + ((i + generation) % 7) as f64 / 8.))
                .collect();
            let fine = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
            let limits = PreparedHierarchyLimits {
                maximum_transitions: fixture.oracle_maps.len(),
                maximum_total_tuples: usize::MAX,
                maximum_total_coefficients: usize::MAX,
                require_strict_dimension_reduction: true,
            };
            let mut b = PreparedHierarchyBuilder::try_new(&t, limits, B)?;
            let mut expected = ThreeWayProblem::from_observations(
                t.topology().level_counts(),
                t.topology().tuples(),
                &weights,
            )?;
            let mut previous = None;
            for (index, map) in fixture.oracle_maps.iter().enumerate() {
                let input = previous
                    .take()
                    .map(ProvisionalWeightInput::Owned)
                    .unwrap_or(ProvisionalWeightInput::Frame(&fine));
                let owned = matches!(input, ProvisionalWeightInput::Owned(_));
                let owned_input_bytes = match &input {
                    ProvisionalWeightInput::Owned(w) => std::mem::size_of::<f64>() * w.capacity(),
                    ProvisionalWeightInput::Frame(_) => 0,
                };
                // Application state only; independent test references are excluded.
                b.try_append(
                    map.clone(),
                    PreparedHierarchyBudget {
                        additional_live_payload_bytes: fine.retained_payload_bytes()?
                            + owned_input_bytes,
                        ..B
                    },
                )?;
                expected = map.coarsen(&expected)?;
                let p = PreparedProvisionalFrame::try_replay_last(
                    &b,
                    input,
                    PreparedHierarchyBudget {
                        additional_live_payload_bytes: if owned {
                            fine.retained_payload_bytes()?
                        } else {
                            0
                        },
                        ..B
                    },
                )?;
                assert_eq!(p.setup_report().source_level, index);
                assert_eq!(
                    p.frame().topology().topology().tuples(),
                    expected.topology().tuples()
                );
                bits(p.frame().weights(), expected.weights());
                let fresh = ThreeWayWeightFrame::try_new(
                    b.current_level(),
                    WeightFrameInput::Tuples(expected.weights()),
                )?;
                bits(p.frame().square_root_weights(), fresh.square_root_weights());
                bits(p.frame().diagonal(), fresh.diagonal());
                assert_eq!(p.frame().component_ranges(), fresh.component_ranges());
                let options = PairNeighborhoodAggregationOptions::default();
                let candidate = PreparedPairNeighborhoodCandidate::try_new(
                    p.frame(),
                    options,
                    PreparedHierarchyBudget {
                        additional_live_payload_bytes: t.retained_payload_bytes()?
                            + b.retained_payload_bytes()?
                            - b.current_level().retained_payload_bytes()?
                            + fine.retained_payload_bytes()?,
                        ..B
                    },
                )?;
                assert_eq!(
                    candidate.aggregation(),
                    &build_pair_neighborhood_aggregation(&expected, options)?
                );
                drop(candidate);
                drop(fresh);
                let ptr = p.frame().weights().as_ptr();
                let current = p.into_tuple_weights();
                assert_eq!(ptr, current.as_ptr());
                previous = Some(current);
            }
            let structural = b.finish();
            let frames = HierarchyWeightFrames::try_new(&structural, &fine)?;
            bits(
                frames.frame(frames.level_count() - 1).unwrap().weights(),
                previous.as_ref().unwrap(),
            );
            let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
            let mut pcg = PreparedPcgWorkspace::try_new(&hierarchy, PreparedPcgOptions::default())?;
            #[cfg(feature = "lsmr")]
            let mut lsmr = multiway_mg::PreparedLsmrWorkspace::try_new(
                &hierarchy,
                multiway_mg::PreparedLsmrOptions::default(),
            )?;
            for column in 0..3 {
                let targets: Vec<_> = (0..fine.weights().len())
                    .map(|i| {
                        if column == 2 {
                            0.
                        } else {
                            ((i + column) as f64 * 0.31).sin()
                        }
                    })
                    .collect();
                assert!(
                    solve_prepared_pcg_least_squares(&hierarchy, &targets, &mut pcg)?
                        .report
                        .accepted
                );
                #[cfg(feature = "lsmr")]
                assert!(
                    multiway_mg::solve_prepared_least_squares_with_certificate_gate(
                        &hierarchy, &targets, &mut lsmr
                    )?
                    .report
                    .solve
                    .accepted
                );
            }
        }
    }
    Ok(())
}
#[test]
fn rank_controls_and_nested_provisional_replay_are_explicit()
-> Result<(), Box<dyn std::error::Error>> {
    use multiway_mg::{DensePseudoinverse, FactorAggregation};
    let latin_keys: Vec<_> = (0..8)
        .flat_map(|i| (0..8).map(move |j| [i, j, (i + j) % 8]))
        .collect();
    let latin = ThreeWayProblem::from_observations([8; 3], &latin_keys, &[1.; 64])?;
    assert_eq!(DensePseudoinverse::from_problem(&latin, 1e-12)?.rank(), 22); // exactly two structural modes
    let mut keys: Vec<_> = (0..4)
        .flat_map(|i| (0..3).map(move |j| [i, j, j]))
        .collect();
    keys.push([4, 3, 3]);
    let problem = ThreeWayProblem::from_observations([5, 4, 4], &keys, &[1.; 13])?;
    assert_eq!(DensePseudoinverse::from_problem(&problem, 1e-12)?.rank(), 7);
    let t = PreparedThreeWayTopology::try_from_collapsed([5, 4, 4], &keys)?;
    assert_eq!(t.component_factor_sizes().len(), 2);
    assert_eq!(13 - 7 - 2 * 2, 2); // two additional null directions
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let mut b = PreparedHierarchyBuilder::try_new(
        &t,
        PreparedHierarchyLimits {
            maximum_transitions: 2,
            maximum_total_tuples: 20,
            maximum_total_coefficients: 28,
            require_strict_dimension_reduction: true,
        },
        B,
    )?;
    let first = FactorAggregation::new(
        [5, 4, 4],
        [vec![0, 0, 1, 1, 2], vec![0, 0, 1, 2], vec![0, 0, 1, 2]],
    )?;
    b.try_append(first, B)?;
    let p = PreparedProvisionalFrame::try_replay_last(&b, ProvisionalWeightInput::Frame(&f), B)?;
    assert_eq!(p.frame().topology().component_factor_sizes().len(), 2);
    let weights = p.into_tuple_weights();
    b.try_append(
        FactorAggregation::new([3; 3], [vec![0, 0, 1], vec![0, 0, 1], vec![0, 0, 1]])?,
        B,
    )?;
    let p =
        PreparedProvisionalFrame::try_replay_last(&b, ProvisionalWeightInput::Owned(weights), B)?;
    assert_eq!(p.frame().weights(), &[12., 1.]);
    drop(p);
    let h = b.finish();
    let frames = HierarchyWeightFrames::try_new(&h, &f)?;
    let numerical = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
    let mut pcg = PreparedPcgWorkspace::try_new(&numerical, PreparedPcgOptions::default())?;
    #[cfg(feature = "lsmr")]
    let mut lsmr = multiway_mg::PreparedLsmrWorkspace::try_new(
        &numerical,
        multiway_mg::PreparedLsmrOptions::default(),
    )?;
    for column in 0..3 {
        let y: Vec<_> = (0..13)
            .map(|i| {
                if column == 2 {
                    0.
                } else {
                    ((i + column) as f64 * 0.31).sin()
                }
            })
            .collect();
        assert!(
            solve_prepared_pcg_least_squares(&numerical, &y, &mut pcg)?
                .report
                .accepted
        );
        #[cfg(feature = "lsmr")]
        assert!(
            multiway_mg::solve_prepared_least_squares_with_certificate_gate(
                &numerical, &y, &mut lsmr
            )?
            .report
            .solve
            .accepted
        );
    }
    Ok(())
}
