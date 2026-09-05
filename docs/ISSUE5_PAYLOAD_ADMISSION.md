# Issue 5: resident payload inventory and prepared-solve admission

## Scope and ownership

This increment inventories one **already built fixed MAP hierarchy** and admits
one **already prepared traced-PCG working set**. It does not claim total peak
memory or solve the separate hierarchy-build allocation-admission problem.

`ThreeWayProblem::retained_payload_bytes` includes the objects behind its topology
and component Arcs, their tuple/label/component-size vector capacities, and its
weight, square-root-weight and diagonal Arc slices. An ordinary problem clone
shares all five backing allocations. `shares_storage_with` checks those allocation
identities; value equality is not storage identity or a numerical replay contract.

`CycleScreenedMapHierarchy::retained_payload_report` separates shared problem
payload from exclusive problem/map/smoother descriptor arrays, aggregation parent
arrays and dense terminal factors. Smoother problem clones are not counted again.
Every level is strictly smaller and independently constructed; the inventory checks
the private shape and smoother-sharing invariant rather than silently assuming a
future representation still satisfies it. The terminal matrix uses actual storage
capacity, not its logical matrix dimension. A retained construction plan is outside
this boundary and must be charged separately by a caller that keeps it alive.

One cloned hierarchy shares the level problems but owns another set of descriptors,
parent arrays and terminal factors. When both remain live, charge shared problem
payload once and each hierarchy's exclusive payload once. Independent equal builds
are distinct allocations. These are not general graph-deduplicating reports for an
arbitrary collection of partially shared objects.

## Strict prepared execution

`prepared_map_pcg_payload_report` validates options and RHS dimensions, outer
workspace lengths/component identity/full trace capacity and every active recursive
scratch length and binding. It reports actual capacities, including unused and
inactive scratch, rather than substituting minimum `required_bytes` estimates.
Read-only `CycleScreenedMapHierarchyWorkspace::is_prepared_for` never rebinds or grows.

`solve_projected_pcg_traced_with_payload_budget` accepts the hierarchy, RHS, options,
both workspaces and `PcgPayloadBudget { maximum_bytes, additional_live_bytes }`.
It checks the report and rejects before mutation when the total exceeds the budget;
equality is admitted. The original shared borrowed-solve path is then used without
changing arithmetic, counters, stopping criteria or certification responsibilities.
The numerical operator is the hierarchy's finest problem. Prepared scratch cannot
grow during a successful solve. Even a zero RHS requires both workspaces prepared
for this strict API; existing APIs retain their previous automatic-prepare behavior.

The report charges the hierarchy once, both complete workspace payloads, the RHS
slice, and caller-declared disjoint extra live bytes. The outer workspace already
contains its solution and full trace budget. Borrowed views add no heap payload;
`PcgTraceResult::retained_payload_bytes` charges an explicitly retained owned result.
Use the extra-live category for other RHS columns, input capacity beyond the slice,
retained result copies, old-state overlap and other independent retained buffers.
Do not charge an alias of already-counted storage twice. The library cannot discover
or verify unreported external allocations.

## Exclusions and lifetime boundaries

All counts are checked **payload bytes**, not allocation footprints or RSS. They
exclude inline root objects, Arc control headers/alignment padding (including old
identity tokens held by inactive scratch), allocator metadata/rounding, stack,
construction/repreparation transients and unreported external storage. Heap-resident
descriptor arrays and topology/component objects behind Arc ARE included.

The strict API performs admission after construction and preparation, before
numerical mutation. It does not promise to prevent an out-of-memory failure during
setup, enforce a malloc quota, or cap OS memory. Existing minimum workspace queries
are useful for planning but cannot replace retained-capacity checks after setup.
Numerical-error strings may allocate; static budget/binding/dimension rejection and
successful prepared execution do not. An external original-operator certificate may
need additional storage and remains a separate charged phase.

## Qualification and remaining work

Require the complete Rust 1.85 Actions suite, original numerical regressions and
existing 12-configuration allocator matrix. Instrument hierarchy-clone allocation
and destruction against exclusive payload, owned copies against their reports,
strict report/admission/solve regions, equality and one-byte-short budgets, external
live charges, overflow, inactive scratch, wrong owners and post-rejection reuse.

Boundary-failure tests must distinguish deterministic injected errors from real OS
allocation failures. Full setup-lifetime/allocator admission, exhaustive failure
injection, other hierarchy/pair routes and LSMR remain separate increments. No
numerical replay, fresh holdout, speedup or production-routing change is implied;
ADR 0002 and frozen scientific evidence remain unchanged.



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
