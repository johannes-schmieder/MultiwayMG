# Incremental structural hierarchy admission

M6b adds `PreparedHierarchyBuilder` in `multiway-incidence`. It consumes already
proposed factor maps and publishes the existing immutable
`PreparedHierarchyTopology` only when `finish` consumes the builder. This is an
unscreened structural primitive. No numerical weights, smoother, dense factor,
automatic policy or performance-default change is introduced.

The existing all-at-once constructor remains useful for supplied maps. The new
builder lets a controller propose the next map from the current accepted level,
reject an oversized transition, and retain the preceding structure. Rebuilding
or copying that prefix is unnecessary. The coarse-map helper already moves its
unique canonical keys into their prepared owner, so this stage introduces no
second key copy.

## Storage and admission

The caller supplies explicit integer limits for transitions, total unique tuples
and total coefficients over all accepted levels including the fine level, plus
an explicit strict-dimension-reduction choice. There is no default automatic
policy. The eventual controller must choose and freeze these limits; reducing
coefficient dimension alone does not bound tuple complexity.

For maximum transition count D, reserve D `FactorAggregation` descriptors and D
transition descriptors exactly once after checked sizing and initial admission.
Append moves accepted map/transition owners into that capacity. No descriptor
reallocation or shrinking occurs. Unused reserved capacity remains charged in
actual retained bytes. D=0 borrows the fine owner and requires no descriptor
arrays. Inline roots, allocator metadata and stack are excluded from array
payload and are not represented as zero process overhead.

Every append receives a fresh `PreparedHierarchyBudget`. Its required live
requested-array bound is:

```
fine topology actual capacities
+ accepted hierarchy actual capacities (including all descriptor capacity)
+ submitted map actual parent capacities
+ caller-declared other live payload
+ PreparedCoarseTupleMap::setup_payload_bound(current level, submitted map)
```

The borrowed fine owner is counted once. Current/provisional numerical frames,
old generations and unrelated live objects must be included by the caller; the
submitted map must not be counted again there. All new requested arrays are
admitted before their first reservation. Integer overflow, transition limits,
layout mismatch and coefficient-complexity limits reject before new structural
allocation. Actual coarse unique tuple count is checked after bounded structural
construction and before any numerical setup. A final actual-capacity admission
check precedes publication. As in existing constructors, new allocator excess
capacity is not predicted by the requested bound, and this is not a quota or RSS
cap.

Failed append drops the consumed map and temporary transition, preserving the
accepted levels, totals and retained bytes. Its typed inline report records the
submitted source sizes, proposed dimension, actual coarse tuple count when
known, requested peak when sized, and whether structural construction was
entered. Submitted tuples are not mislabeled as exact visited/operator work.
The largest admitted attempted bound remains in the builder's setup report,
including released failures. Rejected preallocation bounds do not become an
admitted peak. Neither field is a measured timing or process-memory result.
There is no unbounded allocation for failure history.

Current-level borrows exclude mutable append and consuming finish. A proposed
candidate can relinquish its immutable frame borrow by consuming it into the
structural map; the controller then drops provisional frame borrows before
appending. No self references, reference-counted owner token or unsafe lifetime
extension are needed.

## Verification

Private tests cover both initial descriptor reservations and all ten coarse
construction reservations under injected errors and unwinding, then successful
recovery. They verify unchanged accepted owners, descriptor pointers, payload
and logical totals. Other cases cover exact/minus-one budgets, additional live
state changing at append, integer overflow, shape/reduction/transition and
coefficient limits, tuple-complexity rejection, component-crossing rejection,
explicit identity admission and zero-transition publication. Compile-fail
examples enforce the current-level/frame borrow boundary.

Scientific tests compare incremental and supplied construction on all eight
historical recursive fixtures (not the campaign holdout), three current-weight
generations and 32 RHS including zero columns. Canonical keys, parent maps,
merge-group ordering, component correspondences, replay weights/diagonals,
complete PCG and certificate-gated LSMR coefficients and full solve/work reports
match exactly; every compared solve certifies. Ragged disconnected/relabelled
and Latin structural-nullity controls are checked separately.

The isolated allocator process verifies two initial descriptor allocations,
ten reservations per append, no reallocations, exact retained accounting,
complete release after tuple/budget/overflow rejection, allocation-free finish,
and zero arrays for the fine-only case. This is a measured allocation contract
on the qualified platforms, not a claim that descriptor ABI sizes are portable.

Required Rust 1.85 formatting, strict Clippy, all/minimal tests, warning-free
rustdoc and 90 Python validators pass. Release scientific/allocator/private
failure checks and 120 complete-solver protocol comparisons also pass. The
complete existing-solver benchmark regression follows source freeze and is
tracked in the development ledger. Existing
supplied-map timings do not invoke or measure the new builder. Its full setup
cost and usefulness require the later automatic driver and frozen development
comparison.

## Next integration

Partition disconnected components with bounded flat permutations, let finished
components leave deeper work, and construct local structural prefixes. Replay
current numerical values, admit bounded terminals, then screen the actual
recursive cycle from the bottom up before publishing an automatic numerical
hierarchy. Fixed sparse terminals and any candidate-policy decisions still need
separate correctness, resource and complete-cost qualification.


Frozen source `6a212e0` passes the [four-artifact existing-solver regression](../benchmarks/results/2026-09-07/prepared-layout-incremental-hierarchy/README.md):
7,920 processes, 72,300 certified measured columns and exact agreement with
every corresponding M6a input/numerical/work/payload/layout record. No errors,
rejections, timeouts, RSS failures or measured retries occurred. Source and PR
workflows passed; independently revalidated binary/raw artifacts and every
archive member are preserved. Final evidence-head CI and PR55 merge remain.
These supplied-map measurements do not invoke or time the incremental builder.

Qualification correction (2026-09-07): the cyclic Latin square has only structural
nullity. See the [explicit rank audit and new nested controls](ISSUE5_PROVISIONAL_REPLAY.md).
This corrects the former additional-nullity description; raw frozen evidence is
unchanged. The new nested regression is part of M6d, not the older measured source.
