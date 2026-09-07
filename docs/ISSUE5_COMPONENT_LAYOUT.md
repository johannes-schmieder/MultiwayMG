# Flat component layout and temporary recoding

M6c adds a structural partition view for component-local hierarchy construction.
`PreparedComponentLayout` borrows one exact prepared topology and retains only
flat source permutations. `PreparedComponentRecoding` separately owns the
inverse codes needed to materialize local tuple keys. The inverse can be dropped
before numerical frames, smoothers, terminals and Krylov workspaces are built.
This increment does not build an automatic numerical hierarchy or choose a
parallel execution policy.

## Representation and lifetime

A connected input uses implicit source ranges, with zero partition and inverse
arrays. Gather/scatter becomes a checked direct slice copy. Disconnected input
uses three retained arrays:

- One vector containing coefficient and tuple offset tables, each of length C+1.
- Original factor-local level IDs, stored as u32, ordered by component, factor,
  then ascending original ID. Total global V may exceed u32; each source ID
  still names one factor and is representable under the existing topology limit.
- Original canonical tuple IDs, grouped by component in ascending source order.
  IDs use u32 if the largest index fits, including E=2^32 on a 64-bit target;
  otherwise they use native usize. The representation dispatches outside loops.

One C-usize cursor is reused across all factors and tuple placement, then
released. Stable counting placement is linear in V+E+C; no tuple sort,
per-component vector-of-vectors, copied numerical weights, local topology or
reference-counted owner token is needed to construct the partition. Original
observation groups stay in the original topology.

The separate recoder builds a 4V-byte inverse in one stable pass over the level
rows. Each source tuple then maps to local factor codes in constant time.
Recoding is monotone within each factor, so the local keys remain canonical and
unique without sorting. Keeping this inverse temporary saves 4V retained bytes
relative to storing it permanently in the partition. This is a lifetime choice,
not a measured end-to-end speedup.

## Memory admission

For disconnected input, requested exclusive partition bytes are:

```
4V + I*E + 2*sizeof(usize)*(C+1)
```

I is 4 or sizeof(usize). Partition setup adds one C-usize cursor. Recoder setup
adds 4V while the partition and original owner remain live. Thus on the qualified
64-bit targets the partition is 4V+I*E+16(C+1), setup adds 8C, and later recoding
adds a separately released 4V. Connected input requires zero new arrays at both
stages.

Each constructor first checks every array size and complete sum, then admits
original topology actual capacities, current partition when applicable, all
requested new arrays and caller-declared other live payload. Actual retained
capacities are reported and checked after construction. Callers must charge
accumulating local roots, output-key buffers and numerical frames in the other
live state. Allocator-provided new excess capacity is not predicted by requested
bounds. Inline roots, stack, allocator metadata and process RSS are excluded;
these are array-payload admission bounds, not process-memory quotas.

Prepared topology already guarantees every level is used and every exact
component contains all three factors. The new layout relies on that invariant;
it does not rescan support or silently delete unused levels. An initial draft
test mistakenly tried to construct an unused-level owner and was rejected by
the existing constructor. The corrected test asserts that upstream boundary.

## Operations and verification

A checked component view exposes source IDs and generic coefficient/tuple
permutations. All source/destination dimensions and component IDs are validated
before writes. Operations preserve arbitrary copied bits, including signed zero
and NaN payloads; pure permutation does not claim numerical weight validation.
Invalid factor/component views return None, and recoding rejects an out-of-range
component with a typed error. Explicit identity checks reject even value-equal
reconstructed source owners. Borrowed views and recoders cannot survive moving
or dropping their structural owners.

Tests cover connected/ragged/interleaved layouts, separated singleton components,
original duplicate observations and disconnected Latin extra nullity. Every
source level and canonical tuple occurs exactly once, each ID row is ascending,
local keys are sorted unique, and scatter/gather composes to identity. Local
weighted Gramians agree bitwise with restrictions of the original operator.
Malformed outputs remain unchanged. Narrow and forced-wide small layouts agree;
last-index and checked-size overflow boundaries require no huge allocations.

Private tests inject errors and unwinding at all four partition reservations
and the single recoder reservation, followed by successful recovery. Exact and
minus-one budgets include source, retained partition and changing additional
live state. The isolated allocator test verifies three retained partition
arrays, one released cursor, one separately released inverse, no reallocations,
allocation-free first/repeated permutation and recoding, exact byte release,
and a usable partition after the inverse is dropped. Compile-fail tests enforce
the source/layout/view/recoder lifetime boundaries.

## Remaining integration

Materialized disconnected local roots, if chosen, still consume recoded keys,
component metadata and potentially gathered numerical values alongside the
original owners. This primitive does not establish zero-copy numerical solves
or the campaign's 25% RSS target. Small dense components may permit direct
assembly from these permutations; larger components may benefit from canonical
local roots. Compare complete construction and repeated-RHS costs before
selecting a representation.

Next build component-local structural prefixes with one-transition provisional
weight replay, admit numerical terminals, and screen the actual recursive cycle
from the bottom upward. Completed components must leave deeper work. Full
original-operator certification remains mandatory after any component-local
solver integration. M6 and competitive qualification remain open.


Final Rust 1.85 formatting, strict Clippy, all/minimal tests and warning-free
rustdoc pass, as do 90 Python validators. Release component tests in all/minimal
configurations, private failure/width/budget checks, complete existing solvers
and allocator checks, and 120 actual protocol comparisons pass. Initial failed
unused-level-fixture and redundant-lifetime lint logs are preserved alongside
passing final checks. Complete existing-solver regression follows frozen source;
it does not invoke or time the new component partition.


Frozen source `1b4baae` passes the [four-artifact existing-solver regression](../benchmarks/results/2026-09-07/prepared-layout-component-layout/README.md):
7,920 processes, 72,300 certified measured columns and exact agreement with
every corresponding M6b input/numerical/work/payload/layout record. No errors,
rejections, timeouts, RSS failures or measured retries occurred. Source/PR
workflows passed; independently revalidated binary/raw copies and every archived
member are preserved. Final evidence-head CI and PR56 merge remain. These
supplied-map measurements do not invoke or time component layout construction.
