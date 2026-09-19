# Dense terminal reconstruction (M6m)

M6l PR65 merged as `4fdce09efb33b0382ba32bb6ad679faa7b892d76`; clean main
matches reviewed tree `f25d8217672a91c2b8640d70c756ebecc75fb2ea`. All 34 final
jobs pass: source CI35444513174/35444513177 and PR CI35444516200/35444516201.
Post-merge CI35444983748/35444983750 also pass every job.
The preserved provenance failure and narrow repair are in the
[preceding result](ISSUE5_AUTOMATIC_LAYOUT_ECONOMICS.md). Complete grouped
economics remain negative: automatic all-image is 1.86x/2.80x slower than grouped
MAP on development/expanded. No default or competitive promotion follows.

## Candidate and numerical boundary

The eigenvectors already use column-major storage. Before M6m, reconstruction
reads each row with a stride and a dependent FMA chain. The candidate streams
columns, updating independent output entries. Each output starts at positive
zero and visits modes in exactly the original ascending order, with the same
`mul_add(coefficient, modal, sum)` arithmetic. Do not skip zero modes: signed
zeros and nonfinite propagation must retain the old behavior. The forward modal
transform, factorization, rank, tolerance, terminal cap and routing are unchanged.

All three vector lengths are checked before either mutable buffer changes,
with unchanged error precedence. The private zero-dimensional boundary remains
valid. There is no transpose, new allocation, extra retained array, public API,
CPU dispatch or native compiler flag. The existing modal vector and matrix
remain the entire application storage. This affects dense application during
screening and solves; it does not optimize eigendecomposition or coarse setup.

Private tests compare every finite output and modal bit against the independent
old loop at vector and cap boundaries, including zero, signed-zero and subnormal
RHS and reused buffers. Nonfinite classification remains checked without making
a portable NaN-payload promise. Public tests compare real eigensystems against
the older independent factorization/application reference at dimensions31,32,33,
255,256,257 and a disconnected case with additional null directions. Existing
allocation, automatic-driver and prepared-solver gates remain required.

## Frozen application experiment

[dense-reconstruction-v1](../benchmarks/policies/dense-reconstruction-v1.json)
compares the exact old loop and the candidate on the same preallocated private
spectral table, inverse values and RHS. The table is synthetic, not an empirical
fitted terminal. Real eigensystem tests and full charged automatic benchmarks
are separate evidence. The old allocating public wrapper is not the control.

Dimensions are3,6,9,16,31,32,33,63,64,65,127,128,129,255,256,257. For each
arm/dimension/sample, iterations are floor(2^24/(2*n*n)), fixed before timing.
One warmup and five measured repetitions rotate arm order by dimension/round.
Every modal/output bit is checked after each sample. Preserve all192 samples,
including32 warmups; report the five paired old/stream ratios per dimension and
equal-dimension geometric means. Do not select best samples or per-case arms.
The identical indirect-call/black-box boundary prevents whole-loop elimination.

Use Rust1.85 release with no default features and only `lsmr`, profiling disabled,
one worker, fixed thread caps, no compiler overrides, a120-second whole-harness
process timeout and1GiB RSS cap. Preserve the exact test binary, build/raw logs,
source tree/file hashes, hardware/affinity/SDK and process wall/CPU/RSS. A failed
process or missing/mismatched sample invalidates the whole experiment; no retry
is planned. Application intervals exclude preparation, printing and tests.
Whole harness resources include those costs and are not per-arm memory or
complete-solver performance. Linux/macOS derived geometric-mean archival
recomputations may differ by at most1e-14 relative; record each difference and
preserve originals. Raw timings, bits, work, inputs and hashes remain exact.

Before retaining the change, inspect actual ARM and generic-x86 release assembly
and compare application behavior on both. Baseline ARM reconstruction has scalar
strided FMAs; generic x86 calls libm `fma`. Neither fact establishes a candidate
speedup or says how libm internally dispatches. New vectorization claims require
actual assembly evidence. Retain unfavorable sizes/platforms and revise the
candidate explicitly if necessary; do not hide them with undeclared flags.

## Complete qualification and remaining work

Run required Rust checks, release dense/reference, automatic/projection/hierarchy/
PCG/LSMR/allocation gates, and frozen complete automatic layout smoke/development
on Mac plus Linux smoke. Compare every common mathematical/work/payload signature
to M6l; new timings across source revisions are unpaired and cannot establish a
paired speedup. Keep the seventeen prescribed layout/control comparisons,
certificates, rejected work, setup and memory scopes. Expanded reruns are needed
if new failures, size concerns or changed numerical work justify them.

After reviewed exact-source and PR CI, commit full evidence and merge only the
green final head, verifying actual main. This is one application increment;
first-transform interleaving, canonical-sort removal, sparse-terminal admission,
M7 explicit pools, M8 independent RHS panels, M9 current-weight replay and M10
competitive qualification remain separate. Calibration and holdout remain unused.

All required Rust1.85 checks,135 Python tests and the release scientific gates
pass. Both debug/release actual protocols pass1,125 exact pairs,225 legacy and
900 layout comparisons,26 malformed inputs and10 CLI checks. See the
[source qualification](../benchmarks/results/2026-09-19/dense-reconstruction-qualification/README.md).
Source freeze precedes all performance measurements.

## Rejected general-iterator revision

