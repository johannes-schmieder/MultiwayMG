# Issue 5: complete caller-owned MAP hierarchy workspace

## Scope

`CycleScreenedMapHierarchyWorkspace` now retains traversal, MAP, structural
projection and dense-terminal modal scratch. `apply_with_workspace` and ordinary
`Preconditioner::apply` enter one production V-cycle recurrence; ordinary apply
creates a temporary workspace. Hierarchy construction, map selection, weights,
numerical operation order, tolerances, and ADR 0002 are unchanged.

This supersedes the traversal-only boundary of PR #22. The prepared complete
MAP hierarchy application is allocation-free. It does **not** make the outer
PCG/LSMR recurrence, trace/result creation, setup, other hierarchy types, or
pair-CMG/within routes allocation-free. See the measured gate below.

## API and preparation

```rust,no_run
use multiway_mg::{CycleScreenedMapHierarchy, MultiwayError, Preconditioner};
fn repeated(hierarchy: &CycleScreenedMapHierarchy, rhs: &[Vec<f64>]) -> Result<(), MultiwayError> {
    let mut workspace = hierarchy.application_workspace()?;
    let mut out = vec![0.0; hierarchy.dimension()];
    for right in rhs {
        hierarchy.apply_with_workspace(right, &mut out, &mut workspace)?;
    }
    let _ = (hierarchy.workspace_required_bytes()?, workspace.retained_bytes()?);
    Ok(())
}
```

`new()` is empty. `application_workspace()` eagerly and fallibly prepares all
scratch. `workspace.try_prepare_for(&hierarchy)` is an explicit setup boundary.
The existing `apply_with_workspace` also prepares after checking both public
vector dimensions, preserving automatic cross-instance and cross-size reuse.
For an already prepared layout this full entry point allocates nothing, even on
its first numerical application.

Traversal vectors and terminal modal storage are anonymous. Per-level projection
subworkspaces are explicitly prepared for the current component owner; direct
projection and MAP applications still reject incompatible bindings rather than
silently rebinding. Existing problem clones share component identity. A new
hierarchy does not have to share identity to reuse the outer workspace: setup
refreshes the semantic subworkspaces first. No weights, factors or numerical
operators are stored in the workspace. Scratch reuse is not weight replay.

## Ownership and errors

The traversal arena retains one finest-result vector and seven vectors per
nonterminal level. Per-level operator storage retains one projection workspace,
and MAP scratch where needed. Terminal modal scratch is retained separately.
Disjoint mutable tails lend vector and operator frames to recursive children.
No buffer is moved out of the arena during application, including on unwind.
There are no locks, global pools, unsafe code or mutable numerical operators.

All scratch is overwritten or initialized before consumption. Inactive levels,
former nonterminal MAP scratch and vector capacities remain retained and counted
when a later hierarchy is smaller. Preparation failure may leave partially grown
or rebound scratch; caller output is untouched, and the next successful
preparation restores all active lengths and identities. The complete numerical
result is copied to caller output only on success. Existing poisoned-vector,
forced-child-unwind, dimension-rejection and concurrent-workspace tests remain.

## Memory boundary

`workspace_required_bytes()` counts minimum exclusive fresh heap payload.
`retained_bytes()` counts actual capacities, including inactive storage and both
heap descriptor arrays. `traversal_retained_bytes()` and
`operator_retained_bytes()` give disjoint contributions to that total.
`retained_buffer_count()` deliberately preserves the old traversal-only count;
it is neither a total vector count nor an allocator-call count.

These reports exclude the inline workspace object, shared identity tokens'
reference-count metadata, allocator overhead, immutable hierarchy and caller
arrays. They are not process RSS or full solver peak-lifetime accounting. The
allocator gate checks that fresh workspace bytes requested from the allocator
match this exclusive retained payload; existing shared identities are cloned
without allocating. Workspace construction can fail; full malloc-failure
injection remains a separate gate from deterministic capacity-overflow tests.

## Regression and actual allocation gates

The existing hierarchy tests compare against the allocating pre-change recurrence
on all eight already-revealed recursive fixtures. Independent pre-change MAP and
dense references introduced in PR #26 protect nested arithmetic. Traced-PCG
bitwise samples/counters, spectrum, complete-cycle and scientific gates remain.

`tests/workspace_allocations.rs` is a harness-free executable run by Cargo in a
separate process, not a test sharing allocator counters with concurrent tests.
A pinned **dev-only** `stats_alloc` wrapper instruments the system allocator;
repository source still forbids unsafe code. Live allocation and reallocation
positive controls detect a disabled instrument, and ordinary hierarchy apply
must allocate. Measured regions exclude fixture/setup/output construction and
printing, and use black-box inputs/outputs. They check zero allocation,
reallocation and deallocation for direct prepared MAP/terminal operations and
complete hierarchy first, 64-repeat, and explicitly reprepared applications.
The 17 hierarchy cases cover eight revealed fixtures, independently reconstructed
nonuniform weight variants, and a disconnected terminal-only control. All fresh
workspace setup bytes must match the reported exclusive retained payload;
dropping that workspace while its hierarchy lives must release exactly that
payload. Reprepare allocator traffic is reported separately, and signed/scaled
and zero RHS checks run outside the fixture-construction boundary.

The permanent read-only Actions workflow runs minimal/all features, debug/release,
on Linux, macOS and Windows with Rust 1.85 and archives exact-head metadata and
logs. This is a regression contract, not a speed comparison or fresh holdout.
