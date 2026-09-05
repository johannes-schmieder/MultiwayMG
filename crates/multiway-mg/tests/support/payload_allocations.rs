//! Actual allocation/lifetime checks at the strict prepared payload boundary.

use super::{GLOBAL, Result, no_events};
use multiway_mg::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, MultiwayError, PcgPayloadBudget,
    PcgTraceOptions, PcgTraceResult, PcgTraceWorkspace, prepared_map_pcg_payload_report,
    solve_projected_pcg_traced_with_payload_budget,
};
use std::hint::black_box;

pub(super) fn check(
    hierarchy: &CycleScreenedMapHierarchy,
    rhs: &[f64],
    options: PcgTraceOptions,
    outer: &mut PcgTraceWorkspace,
    inner: &mut CycleScreenedMapHierarchyWorkspace,
    expected: &PcgTraceResult,
) -> Result<()> {
    let before = GLOBAL.stats();
    let clone = black_box(hierarchy.clone());
    let cloned = GLOBAL.stats() - before;
    let inventory = clone.retained_payload_report()?;
    assert_eq!(
        inventory.shared_problem_bytes,
        hierarchy.retained_payload_report()?.shared_problem_bytes
    );
    assert_eq!(cloned.bytes_allocated, inventory.exclusive_bytes()?);
    assert_eq!(cloned.reallocations, 0);
    assert_eq!(cloned.deallocations, 0);
    let before = GLOBAL.stats();
    drop(clone);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, inventory.exclusive_bytes()?);
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);

    let before = GLOBAL.stats();
    let copy = black_box(expected.clone());
    let copied = GLOBAL.stats() - before;
    assert_eq!(copied.bytes_allocated, copy.retained_payload_bytes()?);
    assert_eq!(copied.allocations, 2);
    let extra = copy.retained_payload_bytes()?;
    let before = GLOBAL.stats();
    let report = prepared_map_pcg_payload_report(hierarchy, rhs, options, outer, inner, extra)?;
    let total = report.total_bytes()?;
    let events = GLOBAL.stats() - before;
    no_events(events);
    assert_eq!(report.outer_workspace_bytes, outer.retained_bytes()?);
    assert_eq!(report.hierarchy_workspace_bytes, inner.retained_bytes()?);
    assert_eq!(report.rhs_bytes, core::mem::size_of_val(rhs));
    assert_eq!(report.additional_live_bytes, extra);
    let snapshot = format!("{outer:?}{inner:?}");
    let before = GLOBAL.stats();
    let error = solve_projected_pcg_traced_with_payload_budget(
        hierarchy,
        rhs,
        options,
        outer,
        inner,
        PcgPayloadBudget {
            maximum_bytes: total - 1,
            additional_live_bytes: extra,
        },
    )
    .unwrap_err();
    let events = GLOBAL.stats() - before;
    no_events(events);
    assert!(
        matches!(error, MultiwayError::PayloadBudgetExceeded { required, budget }
        if required == total && budget == total - 1)
    );
    assert_eq!(format!("{outer:?}{inner:?}"), snapshot);
    let before = GLOBAL.stats();
    let error = solve_projected_pcg_traced_with_payload_budget(
        hierarchy,
        rhs,
        options,
        outer,
        inner,
        PcgPayloadBudget {
            maximum_bytes: usize::MAX,
            additional_live_bytes: usize::MAX,
        },
    )
    .unwrap_err();
    let events = GLOBAL.stats() - before;
    no_events(events);
    assert!(matches!(error, MultiwayError::WorkspaceSizeOverflow { .. }));
    assert_eq!(format!("{outer:?}{inner:?}"), snapshot);
    for _ in 0..4 {
        let before = GLOBAL.stats();
        let result = solve_projected_pcg_traced_with_payload_budget(
            hierarchy,
            black_box(rhs),
            options,
            outer,
            inner,
            PcgPayloadBudget {
                maximum_bytes: total,
                additional_live_bytes: extra,
            },
        )?;
        let events = GLOBAL.stats() - before;
        no_events(events);
        super::pcg_allocations::equal(result, expected);
    }
    assert_eq!(
        prepared_map_pcg_payload_report(hierarchy, rhs, options, outer, inner, extra)?,
        report
    );
    assert_eq!(copy, *expected);
    let before = GLOBAL.stats();
    drop(copy);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, extra);
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    println!(
        "payload dimension={} shared={} exclusive={} working_set={} extra_live={} clone_allocate_release=exact report/reject/solve_allocations=0",
        rhs.len(),
        inventory.shared_problem_bytes,
        inventory.exclusive_bytes()?,
        total,
        extra
    );
    Ok(())
}