Source `8655c351d022d48f8e51adc6406e56149ab12e8c` uses nalgebra's general
column iterator. Its complete fixed Mac microbenchmark regresses at all sixteen
dimensions: balanced old/new time ratio0.613554 (about1.63x slower). Actual ARM
assembly retains per-element iterator state/branches and scalar FMAs. The Linux
microbenchmark is near parity at0.991946, with every size retained. Its provider
ZIP digest was independently verified and its complete summary recomputes exactly.

Mac smoke completes all15,120 processes and144,000 measured columns with zero
failures. The unfinished Mac development run was explicitly withdrawn after the
negative microbenchmark/assembly diagnosis:2,154 journaled attempts remain,
with147.764251434 seconds of completed process cost. Its original manifest,
journal and raw files are unchanged. A possibly interrupted in-flight attempt
has unavailable complete output/cost, not zero cost. No complete development
summary, selected-subset aggregate or speed claim is made for this revision.
All completed and partial evidence is preserved separately.

The next source revision retains the same column traversal but uses the column's
plain contiguous slice, removing the general matrix iterator. This changes no
matrix storage, FMA order, work policy or experiment schedule. It is a distinct
candidate with fresh collection directories, not a retry or replacement of the
rejected revision. New correctness/release checks and frozen platform measurements
must qualify it. Preliminary compilation now emits vector FMAs on ARM; assembly
of the exact measured executable must confirm that after the source freeze.

The plain-slice revision passes all repeated required/135 Python, release and
actual debug/release protocol gates. Both protocols retain1,125 exact pairs.
[Rejected-revision evidence](../benchmarks/results/2026-09-19/dense-reconstruction-v1/README.md)
and independently verified copies/archives are preserved before the new freeze.

## Qualified contiguous-slice revision

Source `55388b14028386effa35e7faaa3a6aa7a9e02b5e`, tree
`50c9adc72b7452b2cd906dd5d914801670846dc6`, passes the complete declared
qualification. Its fixed old/stream application geometric means are1.377653 on
Mac ARM and1.110297 on generic Linux x86. Dimensions3/6 still regress on Mac;
dimension3 regresses on x86. Preserve every size and sample; no size selector,
extra storage, native flags, rank threshold or default changes are introduced.
The exact measured ARM binary uses vector FMAs across independent outputs;
generic x86 retains libm fma calls. These are application-only results.

Mac smoke/development and Linux smoke certify432,000 measured columns across
45,360 processes, with zero failed warmup/measured processes. All45,360 records
exactly match M6l schedule/input/math/work/payload/layout/routing signatures.
Six debug/release Linux/macOS/Windows allocation records match all39 controls
and four allocation-free denials. The measured-source and PR CI runs pass all36
jobs:35446761648/35446761654 and35446763608/35446763630. Final evidence-head
CI and guarded review remain required before merging PR66.

All17 complete-cost comparisons are retained. Development automatic all-image
is1.234817x automatic scalar, but0.554050x grouped MAP (about1.81x slower),
withK1/repeated ratios0.400390/0.584871. Every whole family remains below grouped
MAP. These are within-source layout/control comparisons; cross-source full-solver
timings are unpaired. Nine development quality rejections and the smoke global
fallback remain charged internal work, not missing or discarded processes.
Smoke has no recursive hierarchy attempts and supplies no MG performance claim.

Originals, independent copies and every archive member validate; Linux derived
geometric means have eight archival last-bit differences in each completed
layout artifact, within the predeclared1e-14 relative bound. Raw fields, hashes
and scientific signatures remain exact. Both micro summaries and Mac layout
summaries recompute exactly. The rejected iterator's Linux smoke is also fully
preserved and matches15,120 M6l records. It does not complete the withdrawn Mac
development collection. See [all evidence](../benchmarks/results/2026-09-19/dense-reconstruction-v1/README.md)
and [full-cost tables](../benchmarks/results/2026-09-19/dense-reconstruction-v1/complete-layouts.md).

Retain the contiguous slice implementation as a bounded application improvement.
Expanded reruns are not required because numerical/work/storage/routing policy,
terminal cap and correctness at the tested cap/tail boundaries remain unchanged.
M6 terminal admission and M7–M10 remain open; no competitive claim follows.

## Next development increments

First remove the provably redundant canonical pair-marginal sort at the prepared
boundary. Preserve all six visitation orders and the proposal reduction order;
reset existing source IDs only for the canonical neighbor0/factor1 order. Generic
unprepared topology has no canonical-order guarantee. Verify irregular tuples,
compact/wide IDs, arithmetic, admission and all downstream scientific records.
Do not introduce a second ID buffer or radix histograms without charging them.

Then evaluate forward-transform interleaving independently: several modal dot
products can share a RHS load while preserving each ascending-row FMA chain.
Generic x86 libm calls may spill accumulators, and tiny sizes already regress;
inspect and measure both platforms before retention. No transpose is justified
by the current reconstruction evidence.

A cheaper forward/adjoint triangular cycle is a separate numerical-policy
experiment, not a loop substitution. It requires a symmetry/range/positivity
argument including projection and unobserved coordinates, actual-cycle screening,
and full charged comparisons against the existing symmetric-MAP cycle. Current
modified Golub–Kahan LSMR uses the preconditioner as a metric; do not incorrectly
analyze it as ordinary right preconditioning by an inverse Gramian.

The sparse-terminal boundary remains mandatory before declaring M6 complete:
fixed-generation bounded setup/fill/retained memory, explicit serial application
with caller-owned scratch, and fail-closed fallback. An opaque pooled dependency
or RSS measured after factorization does not establish pre-admission. Then follow
M7's one explicit pool and edge-balanced deterministic reductions, M8's independent
bounded panels, M9's current-weight replay and M10's frozen untouched holdout.
