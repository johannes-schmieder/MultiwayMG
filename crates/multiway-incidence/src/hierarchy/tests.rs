use super::*;
use crate::{CoarseWeightReplay, ThreeWayProblem};

fn grid() -> PreparedThreeWayTopology {
    let tuples: Vec<_> = (0..4)
        .flat_map(|a| (0..4).flat_map(move |b| (0..4).map(move |c| [a, b, c])))
        .collect();
    PreparedThreeWayTopology::try_from_collapsed([4; 3], &tuples).unwrap()
}
fn maps() -> Vec<FactorAggregation> {
    vec![
        FactorAggregation::new(
            [4; 3],
            [vec![1, 0, 1, 0], vec![0, 1, 1, 0], vec![1, 1, 0, 0]],
        )
        .unwrap(),
        FactorAggregation::consecutive_halving([2; 3]).unwrap(),
    ]
}
fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(
        a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
    );
}

#[test]
fn all_levels_match_fresh_coarsening_and_single_map_replay() {
    let topology = grid();
    let hierarchy = PreparedHierarchyTopology::try_new(&topology, maps()).unwrap();
    assert_eq!(hierarchy.level_count(), 3);
    assert!(hierarchy.level(3).is_none());
    assert!(hierarchy.aggregation(2).is_none());
    assert!(hierarchy.merge_groups(2).is_none());
    for step in 0..4 {
        let weights: Vec<_> = (0..64)
            .map(|i| (i % 11 + 1) as f64 * (step + 1) as f64)
            .collect();
        let fine =
            ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&weights)).unwrap();
        let replay = HierarchyWeightFrames::try_new(&hierarchy, &fine).unwrap();
        let mut fresh = ThreeWayProblem::from_collapsed_parts(
            [4; 3],
            topology.topology().tuples().to_vec(),
            weights,
        )
        .unwrap();
        for index in 0..replay.level_count() {
            let actual = replay.frame(index).unwrap();
            assert_eq!(
                actual.topology().topology().tuples(),
                fresh.topology().tuples()
            );
            bits(actual.weights(), fresh.weights());
            bits(actual.diagonal(), fresh.diagonal());
            if let Some(map) = hierarchy.aggregation(index) {
                let one =
                    PreparedCoarseTupleMap::try_new(hierarchy.level(index).unwrap(), map).unwrap();
                let one_replay = CoarseWeightReplay::try_new(&one, actual).unwrap();
                bits(
                    one_replay.frame().weights(),
                    replay.frame(index + 1).unwrap().weights(),
                );
                assert_eq!(
                    hierarchy.coarse_to_fine_components(index).unwrap(),
                    one.coarse_to_fine_components()
                );
                assert_eq!(
                    hierarchy.fine_to_coarse_components(index).unwrap(),
                    one.fine_to_coarse_components()
                );
                // Test Galerkin action, independently using restriction/prolongation.
                let coarse = replay.frame(index + 1).unwrap().operator_view();
                let x: Vec<_> = (0..coarse.dimension())
                    .map(|i| (i as f64 * 0.19).sin())
                    .collect();
                let mut px = vec![0.0; actual.diagonal().len()];
                let mut fine_image = px.clone();
                let mut restricted = x.clone();
                let mut coarse_image = x.clone();
                map.prolong(&x, &mut px).unwrap();
                actual
                    .operator_view()
                    .apply_gramian(&px, &mut fine_image)
                    .unwrap();
                map.restrict(&fine_image, &mut restricted).unwrap();
                coarse.apply_gramian(&x, &mut coarse_image).unwrap();
                for (&a, &b) in restricted.iter().zip(&coarse_image) {
                    assert!((a - b).abs() <= 1e-12 * (1.0 + a.abs()));
                }
                fresh = map.coarsen(&fresh).unwrap();
            }
        }
        assert!(replay.frame(3).is_none());
    }
}

