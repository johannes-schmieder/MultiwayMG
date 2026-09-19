# Complete automatic construction economics (M6i)

The [terminal-first driver](ISSUE5_COMPONENT_DRIVER.md) is implemented and
numerically qualified. This separate development experiment measures its full
cost against component scheduling with construction disabled and whole-problem
baselines. The [frozen policy](../benchmarks/policies/prepared-automatic-v1.json)
is explicit; passing this experiment does not select an automatic default or
qualify competitive performance. M6 economics and M7–M10 remain open.

## Routes and submitted problem

| Route | Construction and execution |
| --- | --- |
| automatic | Terminal-first components, adjacent-pair hierarchy attempts, current numerical replay, actual recursive screening, gated LSMR, local and final global fallback |
| components-map | Same component schedule and small dense terminals; hierarchy disabled; large components use symmetric MAP |
| global-map | One whole-problem symmetric MAP owner and gated LSMR workspace |
| global-diagonal | One whole-problem inverse-diagonal owner and gated LSMR workspace |
| global-identity | One whole-problem identity owner and gated LSMR workspace |

Every route uses original-operator certification at 1e-8, native tolerance 1e-8,
maximum 1000 iterations and an eight-vector local window. The automatic policy
uses at most eight transitions, tuple/coefficient complexity multipliers 4/3,
minimum affinity .02, dense terminal tolerance 1e-12, two screen starts with
eight power steps and four tail steps. Its exact seed, damping and rejection
criteria are in the policy and emitted by the binary. Component-terminal rank
truncation and quality rejection remain visible and charged.

Inputs are canonical unique tuples and current positive weights, not observation
records. Draw duplicates are removed; their multiplicities are not weights.
Observation aggregation, multiplicity economics and estimator integration are
outside this experiment. There are no supplied maps. The independent binary
input format is `MG3AUT1` plus a zero byte, three little-endian u32 factor counts,
u64 tuple count, u32 RHS count, canonical u32 triples, f64 weights and column-major
f64 targets. Bounds are 4096 levels per factor, 100000 tuples and 32 RHS. The
reader rejects unsupported dimensions, malformed/trailing input and nonfinite
values. Topology construction verifies canonical keys and complete support.

## Recipes and schedule

Ten deterministic families cover uniform incidence, four communities with weak
bridges, local chains, a dominant factor pair, nested factors with additional
null modes, hubs with star coverage, ragged disconnected components, Latin
squares, worker–firm–occupation and exporter–importer–product patterns. The exact
integer SplitMix recipes and coverage scaffolds are in `automatic_recipes.py`.
Targets alternate manufactured sums and deterministic random dyadic values;
columns 16 and 31 (zero-based) are exactly zero. Every smaller K is the exact
prefix of K=32. Latin inputs are not claimed to have extra nullity; the nested
family supplies that control. Ragged sampling is proportional to first-factor
component size, preserving a giant component rather than equally sampling tiny
components. Heterogeneous weights are positive dyadic values; community bridges
receive an additional factor 2^-10.

| Profile | Balanced factor sizes | Unbalanced factor sizes | Random draws |
| --- | --- | --- | --- |
| smoke | 16,16,16 | 32,8,4 | 512 |
| development | 128,128,128 | 512,32,16 | 16384 |

Each profile includes both weight regimes, seed10001 only and K=1,2,4,8,16,17,32:
280 cells, five routes, one warmup and five measured repetitions, or 8400
processes and 80000 measured columns. All five routes occupy every measured
position once per cell. Each K starts a new process and pays the full setup
cost. Warmup only warms the OS cache; no numerical state is reused across
processes. Smoke is predominantly a dense-terminal control; separate actual
protocol tests exercise accepted recursive construction and rejected attempts.
Development seeds10002/10003, calibration and campaign holdout remain untouched.

## Charged cost and memory

A committed clean tree builds Rust1.85 release with locked dependencies, LSMR
and no profiling or compiler overrides. Six thread environment caps are one.
Each isolated process has a 60-second process-group timeout and 1GiB requested
array admission and observed RSS limit. The journal preserves all attempts and
raw outputs, including errors/timeouts; no retry or dropped route is allowed.

