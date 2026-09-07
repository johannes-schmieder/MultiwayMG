use super::*;
use crate::{FactorAggregation, PreparedHierarchyLimits, PreparedThreeWayTopology};
use std::panic::{AssertUnwindSafe, catch_unwind};
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
fn topology() -> PreparedThreeWayTopology {
    let keys: Vec<_> = (0..4)
        .flat_map(|a| (0..4).flat_map(move |b| (0..4).map(move |c| [a, b, c])))
        .collect();
    PreparedThreeWayTopology::try_from_collapsed([4; 3], &keys).unwrap()
}
fn builder(t: &PreparedThreeWayTopology) -> PreparedHierarchyBuilder<'_> {
    let mut b = PreparedHierarchyBuilder::try_new(
        t,
        PreparedHierarchyLimits {
            maximum_transitions: 3,
            maximum_total_tuples: usize::MAX,
            maximum_total_coefficients: usize::MAX,
            require_strict_dimension_reduction: true,
        },
        B,
    )
    .unwrap();
    b.try_append(FactorAggregation::consecutive_halving([4; 3]).unwrap(), B)
        .unwrap();
    b
}
#[test]
fn all_five_reservations_fail_or_unwind_without_changing_accepted_structure() {
    let t = topology();
    let b = builder(&t);
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    let mut count = 0;
    PreparedProvisionalFrame::build_with(&b, ProvisionalWeightInput::Frame(&f), B, &mut |_| {
        count += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(count, 5);
    let retained = b.retained_payload_bytes().unwrap();
    let ptr = b.current_level().topology().tuples().as_ptr();
    for failure in 0..5 {
        for unwind in [false, true] {
            let mut count = 0;
            let result = catch_unwind(AssertUnwindSafe(|| {
                PreparedProvisionalFrame::build_with(
                    &b,
                    ProvisionalWeightInput::Frame(&f),
                    B,
                    &mut |context| {
                        let i = count;
                        count += 1;
                        if i == failure {
                            assert!(!unwind, "injected provisional panic");
                            Err(IncidenceError::WeightFrameAllocation { context })
                        } else {
                            Ok(())
                        }
                    },
                )
            }));
            if unwind {
                assert!(result.is_err());
            } else {
                let e = result.unwrap().unwrap_err();
                assert!(matches!(
                    e.source,
                    IncidenceError::WeightFrameAllocation { .. }
                ));
                assert_eq!(
                    e.stage,
                    if failure == 0 {
                        ProvisionalFrameStage::Reduction
                    } else {
                        ProvisionalFrameStage::Finishing
                    }
                );
                assert!(e.admitted_payload_bound.is_some());
            }
            assert_eq!(count, failure + 1);
            assert_eq!(b.level_count(), 2);
            assert_eq!(retained, b.retained_payload_bytes().unwrap());
            assert_eq!(ptr, b.current_level().topology().tuples().as_ptr());
            let p =
                PreparedProvisionalFrame::try_replay_last(&b, ProvisionalWeightInput::Frame(&f), B)
                    .unwrap();
            assert_eq!(p.frame().weights(), &[8.; 8]);
        }
    }
    assert!(size_of::<PreparedProvisionalFailure>() <= 128);
}
#[test]
fn borrowed_and_owned_lifetimes_have_distinct_exact_peak_formulas() {
    let t = topology();
    let b = builder(&t);
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    let budget = PreparedHierarchyBudget {
        additional_live_payload_bytes: 123,
        ..B
    };
    let frame = PreparedProvisionalFrame::setup_payload_report(
        &b,
        &ProvisionalWeightInput::Frame(&f),
        budget,
    )
    .unwrap();
    let base = t.retained_payload_bytes().unwrap() + b.retained_payload_bytes().unwrap() + 123;
    assert_eq!(
        frame.reduction_payload_bound,
        base + f.retained_payload_bytes().unwrap() + 64
    );
    assert_eq!(
        frame.finishing_payload_bound,
        base + f.retained_payload_bytes().unwrap() + 256
    );
    assert_eq!(frame.total_payload_bound, frame.finishing_payload_bound);
    let mut weights = Vec::with_capacity(128);
    weights.extend_from_slice(f.weights());
    let input = ProvisionalWeightInput::Owned(weights);
    let owned = PreparedProvisionalFrame::setup_payload_report(&b, &input, budget).unwrap();
    assert_eq!(owned.input_payload_bytes, 1024);
    assert!(!owned.input_live_during_finishing);
    assert_eq!(owned.reduction_payload_bound, base + 1024 + 64);
    assert_eq!(owned.finishing_payload_bound, base + 256);
    assert_eq!(owned.total_payload_bound, base + 1088);
    let p = PreparedProvisionalFrame::try_replay_last(
        &b,
        input,
        PreparedHierarchyBudget {
            maximum_payload_bytes: owned.total_payload_bound,
            ..budget
        },
    )
    .unwrap();
    assert_eq!(p.setup_report(), owned);
    let ptr = p.frame().weights().as_ptr();
    let weights = p.into_tuple_weights();
    assert_eq!(ptr, weights.as_ptr());
    assert_eq!(weights, &[8.; 8]);
    let e = PreparedProvisionalFrame::build_with(
        &b,
        ProvisionalWeightInput::Frame(&f),
        PreparedHierarchyBudget {
            maximum_payload_bytes: frame.total_payload_bound - 1,
            ..budget
        },
        &mut |_| panic!("not admitted"),
    )
    .unwrap_err();
    assert_eq!(e.stage, ProvisionalFrameStage::Admission);
    assert_eq!(e.admitted_payload_bound, None);
    PreparedProvisionalFrame::try_replay_last(
        &b,
        ProvisionalWeightInput::Frame(&f),
        PreparedHierarchyBudget {
            maximum_payload_bytes: frame.total_payload_bound,
            ..budget
        },
    )
    .unwrap();
}
#[test]
fn no_transition_wrong_owner_shape_values_and_overflow_reject_before_reservation() {
    let t = topology();
    let b = builder(&t);
    let equal = topology();
    let f = ThreeWayWeightFrame::try_new(&equal, WeightFrameInput::UnitTuples).unwrap();
    for input in [
        ProvisionalWeightInput::Frame(&f),
        ProvisionalWeightInput::Owned(vec![1.; 63]),
    ] {
        let e = PreparedProvisionalFrame::build_with(&b, input, B, &mut |_| {
            panic!("invalid static input")
        })
        .unwrap_err();
        assert_eq!(e.stage, ProvisionalFrameStage::Sizing);
        assert_eq!(e.admitted_payload_bound, None);
    }
    for value in [0., -1., f64::NAN, f64::INFINITY] {
        let mut values = vec![1.; 64];
        values[17] = value;
        let e = PreparedProvisionalFrame::build_with(
            &b,
            ProvisionalWeightInput::Owned(values),
            B,
            &mut |_| panic!("invalid values"),
        )
        .unwrap_err();
        assert!(matches!(
            e.source,
            IncidenceError::InvalidWeight {
                tuple_index: 17,
                ..
            }
        ));
        assert_eq!(e.stage, ProvisionalFrameStage::InputValidation);
    }
    let e = PreparedProvisionalFrame::build_with(
        &b,
        ProvisionalWeightInput::Owned(vec![1.; 64]),
        PreparedHierarchyBudget {
            additional_live_payload_bytes: usize::MAX,
            ..B
        },
        &mut |_| panic!("overflow"),
    )
    .unwrap_err();
    assert_eq!(e.stage, ProvisionalFrameStage::Sizing);
    let empty = PreparedHierarchyBuilder::try_new(&t, b.limits(), B).unwrap();
    let e = PreparedProvisionalFrame::try_replay_last(
        &empty,
        ProvisionalWeightInput::Owned(vec![1.; 64]),
        B,
    )
    .unwrap_err();
    assert!(matches!(
        e.source,
        IncidenceError::HierarchyTransitionMissing
    ));
}

#[test]
fn numerical_failures_preserve_prefix_and_allow_recovery() {
    let t = PreparedThreeWayTopology::try_from_collapsed(
        [2; 3],
        &[[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]],
    )
    .unwrap();
    let f =
        ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[f64::MAX * 0.3; 4])).unwrap();
    for (map, stage) in [
        (
            FactorAggregation::consecutive_halving([2; 3]).unwrap(),
            ProvisionalFrameStage::Reduction,
        ),
        (
            FactorAggregation::new([2; 3], [vec![0, 0], vec![0, 1], vec![0, 1]]).unwrap(),
            ProvisionalFrameStage::Finishing,
        ),
    ] {
        let mut b = PreparedHierarchyBuilder::try_new(
            &t,
            PreparedHierarchyLimits {
                maximum_transitions: 2,
                maximum_total_tuples: 32,
                maximum_total_coefficients: 32,
                require_strict_dimension_reduction: true,
            },
            B,
        )
        .unwrap();
        b.try_append(map, B).unwrap();
        let ptr = b.current_level().topology().tuples().as_ptr();
        let bytes = b.retained_payload_bytes().unwrap();
        for owned in [false, true] {
            let input = if owned {
                ProvisionalWeightInput::Owned(f.weights().to_vec())
            } else {
                ProvisionalWeightInput::Frame(&f)
            };
            let e = PreparedProvisionalFrame::try_replay_last(&b, input, B).unwrap_err();
            assert_eq!(e.stage, stage);
            assert!(e.admitted_payload_bound.is_some());
            assert_eq!(b.retained_payload_bytes().unwrap(), bytes);
            assert_eq!(b.current_level().topology().tuples().as_ptr(), ptr);
            assert_eq!(f.weights(), &[f64::MAX * 0.3; 4]);
            let p = PreparedProvisionalFrame::try_replay_last(
                &b,
                ProvisionalWeightInput::Owned(vec![1.; 4]),
                B,
            )
            .unwrap();
            assert!(p.frame().weights().iter().all(|w| w.is_finite() && *w > 0.));
            p.validate_for(&b).unwrap();
            let other = PreparedHierarchyBuilder::try_new(&t, b.limits(), B).unwrap();
            assert!(matches!(
                p.validate_for(&other),
                Err(IncidenceError::HierarchyBindingMismatch)
            ));
        }
    }
}
