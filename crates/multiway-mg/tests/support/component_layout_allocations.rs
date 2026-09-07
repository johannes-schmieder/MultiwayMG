use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    PreparedComponentLayout, PreparedComponentRecoding, PreparedHierarchyBudget,
    PreparedThreeWayTopology,
};
use std::hint::black_box;
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
pub fn run() -> Result<()> {
    let t = PreparedThreeWayTopology::try_from_collapsed([3; 3], &[[0; 3], [1; 3], [2; 3]])?;
    let setup = PreparedComponentLayout::setup_payload_bound(&t, B)?;
    let before = GLOBAL.stats();
    assert!(
        PreparedComponentLayout::try_new(
            &t,
            PreparedHierarchyBudget {
                maximum_payload_bytes: setup - 1,
                ..B
            }
        )
        .is_err()
    );
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let a = black_box(PreparedComponentLayout::try_new(&t, B)?);
    let delta = GLOBAL.stats() - before;
    let retained = a.retained_payload_bytes()?;
    assert_eq!(delta.allocations, 4);
    assert_eq!(delta.deallocations, 1);
    assert_eq!(delta.reallocations, 0);
    assert_eq!(delta.bytes_allocated - delta.bytes_deallocated, retained);
    assert_eq!(retained, 4 * 9 + 4 * 3 + 2 * size_of::<usize>() * 4);
    assert_eq!(delta.bytes_deallocated, 3 * size_of::<usize>());
    let setup = PreparedComponentRecoding::setup_payload_bound(&a, B)?;
    let before = GLOBAL.stats();
    assert!(
        PreparedComponentRecoding::try_new(
            &a,
            PreparedHierarchyBudget {
                maximum_payload_bytes: setup - 1,
                ..B
            }
        )
        .is_err()
    );
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let r = black_box(PreparedComponentRecoding::try_new(&a, B)?);
    let delta = GLOBAL.stats() - before;
    assert_eq!(delta.allocations, 1);
    assert_eq!(delta.deallocations, 0);
    assert_eq!(delta.reallocations, 0);
    assert_eq!(delta.bytes_allocated, 4 * 9);
    assert_eq!(delta.bytes_allocated, r.retained_payload_bytes()?);
    let global = [1.0; 9];
    let values = [2.0; 3];
    let mut out = [0.0; 9];
    let mut tuples = [0.0; 3];
    let mut local = [0.0; 3];
    let mut value = [0.0];
    let mut keys = [[0; 3]];
    let before = GLOBAL.stats();
    for _ in 0..32 {
        for c in 0..3 {
            let view = a.component(c).unwrap();
            view.gather_coefficients(&global, &mut local)?;
            view.scatter_coefficients(&local, &mut out)?;
            view.gather_tuple_values(&values, &mut value)?;
            view.scatter_tuple_values(&value, &mut tuples)?;
            r.write_keys_into(c, &mut keys)?;
            assert!(view.gather_coefficients(&[], &mut local).is_err());
            assert!(r.write_keys_into(3, &mut keys).is_err());
        }
        a.validate_for(&t)?;
        a.retained_payload_bytes()?;
        r.retained_payload_bytes()?;
    }
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    drop(r);
    let delta = GLOBAL.stats() - before;
    assert_eq!(delta.allocations, 0);
    assert_eq!(delta.deallocations, 1);
    assert_eq!(delta.bytes_deallocated, 4 * 9);
    assert_eq!(a.component(0).unwrap().dimension(), 3);
    let before = GLOBAL.stats();
    drop(a);
    let delta = GLOBAL.stats() - before;
    assert_eq!(delta.allocations, 0);
    assert_eq!(delta.deallocations, 3);
    assert_eq!(delta.bytes_deallocated, retained);
    let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]])?;
    let before = GLOBAL.stats();
    let a = PreparedComponentLayout::try_new(&t, B)?;
    let r = PreparedComponentRecoding::try_new(&a, B)?;
    r.write_keys_into(0, &mut keys)?;
    drop(r);
    drop(a);
    no_events(GLOBAL.stats() - before);
    println!(
        "component permutation: three retained arrays, one released cursor; one separately released temporary inverse; connected identity and first/repeat permutations allocate zero; exact byte release"
    );
    Ok(())
}
