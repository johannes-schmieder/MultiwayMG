# Issue 5: symbolic coarse-tuple and pair-edge maps

## Boundary

`PreparedCoarseTupleMap` represents one transition from a borrowed prepared
source through a borrowed `FactorAggregation`. `PreparedPairEdgeMap` represents
one explicit factor pair of a borrowed source. Both retain canonical mapped keys,
fine-tuple-to-group IDs, grouped fine tuple IDs and offsets, but no numerical
weights, degrees, conductances, smoother coefficients or terminal factors.

Grouping is over unique canonical fine tuples, not original observations. A
single grouping builder orders by (mapped key, canonical fine tuple ID), so every
merge group preserves increasing fine tuple order for a future deterministic
compensated reduction. No reduction or weight-frame API is provided here.

The unchanged `FactorPair` definition now lives in `multiway-incidence`, keeping
this structural layer independent of CMG/within. The prior CMG-feature research
re-export in `multiway-mg` remains available; no numerical pair route changes.

## Owners and component preservation

Coarse maps borrow both source and exact factor aggregation. `validate_for` and
symbolic scattering reject a source from another owner or a value-equal cloned
aggregation. Borrowing prevents replacement or movement while a derived map is
used; no global counter, hash identity, unsafe state or owning clone is added.
Pair maps check the exact source plus the explicit factor pair. Raw getter slices
are inspection views, not independent generation-bearing certificates.

Factor-local aggregation alone is insufficient: two fine levels in distinct
incidence components must not have the same parent. Construction checks this in
linear vertex work using coarse-vertex ownership scratch and rejects crossings.
The coarse topology is finished through the same existing component routine,
with keys moved rather than copied. Both directions of component correspondence
are retained and checked as a bijection, including possible component renumbering.

Identity maps and pure relabelings are structurally legal. A successful symbolic
constructor is NOT hierarchy admission, proof of dimension reduction, numerical
rank, smoother quality or a license to reuse maps after arbitrary weight changes.
The owned coarse topology can be borrowed for another transition; this is not yet
an owning multi-level prepared-hierarchy container.

Pair endpoints are lexicographically sorted factor-local `[left, right]` keys.
Graph-local endpoints are `[left, left_count + right]`, with dimension
`left_count + right_count`. The explicit endpoint writer and both symbolic
scatters validate owners/pair/dimensions before any output mutation. Values are
copied bit-for-bit, not validated as weights. A pair graph can have more connected
components than its parent three-way incidence. This API does not supply pair
components, gauges or rank and must not substitute three-way labels for them.

## Allocation and memory

Every new array uses checked fallible reservation. The budgeted constructors
check conservative ADDITIONAL requested-array setup payload before allocating.
Fine tuple count bounds unique mapped keys; coarse dimension bounds component
arrays. Coarse bounds include merge arrays, component construction scratch,
coarse-vertex ownership scratch and both correspondence arrays. Pair bounds
include endpoint keys and grouping arrays. Equality is admitted; one byte short
rejects. The bound may exceed actual requirements and excludes caller/borrowed
source and aggregation payload, inline descriptors, sorting stack, allocator
metadata and excess allocator-provided capacity. It is not OS/RSS admission.

Retained reports count actual exclusive capacities. Coarse maps include owned
coarse topology, groups and both component maps; pair maps include keys/groups.
Borrowed source/maps add no exclusive heap allocation but must still be charged
once in a caller's full live-memory ledger. No hierarchy payload-budget API is
silently extended to count these external objects.

All partial construction state is ordinarily owned and dropped on error/unwind.
Private local callbacks test reservation boundaries without retained hooks or
global state. This is boundary injection, not an actual OS allocator returning
null. Existing infallible FactorAggregation constructors remain outside this new
fallible construction boundary.

## Qualification

Require exact-head GitHub Actions on Rust 1.85, full existing scientific gates,
and the Linux/macOS/Windows x debug/release x minimal/all-feature allocation
matrix. Add independent key/group references, fresh Galerkin comparisons,
source/map/pair mismatch rejection, component renumbering/crossing, chaining,
borrowed lifetime tests, all ten coarse and four per-pair reservation failures
and unwinds, plus construction/destruction accounting and allocation-free reads,
scattering, endpoint writes and static rejection.

## Next

Explicit numerical-weight frames need their own validated generation identity.
Then implement compensated fine/coarse/pair weight replay and generation-safe
numerical hierarchies, rebuilding every weight-dependent quantity and re-screening
map quality. Existing solver numerics, original-operator certification, frozen
scientific evidence and ADR 0002 remain unchanged. No speedup, production-routing
change, new holdout or closure of issue #5 follows from this symbolic increment.
