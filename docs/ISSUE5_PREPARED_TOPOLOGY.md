# Issue 5: prepared immutable incidence topology

## Qualified boundary

`PreparedThreeWayTopology` in `multiway-incidence` retains only canonical tuple
keys, deterministic observation groups and structural component metadata. It
contains no weights, diagonals, operator images, smoother parameters or factors.
Existing weighted-problem and hierarchy constructors remain the numerical path;
this increment does not implement a weight frame or numerical hierarchy replay.

Raw observations are sorted by (tuple key, original row index), with an in-place
unstable sort on that total order. The retained arrays are original-row-to-tuple,
grouped original rows and tuple-group offsets. Every group's rows are increasing
in original input order, matching the order needed by subsequent deterministic
compensated duplicate aggregation. This increment does not itself sum weights.

The collapsed-input constructor requires strictly increasing unique tuple keys.
It rejects duplicates or unsorted input rather than silently changing the caller's
weight layout. Its source-to-tuple mapping is implicit identity: it allocates no
observation-group arrays and makes no claim to recover original physical rows.
Both constructors reject empty inputs, invalid codes/counts and unused levels.
Extra null directions beyond structural factor shifts are not rank-certified here.

## Owner identity and invalidation

`PreparedTopologyBinding` borrows the exact immutable prepared owner. Copies and
shared references are cheap and allocation-free. Rust prevents a token outliving
its owner, and prevents moving/replacing that owner while its binding remains live.
Independent equal builds have different bindings. There is no owning Clone, global
ID generator, hash-equality shortcut, unsafe pointer storage, Arc allocation or
serialized token. The identity is an in-process symbolic lifetime boundary only.

`validate_input_layout` checks factor cardinalities and every coded source tuple
in its original row order. It catches changed support, row count, factor codes and
reordering of distinguishable rows. It cannot detect external observation IDs,
reordering of identical coded rows, changed factor meanings or estimator sample
policy when the supplied tuples are identical. Such changes must build a new owner.
Retained-sample, aggregation-map and numerical-weight generations are still separate
future responsibilities; passing the structural check authorizes no numerical reuse.

`scatter_tuple_values_into` is a symbolic copy into original source order. It
checks the supplied binding and both dimensions before output mutation. It preserves
all bit patterns and is intentionally not a positive-weight or finite-value check.
No output or previous owner is modified by a failed constructor or rejected scatter.

## Construction and memory

All new arrays use checked `try_reserve_exact` at explicit setup boundaries. The
prepared owner has no hidden infallible identity allocation. A private per-call
no-op callback allows tests to inject failure or unwinding before each reservation;
no callback, allocator override or mutable global is retained in production state.

`setup_payload_bound(counts, input_count, source)` conservatively charges the maximum
unique-tuple count (input count), maximum component count (coefficient dimension),
retained arrays and component root-label scratch together. This is deliberately an
upper bound even for invalid inputs with unused levels. The `_with_budget`
constructors reject when that requested-array bound exceeds the budget, before any
array reservation. Equality is admitted. The bound may reject a duplicate-heavy
input whose eventual retained representation would be smaller.

This is not OS memory admission: caller input storage, inline descriptors, sorting
stack, allocator metadata/rounding and excess allocator-provided capacity are outside
the requested-array bound. All individual arrays and combined arithmetic are checked.
`retained_payload_bytes` instead counts actual retained capacities after construction.
The isolated allocator gate reconciles construction allocations minus released setup
scratch to that report and destruction to the same payload, with no Arc exclusions.

Component discovery is one shared fallible array routine used by prepared topology
and the existing `IncidenceComponents::from_topology`. Minimum-root union and
first-global-vertex label ordering are unchanged. Compressed roots become labels in
the same array, reducing scratch duplication; component-size storage reserves its
exact discovered count. The old public component-only API remains infallible and
may still construct isolated components; it unwraps reservation errors and keeps
its existing Arc projection identity. Prepared construction does not call that
infallible wrapper. Projection arithmetic, weighted kernels and solver recurrences
are untouched. Historical exact byte figures may differ because metadata capacities
are now sized by discovered component count; reports use actual capacities.

## Tests and limitations

Require exact-head Rust 1.85 Actions and all permanent scientific gates. New tests
cover exact row groups and bit-preserving scatter, canonical collapsed layout,
wrong owner/dimensions/source layout, exhaustive nonempty supports on a 2x2x2 domain
against independent graph search, empty legacy component topology, unused levels,
setup overflow and one-byte-short budgets, shared concurrent references and two
compile-fail binding lifetime examples.

Error and unwind injection covers all seven observation-construction reservations
and all four collapsed-construction reservations, including shared component setup.
It is deterministic boundary injection, not an OS allocator returning null. Older
owners and source rows remain reusable; partial arrays are ordinary RAII-owned Vecs.
The existing 12-configuration allocation executable adds eight prepared-source cases
and verifies first/64-repeated symbolic calls, static rejection and exact array
allocation/destruction accounting. Existing complete-cycle/PCG gates stay intact.

No weight-generation safety, numerical replay, setup peak-RSS cap, production solver
choice, timing win, fresh holdout or closure of issue #5 is implied. Next: symbolic
coarse-tuple/pair-edge grouping, explicit validated weight frames and complete numeric
replay with map quality screening in separately reviewed increments. ADR 0002 remains
unchanged.
