from pathlib import Path

def replace(path, old, new):
    p = Path(path)
    text = p.read_text()
    assert text.count(old) == 1, (path, old, text.count(old))
    p.write_text(text.replace(old, new))

def new(path, text):
    p = Path(path)
    assert not p.exists(), path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text)

def append(path, text):
    p = Path(path)
    p.write_text(p.read_text() + '\n' + text)

p = 'crates/multiway-mg/src/pcg_trace/workspace.rs'
replace(p, '''    ) -> Result<(), MultiwayError> {
        Self::required_bytes(problem, options)?;
        let dimension = problem.dimension();''', '''    ) -> Result<(), MultiwayError> {
        self.prepare_with(problem, options, || Ok(()))
    }

    // A local boundary callback permits deterministic failure/unwind tests.
    // The public path passes a no-op; no hook, allocator or mutable global is retained.
    fn prepare_with<F>(
        &mut self,
        problem: &ThreeWayProblem,
        options: PcgTraceOptions,
        mut before_reservation: F,
    ) -> Result<(), MultiwayError>
    where
        F: FnMut() -> Result<(), MultiwayError>,
    {
        Self::required_bytes(problem, options)?;
        let dimension = problem.dimension();''')
replace(p, '            reserve(vector, dimension)?;', '''            if dimension > vector.capacity() {
                before_reservation()?;
            }
            reserve(vector, dimension)?;''')
replace(p, '''        reserve(&mut self.samples, samples)?;
        if let Some(projection)''', '''        if samples > self.samples.capacity() {
            before_reservation()?;
        }
        reserve(&mut self.samples, samples)?;
        // This is a delegated preparation boundary, not a hook into the incidence allocator.
        if self.projection.as_ref().is_none_or(|p| !p.is_compatible_with(problem.components())) {
            before_reservation()?;
        }
        if let Some(projection)''')
