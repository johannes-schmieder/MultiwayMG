use super::*;
use crate::WeightFrameInput;
use std::panic::{AssertUnwindSafe, catch_unwind};
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;

fn source() -> PreparedThreeWayTopology {
    // Interleaved source levels and tuple IDs; first component also has extra
    // nullity through nested second/third factors. Separate singleton included.
    let keys = [
        [0, 0, 1],
        [0, 2, 3],
        [1, 1, 0],
        [2, 0, 1],
        [2, 2, 3],
        [3, 3, 2],
    ];
    PreparedThreeWayTopology::try_from_collapsed([4; 3], &keys).unwrap()
}

#[test]
fn component_root_all_reservations_and_admission_recover() {
    let t = source();
    let layout = PreparedComponentLayout::try_new(&t, B).unwrap();
    let recoding = PreparedComponentRecoding::try_new(&layout, B).unwrap();
    let source_frame = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    let extra = PreparedHierarchyBudget {
        additional_live_payload_bytes: 73,
        ..B
    };
    let mut seen = 0;
    let root = PreparedComponentRoot::build_with(&recoding, 0, B, &mut |_| {
        seen += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(seen, 3);
    seen = 0;
    root.frame_with(&source_frame, B, &mut |_| {
        seen += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(seen, 5);
    for frame_stage in [false, true] {
        let count = if frame_stage { 5 } else { 3 };
        for failure in 0..count {
            for unwind in [false, true] {
                let mut seen = 0;
                let mut hook = |context| {
                    let current = seen;
                    seen += 1;
                    if current == failure {
                        assert!(!unwind, "injected component root/frame panic");
                        Err(IncidenceError::TopologyAllocation { context })
                    } else {
                        Ok(())
                    }
                };
                let result = catch_unwind(AssertUnwindSafe(|| {
                    if frame_stage {
                        root.frame_with(&source_frame, B, &mut hook).map(|_| ())
                    } else {
                        PreparedComponentRoot::build_with(&recoding, 0, B, &mut hook).map(|_| ())
                    }
                }));
                if unwind {
                    assert!(result.is_err());
                } else {
                    assert!(result.unwrap().is_err());
                }
                assert_eq!(seen, failure + 1);
                let fresh = PreparedComponentRoot::try_new(&recoding, 0, B).unwrap();
                assert_eq!(
                    fresh.topology().topology().tuples(),
                    root.topology().topology().tuples()
                );
                assert_eq!(
                    fresh.try_weight_frame(&source_frame, B).unwrap().weights(),
                    &[1.; 4]
                );
            }
        }
        let bound = if frame_stage {
            root.weight_frame_setup_payload_bound(&source_frame, extra)
                .unwrap()
        } else {
            PreparedComponentRoot::setup_payload_bound(&recoding, 0, extra).unwrap()
        };
        for delta in [0, 1] {
            let b = PreparedHierarchyBudget {
                maximum_payload_bytes: bound - delta,
                ..extra
            };
            let mut calls = 0;
            let mut hook = |_| {
                calls += 1;
                Ok(())
            };
            let result = if frame_stage {
                root.frame_with(&source_frame, b, &mut hook).map(|_| ())
            } else {
                PreparedComponentRoot::build_with(&recoding, 0, b, &mut hook).map(|_| ())
            };
            assert_eq!(result.is_ok(), delta == 0);
            assert_eq!(calls, if delta == 0 { count } else { 0 });
        }
    }
    assert!(PreparedComponentRoot::setup_payload_bound(&recoding, usize::MAX, B).is_err());
    assert!(
        PreparedComponentRoot::setup_payload_bound(
            &recoding,
            0,
            PreparedHierarchyBudget {
                additional_live_payload_bytes: usize::MAX,
                ..B
            }
        )
        .is_err()
    );
    let other = source();
    let wrong = ThreeWayWeightFrame::try_new(&other, WeightFrameInput::UnitTuples).unwrap();
    let mut calls = 0;
    assert!(
        root.frame_with(&wrong, B, &mut |_| {
            calls += 1;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(calls, 0);
    assert!(
        root.weight_frame_setup_payload_bound(
            &source_frame,
            PreparedHierarchyBudget {
                additional_live_payload_bytes: usize::MAX,
                ..B
            }
        )
        .is_err()
    );
    drop(recoding);
    assert_eq!(
        root.try_weight_frame(&source_frame, B).unwrap().weights(),
        &[1.; 4]
    );
}
