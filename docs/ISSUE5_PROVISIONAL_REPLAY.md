# One-transition provisional numerical replay

M6d supplies temporary numerical state for choosing the next structural map.
`PreparedProvisionalFrame` borrows an unpublished `PreparedHierarchyBuilder`,
reduces only its last accepted transition, and moves the resulting weights into
the existing frame-finishing implementation. It avoids replaying the accepted
prefix or copying the current weights at every structural extension. This is
an ownership and construction primitive, not an automatic solver or a passing
recursive-cycle quality decision.

## Lifetime and numerical contract

The first transition can borrow the original fine `ThreeWayWeightFrame` through
`ProvisionalWeightInput::Frame`. Its exact previous-level topology is checked;
the full external frame stays live and charged throughout the operation.
Later transitions consume `Owned(Vec<f64>)` in the previous level's canonical
tuple order. Shape and positive finite values are checked before reservation.
Raw weights carry no generation or replay certificate.

Shared compensated merge-group reduction produces one new coarse-weight array.
An owned predecessor is released immediately after reduction and its actual
input/output capacity check, before roots, degrees, corrections or component
extrema are allocated. Shared `ThreeWayWeightFrame::finish_with` consumes the
new weights and rebuilds all derived values. A provisional owner exposes a
borrowed frame for candidate selection. Consuming it into tuple weights releases
all three derived arrays and returns the same weight allocation; no frame or
binding survives that transfer. Compile-fail examples enforce the builder,
provisional-frame and numerical-binding lifetimes.

A driver chooses its initial map from the original frame, appends that map,
replays this one transition, chooses the next map, consumes the provisional
frame into weights, and only then appends again. It releases provisional state
before publishing structure and constructing the complete immutable numerical
hierarchy from the exact submitted fine frame. Existing `CoarseWeightReplay`
and `HierarchyWeightFrames` owner/generation contracts are unchanged. Passing
provisional setup does not replace complete replay, cycle screening or final
original-operator certification.

## Staged memory admission and failures

Let B be the actual fine topology plus retained structural builder arrays and
caller-declared other live state; I be the actual input payload; W the requested
new tuple-weight payload; and F the existing frame constructor's requested
new-frame-plus-scratch bound. The requested peaks are:

| Input | Reduction | Finishing | Admitted peak |
|---|---:|---:|---:|
| Borrowed frame | B + I + W | B + I + F | max of the two |
| Consumed weights | B + I + W | B + F | max of the two |

I uses full frame exclusive capacity for a borrowed frame and Vec capacity for
owned weights. The common frame sizing implementation supplies F; this code
does not duplicate its formula. Checked arithmetic and complete requested peak
admission precede the first allocation. Visible actual reduction capacities and
final retained payload are also checked. Caller other-live state must include
still-live original fine numerics on later owned-input transitions, accumulated
component owners, candidate scratch or other application state as appropriate.

These are requested array-payload bounds with actual-capacity checks, not RSS
quotas. They exclude allocator metadata, stack and inline roots. New allocator
excess capacity during finishing is not predicted or instrumented as an exact
instantaneous peak. Setup reports expose both stage bounds; submitted tuple
counts are sizes, not exact visited CPU work.

Failures carry a compact typed error, last entered stage, submitted input count
and optional admitted bound. Invalid owned values and budget rejection allocate
nothing. Reduction overflow and later degree overflow release all temporary
state. The immutable accepted structural prefix and original fine frame remain
usable. A later automatic driver must charge all attempted work and elapsed
cost before fallback; this primitive does not keep an unbounded failure history.

## Scientific and allocator gates

Private tests inject errors and unwinding at all five reservations with recovery,
exercise exact/minus-one stage budgets, oversized input capacity, checked-size
overflow, missing transitions, wrong owners and invalid weights. Numeric tests
separately overflow a merged tuple total and a coarse weighted degree; both
leave the accepted prefix unchanged and allow a subsequent valid replay.

The isolated allocator test requires five successful setup allocations (new
weights, roots, degrees, correction scratch, component extrema), one released
scratch allocation plus any consumed predecessor, and no reallocations. It
checks exact retained/released bytes, zero-copy consumption, zero-allocation
admission/invalid-value failures and complete release on both overflow stages.

Across the eight historical recursive fixtures and three weight generations,
successive provisional weights, roots, degrees and extrema match fresh coarsening
and complete replay. Candidate maps match fresh finite legacy proposals at every
accepted level. Full PCG and certificate-gated LSMR then certify three RHS per
generation, including zero. A separate disconnected nested-factor case asserts
rank and additional nullity, preserves components through two transitions, and
certifies the assembled original problem. The historical recursive fixtures are
not the campaign holdout.

## Rank-fixture correction (2026-09-07)

The full cyclic Latin-square fixture `[i,j,(i+j)%8]` has 24 coefficients, rank 22
and only the two structural factor-shift modes. Earlier M6a–M6c prose incorrectly
called it an additional-nullity fixture. Independent exact rational elimination
and an explicit dense pseudoinverse rank assertion establish the correction.
The earlier ragged-component controls include additional nullity, but the Latin
label did not establish that coverage.

New explicit controls use `[i,j,j]` with counts [4,3,3]: 10 coefficients, rank 6,
nullity 4, two additional directions. Adding a disjoint singleton with counts
[5,4,4] yields 13 coefficients, rank 7, two components, four structural modes
and two additional directions. Component tests construct both independent
nonconstant null vectors explicitly. Candidate, permutation and provisional/full
solver tests now include the nested structure. Frozen raw numerical evidence,
source hashes and historical benchmark measurements are unchanged; this is a
correction of the interpretation and an added regression gate.

## Remaining integration

Next assemble component-local construction and numerical owners, admit bounded
terminals and screen the actual recursive tails from the bottom upward. Completed
components must leave deeper work. Candidate coverage, terminal choices and the
complete automatic policy need declared development comparisons before policy
freeze. CPU parallelism, RHS panels, changed-weight reuse and competitive
qualification remain M7–M10. Scalar remains the default layout.

Required Rust 1.85 formatting, strict Clippy, all/minimal workspace tests and
warning-free rustdoc pass, as do all 90 Python validators. Release all/minimal
provisional/full-solver/allocator/component checks, private failure/budget gates
and all 120 actual protocol comparisons pass. Failed development scaffolding
logs and the independent rational-rank audit are preserved beside final logs.
The source is frozen before the unchanged supplied-map regression; no new
construction performance claim follows from that regression.


Frozen source `3e9028f` passes the [four-artifact supplied-map regression](../benchmarks/results/2026-09-07/prepared-layout-provisional-replay/README.md):
7,920 processes, 72,300 certified measured columns, 6,336 exact within-source
layout comparisons and exact agreement with all corresponding M6c input,
numerical, work, payload and layout records. No measured retries or failed gates
occurred. Source/PR workflows passed; raw/binary copies and every archive member independently revalidate. Seven
derived timing geomeans differ by one ULP under local recomputation; receipts
record these within the existing archival tolerance, with raw records exact. Final evidence-head CI and PR57 merge
remain. The measured path does not invoke the provisional constructor.