append(p, '#[cfg(test)]\nmod failure_tests;\n')
new('crates/multiway-mg/src/pcg_trace/workspace/failure_tests.rs', r'''//! Deterministic reservation-boundary failure injection, not malloc-failure coverage.

use super::*;
use crate::{CycleScreenedMapHierarchy, solve_projected_pcg_traced_with_workspaces};

fn problem(levels: usize) -> ThreeWayProblem {
    let tuples: Vec<_> = (0..levels).map(|i| [i as u32; 3]).collect();
    ThreeWayProblem::from_observations([levels; 3], &tuples, &vec![1.0; levels]).unwrap()
}
fn injected() -> MultiwayError {
    let source = Vec::<u8>::new().try_reserve(usize::MAX).unwrap_err();
    MultiwayError::WorkspaceAllocation { context: "injected outer reservation boundary", source }
}

#[test]
fn every_outer_growth_boundary_preserves_old_state_on_error_and_unwind() {
    let owner = problem(2);
    let larger = problem(16);
    let hierarchy = CycleScreenedMapHierarchy::from_maps(owner.clone(), vec![], 1.0e-12).unwrap();
    let options = PcgTraceOptions { max_iterations: 4, ..PcgTraceOptions::default() };
    let next_options = PcgTraceOptions { max_iterations: 64, ..options };
    let rhs = vec![1.0; owner.dimension()];
    // Six coefficient growth reservations, one trace reservation, one projection delegation.
    for unwind in [false, true] {
        for fail_at in 0..8 {
            let mut outer = PcgTraceWorkspace::try_new(&owner, options).unwrap();
            let mut inner = hierarchy.application_workspace().unwrap();
            let expected = solve_projected_pcg_traced_with_workspaces(
                &owner, &rhs, &hierarchy, options, &mut outer, &mut inner).unwrap().to_owned();
            let before = format!("{outer:?}");
            let before_bytes = outer.retained_bytes().unwrap();
            let mut reached = 0;
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                outer.prepare_with(&larger, next_options, || {
                    let index = reached;
                    reached += 1;
                    if index == fail_at {
                        assert!(!unwind, "injected preparation unwind");
                        return Err(injected());
                    }
                    Ok(())
                })
            }));
            assert_eq!(reached, fail_at + 1);
            if unwind {
                assert!(outcome.is_err());
            } else {
                assert!(matches!(outcome.unwrap(), Err(MultiwayError::WorkspaceAllocation { .. })));
            }
            assert_eq!(format!("{outer:?}"), before);
            assert!(outer.retained_bytes().unwrap() >= before_bytes);
            outer.validate(&owner, options).unwrap();
            assert!(outer.validate(&larger, next_options).is_err());
            let recovered = solve_projected_pcg_traced_with_workspaces(
                &owner, &rhs, &hierarchy, options, &mut outer, &mut inner).unwrap().to_owned();
            assert_eq!(recovered, expected);
            outer.try_prepare_for(&larger, next_options).unwrap();
            outer.validate(&larger, next_options).unwrap();
        }
    }
}

#[test]
fn each_fresh_preparation_failure_can_be_retried() {
    let problem = problem(8);
    let options = PcgTraceOptions::default();
    for fail_at in 0..8 {
        let mut outer = PcgTraceWorkspace::new();
        let mut reached = 0;
        assert!(outer.prepare_with(&problem, options, || {
            let index = reached;
            reached += 1;
            if index == fail_at { Err(injected()) } else { Ok(()) }
        }).is_err());
        assert_eq!(reached, fail_at + 1);
        assert!(outer.projected_rhs.is_empty());
        assert!(outer.solution.is_empty());
        assert!(outer.residual.is_empty());
        assert!(outer.preconditioned.is_empty());
        assert!(outer.direction.is_empty());
        assert!(outer.applied.is_empty());
        assert!(outer.samples.is_empty());
        assert!(outer.projection.is_none());
        outer.try_prepare_for(&problem, options).unwrap();
        outer.validate(&problem, options).unwrap();
        outer.prepare_with(&problem, options, || panic!("prepared storage must not reserve or rebind")).unwrap();
    }
}
''')
new('crates/multiway-mg/tests/payload_admission.rs', r'''//! Prepared capacity and owner rejection, with no change to numerical policy.

use multiway_mg::{CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace,
    FactorAggregation, MultiwayError, PcgPayloadBudget, PcgTraceOptions, PcgTraceWorkspace,
    ThreeWayProblem, prepared_map_pcg_payload_report,
    solve_projected_pcg_traced_with_payload_budget};

fn hierarchy(levels: usize) -> CycleScreenedMapHierarchy {
    let tuples: Vec<_> = (0..levels).flat_map(|i| (0..levels).flat_map(move |j|
        (0..levels).map(move |k| [i as u32, j as u32, k as u32]))).collect();
    let problem = ThreeWayProblem::from_observations([levels; 3], &tuples,
        &vec![1.0; tuples.len()]).unwrap();
    let mut maps = Vec::new();
    let mut counts = [levels; 3];
    while counts[0] > 1 {
        let map = FactorAggregation::consecutive_halving(counts).unwrap();
        counts = map.coarse_counts();
        maps.push(map);
    }
    CycleScreenedMapHierarchy::from_maps(problem, maps, 1.0e-12).unwrap()
}

#[test]
fn strict_zero_rhs_rejects_empty_and_foreign_nested_storage_without_mutation() {
    let hierarchy = hierarchy(4);
    let other = self::hierarchy(4);
    let options = PcgTraceOptions::default();
    let rhs = vec![0.0; hierarchy.finest_problem().dimension()];
    let mut outer = PcgTraceWorkspace::try_new(hierarchy.finest_problem(), options).unwrap();
    let mut empty = CycleScreenedMapHierarchyWorkspace::new();
    let before = format!("{outer:?}");
    let budget = PcgPayloadBudget { maximum_bytes: usize::MAX, additional_live_bytes: 0 };
    assert!(matches!(solve_projected_pcg_traced_with_payload_budget(
        &hierarchy, &rhs, options, &mut outer, &mut empty, budget),
        Err(MultiwayError::WorkspaceNotPrepared { .. })));
    assert_eq!(empty.retained_bytes().unwrap(), 0);
    assert_eq!(format!("{outer:?}"), before);
    let mut foreign = other.application_workspace().unwrap();
    assert!(!foreign.is_prepared_for(&hierarchy));
    let foreign_before = format!("{foreign:?}");
    assert!(solve_projected_pcg_traced_with_payload_budget(
        &hierarchy, &rhs, options, &mut outer, &mut foreign, budget).is_err());
    assert_eq!(format!("{foreign:?}"), foreign_before);
    assert_eq!(format!("{outer:?}"), before);
    foreign.try_prepare_for(&hierarchy).unwrap();
    assert!(foreign.is_prepared_for(&hierarchy));
    let result = solve_projected_pcg_traced_with_payload_budget(
        &hierarchy, &rhs, options, &mut outer, &mut foreign, budget).unwrap();
    assert!(result.converged());
    assert_eq!(result.iterations(), 0);
}

#[test]
fn retained_inactive_capacity_not_minimum_sizes_controls_admission() {
    let large = hierarchy(8);
    let small = hierarchy(2);
    let large_options = PcgTraceOptions { max_iterations: 256, ..PcgTraceOptions::default() };
    let options = PcgTraceOptions { max_iterations: 8, ..large_options };
    let mut outer = PcgTraceWorkspace::try_new(large.finest_problem(), large_options).unwrap();
    let mut inner = large.application_workspace().unwrap();
    outer.try_prepare_for(small.finest_problem(), options).unwrap();
    inner.try_prepare_for(&small).unwrap();
    assert!(inner.is_prepared_for(&small));
    let rhs = vec![1.0; small.finest_problem().dimension()];
    let report = prepared_map_pcg_payload_report(&small, &rhs, options, &outer, &inner, 0).unwrap();
    assert!(report.outer_workspace_bytes > PcgTraceWorkspace::required_bytes(small.finest_problem(), options).unwrap());
    assert!(report.hierarchy_workspace_bytes > small.workspace_required_bytes().unwrap());
    let minimum = small.retained_payload_report().unwrap().total_bytes().unwrap()
        + PcgTraceWorkspace::required_bytes(small.finest_problem(), options).unwrap()
        + small.workspace_required_bytes().unwrap() + core::mem::size_of_val(rhs.as_slice());
    let before = format!("{outer:?}{inner:?}");
    assert!(matches!(solve_projected_pcg_traced_with_payload_budget(
        &small, &rhs, options, &mut outer, &mut inner,
        PcgPayloadBudget { maximum_bytes: minimum, additional_live_bytes: 0 }),
        Err(MultiwayError::PayloadBudgetExceeded { .. })));
    assert_eq!(format!("{outer:?}{inner:?}"), before);
    let result = solve_projected_pcg_traced_with_payload_budget(
        &small, &rhs, options, &mut outer, &mut inner,
        PcgPayloadBudget { maximum_bytes: report.total_bytes().unwrap(), additional_live_bytes: 0 }).unwrap();
    assert!(result.converged());
}

#[test]
fn shared_identity_is_distinct_from_value_equality_and_owned_copy_payload() {
    let first = hierarchy(4);
    let clone = first.clone();
    let independent = hierarchy(4);
    assert!(first.finest_problem().shares_storage_with(clone.finest_problem()));
    assert!(!first.finest_problem().shares_storage_with(independent.finest_problem()));
    assert_eq!(first.finest_problem(), independent.finest_problem());
    let a = first.retained_payload_report().unwrap();
    let b = clone.retained_payload_report().unwrap();
    assert_eq!(a.shared_problem_bytes, b.shared_problem_bytes);
    assert_eq!(a.total_bytes().unwrap(), a.shared_problem_bytes + a.exclusive_bytes().unwrap());
    assert!(a.shared_problem_bytes >= first.finest_problem().retained_payload_bytes().unwrap());
    assert!(a.terminal_bytes > 0);
    assert!(a.aggregation_bytes > 0);
}
''')
# Add the fifth API to the existing independent pre-change bitwise comparison.
replace('crates/multiway-mg/tests/pcg_storage.rs', '''            assert_reference(&prepared, &expected);
            certify(&problem, &rhs, &prepared);''', '''            assert_reference(&prepared, &expected);
            let budgeted = multiway_mg::solve_projected_pcg_traced_with_payload_budget(
                &hierarchy, &rhs, options, &mut storage, &mut hierarchy_storage,
                multiway_mg::PcgPayloadBudget { maximum_bytes: usize::MAX, additional_live_bytes: 0 },
            )?.to_owned();
            assert_reference(&budgeted, &expected);
            certify(&problem, &rhs, &budgeted);
            certify(&problem, &rhs, &prepared);''')