Decode, fine topology, fine frame, caller output, whole driver and report
materialization have disjoint timers. Their sum plus measured bookkeeping/drop
overhead equals the inner total. The whole-driver interval includes every
construction, replay, screen, failed attempt, fallback, solve and original
certificate. Separate internal stage timings are unavailable, not zero. The
outer process time additionally charges startup, printing and destruction.
Canonical input generation, compilation and harness validation are outside the
submitted-tuple boundary. The collection span includes interprocess generation
and journal overhead; build logs are retained separately.

Report original topology/frame capacities, retained decoded tuple/weight/target
and output/report capacities, maximum requested and maximum admitted array
payload, inline record size, and full-process RSS. One current component's
numerical owner and reusable solver workspace are live at a time; no forest or
RHS workspace pool is retained. Bounds distinguish requested construction arrays
from checked actual capacities and from allocator high-water. Fixed dense stack
scratch, runtime, allocator metadata/rounding and error strings are excluded
from array bounds but included in RSS. Dense dependency allocation failures are
still infallible. Separate M6h allocation qualification measures exact requested
allocator peaks; this timing binary contains no allocator instrumentation.

OS user/system CPU times are retained at their printed precision (often .01s).
A displayed zero is below resolution, not zero CPU use. No CPU speedup is inferred
from such values. Wall time is authoritative here. Record actual hardware,
compiler/target, source tree, binary/source/policy/input hashes, Cargo config,
SDK environment, OS affinity where available, and explicit unpinned placement
on Mac. This experiment cannot establish parallel scaling or cache-capacity
crossovers.

## Validation and interpretation

The validator regenerates inputs/components and the entire paired schedule,
verifies committed source and raw-log hashes, and checks exact numerical,
work, payload, rejection and fingerprint repeatability within each fixed route.
Different routes may produce different valid coefficients in a singular system;
they are not required to share fingerprints. It checks original certificates,
zero columns, fallback/rejection consistency, native global-baseline diagnostics,
complete cost, caller/fine memory lower bounds and resource limits. The automatic
report does not invent per-column component-native diagnostics unavailable in
the public driver. Every successful column requires its own accepted report.

Paired process speedups use global-map as control. A cell needs five eligible
pairs; missing/rejected/over-budget pairs have no speedup. Geometric means give
equal weight to each family and K, with balanced/unbalanced and both weight
regimes inside each group. Single and repeated RHS are also reported separately.
No incomplete route receives an overall speedup. Measured and warmup failures
are counted separately; either invalidates the complete certification gate.
All failed process time stays charged. Native candidates are diagnostics;
original certificates decide eligibility.

The actual debug/release protocol suite covers every recipe and route, K17/32
zero columns, accepted multilevel construction, construction rejection with MAP
fallback, numerical errors, malformed input and CLI failures. Synthetic validator
tests reject missing pairs, false success, absent work/cost, changed input,
nonfinite data, drift, missing provenance and hidden failure costs. These tests
are qualification checks, not comparative timing evidence.

Run `python3 scripts/prepared_automatic.py <fresh-directory> --profile smoke`
(or `development`) and independently revalidate with
`python3 scripts/validate_prepared_automatic.py <directory>`.
`--require-certification` additionally requires all attempted runs to certify.
Permanent Linux CI preserves the complete smoke and both protocol qualifications.
Any negative result remains in the versioned evidence and informs the next
increment; policy changes require a new version before new measurements.

## Frozen v1 outcome

The [three complete artifacts](../benchmarks/results/2026-09-19/prepared-automatic-v1/README.md)
retain 25,200 processes and 239,925 certified measured columns. Automatic,
component-disabled MAP, MAP and diagonal certify all measured columns; identity
misses 75 original certificates in 35 measured development processes. The
all-control development gate is false and identity has no full aggregate.
Mac automatic development speedup is 0.5409x versus MAP, despite useful
iteration reductions. No family-wide development win is established. Preserve
these negatives and the complete policy before explicit grouped application,
phase diagnosis, larger development and terminal/admission trials.
