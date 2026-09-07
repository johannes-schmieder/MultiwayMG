use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
fn limits() -> PreparedHierarchyLimits {
    PreparedHierarchyLimits {
        maximum_transitions: 4,
        maximum_total_tuples: usize::MAX,
        maximum_total_coefficients: usize::MAX,
        require_strict_dimension_reduction: true,
    }
}
fn grid() -> PreparedThreeWayTopology {
    let keys: Vec<_> = (0..4)
        .flat_map(|a| (0..4).flat_map(move |b| (0..4).map(move |c| [a, b, c])))
        .collect();
    PreparedThreeWayTopology::try_from_collapsed([4; 3], &keys).unwrap()
}
fn half(n: usize) -> FactorAggregation {
    FactorAggregation::consecutive_halving([n; 3]).unwrap()
}
fn builder(t: &PreparedThreeWayTopology) -> PreparedHierarchyBuilder<'_> {
    let mut b = PreparedHierarchyBuilder::try_new(t, limits(), B).unwrap();
    b.try_append(half(4), B).unwrap();
    b
}
fn prefix(b: &PreparedHierarchyBuilder<'_>) {
    assert_eq!(b.level_count(), 2);
    assert_eq!(b.total_tuple_count(), 72);
    assert_eq!(b.total_coefficient_count(), 18);
    assert_eq!(b.current_level().topology().level_counts(), [2; 3]);
}

#[test]
fn all_initial_and_append_reservations_fail_or_unwind_without_losing_prefix() {
    let t = grid();
    for fail in 0..2 {
        let mut count = 0;
        let result = PreparedHierarchyBuilder::build_with(&t, limits(), B, &mut |context| {
            let i = count;
            count += 1;
            if i == fail {
                Err(IncidenceError::TopologyAllocation { context })
            } else {
                Ok(())
            }
        });
        assert!(matches!(
            result,
            Err(IncidenceError::TopologyAllocation { .. })
        ));
        assert_eq!(count, fail + 1);
        let mut count = 0;
        assert!(
            catch_unwind(AssertUnwindSafe(|| PreparedHierarchyBuilder::build_with(
                &t,
                limits(),
                B,
                &mut |_| {
                    let i = count;
                    count += 1;
                    assert_ne!(i, fail, "injected descriptor panic");
                    Ok(())
                }
            )))
            .is_err()
        );
    }
    let mut b = builder(&t);
    let mut reservations = 0;
    b.append_with(half(2), B, &mut |_| {
        reservations += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(reservations, 10);
    for fail in 0..reservations {
        for unwind in [false, true] {
            let mut b = builder(&t);
            let retained = b.retained_payload_bytes().unwrap();
            let map_ptr = b.hierarchy.aggregations.as_ptr();
            let transition_ptr = b.hierarchy.transitions.as_ptr();
            let key_ptr = b.current_level().topology().tuples().as_ptr();
            let mut count = 0;
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                b.append_with(half(2), B, &mut |context| {
                    let i = count;
                    count += 1;
                    if i == fail {
                        assert!(!unwind, "injected coarse panic");
                        Err(IncidenceError::TopologyAllocation { context })
                    } else {
                        Ok(())
                    }
                })
            }));
            if unwind {
                assert!(outcome.is_err());
            } else {
                let e = outcome.unwrap().unwrap_err();
                assert!(matches!(
                    e.source,
                    IncidenceError::TopologyAllocation { .. }
                ));
                assert!(e.report.structural_build_attempted);
                assert_eq!(e.report.coarse_tuples, None);
                assert_eq!(e.report.source_tuples, 8);
                assert!(
                    b.setup_peak_payload_bound() >= e.report.requested_peak_payload_bytes.unwrap()
                );
            }
            assert_eq!(count, fail + 1);
            prefix(&b);
            assert_eq!(retained, b.retained_payload_bytes().unwrap());
            assert_eq!(key_ptr, b.current_level().topology().tuples().as_ptr());
            assert_eq!(map_ptr, b.hierarchy.aggregations.as_ptr());
            assert_eq!(transition_ptr, b.hierarchy.transitions.as_ptr());
            b.try_append(half(2), B).unwrap();
            assert_eq!(map_ptr, b.hierarchy.aggregations.as_ptr());
            assert_eq!(transition_ptr, b.hierarchy.transitions.as_ptr());
            assert_eq!(b.finish().level_count(), 3);
        }
    }
}

