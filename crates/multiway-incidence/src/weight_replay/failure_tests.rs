//! Private reservation-boundary injection, distinct from allocator-null testing.
use super::*;
use crate::{FactorAggregation, FactorPair, PreparedCoarseTupleMap, PreparedPairEdgeMap, WeightFrameInput};

#[test]
fn all_coarse_and_pair_reservation_boundaries_leave_previous_owners_usable() {
    let tuples: Vec<_> = (0..2).flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k]))).collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &tuples).unwrap();
    let a = FactorAggregation::consecutive_halving([2; 3]).unwrap();
    let c = PreparedCoarseTupleMap::try_new(&t, &a).unwrap();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    let old = CoarseWeightReplay::try_new(&c, &f).unwrap();
    let saved = format!("{t:?}{a:?}{c:?}{f:?}{old:?}");
    for unwind in [false, true] {
        for fail_at in 0..5 {
            let mut reached = 0;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                CoarseWeightReplay::build_with(&c, &f, WeightReplayPayloadBudget::UNLIMITED, &mut |context| {
                    let index = reached;
                    reached += 1;
                    if index == fail_at {
                        assert!(!unwind, "injected coarse replay unwind");
                        return Err(IncidenceError::WeightFrameAllocation { context });
                    }
                    Ok(())
                })
            }));
            assert_eq!(reached, fail_at + 1);
            if unwind { assert!(result.is_err()); }
            else { assert!(matches!(result.unwrap(), Err(IncidenceError::WeightFrameAllocation { .. }))); }
            assert_eq!(format!("{t:?}{a:?}{c:?}{f:?}{old:?}"), saved);
            old.validate_for(&c, &f).unwrap();
            CoarseWeightReplay::try_new(&c, &f).unwrap();
        }
        for pair in FactorPair::ALL {
            let p = PreparedPairEdgeMap::try_new(&t, pair).unwrap();
            let old_pair = PairConductanceReplay::try_new(&p, &f).unwrap();
            let saved_pair = format!("{p:?}{old_pair:?}");
            for fail_at in 0..3 {
                let mut reached = 0;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    PairConductanceReplay::build_with(&p, &f, WeightReplayPayloadBudget::UNLIMITED, &mut |context| {
                        let index = reached;
                        reached += 1;
                        if index == fail_at {
                            assert!(!unwind, "injected pair replay unwind");
                            return Err(IncidenceError::WeightFrameAllocation { context });
                        }
                        Ok(())
                    })
                }));
                assert_eq!(reached, fail_at + 1);
                if unwind { assert!(result.is_err()); }
                else { assert!(matches!(result.unwrap(), Err(IncidenceError::WeightFrameAllocation { .. }))); }
                assert_eq!(format!("{p:?}{old_pair:?}"), saved_pair);
                old_pair.validate_for(&p, &f).unwrap();
                PairConductanceReplay::try_new(&p, &f).unwrap();
            }
        }
    }
}

#[test]
fn wrong_topology_insufficient_budget_and_integer_overflow_precede_reservations() {
    let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let foreign = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let a = FactorAggregation::identity([1; 3]).unwrap();
    let c = PreparedCoarseTupleMap::try_new(&t, &a).unwrap();
    let p = PreparedPairEdgeMap::try_new(&t, FactorPair::OneTwo).unwrap();
    let wrong = ThreeWayWeightFrame::try_new(&foreign, WeightFrameInput::UnitTuples).unwrap();
    assert!(CoarseWeightReplay::build_with(&c, &wrong, WeightReplayPayloadBudget::UNLIMITED,
        &mut |_| panic!("owner must reject first")).is_err());
    assert!(PairConductanceReplay::build_with(&p, &wrong, WeightReplayPayloadBudget::UNLIMITED,
        &mut |_| panic!("owner must reject first")).is_err());
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    for budget in [
        WeightReplayPayloadBudget { maximum_payload_bytes: 0, additional_live_payload_bytes: 0 },
        WeightReplayPayloadBudget { maximum_payload_bytes: usize::MAX, additional_live_payload_bytes: usize::MAX },
    ] {
        assert!(CoarseWeightReplay::build_with(&c, &f, budget, &mut |_| panic!("budget first")).is_err());
        assert!(PairConductanceReplay::build_with(&p, &f, budget, &mut |_| panic!("budget first")).is_err());
    }
}
