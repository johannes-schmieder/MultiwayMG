# Serial Schwarz action boundary (M6o)

M6n PR67 merged as `172f0d8521e7a59073ca68769533e7b6afa009b6` with clean main
matching reviewed tree `893f24d6ce84e5e2bfb06273b4a598ddd9993616`. All 36 final
and 18 post-merge jobs pass. It preserves 45,360 exact prior records and 432,000
measured certificates. Development automatic all-image still trails grouped
MAP by about 1.80x; the terminal/admission requirement remains open.

## Dependency implementation

Within PR4 merged as `69110db2733a29bce7b6151466ece95c6249713f`, reviewed tree
`b8f17126eeb750e6775a7bede3b7975dd0bca153`, after all 13 source/PR jobs passed. All four post-merge jobs also pass.
Both runtime pins move together. No runtime package versions or owner wire
layouts change. Existing solver APIs, local factor kernels and complete automatic
routing remain unchanged. The new serial API is a prerequisite for a later
terminal policy, not an admitted terminal or a performance promotion.

`SchwarzPreconditioner::try_serial_workspace(limit)` borrows its immutable owner
and allocates one flat array: V transaction-result entries and two maximum-local
scratch slices, including ground/cover augmentation. Checked requested payload
is `8*(V+2*Smax)`. Domains and requested bytes validate before one fallible
reservation. Actual retained capacity is separately queryable. The limit
excludes allocator excess/headers, inline descriptors, the owner, previous pooled
buffers and local-factor internals; it is not total memory admission.

Apply checks shapes/current local dimensions, then visits stored domains in a
fixed order using existing gather/solve/scatter arithmetic with inner parallelism
disabled. Immutable core indices validate once when binding. The new outer path
uses no Rayon, atomics, locks or per-worker V-vectors. Output is copied from the
transaction buffer only after every domain succeeds. Static typed errors leave
scratch/output intact; a local failure can dirty scratch, preserves output and
allows later reuse. The owner cannot be replaced or outlived. Generic local
solvers still control their own threads, allocations and error strings.

`within::SerialPreconditionerWorkspace` exposes the private concrete solver
through `OperatorMut`; diagonal actions need no workspace array. It borrows a
fixed within preconditioner. A future prepared MultiwayMG adapter must also
establish the exact external topology/numerical-frame binding and full admission;
this API does not silently confer those additional guarantees.

## Qualification and limits

Eight fork-local Rust1.85 required/release groups pass. Independent dense and
ordered-cancellation references cover partition weights, overlap, uncovered and
empty domains, augmentation, shape/budget/size rejection, late local failure,
recovery, lifetime, reservation failure/unwind and changed generic scratch.
Sixteen real dense/approximate-factor configurations cover unit/dyadic weights,
Latin, nested, disconnected interleaved and unbalanced inputs. Tests compare
one-worker pooled arithmetic, exact repeatability across caller pools and
independent concurrent workspaces. A 33,792-coordinate existing local fixture
exercises serial action above its internal parallel thresholds.

The generic allocator control retains 288,016 bytes at V=12,000. Concrete outer
payloads are 288/360/632 bytes by fixture. First/repeated actions, static rejection
and exact destruction pass in debug/release on Linux/macOS/Windows; generic
allocation-free local-error/recovery also passes. Provider ZIP digests and every
raw file are preserved in [the qualification evidence](../benchmarks/results/2026-09-19/serial-schwarz-workspace/README.md).
Initial allocator-harness failures remain recorded: delayed setup-thread activity
contaminated process-wide counts. Explicit OS-thread joins before the first action
fix isolation without warming the action or changing production arithmetic.

The downstream test exercises the new action through gated LSMR for 160 independent
RHS across widths 1/2/4/8/16/17/32, including legitimate zero RHS and additional
nullity. It checks original weighted normal equations independently and compares
fitted values with the dense reference. It does not require identical coefficients
when nullity permits alternatives. The permanent three-platform debug/release
allocation workflow also runs this test in its all-feature configurations.

The old automatic benchmark paths do not call the new API. Keep their unchanged
scientific/protocol and CI complete-smoke regressions; this increment makes no
new full-process performance comparison or Mac-development speedup claim.

## Required next boundaries

The pinned approx-chol factor still can allocate permutation scratch, and its
nested capacities/build peak are opaque. Current within setup remains parallel
and uses unchecked factor/Schur reservations. A separate reviewed dependency
change must expose capacities, permutation scratch, explicit construction
execution and checked fill/fallback/overlap bounds before terminal admission.

A read-only [liveness audit](../benchmarks/results/2026-09-19/serial-schwarz-workspace/permutation-scratch-liveness.md)
identifies existing space for permutation scratch: Direct can reuse the dead
reduced RHS after copying it to the solution; Cover can reuse the dead solution
tail while keeping its embedded cover values in the RHS buffer. Qualify these
disjoint regions through a new factor scratch API before use. This suggests no
increase in the current outer capacity is needed; it is not implemented here.

Full numerical range is a separate gate. An external audit retains a valid
connected V=12/rank7 support with range leakage 0.252389/unit and 0.176210/dyadic
weights after two-sided structural projection. The action is symmetric with
positive quotient energy in that audit, but fails the accepted full-range gate.
All audited positive/negative cases and original recipes are preserved. This
does not invalidate independently certified existing LSMR results. A future
terminal needs additional treatment or conservative eligibility/rejection;
structural factor-shift projection alone is insufficient.

Continue the [sparse-terminal sequence](ISSUE5_SPARSE_TERMINAL_BOUNDARY.md).
M6 admission/integration, M7 bounded CPU pools, M8 independent panels, M9 changing-
weight replay and M10 untouched-holdout qualification remain open. No default,
calibration, holdout or competitive claim changes here.

All eight downstream required groups (including 135 Python tests) and eight
release scientific/protocol groups pass locally at the paired pins. Both actual
protocol builds preserve 1,125 exact reference/profile pairs, 225 legacy and900
layout comparisons, with malformed/CLI rejection checks. Source/PR CI and guarded
actual-main verification remain before integration delivery.