#[test]
fn disconnected_relabeling_preserves_component_bijections() {
    let topology = PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0; 3], [1; 3]]).unwrap();
    let reversed = FactorAggregation::new([2; 3], [vec![1, 0], vec![1, 0], vec![1, 0]]).unwrap();
    let hierarchy = PreparedHierarchyTopology::try_new(
        &topology,
        vec![reversed, FactorAggregation::identity([2; 3]).unwrap()],
    )
    .unwrap();
    assert_eq!(
        hierarchy.coarse_to_fine_components(0),
        Some([1, 0].as_slice())
    );
    assert_eq!(
        hierarchy.fine_to_coarse_components(0),
        Some([1, 0].as_slice())
    );
    let fine =
        ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[3.0, 7.0])).unwrap();
    let replay = HierarchyWeightFrames::try_new(&hierarchy, &fine).unwrap();
    assert_eq!(replay.frame(2).unwrap().weights(), &[7.0, 3.0]);
    assert!(matches!(
        PreparedHierarchyTopology::try_new(
            &topology,
            vec![FactorAggregation::consecutive_halving([2; 3]).unwrap()]
        ),
        Err(IncidenceError::CrossComponentAggregation { .. })
    ));
}

#[test]
fn exact_owners_empty_hierarchy_and_old_generations_remain_valid() {
    let topology = grid();
    let foreign = grid();
    let hierarchy = PreparedHierarchyTopology::try_new(&topology, maps()).unwrap();
    let other_hierarchy = PreparedHierarchyTopology::try_new(&topology, maps()).unwrap();
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let equal = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let changed =
        ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[2.0; 64])).unwrap();
    let foreign_frame =
        ThreeWayWeightFrame::try_new(&foreign, WeightFrameInput::UnitTuples).unwrap();
    let old = HierarchyWeightFrames::try_new(&hierarchy, &fine).unwrap();
    let new = HierarchyWeightFrames::try_new(&hierarchy, &changed).unwrap();
    old.validate_for(&hierarchy, &fine).unwrap();
    assert_eq!(
        old.validate_for(&hierarchy, &equal),
        Err(IncidenceError::WeightFrameBindingMismatch)
    );
    assert!(old.validate_for(&hierarchy, &changed).is_err());
    assert_eq!(
        old.validate_for(&other_hierarchy, &fine),
        Err(IncidenceError::HierarchyBindingMismatch)
    );
    assert!(hierarchy.validate_for(&foreign).is_err());
    assert!(HierarchyWeightFrames::try_new(&hierarchy, &foreign_frame).is_err());
    assert_eq!(old.frame(2).unwrap().weights(), &[64.0]);
    assert_eq!(new.frame(2).unwrap().weights(), &[128.0]);
    let empty = PreparedHierarchyTopology::try_new(&topology, vec![]).unwrap();
    let replay = HierarchyWeightFrames::try_new(&empty, &fine).unwrap();
    assert_eq!(empty.retained_payload_bytes().unwrap(), 0);
    assert_eq!(replay.retained_payload_bytes().unwrap(), 0);
    assert!(core::ptr::eq(replay.frame(0).unwrap(), &fine));
}

#[test]
fn numerical_overflow_publishes_no_partial_generation() {
    let topology =
        PreparedThreeWayTopology::try_from_collapsed([2, 2, 3], &[[0, 0, 0], [0, 1, 2], [1, 1, 1]])
            .unwrap();
    let aggregation =
        FactorAggregation::new([2, 2, 3], [vec![0; 2], vec![0; 2], vec![0; 3]]).unwrap();
    let hierarchy = PreparedHierarchyTopology::try_new(&topology, vec![aggregation]).unwrap();
    let ordinary = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let old = HierarchyWeightFrames::try_new(&hierarchy, &ordinary).unwrap();
    let extreme =
        ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[9e307, 1.0, 9e307]))
            .unwrap();
    assert!(matches!(
        HierarchyWeightFrames::try_new(&hierarchy, &extreme),
        Err(IncidenceError::InvalidWeightFrameDerivedValue { .. })
    ));
    old.validate_for(&hierarchy, &ordinary).unwrap();
    assert_eq!(old.frame(1).unwrap().weights(), &[3.0]);
    HierarchyWeightFrames::try_new(&hierarchy, &ordinary).unwrap();
}

