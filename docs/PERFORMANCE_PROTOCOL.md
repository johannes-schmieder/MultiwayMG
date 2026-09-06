# Standalone CPU performance protocol v1

Frozen design contract, 2026-09-06, before optimization. This does not declare a
qualified candidate. Existing issue-2/3/4 frozen evidence remains unchanged.
Generator recipes, exact dimensions and selector constants must additionally be
committed and checksummed before calibration/holdout execution respectively.
Missing provenance or failed certification makes a comparison ineligible.

## Immutable baselines

- MultiwayMG main: `989d72081bbe3eb1fb56db1cc920cd094595e312`,
  tree `f475af3cb2a14e0ae57c4b349ee78f44f95357e5`.
- Upstream within and schwarz-precond:
  `b7779cbab7a3116be56aae4389fde1f6e6a99a9f` at
  `https://github.com/py-econometrics/within`.
- CMG: `90e1fe0b0c14065155532711246ede6678bb4935` at
  `https://github.com/johannes-schmieder/CMG`.
- Rust 1.85.0, locked dependencies, release builds for timings. Record all
  compiler flags, executable/source/lock hashes, target triple, CPU model,
  OS, worker limits, scheduler allocation and actual affinity.

Run original upstream within and original MultiwayMG in separate executables
from the forked candidate. Include optimized LSMR without a hierarchy and
MAP/diagonal controls. An oracle hierarchy is a diagnostic ceiling, not an
automatic candidate. A mandatory hierarchy-disabled ablation separates CPU and
workspace gains from the three-way coarse correction itself.

## Workload matrix

Use these ten predeclared synthetic structural families:

| ID | Family | Classification |
| --- | --- | --- |
| uniform | Uniform sparse connected | Easy control |
| communities | Planted weak communities | Difficult |
| chain | Weak global chain | Difficult |
| pair-dominant | Dominant pair with weak third factor | Difficult |
| nested | Nearly nested factors with additional nullity | Difficult |
| hubs | Hub and power-law degree variants | Stress/control |
| ragged | Disconnected components with ragged sizes | Stress/control |
| tensor | Tensor/Latin incidence variants | Stress/control |
| worker-firm-occupation | Synthetic labor-panel incidence | Difficult |
| exporter-importer-product | Synthetic trade-panel incidence | Difficult |

Cross unit and heterogeneous positive weights, balanced and unbalanced factor
sizes. Test observation/tuple ratios 1,4,16 in input-preparation experiments.
Solve widths 1,2,4,8,16,17,32, including zero and mixed-convergence lanes. Charge
setup at every RHS prefix; report single-RHS and repeated-RHS results separately.
Balanced aggregate weighting means equal weight per family and per RHS width,
not weighting by runtime, row count or number of generated variants.

Use tiny dense references first; development E approximately 1e4–1e5; full
calibration approximately 1e6; then approximately 1e7 on representative easy,
weakly coupled, skewed and disconnected cases. Record realized E and V because
duplicate collapse changes unique tuple counts. Exact recipes/dimensions are
required before generating a benchmark; a family name alone is not provenance.

Development seeds: 10001–10003. Calibration: 20001–20003. Untouched qualification
seeds: 90001–90005. Do not run holdout until source, dependency pins, dimensions,
selector, eligible regimes and acceptance criteria are frozen and calibration
passes. After failure preserve the negative result; changed candidates need a
new preregistered holdout, not repeated tuning against these seeds.

Changing-weight tests include smooth drift, large localized shocks, weight
concentration within an aggregate, disconnected-component shocks, and loss of
map quality. Compare complete fresh construction with exact replay plus quality
checks and any suffix rebuild. Active sample/support changes require rebuilding
topology and must not be reported as numerical replay.

## Correctness and acceptance

Every accepted candidate must independently certify against the original tuple
operator at requested tolerance (default 1e-8 here; historical tighter gates
remain unchanged). Native convergence is only a candidate. NaN, invalid scales,
unavailable diagnostics, mismatched input hashes, missing cells and invalid
memory/accounting are fail-closed errors, never zero cost or successful solves.
Record legitimate zero-RHS certificates explicitly. Count iteration caps,
rejections, errors and timeouts in the denominator of coverage.