#[test]
fn initial_and_append_limits_precede_first_reservation() {
    let t = grid();
    let required = PreparedHierarchyBuilder::initial_payload_bound(&t, limits(), B).unwrap();
    for (l, b) in [
        (
            limits(),
            PreparedHierarchyBudget {
                maximum_payload_bytes: required - 1,
                ..B
            },
        ),
        (
            PreparedHierarchyLimits {
                maximum_transitions: usize::MAX,
                ..limits()
            },
            B,
        ),
        (
            PreparedHierarchyLimits {
                maximum_total_tuples: 63,
                ..limits()
            },
            B,
        ),
        (
            PreparedHierarchyLimits {
                maximum_total_coefficients: 11,
                ..limits()
            },
            B,
        ),
        (
            limits(),
            PreparedHierarchyBudget {
                additional_live_payload_bytes: usize::MAX,
                ..B
            },
        ),
    ] {
        assert!(
            PreparedHierarchyBuilder::build_with(&t, l, b, &mut |_| panic!("not admitted"))
                .is_err()
        );
    }
    PreparedHierarchyBuilder::try_new(
        &t,
        limits(),
        PreparedHierarchyBudget {
            maximum_payload_bytes: required,
            ..B
        },
    )
    .unwrap();
    let mut b = builder(&t);
    for map in [half(4), FactorAggregation::identity([2; 3]).unwrap()] {
        let e = b
            .append_with(map, B, &mut |_| panic!("not admitted"))
            .unwrap_err();
        assert!(!e.report.structural_build_attempted);
        prefix(&b);
    }
    b.limits.maximum_transitions = 1;
    assert!(
        b.append_with(half(2), B, &mut |_| panic!("not admitted"))
            .is_err()
    );
    b.limits = limits();
    b.limits.maximum_total_coefficients = 20;
    let e = b
        .append_with(half(2), B, &mut |_| panic!("not admitted"))
        .unwrap_err();
    assert!(matches!(
        e.source,
        IncidenceError::HierarchyStructureLimit {
            actual: 21,
            maximum: 20,
            ..
        }
    ));
    b.limits = limits();
    let map = half(2);
    let required = t.retained_payload_bytes().unwrap()
        + b.retained_payload_bytes().unwrap()
        + map.retained_payload_bytes().unwrap()
        + PreparedCoarseTupleMap::setup_payload_bound(b.current_level(), &map).unwrap()
        + 321;
    let budget = PreparedHierarchyBudget {
        maximum_payload_bytes: required - 1,
        additional_live_payload_bytes: 321,
    };
    let before = b.setup_peak_payload_bound();
    let e = b
        .append_with(map, budget, &mut |_| panic!("not admitted"))
        .unwrap_err();
    assert_eq!(e.report.requested_peak_payload_bytes, Some(required));
    assert!(!e.report.structural_build_attempted);
    assert_eq!(b.setup_peak_payload_bound(), before);
    prefix(&b);
    b.try_append(
        half(2),
        PreparedHierarchyBudget {
            maximum_payload_bytes: required,
            ..budget
        },
    )
    .unwrap();
    assert_eq!(b.level_count(), 3);
}

#[test]
fn tuple_rejection_counts_attempt_and_preserves_prefix() {
    let t = grid();
    let mut b = builder(&t);
    b.limits.maximum_total_tuples = 72;
    let retained = b.retained_payload_bytes().unwrap();
    let e = b.try_append(half(2), B).unwrap_err();
    prefix(&b);
    assert!(matches!(
        e.source,
        IncidenceError::HierarchyStructureLimit {
            actual: 73,
            maximum: 72,
            ..
        }
    ));
    assert!(e.report.structural_build_attempted);
    assert_eq!(e.report.coarse_tuples, Some(1));
    assert_eq!(retained, b.retained_payload_bytes().unwrap());
    // An exact identity is admissible only when explicitly allowed, and still
    // charges every structural level. No implicit dimension policy is selected.
    let mut l = limits();
    l.require_strict_dimension_reduction = false;
    let mut b = PreparedHierarchyBuilder::try_new(&t, l, B).unwrap();
    b.try_append(FactorAggregation::identity([4; 3]).unwrap(), B)
        .unwrap();
    assert_eq!(b.total_tuple_count(), 128);
}

#[test]
fn component_failure_is_atomic_and_terminal_only_has_no_descriptors() {
    let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0; 3], [1; 3]]).unwrap();
    let mut b = PreparedHierarchyBuilder::try_new(&t, limits(), B).unwrap();
    let retained = b.retained_payload_bytes().unwrap();
    let e = b.try_append(half(2), B).unwrap_err();
    assert!(matches!(
        e.source,
        IncidenceError::CrossComponentAggregation { .. }
    ));
    assert!(e.report.structural_build_attempted);
    assert_eq!(b.level_count(), 1);
    assert_eq!(retained, b.retained_payload_bytes().unwrap());
    let zero = PreparedHierarchyLimits {
        maximum_transitions: 0,
        ..limits()
    };
    let b = PreparedHierarchyBuilder::build_with(&t, zero, B, &mut |_| panic!("no descriptors"))
        .unwrap();
    assert_eq!(b.retained_payload_bytes().unwrap(), 0);
    assert!(std::ptr::eq(b.current_level(), &t));
    assert_eq!(b.finish().level_count(), 1);
    // Total coefficient dimension is at least three for every valid topology.
    assert!(std::mem::size_of::<PreparedHierarchyAppendFailure>() <= 128);
    assert!(add(usize::MAX, 1).is_err());
}
