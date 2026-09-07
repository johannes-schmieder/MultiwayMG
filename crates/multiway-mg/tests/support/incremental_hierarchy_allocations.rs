//! Incremental setup retains descriptors, rejects atomically and releases exactly.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    FactorAggregation, PreparedHierarchyBudget, PreparedHierarchyBuilder, PreparedHierarchyLimits,
    PreparedThreeWayTopology,
};
use std::hint::black_box;
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
pub fn run() -> Result<()> {
    let keys: Vec<_> = (0..4)
        .flat_map(|a| (0..4).flat_map(move |b| (0..4).map(move |c| [a, b, c])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([4; 3], &keys)?;
    let limits = PreparedHierarchyLimits {
        maximum_transitions: 4,
        maximum_total_tuples: 72,
        maximum_total_coefficients: 21,
        require_strict_dimension_reduction: true,
    };
    let bound = PreparedHierarchyBuilder::initial_payload_bound(&t, limits, B)?;
    let before = GLOBAL.stats();
    assert!(
        PreparedHierarchyBuilder::try_new(
            &t,
            limits,
            PreparedHierarchyBudget {
                maximum_payload_bytes: bound - 1,
                ..B
            }
        )
        .is_err()
    );
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let mut b = black_box(PreparedHierarchyBuilder::try_new(&t, limits, B)?);
    let setup = GLOBAL.stats() - before;
    assert_eq!(setup.allocations, 2);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.deallocations, 0);
    assert_eq!(setup.bytes_allocated, b.retained_payload_bytes()?);
    let map = FactorAggregation::consecutive_halving([4; 3])?;
    let map_bytes = map.retained_payload_bytes()?;
    let old = b.retained_payload_bytes()?;
    let before = GLOBAL.stats();
    b.try_append(map, B)?;
    let append = GLOBAL.stats() - before;
    assert_eq!(append.allocations, 10);
    assert_eq!(append.reallocations, 0);
    assert_eq!(
        append.bytes_allocated - append.bytes_deallocated + map_bytes + old,
        b.retained_payload_bytes()?
    );
    for mode in 0..3 {
        let map = FactorAggregation::consecutive_halving([2; 3])?;
        let map_bytes = map.retained_payload_bytes()?;
        let budget = match mode {
            0 => B,
            1 => PreparedHierarchyBudget {
                maximum_payload_bytes: 0,
                ..B
            },
            _ => PreparedHierarchyBudget {
                additional_live_payload_bytes: usize::MAX,
                ..B
            },
        };
        let retained = b.retained_payload_bytes()?;
        let before = GLOBAL.stats();
        let failure = black_box(b.try_append(map, budget).unwrap_err());
        let failed = GLOBAL.stats() - before;
        assert_eq!(failed.allocations, if mode == 0 { 10 } else { 0 });
        assert_eq!(failed.reallocations, 0);
        assert_eq!(failed.bytes_allocated + map_bytes, failed.bytes_deallocated);
        assert_eq!(failure.report.structural_build_attempted, mode == 0);
        assert_eq!(retained, b.retained_payload_bytes()?);
        assert_eq!(b.level_count(), 2);
    }
    let retained = b.retained_payload_bytes()?;
    let before = GLOBAL.stats();
    let h = black_box(b.finish());
    no_events(GLOBAL.stats() - before);
    assert_eq!(retained, h.retained_payload_bytes()?);
    let before = GLOBAL.stats();
    drop(h);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    assert_eq!(released.bytes_deallocated, retained);
    let before = GLOBAL.stats();
    let b = PreparedHierarchyBuilder::try_new(
        &t,
        PreparedHierarchyLimits {
            maximum_transitions: 0,
            ..limits
        },
        B,
    )?;
    let h = b.finish();
    assert_eq!(h.retained_payload_bytes()?, 0);
    drop(h);
    no_events(GLOBAL.stats() - before);
    println!(
        "incremental structure: two fixed descriptor arrays, ten reservations per append, zero descriptor reallocations/finish allocations; exact failed and retained release; terminal-only zero arrays"
    );
    Ok(())
}
