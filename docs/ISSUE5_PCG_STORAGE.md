# Issue 5: reusable outer traced-PCG storage

## Numerical and ownership boundaries

`PcgTraceWorkspace` owns six coefficient vectors (projected RHS, solution,
residual, preconditioned residual, direction and Gramian image), one
component-bound structural-projection workspace, and the entire trace budget.
It retains no weights, factors or numerical operator. Every solve starts from
zero and reinitializes its scratch before consumption.

`PcgTraceWorkspace::try_new(problem, options)` and `try_prepare_for` reserve at
explicit fallible setup boundaries. Borrowed-result solves validate options,
public dimensions, scratch dimensions, component identity and full trace capacity
before mutation; they do not grow or rebind outer storage implicitly. Ordinary
problem clones share identity. Independent builds require explicit preparation,
even if dimensions and component counts agree. Repreparing scratch is not
replaying a numerical hierarchy under new weights.

Two new entry points return `PcgTraceResultRef`:

- `solve_projected_pcg_traced_with_workspace` uses any `Preconditioner`; its own
  allocations remain that preconditioner's responsibility.
- `solve_projected_pcg_traced_with_workspaces` uses both outer storage and the
  existing caller-owned recursive MAP hierarchy workspace. The latter retains
  its automatic preparation semantics. Prepare both before claiming a
  zero-allocation complete solve.

The view borrows solution and trace storage, prevents concurrent mutable reuse,
and exposes only a completed successful result (including iteration-limit
results with `converged=false`). `to_owned()` explicitly allocates copies of both
arrays. There is no hidden result clone or trace growth in a borrowed solve.
Existing owned-return convenience APIs create local outer storage and move the
solution and trace out on success. Their output ownership and numerical behavior
remain intact; their allocation pattern is not promised unchanged.

```rust,no_run
use multiway_mg::{CycleScreenedMapHierarchy, MultiwayError, PcgTraceOptions,
    PcgTraceWorkspace, solve_projected_pcg_traced_with_workspaces};
fn repeated(hierarchy: &CycleScreenedMapHierarchy, rhs: &[Vec<f64>]) -> Result<(), MultiwayError> {
    let problem = hierarchy.finest_problem();
    let options = PcgTraceOptions::default();
    let mut outer = PcgTraceWorkspace::try_new(problem, options)?;
    let mut inner = hierarchy.application_workspace()?;
    for right in rhs {
        let result = solve_projected_pcg_traced_with_workspaces(
            problem, right, hierarchy, options, &mut outer, &mut inner)?;
        let _ = (result.solution(), result.samples(), result.converged());
        // Independently certify the candidate against the submitted operator.
    }
    Ok(())
}
```

## One recurrence and failure behavior

All four public entry points enter one private FnMut-based recurrence. The
residual's existing allocating wrapper is replaced with `residual_into`, and
projection calls use retained scratch. Arithmetic order, compensated dot products,
FMA expressions, residual replacement, tolerance decisions and finite-value guards
are unchanged. In particular, the original final nonconverged preconditioner call
at the iteration limit remains counted. A zero projected RHS still records one
sample and uses no Gramian or hierarchy applications. Invalid initial numerical
states do not prepare empty hierarchy scratch.

Setup checks `max_iterations + 1`, the worst-case two-Gramian-per-iteration count,
and all requested byte arithmetic. Unrepresentable budgets now fail closed even
for a RHS that would have returned immediately. The full budget is reserved up
front rather than being grown until convergence. This is an intentional storage
contract, not a change in numerical stopping tolerances.

Numerical errors can overwrite scratch but publish no result view; the next solve
reinitializes it. No scratch leaves its owner during error or unwind. Invalid
options, dimensions, bindings and trace capacities fail before scratch mutation.
Setup failure may retain additional capacities, but all reservations precede
publication of new dimensions and binding. Full allocator-failure injection and
whole-ownership-graph admission are separate engineering tasks.

## Memory and qualification

`required_bytes` counts minimum exclusive coefficient/projection/trace payload;
`retained_bytes` uses actual capacities including unused storage. Trace payload
is included, not hidden as external output. Inline descriptors, shared identity
control blocks, allocator overhead, caller input and immutable hierarchy state
are excluded. Add the hierarchy workspace's disjoint payload separately. These
reports are not process RSS or total lifetime peak memory.

The independent test-only PCG and finite-check modules preserve the allocating
solver at `78f13b1`. Numerical code is not shared with the new recurrence. Regression
checks cover all four APIs, every solution/trace bit and counter on the eight
revealed recursive fixtures, separate original-operator certification, zero RHS,
iteration limits, explicit owned-copy lifetime, option/dimension/binding/budget
rejection, changed-size preparation, non-finite/overflow errors, injected generic
preconditioner errors/unwinds and independent concurrent workspaces. A compile-fail
doctest protects borrowed-result lifetime discipline.

The existing isolated allocation executable now measures complete borrowed solves
in `tests/support/pcg_allocations.rs`: 17 hierarchy cases plus an allocation-free
generic-preconditioner control. It asserts zero allocations, reallocations and
deallocations on the first prepared solve, eight repeated signed/zero/scaled RHS
solves, the iteration-limit path, explicit repreparation and recovery after an
invalid numerical state. Solution and trace pointers remain stable. Static
binding/dimension/budget rejection allocates nothing. Error-message construction
for numerical failures is separately measured and is not claimed allocation-free.
Fresh outer storage allocation and release must equal its exclusive byte report;
an explicit `to_owned` copy must make the separately charged two array allocations.
Input construction, setup, reference solves, logging and external independent
certification are outside the measured solve region. The pre-existing complete
MAP-cycle allocation tests remain intact.

The unchanged three-platform debug/release, minimal/all-feature Actions workflow
runs this expanded executable. Full Rust 1.85/scientific Actions and an exact-diff
review are required. No speedup, complete LSMR workspace, total peak
memory, numerical weight replay, default-solver change or production readiness is
claimed. ADR 0002 remains in force; revealed fixtures are not a fresh holdout.
