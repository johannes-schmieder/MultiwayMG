# Issue 5: reuse hierarchy scratch throughout traced PCG

## Entry points and ownership

`solve_projected_pcg_traced_with_hierarchy_workspace(problem, rhs, hierarchy,
options, &mut workspace)` uses one caller-owned
`CycleScreenedMapHierarchyWorkspace` for every preconditioner application in a
solve. The same workspace may be reused for later right-hand sides, including
with independently constructed hierarchies. A hierarchy's numerical state must
still correspond to its own weights: anonymous scratch reuse is not numerical
weight replay.

The existing generic `solve_projected_pcg_traced` remains available. Both public
functions call one private `FnMut`-based core. The ordinary closure calls
`Preconditioner::apply`; the workspace closure calls
`hierarchy.apply_with_workspace`. There is no second PCG recurrence, mutable
hierarchy, `RefCell`, lock, or global pool.

```rust,no_run
use multiway_mg::{
    CycleScreenedMapHierarchy, MultiwayError, PcgTraceOptions, ThreeWayProblem,
    solve_projected_pcg_traced_with_hierarchy_workspace,
};

fn repeated_solves(
    problem: &ThreeWayProblem,
    hierarchy: &CycleScreenedMapHierarchy,
    right_hand_sides: &[Vec<f64>],
) -> Result<(), MultiwayError> {
    let mut workspace = hierarchy.application_workspace()?;
    for rhs in right_hand_sides {
        let result = solve_projected_pcg_traced_with_hierarchy_workspace(
            problem,
            rhs,
            hierarchy,
            PcgTraceOptions::default(),
            &mut workspace,
        )?;
        // Downstream acceptance still requires the submitted-operator certificate.
        let _ = result;
    }
    Ok(())
}
```

## Compatibility contract

The refactor changes only preconditioner dispatch. Option validation, RHS and
preconditioner dimension validation, initial projection, recurrence arithmetic,
true-residual replacement, structural projections, stopping decisions, trace
storage, and work accounting retain their original order.

A zero projected RHS returns without applying the preconditioner or preparing
an empty workspace. Invalid options and dimensions fail before touching
workspace capacity. Numerical breakdowns propagate normally; retained scratch
remains reusable after a rejected solve. The final nonconverged iteration's
preconditioner application is intentionally preserved, including its counter.

Regression tests compare every solution coefficient and residual sample
bit-for-bit, as well as iteration count, convergence status, RHS projection
norm, Gramian count, preconditioner count, and final relative residual. They
cover repeated signed/scaled right-hand sides, zero RHS, an iteration limit,
invalid inputs, finite-input metric overflow, and successful reuse after a
breakdown. Successful repeated solves are also independently checked against
the submitted original Gramian.

## Scope boundary

The hierarchy traversal workspace and its checked retained-byte accounting are
documented in [ISSUE5_WORKSPACES.md](ISSUE5_WORKSPACES.md). This slice retains
that scratch throughout the solve but does not yet reuse outer PCG vectors or
trace storage. The existing MAP and structural projection internals also still
allocate. It is not a claim of allocation-free complete solves, full peak-memory
admission, numerical generation safety, or improved CMG routing economics.

Rust 1.85 GitHub Actions and the unchanged scientific gates are the authoritative
validation. ADR 0002 and the already-frozen evidence remain unchanged.
