# Issue 5: caller-owned hierarchy traversal workspace

## Scope

`CycleScreenedMapHierarchyWorkspace` reuses the recursive V-cycle's vector
scratch. `CycleScreenedMapHierarchy::apply_with_workspace` and the existing
`Preconditioner::apply` enter one production recurrence; the latter creates a
temporary workspace. Hierarchy construction, weighted operators, map selection,
projections, smoother arithmetic, terminal arithmetic, tolerances, and ADR 0002
are unchanged.

This is a bounded traversal slice, **not a claim that the complete cycle or
Krylov solver is allocation-free**. Existing symmetric MAP and structural
projection calls still allocate internally. Removing those allocations belongs
to subsequent issue-5 slices, as do prepared topology, weight generations,
repeated-RHS panels, and full peak-lifetime accounting.

## API

```rust,no_run
use multiway_mg::{CycleScreenedMapHierarchy, MultiwayError, Preconditioner};

fn apply_repeatedly(
    hierarchy: &CycleScreenedMapHierarchy,
    right_hand_sides: &[Vec<f64>],
) -> Result<(), MultiwayError> {
    let mut workspace = hierarchy.application_workspace()?;
    let mut out = vec![0.0; hierarchy.dimension()];
    for rhs in right_hand_sides {
        hierarchy.apply_with_workspace(rhs, &mut out, &mut workspace)?;
    }
    let retained_heap_bytes = workspace.retained_bytes()?;
    let retained_vectors = workspace.retained_buffer_count();
    let _ = (retained_heap_bytes, retained_vectors);
    Ok(())
}
```

`CycleScreenedMapHierarchyWorkspace::new()` is initially empty.
`application_workspace()` eagerly and fallibly prepares the traversal layout,
without running numerical operators. Both APIs permit subsequent reuse on
independently constructed hierarchies of different dimensions, depths, or
weights. Reuse applies only to anonymous scratch: it never reuses a numerical
hierarchy for different weights.

## Ownership and failure contracts

The arena contains one finest-result buffer and seven buffers per nonterminal
level. A level leases a disjoint prefix and lends its tail to its recursive
child. Buffers never leave the arena. Rust's lexical mutable borrows enforce
non-aliasing and stack-ordered release on success, error, or panic unwind; there
is no thread-local state, global pool, lock, interior-mutability adapter, or
unsafe code. Independent caller-owned workspaces can use a shared immutable
hierarchy concurrently.

Preparation resizes each active buffer. Every lease initializes its contents
before use. Inactive buffers from larger previous hierarchies remain retained
and counted. This is deliberately different from
`StructuralProjectionWorkspace`: there is no component-partition identity in
anonymous traversal scratch.

Both public vector dimensions are checked before workspace preparation or
caller-output mutation. Preparation uses fallible reservations and checked
size arithmetic. A failed reservation may leave some newly acquired workspace
capacity retained; it cannot alter caller output. Numerical work writes a
workspace-owned result, copied into caller output only after the full cycle
succeeds. A panic does not lose scratch buffers; callers that catch an unwind
can reuse the workspace.

## Memory boundary

`retained_bytes()` returns a checked capacity-based sum of:

1. `buffers.capacity() * size_of::<Vec<f64>>()`, and
2. the capacities of **all** retained f64 buffers, including inactive ones.

It excludes the inline workspace object, allocator-internal metadata, immutable
hierarchy data, caller input/output, and temporary storage allocated by the
unchanged MAP/projection implementations. It is not a whole-solve peak-memory
report. Buffer count measures retained vectors, not allocator calls. For an
unchanged prepared layout the traversal buffers keep their pointers and
capacities; this does not imply that nested routines perform no allocations.

## Regression evidence

The tests compare both public entry points bit-for-bit against a test-only
transcription of the allocating recurrence at `b4f41fd`, on all eight
already-revealed issue-3 recursive fixtures and several signed/zero RHS scales.
They also cover independently built hierarchies, changing weights and sizes,
terminal-only application, stable repeated-portfolio capacities, short/long
input and output rejection, reusable scratch after rejection, poisoned scratch,
checked oversized requests, descriptor-inclusive accounting, independent
concurrent workspaces, and a forced nested private-frame unwind.

The frozen fixtures are compatibility regressions, not a new holdout or a
routing calibration. Rust 1.85 GitHub Actions remains the authoritative gate,
including the existing numerical and scientific jobs. No local Rust run is
used to qualify this change.
