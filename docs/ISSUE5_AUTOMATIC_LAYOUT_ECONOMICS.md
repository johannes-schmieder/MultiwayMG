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
Exact-source CI and committed-source collection follow.
M6 terminal/admission and screening choices, M7 parallelism, M8 panels, M9 weights
and M10 competitive qualification remain open.