Numerical gates cover dense fitted values/residuals; components and extra
nullity; exact adjoints/Galerkin; range/symmetry/positivity; observation collapse
and explicit/unit weights; tuple/label/component permutations; extreme finite
scales and subnormals; incompatible RHS; stale/equal-but-distinct owners; failure
cleanup and reuse; allocations and concurrent workspaces. Cross-architecture
comparison is numerical with independent certificates. Fixed thread/config
runs must repeat their numerical results and work reports.

Initial advancement gates, evaluated separately on each qualified hardware class:

1. Every accepted solve certifies; coverage is complete for its declared regime.
2. At least 1.25x geometric-mean fully charged speedup over upstream Schwarz-LSMR
   on the six declared difficult families, separately for width 1 and equal-
   weighted repeated widths 2,4,8,16,17,32.
3. At least 2x on one structurally interpretable difficult family; identify the
   whole family/regime, not a selected favorable row.
4. No geometric-mean regression over the whole balanced 1–32 RHS matrix.
5. Easy-control p95 charged overhead <= 10%, including rejected setup/fallback.
6. Peak RSS fits the explicit process budget. Promoted regimes target <= 25%
   extra peak memory relative to the baseline; otherwise restrict and label the
   memory regime before qualification, without claiming a general memory win.

These are research gates, not promised speedups. Failure returns development to
the largest measured bottleneck. Preserve timing uncertainty and do not count
a timeout as an infinite speedup. Report five paired repetitions, rotated solver
order on the same host/input, with one untimed warmup per executable. Aggregate
paired ratios by cell first and use equal family/width weights. Retain raw
repetitions and dispersion so noisy threshold crossings remain visible.

## Complete cost and memory boundaries

Measure input validation/collapse; topology/groups; candidate maps; numerical
setup by level; screening; workspace construction; operator/smoother/transfer;
Krylov vector/reduction work; independent certificate; total cold and all RHS
prefixes; replay and complete fresh build. Include failed attempts, fallbacks,
quality checks and output materialization. Report work counters alongside time.
Shared preprocessing must be charged identically or reported outside both totals
with an additional true end-to-end comparison; do not hide it on one side.

Report separately logical retained payload, reserved allocation capacity,
construction peak live allocations, scratch/workspace pool, old/new overlap,
opaque dependency memory and process peak RSS. Unknown is not zero. State the
scope of exact byte reports; payload admission is not a total-RSS guarantee.
Count shared allocations once at their owning root. Record actual CPU utilization
and placement. Isolate executable runs for meaningful process peak measurements.

Before the first performance claim, the machine-readable result validator must
reject missing/duplicate cells, mismatched provenance, non-finite values,
uncertified success, omitted failed-route cost, invalid phase totals and missing
memory scopes. Keep adversarial validator tests in permanent CI. Documentation
alone is not an executable evidence gate; no performance claim is made by M0.

## Resource envelope

Local calibration: Apple Silicon Mac14,14, 192 GiB RAM, 16 performance + 8
efficiency cores observed during planning. Record live values per run. Test
1,2,4,8,16,24 workers, distinguishing mixed-core execution in reports. Start
bounded; do not consume all memory simply because it is available.

SCC campaign root: `/projectnb/welfgr/multiwaymg`, after checking existing state
and instructions. Use project `welfgr`, scheduler jobs for compute, 4-slot smokes,
normal 16-slot/4 GiB-per-slot jobs (64 GiB), at most 64 reserved campaign slots
concurrently, initial 2-hour limits. Use 28/32 slots only for declared scaling or
placement experiments. Respect NSLOTS and cap nested math library threads.
Verify actual CPU affinity/NUMA placement; scheduler allocation is not proof of
binding. Do not disturb other projects' running jobs.

Keep generated inputs in job scratch with deterministic recipes/checksums.
Retain compact evidence, scheduler receipts and necessary raw tables; cap this
campaign's persistent SCC footprint at 20 GiB and recheck project quota before
submitting. Planning observed only about 334 GB free in the shared projectnb
allocation. Collect each task's scheduler status, application exit and validator
result. Retry at most once for a demonstrated infrastructure failure, never
silently rerun scientific failures or retune policies. No Telegram requested.

## Evidence and publication

Store canonical compact evidence under `benchmarks/results/<date>/` or a
specifically documented evidence directory. Keep raw larger artifacts outside
Git with immutable hashes and a durable retrieval path. Source, generator,
policy and dependency hashes are required. Results commits must identify the
actual measured source. Release candidate documentation states supported
regimes, costs and limitations; passing tests alone is not competitive-solver
qualification. No estimator integration or package publication in this plan.
