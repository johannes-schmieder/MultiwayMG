# Issue 5: validated immutable numerical weight frames

## Implemented boundary

`ThreeWayWeightFrame` borrows an exact `PreparedThreeWayTopology` and owns one
immutable numerical generation: canonical positive finite tuple weights, their
positive finite square roots, positive finite weighted degrees, and per-component
minimum/maximum tuple weights and degrees. Construction never sorts tuple keys,
rediscovers components or copies symbolic topology. Getters and validation reports
are allocation-free. The frame owns its numerical arrays independently of the
submitted slice, which may be changed or discarded after successful construction.

`WeightFrameInput` explicitly distinguishes original `Observations`, canonical
`Tuples`, `UnitObservations` and `UnitTuples`. Lengths are checked, not used to infer
meaning. Observation variants require an observation-prepared topology; a collapsed
source has no original physical-row groups. Tuple variants work with either source
kind and always refer to canonical unique tuples. Unit observation input includes
duplicate multiplicity; unit tuple input does not. Implicit ones avoid allocating an
input ones-vector, but the frame's retained weight/root arrays are materialized.

Observation duplicate sums use the existing `CompensatedSum` in increasing original
row order within each prepared group. The degree recurrence is extracted unchanged
into one private kernel shared with existing `ThreeWayProblem` construction. It
visits canonical tuples and factors in the original order. Neither solver/kernel
application paths nor floating-point stopping rules change. The ordinary problem
constructor keeps its prior acceptance behavior; only this new frame path adds
explicit finite/positive checks on every published derived quantity.

## Numerical acceptance and deliberate limits

Explicit inputs reject nonpositive or nonfinite values before array reservation;
`InvalidWeight.tuple_index` names the submitted observation or canonical-tuple row
according to the explicit input variant. Duplicate overflow rejects with its tuple
key. Individually finite tuple totals can still overflow a degree shared by several
tuples; the frame rejects and reports the factor/level. Square roots and all component
extrema must also pass finite/positive validation before a frame is published.

No positive weight is thresholded, deleted, rescaled or floored. Positive subnormals
and a singleton `f64::MAX` weight are retained exactly. Min/max diagnostics avoid
forming an overflow-prone ratio. A valid frame does NOT ensure reciprocal degrees,
operator images, norm products or solves are representable, well-conditioned or
identified. Safe scaled norm-bound reporting, rank diagnostics and downstream
operator certification remain separate work; this first frame stores no inverse
degrees, floating-point norm bound, numerical factors or hierarchy quality decision.

## Numerical identity versus symbolic identity

`frame.topology_binding()` names the prepared symbolic owner. `frame.binding()`
returns `WeightFrameBinding`, which borrows this exact immutable numerical frame.
Independent frames have different bindings even if every numerical value is equal.
Two frames under the same topology share its symbolic binding, not numerical identity.
An old frame may remain valid and live alongside a new frame; it is not globally
revoked. Any future derived factorization must bind to the specific frame used to
construct it and reject a different frame, rather than infer validity from topology.

There is no owning Clone, in-place reweighting API, global generation counter, hash
identity, hidden heap token, unsafe state, interior mutability or serialization ID.
Bindings cannot outlive a frame or remain usable after moving/replacing its owner;
frames cannot outlive topology. Immutable references support concurrent use. Three
compile-fail examples protect these lifetime boundaries. `copy_weights_into` checks
exact topology and frame ownership plus output length before touching caller output.
Raw getter slices remain inspection views, not downstream-generation certificates.

Numerical slices cannot reveal external sample IDs, reordered rows or changed factor
meanings. Callers must still preserve the declared layout and rebuild topology when
those semantics change. Existing hierarchy/solver APIs do not accept these frames
and are NOT retroactively protected by the new numerical binding. No production
coarse-weight or pair-conductance replay is included in this increment.

## Construction memory and failure behavior

All five new arrays use checked fallible reservation: weights, roots, degrees,
temporary degree corrections, and component ranges. `setup_payload_report` counts
new requested arrays (including scratch), actual borrowed-topology payload, the
submitted numerical slice by length, and caller-declared other live payload.
Unit inputs contribute no input-slice bytes. Additional live payload must include,
for example, an older frame kept alive during replacement and unused input-vector
capacity. Equality admits and one byte short rejects before any new reservation.
Integer overflow rejects. Reports check layout/size, not numerical input validity.

This is a conservative live REQUESTED-ARRAY ledger, not exact allocator peak memory
or an RSS cap. Some requested arrays have disjoint lifetimes. Allocator headers,
rounding/excess capacity, inline objects and stack are excluded. A submitted slice
aliasing an already charged older frame is conservatively counted twice; no general
alias deduplication is attempted. Existing PCG/hierarchy payload reports do not
silently discover these external objects. After construction, `retained_payload_bytes`
counts actual exclusive capacities, excluding borrowed topology and released scratch.
Charge each distinct old/new numerical owner separately and shared topology once.

Failures publish no partial frame. Source topology, submitted weights and old frames
are unchanged; owned partial arrays are dropped normally. Local private callbacks
exercise every reservation boundary with errors and caught unwinds, but these are
not OS allocator-null tests. Actual duplicate/degree-overflow cleanup is separately
measured by the isolated allocator executable.

## Qualification

Require exact-head Rust 1.85 GitHub Actions and all existing permanent scientific
checks, plus the unchanged 12-configuration allocator matrix: Linux/macOS/Windows,
debug/release, minimal/all features. Independent test-only compensated references
check duplicate totals and degrees without calling the shared production kernel;
valid values also match fresh ordinary construction bit-for-bit. Unit variants,
input ownership, disconnected extreme weights, malformed values, duplicate/degree
overflow, topology/frame mismatch, budget/overflow, concurrency and lifetimes are
covered. No tolerance or frozen scientific evidence is weakened.

The isolated allocator process adds 24 frame cases (four datasets, explicit/unit
observation input, and explicit/unit tuple input for raw/collapsed sources). It checks
first/64-repeated report/identity/copy calls and static rejection for zero allocations,
reallocations and deallocations. Five setup allocations minus released correction
scratch must equal the retained report; destroying a frame releases exactly that
payload while topology and an old frame remain alive. Sixteen rebuild/drop cycles
per case balance their individual ledgers. Numeric failures must release all partial
arrays, not merely recover at the end of a process. Previous allocation tests remain.

## Next

Replay coarse weights and pair conductances through the stored deterministic groups,
with explicit parent-frame and map provenance. Then integrate generation-bound
numerical hierarchies, rebuilding every numerical quantity and re-screening map
quality. The old pair constructors' ordinary accumulation versus compensated replay
must be compared explicitly. Owning multi-level prepared containers, pair-component
metadata, scaled norms, full construction-lifetime admission, LSMR storage, panels,
pools and performance/changing-weight qualification remain open. ADR 0002 is unchanged;
no fresh holdout, timing win, changed-weight solve or closure of issue #5 is implied.
