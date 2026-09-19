# Complete automatic layout economics (M6l)

M6k [PR64](https://github.com/johannes-schmieder/MultiwayMG/pull/64) merged as
`04b2184e70dc5954ce97d63d7a96b9023f052f2f`; verified clean main matches reviewed
tree `25571d67155a4ad3f597ba458278b7a82cbec842`. Final source Rust/allocation
CI35436752292/35436752284 and PR CI35436754785/35436754800 all pass, as do
post-merge CI35437114171/35437114180. Its
[diagnostic evidence](../benchmarks/results/2026-09-19/prepared-automatic-diagnostic-v1/README.md)
identifies different one-RHS and repeated-RHS costs but does not rank layout
performance. M6i/M6j negative scalar economics and identity failures remain.

## Fixed experiment

The separate [layout policy](../benchmarks/policies/prepared-automatic-layout-v1.json)
compares nine arms: automatic scalar/fine-row/all-row/fine-image/all-image and
component/global MAP with scalar/all-row. Global LSMR never applies a Gramian;
its image requests have identical row storage, so duplicate aliases are omitted.
Numerical parameters, maps, screens, terminal cutoffs, weights and certificates
remain exactly the existing policy. This increment changes no Rust source,
production API, dependency pin, numerical choice or default layout.

Reuse `prepared_automatic_diagnostic ROUTE LAYOUT` only in its qualified
**uninstrumented** release build (`--no-default-features --features lsmr`). The
schema2 boundary is useful independently of its optional observer. The new
performance validator rejects profiling=true, any regions/report or either
nonzero profiler TLS capacity before numerical/outer-cost schema projection.
It retains explicit layout/grouping and common inline record size. The diagnostic
policy and frozen v1 timing validators remain unchanged. Instrumented records
cannot enter this timing experiment.

Each case runs one warmup and five measured isolated processes per arm. Rotate
arm order by case/repetition; pair identical repeat indices. Prespecified
comparisons include automatic layouts versus automatic scalar, every automatic
layout versus both scalar and grouped global MAP, component MAP versus its
matching global MAP layout, and grouped global MAP versus scalar. There are no
per-case best-layout choices or post-hoc control substitutions.

A comparison requires every measured and warmup attempt for both arms to certify
and satisfy the RSS budget. A failed warmup invalidates the complete cell even
if every measured pair succeeds. No failed cost is dropped. Unavailable cells
prevent the corresponding complete balanced aggregate. Report equal family/
width means, single/repeated RHS separately, both all ten and the declared hard
six families. These are development comparisons, not campaign qualification or
permission to select a default.

## Input and cost boundary

Seed10001 only; ten families, two shapes and unit/heterogeneous positive weights.
Smoke/development retain the v1 sizes and K1,2,4,8,16,17,32. A separate expanded
profile uses balanced512/512/512, unbalanced2048/128/64, 65,536 draws and K1/8/32.
Topology-only preflight finds at most 69,487 unique tuples (19,178,444 input bytes
at K32), below the unchanged 100,000-tuple/4,096-level-per-factor boundary. This
is larger bounded development, not proof of out-of-cache or SCC performance.
No new calibration or holdout inputs are used. Columns indexed16/31 remain
exact zeros and RHS prefixes remain identical.

Smoke and development each have 15,120 processes and 144,000 measured columns;
expanded has 6,480 processes and 73,800 measured columns. Linux smoke repeats
the complete smoke policy. Input recipes remain canonical unique tuples; this
experiment does not time raw observation collapse or multiplicity expansion.

Complete cold process wall time includes startup/teardown, decoder, fine topology,
frame, output allocation, full driver and materialization. Failed construction,
screening, solves, fallback and certificates remain charged. Record all six
outer phases, bookkeeping/destruction, requested/admitted live payload, grouping
and full process peak RSS. Requested setup bounds and checked actual retained
capacities are not an exact allocator high-water measurement. Fixed dense stack
scratch, runtime, allocator metadata/rounding and error strings are outside array
bounds but inside RSS; isolated M6k allocation qualification is separate. OS
printed user/system CPU often has 0.01-second resolution; zero means below
resolution and no CPU speedup is inferred.

## Qualification and preservation

Fixed arm/case math, work, payload, routing and failures must repeat exactly;
certified layouts within a route must preserve math/work, while their layout
payload is allowed to differ. The validator reconstructs all schedules, inputs,
source/build hashes, raw streams, process resources and declared comparisons.
Only clean committed Rust1.85 source with no compiler overrides can be collected;
Python3.11+ is required. Binaries, build logs, failed attempts and all raw streams
are preserved outside the repository, with canonical evidence/checksums added
under `benchmarks/results/` after independent copy/archive validation.

Eleven new tests cover observer rejection, paired schedules/coverage, false
certificates, fixed-configuration/layout differences, failed warmup/measurement
costs, actual process timeout/protocol errors, and expanded input/zero-lane bounds.
All required Rust1.85 checks and all 126 Python tests now pass. See the
[qualification receipts](../benchmarks/results/2026-09-19/automatic-layout-qualification/README.md).
Exact-source Rust/allocation CI35437783030/35437783035 and PR
CI35437785190/35437785213 pass. Six Linux/macOS/Windows debug/release logs exactly
match all 39 allocation controls and four denials.

## Complete results and next increments

The [four preserved collections](../benchmarks/results/2026-09-19/prepared-automatic-layout-v1/README.md)
retain 51,840 processes and 505,800 certified measured columns, with no failed
warmups/measurements or RSS violations. All 15,120 overlapping M6j scalar and
19,440 overlapping M6k reference process signatures match exactly; those sets
overlap. Expanded records pass exact repeat and within-route layout checks.
Three Mac summaries recompute exactly; eight Linux derived geometric means
match within the preregistered archival-only tolerance. All raw/scientific data
and every archive member are exact; all provider ZIP digests were verified.

All-level image improves automatic scalar by 1.2321x on development and 1.3128x
on expanded, but is 1.86x/2.80x slower than matched grouped global MAP. Its
single/repeated ratios against that baseline are 0.3905/0.5654 and 0.1974/0.4797.
Every whole-family aggregate remains below grouped MAP; expanded chain repeated
RHS is a limited 1.2101x favorable subset, while its full family remains 0.8769x.
All seventeen comparisons, memory scopes and negatives are reported. Smoke
executes direct dense components, not multigrid hierarchies. Profile width mixes
differ, so cross-profile differences do not isolate size effects.

Component scheduling without hierarchy is near matched MAP (0.9877x/0.9797x).
At K1, development accepts 29/rejects nine and expanded accepts 32/rejects twelve actual
hierarchy screens. Thus optimize hierarchy setup/application while keeping its
work and failed screens visible. The expanded all-image array maximum is
37,647,224 bytes and RSS maximum 42,156,032 bytes; the 1GiB cap is not binding.

1. Test dense reconstruction in native column-major order. Walking modes outside
   output rows preserves each output's ascending-mode `mul_add` chain without a
   transpose or new arrays. Keep first-transform/factorization/rank policy fixed;
   compare independent old arithmetic, dimensions/tails, failures and allocation
   records. Inspect candidate assembly and rerun charged smoke/development.
2. Separately test avoiding the canonical `(factor0,factor1,id)` source-ID sort
   during prepared candidate construction. Preserve source addition order, map
   ties, work and budgets; canonicality belongs to the prepared topology boundary,
   not arbitrary `ThreeWayTopology::new` input. Larger sorting schemes require
   explicit extra-scratch admission and evidence.
3. Evaluate terminal/admission and screening policies separately. The current
   within/Schwarz API has opaque factor storage and pooled/implicit parallel
   scratch; a sparse terminal needs checked setup/fill/retained accounting and
   fixed caller-owned application before integration.

The frozen ARM64 probe confirms scalar FMA with strided reconstruction loads.
The generic Linux x86 probe calls scalar libm `fma` per operation; this does not
establish software emulation inside libm. Any native-CPU build needs a separately
declared policy and equally built controls. No compiler flag, default layout or
numerical-policy change is made here. M6 remains open; M7 bounded deterministic
parallelism, M8 independent RHS panels, M9 fresh-weight replay and M10 untouched
competitive qualification follow.
