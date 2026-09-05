//! Error/unwind injection before every new reservation, not allocator-null tests.
use super::*;
use crate::{FactorAggregation, FactorPair, IncidenceError, PreparedThreeWayTopology};

#[test]
fn all_reservation_boundaries_release_partial_owners_and_allow_retry() {
    let rows: Vec<_> = (0..2)
        .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let source = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    let aggregation = FactorAggregation::consecutive_halving([2; 3]).unwrap();
    let snapshot = format!("{source:?}{aggregation:?}");
    for unwind in [false, true] {
        for fail_at in 0..10 {
            let mut reached = 0;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                PreparedCoarseTupleMap::build_with(
                    &source,
                    &aggregation,
                    usize::MAX,
                    &mut |context| {
                        let index = reached;
                        reached += 1;
                        if index == fail_at {
                            assert!(!unwind, "injected coarse setup unwind");
                            return Err(IncidenceError::TopologyAllocation { context });
                        }
                        Ok(())
                    },
                )
            }));
            assert_eq!(reached, fail_at + 1);
            if unwind {
                assert!(result.is_err());
            } else {
                assert!(matches!(
                    result.unwrap(),
                    Err(IncidenceError::TopologyAllocation { .. })
                ));
            }
            assert_eq!(format!("{source:?}{aggregation:?}"), snapshot);
            PreparedCoarseTupleMap::try_new(&source, &aggregation).unwrap();
        }
        for pair in FactorPair::ALL {
            for fail_at in 0..4 {
                let mut reached = 0;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    PreparedPairEdgeMap::build_with(&source, pair, usize::MAX, &mut |context| {
                        let index = reached;
                        reached += 1;
                        if index == fail_at {
                            assert!(!unwind, "injected pair setup unwind");
                            return Err(IncidenceError::TopologyAllocation { context });
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
                        Err(IncidenceError::TopologyAllocation { .. })
                    ));
                }
                assert_eq!(format!("{source:?}{aggregation:?}"), snapshot);
                PreparedPairEdgeMap::try_new(&source, pair).unwrap();
            }
        }
    }
}

#[test]
fn size_and_budget_errors_precede_reservations() {
    assert!(groups::setup_bound::<[u32; 3]>(usize::MAX).is_err());
    assert!(groups::setup_bound::<[u32; 2]>(0).is_err());
    assert!(groups::setup_bound::<[u32; 2]>(isize::MAX as usize).is_err());
    let source = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let aggregation = FactorAggregation::identity([1; 3]).unwrap();
    let bound = PreparedCoarseTupleMap::setup_payload_bound(&source, &aggregation).unwrap();
    assert!(matches!(
        PreparedCoarseTupleMap::build_with(&source, &aggregation, bound - 1, &mut |_| panic!(
            "budget must reject first"
        )),
        Err(IncidenceError::SymbolicSetupBudgetExceeded { .. })
    ));
    let wrong = FactorAggregation::identity([2; 3]).unwrap();
    assert!(
        PreparedCoarseTupleMap::build_with(&source, &wrong, usize::MAX, &mut |_| panic!(
            "shape must reject first"
        ))
        .is_err()
    );
    let bound = PreparedPairEdgeMap::setup_payload_bound(&source).unwrap();
    assert!(
        PreparedPairEdgeMap::build_with(&source, FactorPair::OneTwo, bound - 1, &mut |_| panic!(
            "budget must reject first"
        ))
        .is_err()
    );
}