#[test]
fn complete_budget_and_static_checks_precede_reservations() {
    let topology = grid();
    let hierarchy = PreparedHierarchyTopology::try_new(&topology, maps()).unwrap();
    let bound = hierarchy.setup_peak_payload_bound();
    let budget = |maximum_payload_bytes| PreparedHierarchyBudget {
        maximum_payload_bytes,
        additional_live_payload_bytes: 0,
    };
    assert!(
        PreparedHierarchyTopology::try_new_with_budget(&topology, maps(), budget(bound - 1))
            .is_err()
    );
    PreparedHierarchyTopology::try_new_with_budget(&topology, maps(), budget(bound)).unwrap();
    assert!(
        PreparedHierarchyTopology::build_with(&topology, maps(), budget(0), &mut |_| panic!(
            "budget first"
        ))
        .is_err()
    );
    let invalid = vec![
        FactorAggregation::identity([4; 3]).unwrap(),
        FactorAggregation::identity([2; 3]).unwrap(),
    ];
    assert!(
        PreparedHierarchyTopology::build_with(
            &topology,
            invalid,
            PreparedHierarchyBudget::UNLIMITED,
            &mut |_| panic!("all layouts first")
        )
        .is_err()
    );
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let old = HierarchyWeightFrames::try_new(&hierarchy, &fine).unwrap();
    let old_bytes = old.retained_payload_bytes().unwrap();
    let base = HierarchyWeightFrames::setup_payload_bound(&hierarchy, &fine, 0).unwrap();
    let complete =
        HierarchyWeightFrames::setup_payload_bound(&hierarchy, &fine, old_bytes).unwrap();
    assert_eq!(complete, base + old_bytes);
    assert!(
        HierarchyWeightFrames::build_with(&hierarchy, &fine, budget(base - 1), &mut |_| panic!(
            "replay budget first"
        ))
        .is_err()
    );
    HierarchyWeightFrames::try_new_with_budget(
        &hierarchy,
        &fine,
        PreparedHierarchyBudget {
            maximum_payload_bytes: complete,
            additional_live_payload_bytes: old_bytes,
        },
    )
    .unwrap();
    assert!(HierarchyWeightFrames::setup_payload_bound(&hierarchy, &fine, usize::MAX).is_err());
}

#[test]
fn every_reservation_error_and_unwind_preserves_existing_owners() {
    let topology = grid();
    let hierarchy = PreparedHierarchyTopology::try_new(&topology, maps()).unwrap();
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let old = HierarchyWeightFrames::try_new(&hierarchy, &fine).unwrap();
    let mut structural_calls = 0;
    PreparedHierarchyTopology::build_with(
        &topology,
        maps(),
        PreparedHierarchyBudget::UNLIMITED,
        &mut |_| {
            structural_calls += 1;
            Ok(())
        },
    )
    .unwrap();
    let mut numerical_calls = 0;
    HierarchyWeightFrames::build_with(
        &hierarchy,
        &fine,
        PreparedHierarchyBudget::UNLIMITED,
        &mut |_| {
            numerical_calls += 1;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(structural_calls, 21);
    assert_eq!(numerical_calls, 11);
    for unwind in [false, true] {
        for numerical in [false, true] {
            for fail_at in 0..if numerical {
                numerical_calls
            } else {
                structural_calls
            } {
                let mut reached = 0;
                let mut callback = |context| {
                    reached += 1;
                    if reached == fail_at + 1 {
                        assert!(!unwind, "injected hierarchy unwind");
                        return Err(IncidenceError::TopologyAllocation { context });
                    }
                    Ok(())
                };
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if numerical {
                        HierarchyWeightFrames::build_with(
                            &hierarchy,
                            &fine,
                            PreparedHierarchyBudget::UNLIMITED,
                            &mut callback,
                        )
                        .map(|_| ())
                    } else {
                        PreparedHierarchyTopology::build_with(
                            &topology,
                            maps(),
                            PreparedHierarchyBudget::UNLIMITED,
                            &mut callback,
                        )
                        .map(|_| ())
                    }
                }));
                assert_eq!(reached, fail_at + 1);
                if unwind {
                    assert!(result.is_err());
                } else {
                    assert!(result.unwrap().is_err());
                }
                old.validate_for(&hierarchy, &fine).unwrap();
                assert_eq!(old.frame(2).unwrap().weights(), &[64.0]);
            }
        }
    }
    PreparedHierarchyTopology::try_new(&topology, maps()).unwrap();
    HierarchyWeightFrames::try_new(&hierarchy, &fine).unwrap();
}
