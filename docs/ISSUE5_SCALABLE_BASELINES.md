# Scalable prepared baseline execution (M6g)

A large rejected hierarchy needs an executable fallback. The old prepared
solver owner always required a dense terminal of at most 256 coefficients,
so a zero-transition hierarchy could not provide a scalable fine-level baseline.
M6g adds explicit fixed baseline owners and shares the existing prepared PCG,
native LSMR, certificate-gated LSMR and independent scalar batch implementations.
It does not yet select a fallback or implement the automatic component driver.

## Shared ownership and numerical execution

`PreparedSolverAction` is a sealed, statically dispatched boundary with an
associated exact-owner workspace. `PreparedPcgWorkspace` and
`PreparedLsmrWorkspace` retain `PreparedMapHierarchy` as their default generic
owner, so existing hierarchy callers keep their API and numerical path. Small
hierarchy delegates are inline; there is no virtual dispatch, implicit pool,
per-iteration preparation or duplicate recurrence. The original PCG kernel,
pinned within LSMR recurrence, candidate certificate gate and final independent
certificate remain unchanged.

`PreparedBaseline` borrows the current fine frame and optional exact structural
grouping. It owns no topology, weight, degree, factor or dense-terminal arrays.
The three explicit choices are:

| Action | Mathematical operation | Exclusive action arrays |
| --- | --- | --- |
| Identity | Literal I | None |
| Inverse diagonal | P D^-1 P | One V-vector and one projector array |
| Symmetric MAP | Existing projected symmetric factor sweep | Three V-vectors and one projector array |

P is the existing orthogonal projection away from known structural factor
shifts, and D contains current positive degrees. Identity is the unpreconditioned
control: it copies finite input and needs no projection or coefficient scratch.
PCG retains its outer projections; LSMR retains the final candidate projection
and original-operator certificate. Identity preserves shift vectors, while the
projected diagonal and MAP actions annihilate them. The original operator and
its structural/additional null spaces are not changed by any baseline.

Projected inverse diagonal is symmetric and positive on the original operator
range because that range lies in P's range and D is positive. The existing MAP
action has the corresponding projected symmetric block factorization. These
are fixed preconditioners, not standalone stationary iterations, numerical-rank
claims or inner iterative solves with RHS-dependent stopping. Finite arithmetic
checks can still reject unrepresentable corrections.

Scalar, ordered row-gather and tuple-image Gramian modes remain explicit. A
selected image adds one E-vector shared by outer Gramian calls. Grouping is
borrowed and charged once; its topology must match exactly. No layout is selected
from timing. Baseline workspaces bind the exact action owner, including its kind
and grouping, even when another owner has equal values or the same frame.

## Admission, liveness and failure

Requested exclusive action payload is zero, `8V+S`, or `24V+S` for identity,
diagonal or MAP respectively, where S is the existing projector requirement.
Add `8E` only for a selected tuple image. Complete admission also charges original
topology/frame, borrowed grouping and caller-declared other live arrays. Requested
bytes are checked before reservation; actual retained capacities are checked
before publishing baseline and complete solver workspaces. Dense/coarse report
categories are zero because they are absent. Inline roots, stack, allocator
metadata, new allocator excess during construction and RSS are outside this
array-payload report.

PCG adds its existing ten outer arrays, and native/gated LSMR add the same sixteen
outer arrays on the qualified controls. Their original certificate and candidate
gate share existing buffers. The historically named `hierarchy_applications`
counter counts the complete fixed action when the owner is a baseline; retained
report field names stay compatible with frozen evidence. Wrong owners, malformed
input and nonfinite values reject before publishing action output. Finite
diagonal/MAP correction overflow releases no persistent owner, publishes no
result and permits a later valid call. Scalar batch prefix and per-column
certificate semantics are inherited unchanged.

## Qualification and preserved native negatives

Independent component-sum projectors, ordinary MAP references and dense original
Gramians verify all three actions across original/changed weights and three
layouts. Symmetry and positive restricted spectra are checked on the original
range, including nested additional nullity and a separate singleton. MAP and
Gramian results agree bit for bit with existing arithmetic. Large Latin and
weak-chain/nested controls at V=384 exercise 108 PCG and 108 gated-LSMR solves
across kinds, layouts, weight generations and random/manufactured/zero targets;
all certify, with exact within-configuration layout results and reports.
Batch tests cover RHS1/2/4/8/16/17/32 and unchanged reports after static rejection.

On 48 separate historical MAP baseline cases, 14 native LSMR candidates fail the
original certificate. The unchanged gate performs 18 vetoes and certifies all
48 cases without changing the arrays or retuning native tolerances. Native
success is kept separate from original acceptance. See the
[per-case diagnostics](../benchmarks/results/2026-09-07/baseline-certificate-qualification/records.json).
The historical issue-3 fixtures are not the untouched campaign holdout.

Private tests cover every baseline reservation error/unwind, exact and insufficient
budgets, arithmetic overflow, exact owners, malformed input, numerical failure
and recovery. The isolated allocator verifies zero/two/four action allocations,
one optional image, exact retained/released bytes, no reallocation, allocation-free
first/repeated/RHS32 actions and complete solves, and static/numerical failures.
A borrow test prevents a workspace from outliving its action owner. Existing
hierarchy reference and complete allocation tests pass after generalization.

The first identity design unnecessarily projected and retained one vector and a
projector; it was replaced by literal identity before source freeze or timing.
Both versions' development logs are preserved. A diagnostic test initially read
workspace payload while result candidates were still borrowed; moving that read
after their final use fixed the test without weakening ownership. Final required Rust1.85 formatting, strict Clippy, all/minimal tests and
warning-free rustdoc pass, together with 90 Python checks. All twelve release
groups pass, including numerical/reference/full-solver/allocator/private gates
and all 120 actual protocol comparisons.

After source freeze, the unchanged four-artifact supplied-map regression checks
legacy numerical/work/payload behavior. It does not time baseline or automatic
construction economics. Next compose terminal-first component-local construction,
bounded candidates, full current-weight replay, actual-cycle screens and charged
baseline fallback. M6–M10 remain open. No automatic/default/competitive promotion,
calibration or campaign-holdout result is introduced here.


Frozen source `5660b97` passes the [four-artifact supplied-map regression](../benchmarks/results/2026-09-07/prepared-layout-scalable-baselines/README.md):
7,920 processes, 72,300 certified measured columns, 6,336 exact layout comparisons
and exact M6f input/numerical/work/payload/layout records. No measured retries or
failed gates occurred. Copies and every archived member independently revalidate.
8 derived timing geomeans differ under local recomputation within the existing archival-only relative tolerance of 1e-14. Receipts record every difference; raw numerical/certificate/work/payload comparisons remain exact.
Source/PR checks passed; final evidence-head checks and PR60 merge remain. These
measurements exercise the shared hierarchy path, not baseline economics.
