//! Local reservation-boundary failures, not allocator-null injection.
use super::*;

const UNLIMITED: WeightFramePayloadBudget = WeightFramePayloadBudget {
    maximum_payload_bytes: usize::MAX,
    additional_live_payload_bytes: 0,
};

#[test]
fn all_five_frame_reservations_recover_after_errors_and_unwinds() {
    let rows = [[0; 3], [1; 3], [0; 3]];
    let topology = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    let old = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitObservations).unwrap();
    let saved = format!("{old:?}{topology:?}");
    let token = old.binding();
    for input in [
        WeightFrameInput::Observations(&[1.0, 2.0, 3.0]),
        WeightFrameInput::Tuples(&[4.0, 2.0]),
        WeightFrameInput::UnitObservations,
        WeightFrameInput::UnitTuples,
    ] {
        for unwind in [false, true] {
            for fail_at in 0..5 {
                let mut reached = 0;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    ThreeWayWeightFrame::build_with(&topology, input, UNLIMITED, &mut |context| {
                        let index = reached;
                        reached += 1;
                        if index == fail_at {
                            assert!(!unwind, "injected frame setup unwind");
                            return Err(IncidenceError::WeightFrameAllocation { context });
                        }
                        Ok(())
                    })
                }));
                assert_eq!(reached, fail_at + 1);
                if unwind {
                    assert!(result.is_err());
                } else {
                    assert!(matches!(
                        result.unwrap(),
                        Err(IncidenceError::WeightFrameAllocation { .. })
                    ));
                }
                assert_eq!(format!("{old:?}{topology:?}"), saved);
                token.validate_for(&old).unwrap();
                let fresh = ThreeWayWeightFrame::try_new(&topology, input).unwrap();
                assert_ne!(fresh.binding(), token);
                fresh.validate_for(&topology).unwrap();
            }
        }
    }
}

#[test]
fn layout_budget_arithmetic_and_invalid_inputs_reject_before_reservation() {
    let topology = PreparedThreeWayTopology::try_from_observations([1; 3], &[[0; 3]]).unwrap();
    for input in [
        WeightFrameInput::Tuples(&[]),
        WeightFrameInput::Observations(&[]),
        WeightFrameInput::Tuples(&[f64::NAN]),
        WeightFrameInput::Observations(&[0.0]),
    ] {
        assert!(
            ThreeWayWeightFrame::build_with(&topology, input, UNLIMITED, &mut |_| panic!(
                "preflight failed"
            ))
            .is_err()
        );
    }
    for budget in [
        WeightFramePayloadBudget {
            maximum_payload_bytes: 0,
            additional_live_payload_bytes: 0,
        },
        WeightFramePayloadBudget {
            maximum_payload_bytes: usize::MAX,
            additional_live_payload_bytes: usize::MAX,
        },
    ] {
        assert!(
            ThreeWayWeightFrame::build_with(
                &topology,
                WeightFrameInput::UnitTuples,
                budget,
                &mut |_| panic!("budget preflight failed")
            )
            .is_err()
        );
    }
    let collapsed = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    assert!(
        ThreeWayWeightFrame::build_with(
            &collapsed,
            WeightFrameInput::UnitObservations,
            UNLIMITED,
            &mut |_| panic!("layout preflight failed")
        )
        .is_err()
    );
    // This exercises the real checked reservation helper, not just its hook.
    assert!(
        reserve_frame::<f64, _>(usize::MAX, "impossible frame array", &mut |_| panic!(
            "overflow first"
        ))
        .is_err()
    );
}
