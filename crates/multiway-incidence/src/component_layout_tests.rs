use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
fn disconnected() -> PreparedThreeWayTopology {
    let mut keys: Vec<_> = [0, 2]
        .into_iter()
        .flat_map(|a| {
            [1, 3]
                .into_iter()
                .flat_map(move |b| [0, 2].into_iter().map(move |c| [a, b, c]))
        })
        .collect();
    keys.extend([[1, 0, 1], [1, 2, 1]]);
    keys.sort_unstable();
    PreparedThreeWayTopology::try_from_collapsed([3, 4, 3], &keys).unwrap()
}
#[test]
fn widths_last_index_and_checked_array_overflow() {
    assert!(narrow(0));
    assert!(narrow(1));
    assert!(narrow(u32::MAX as usize));
    if usize::BITS > 32 {
        assert!(narrow(u32::MAX as usize + 1));
        assert!(!narrow(u32::MAX as usize + 2));
    }
    assert!(add(usize::MAX, 1).is_err());
    assert!(array_bytes::<u32>(usize::MAX).is_err());
    let t = disconnected();
    let a = PreparedComponentLayout::build_with(&t, B, true, &mut |_| Ok(())).unwrap();
    let b = PreparedComponentLayout::build_with(&t, B, false, &mut |_| Ok(())).unwrap();
    assert_eq!(a.tuple_index_bytes(), 4);
    assert_eq!(b.tuple_index_bytes(), size_of::<usize>());
    assert_eq!(
        b.retained_payload_bytes().unwrap() - a.retained_payload_bytes().unwrap(),
        (size_of::<usize>() - 4) * 10
    );
    let ar = PreparedComponentRecoding::try_new(&a, B).unwrap();
    let br = PreparedComponentRecoding::try_new(&b, B).unwrap();
    for c in 0..a.component_count() {
        let mut x = vec![];
        let mut y = vec![];
        a.component(c).unwrap().tuple_ids().for_each(|i| x.push(i));
        b.component(c).unwrap().tuple_ids().for_each(|i| y.push(i));
        assert_eq!(x, y);
        let mut x = vec![[0; 3]; x.len()];
        let mut y = x.clone();
        ar.write_keys_into(c, &mut x).unwrap();
        br.write_keys_into(c, &mut y).unwrap();
        assert_eq!(x, y);
    }
}
#[test]
fn all_layout_and_recoder_reservations_fail_and_unwind_then_recover() {
    let t = disconnected();
    for compact in [true, false] {
        let mut count = 0;
        PreparedComponentLayout::build_with(&t, B, compact, &mut |_| {
            count += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(count, 4);
        for failure in 0..4 {
            for unwind in [false, true] {
                let mut seen = 0;
                let result = catch_unwind(AssertUnwindSafe(|| {
                    PreparedComponentLayout::build_with(&t, B, compact, &mut |context| {
                        let i = seen;
                        seen += 1;
                        if i == failure {
                            assert!(!unwind, "injected layout panic");
                            Err(IncidenceError::TopologyAllocation { context })
                        } else {
                            Ok(())
                        }
                    })
                }));
                if unwind {
                    assert!(result.is_err());
                } else {
                    assert!(matches!(
                        result.unwrap(),
                        Err(IncidenceError::TopologyAllocation { .. })
                    ));
                }
                assert_eq!(seen, failure + 1);
                let a = PreparedComponentLayout::try_new(&t, B).unwrap();
                assert_eq!(a.component_count(), 2);
            }
        }
    }
    let a = PreparedComponentLayout::try_new(&t, B).unwrap();
    for unwind in [false, true] {
        let result = catch_unwind(AssertUnwindSafe(|| {
            PreparedComponentRecoding::build_with(&a, B, &mut |context| {
                assert!(!unwind, "injected recoding panic");
                Err(IncidenceError::TopologyAllocation { context })
            })
        }));
        if unwind {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().is_err());
        }
        let r = PreparedComponentRecoding::try_new(&a, B).unwrap();
        assert_eq!(r.retained_payload_bytes().unwrap(), 40);
    }
}
#[test]
fn exact_budgets_include_source_partition_and_changing_other_owners() {
    let t = disconnected();
    let b = PreparedHierarchyBudget {
        additional_live_payload_bytes: 321,
        ..B
    };
    let bound = PreparedComponentLayout::setup_payload_bound(&t, b).unwrap();
    assert!(
        PreparedComponentLayout::build_with(
            &t,
            PreparedHierarchyBudget {
                maximum_payload_bytes: bound - 1,
                ..b
            },
            true,
            &mut |_| panic!("not admitted")
        )
        .is_err()
    );
    let a = PreparedComponentLayout::try_new(
        &t,
        PreparedHierarchyBudget {
            maximum_payload_bytes: bound,
            ..b
        },
    )
    .unwrap();
    assert_eq!(
        a.retained_payload_bytes().unwrap(),
        4 * 10 + 4 * 10 + 2 * size_of::<usize>() * 3
    );
    let bound = PreparedComponentRecoding::setup_payload_bound(&a, b).unwrap();
    assert!(
        PreparedComponentRecoding::build_with(
            &a,
            PreparedHierarchyBudget {
                maximum_payload_bytes: bound - 1,
                ..b
            },
            &mut |_| panic!("not admitted")
        )
        .is_err()
    );
    PreparedComponentRecoding::try_new(
        &a,
        PreparedHierarchyBudget {
            maximum_payload_bytes: bound,
            ..b
        },
    )
    .unwrap();
    let overflow = PreparedHierarchyBudget {
        additional_live_payload_bytes: usize::MAX,
        ..B
    };
    assert!(
        PreparedComponentLayout::build_with(&t, overflow, true, &mut |_| panic!("not admitted"))
            .is_err()
    );
    assert!(
        PreparedComponentRecoding::build_with(&a, overflow, &mut |_| panic!("not admitted"))
            .is_err()
    );
}
#[test]
fn identity_needs_no_arrays_and_source_already_rejects_unused_levels() {
    let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let a = PreparedComponentLayout::build_with(&t, B, true, &mut |_| panic!("identity")).unwrap();
    assert!(a.is_identity());
    assert_eq!(a.retained_payload_bytes().unwrap(), 0);
    let r = PreparedComponentRecoding::build_with(&a, B, &mut |_| panic!("identity")).unwrap();
    assert_eq!(r.retained_payload_bytes().unwrap(), 0);
    let e = PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0; 3]]).unwrap_err();
    assert!(matches!(
        e,
        IncidenceError::UnusedLevel {
            factor: 0,
            level: 1
        }
    ));
}
