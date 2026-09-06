# Issue 5: compensated coarse-weight and pair-conductance replay

## Implemented boundary

`CoarseWeightReplay` borrows an exact `PreparedCoarseTupleMap` and an exact
`ThreeWayWeightFrame`, and owns a new coarse numerical frame. `PairConductanceReplay`
borrows an exact `PreparedPairEdgeMap` and source frame, and owns new pair
conductances and weighted degrees. Neither constructor sorts keys, discovers
components, copies symbolic topology, or reuses old numerical arrays/factors.
Both require the source frame to name the map's exact prepared topology.

These are immutable one-transition numerical records, not a complete numerical
hierarchy. There is no consuming extractor, mutation API, hidden token allocation,
global generation counter, hash authorization or interior-mutability adapter.
Old and new frames may coexist. A replay remains valid for its original map and
frame; it must reject substitution by an independently constructed equal map,
equal-value frame or changed-weight frame. Borrowing protects the lifetimes of
both owners. Coarse frames are exposed by immutable borrow, allowing another
coarse or pair replay to borrow them while their provenance record remains live.
Future factorizations must themselves borrow/bind their numerical input; existing
hierarchy and solver APIs are not retroactively protected by these new records.

## Numerical protocol

Both replay types use one group-reduction function and the existing compensated
accumulator. Each stored merge group is visited in increasing canonical fine-tuple
order, not original observation order. Raw observation duplicates have already
been collapsed by the source weight frame. Every group total must remain positive
and finite. The operation is linear work in stored fine tuples plus mapped groups.

Coarse weights move into the same private finishing routine as ordinary weight
frames, with no extra weight copy. It validates weights and recomputes square roots,
compensated weighted degrees and component extrema. Public frame construction still
uses the same five reservations and the same arithmetic order. No solver or
ordinary problem application recurrence changes.

Coarse references use fresh `FactorAggregation::coarsen` on the already collapsed
fine problem. Its compensated order matches the stored groups bit-for-bit. Directly
collapsing all raw observations to a deep coarse level is NOT the floating-point
reference: regrouping changes rounding, even though the real-arithmetic identity
is the same. Every chained transition starts from its represented parent weights.

Pair conductances use compensated reduction. The legacy private research pair
builder instead uses ordinary addition. An explicit arithmetic regression records
`1e16 + 1 + 1`: the ordinary protocol yields `1e16`; compensated replay yields
`10000000000000002`. That test documents the arithmetic difference; it does not
execute the private legacy builder. No legacy pair route or frozen tolerance is
changed or quietly declared bitwise equivalent.

Pair degrees are recomputed from the represented edge conductances in canonical
edge order. They are not copied from the fine frame: two-stage rounding can make
them differ. A regression exhibits finite differing degrees, and another scales
that construction to the overflow boundary: all fine degrees remain finite, but
a grouped pair degree becomes non-finite. Pair construction rejects before
publishing a record. Finite fine weights/degrees alone are therefore insufficient.
No positive input is thresholded, floored, dropped or rescaled.

The pair edge writer emits `(left, left_count + right, conductance)`. The copy and
writer APIs validate exact map/frame identity and all dimensions before touching
caller output. They allocate nothing after construction. Getter slices are
inspection views, not downstream factorization or solve certificates.

## Memory and failure contracts

Both budgeted constructors use `WeightReplayPayloadBudget`. The setup report counts
actual direct source-topology payload, actual exclusive symbolic-map payload,
factor-aggregation parent arrays for coarse replay, actual direct parent-frame
payload, requested new numerical arrays/scratch and caller-declared other live
payload. Coarse topology is already included in the coarse map; parent weights are
not charged a second time as an input slice. Budget equality admits; one byte short
or integer overflow rejects before any reservation.

For coarse replay, new arrays are weights, roots, degrees, degree-correction scratch
and component extrema: five checked fallible allocations, with weights moved into
the finishing routine. For pair replay, new arrays are conductances, compensated
degree accumulators and final degrees: three checked fallible allocations. Retained
reports count actual exclusive capacities and exclude scratch and borrowed owners.
A new frame/replay still allocates at this explicit construction boundary; only
subsequent reads, reports, validation, copies and edge writes are allocation-free.

This is a live REQUESTED-ARRAY ledger, not allocator peak memory or process RSS.
It can include disjoint array lifetimes. Allocator headers/rounding/excess capacity,
inline descriptors and stack are excluded. Unlisted ancestors of a chained source,
older outputs, output buffers and other caller storage must be declared explicitly.
No recursive ownership discovery or general alias deduplication is attempted.
Shared direct topology is charged once; conservatively charging an ancestor's
complete report can double-count its embedded source topology. Existing PCG memory
admission does not automatically discover any of these external replay records.

Failure publishes no partial record and does not mutate topology, maps, parent
frames or older outputs. Private hooks inject errors and caught unwinds at all five
coarse reservations and all three pair reservations, including delegated frame
finishing. These are reservation-boundary tests, not OS allocator-null injection.
Public numerical-failure cleanup is measured separately: overflowing coarse totals,
coarse degrees and pair degrees release all partially allocated arrays.

## Qualification

Require exact-head GitHub Actions on Rust 1.85, all existing scientific/evidence
checks, and the unchanged Linux/macOS/Windows x debug/release x minimal/all-feature
allocator matrix. Independent BTreeMap/group/compensated references and fresh
Galerkin builds check weights, roots and degrees. Small-support enumeration,
nonmonotone aggregation, raw duplicates, unit inputs, exact-owner rejection,
chained replay, concurrent immutable use, signed pair-principal-block and Galerkin
matrix identities on dyadic weights, extremes and numerical overflow are covered.
No holdout or frozen numerical tolerance is changed.

The allocator executable adds eight coarse and 24 pair cases. It reconciles setup
minus released scratch with actual retained capacities, then destruction with those
retained bytes. First and 64 repeated read/report/copy/writer operations and static
rejection must make zero allocator calls. Sixteen changing-weight construction/drop
cycles per case balance the complete new-frame-plus-replay allocation ledger while
an older generation remains live. Separate numerical equivalence tests use smooth
and concentrated-shock sequences. These are small deterministic engineering tests,
not the full PPML stress or performance qualification matrix.

## Remaining boundary

No pair connected components/gauge, inverse-degree or norm guarantee, numerical rank
certificate, CMG/within reweighting, smoother or terminal replay, complete owning
hierarchy, map-quality re-screening, solve entry point or routing change is included.
The next increment must connect current numerical frames to matrix-free operators
and generation-bound numerical hierarchy construction while rebuilding every
weight-dependent quantity and retaining original-operator certification. Structural
map reuse still requires explicit deterministic quality admission before production
use. Full lifetime admission, LSMR storage, RHS panels/pools and timing/thread/full
changing-weight qualification remain open. ADR 0002 and issue #5 remain unchanged;
this increment is not a speedup claim or closure of the broader milestone.