# Expand the already isolated cross-platform allocator executable, keeping previous gates intact.
replace('crates/multiway-mg/tests/workspace_allocations.rs', 'mod pcg_allocations;', 'mod pcg_allocations;\n#[path = "support/payload_allocations.rs"]\nmod payload_allocations;')
replace('crates/multiway-mg/tests/support/pcg_allocations.rs', "fn equal(view: PcgTraceResultRef<'_>, expected: &PcgTraceResult)", "pub(super) fn equal(view: PcgTraceResultRef<'_>, expected: &PcgTraceResult)")
replace('crates/multiway-mg/tests/support/pcg_allocations.rs', '''    let before = GLOBAL.stats();
    drop(outer);''', '''    super::payload_allocations::check(hierarchy, &rhs, options, &mut outer, &mut inner, &expected)?;
    let before = GLOBAL.stats();
    drop(outer);''')
new('crates/multiway-mg/tests/support/payload_allocations.rs', r'''//! Actual allocation/lifetime checks at the strict prepared payload boundary.

use super::{GLOBAL, Result, no_events};
use multiway_mg::{CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace,
    MultiwayError, PcgPayloadBudget, PcgTraceOptions, PcgTraceResult, PcgTraceWorkspace,
    prepared_map_pcg_payload_report, solve_projected_pcg_traced_with_payload_budget};
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
    assert_eq!(inventory.shared_problem_bytes, hierarchy.retained_payload_report()?.shared_problem_bytes);
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
        hierarchy, rhs, options, outer, inner,
        PcgPayloadBudget { maximum_bytes: total - 1, additional_live_bytes: extra }).unwrap_err();
    let events = GLOBAL.stats() - before;
    no_events(events);
    assert!(matches!(error, MultiwayError::PayloadBudgetExceeded { required, budget }
        if required == total && budget == total - 1));
    assert_eq!(format!("{outer:?}{inner:?}"), snapshot);
    let before = GLOBAL.stats();
    let error = solve_projected_pcg_traced_with_payload_budget(
        hierarchy, rhs, options, outer, inner,
        PcgPayloadBudget { maximum_bytes: usize::MAX, additional_live_bytes: usize::MAX }).unwrap_err();
    let events = GLOBAL.stats() - before;
    no_events(events);
    assert!(matches!(error, MultiwayError::WorkspaceSizeOverflow { .. }));
    assert_eq!(format!("{outer:?}{inner:?}"), snapshot);
    for _ in 0..4 {
        let before = GLOBAL.stats();
        let result = solve_projected_pcg_traced_with_payload_budget(
            hierarchy, black_box(rhs), options, outer, inner,
            PcgPayloadBudget { maximum_bytes: total, additional_live_bytes: extra })?;
        let events = GLOBAL.stats() - before;
        no_events(events);
        super::pcg_allocations::equal(result, expected);
    }
    assert_eq!(prepared_map_pcg_payload_report(hierarchy, rhs, options, outer, inner, extra)?, report);
    assert_eq!(copy, *expected);
    let before = GLOBAL.stats();
    drop(copy);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, extra);
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    println!("payload dimension={} shared={} exclusive={} working_set={} extra_live={} clone_allocate_release=exact report/reject/solve_allocations=0",
        rhs.len(), inventory.shared_problem_bytes, inventory.exclusive_bytes()?, total, extra);
    Ok(())
}
''')
append('docs/ISSUE5_PAYLOAD_ADMISSION.md', r'''

### Outer preparation failure coverage in this increment

The outer workspace's production setup path has a private local no-op callback at
six coefficient growth reservations, trace growth, and delegated projection
preparation. Unit tests substitute a deterministic error or unwind before each of
those eight boundaries and verify old lengths, contents and component binding are
still usable, despite permitted capacity growth. Fresh partially reserved storage
can be retried; fully prepared same-owner storage reaches none of the callbacks.
There is no retained hook, global allocator override or unsafe code. Tests inject a
real `TryReserveError` value obtained from an impossible tiny-vector reservation;
they do not cause the OS allocator itself to fail. Failure inside the delegated
incidence preparation, every recursive hierarchy allocation, and arbitrary external
allocator failures are NOT exhaustively injected by these tests.
''')
print('Permanent admission and failure tests assembled.')
