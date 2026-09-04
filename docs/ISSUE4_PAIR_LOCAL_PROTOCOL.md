# Issue 4: identical-domain pair-local protocol

Status: research infrastructure and a numerical/economics smoke matrix. This is
not completion of issue #4 and does not authorize a production routing rule.
The issue #3 automatic coarse-space policy is unchanged.

## Question and controlled action

Compare Jacobi, an exact dense pseudoinverse, one fixed CMG cycle, and the
pinned public `within` default additive-Schwarz preconditioner on the **same
connected bipartite weighted graph**. A single factor pair isolates local
inverse quality from three-way domain overlap and coarse-space benefit.

Let S change the sign of the second factor and let k=(1,-1). The pair Gramian
is G=SLS, and Q=I-kk'/n projects into its range. Each action is Q M Q. CMG uses
S C S with one stationary `apply_into` call; it never calls an inner PCG.
Connected domains, factor boundaries, arbitrary restricted-RHS projection,
linearity, symmetry, positivity, failure output, and independently recomputed
normal residuals are checked. Positive weights are never thresholded away.

The two-factor within domain has one pair and unit overlap weight. This is
**not** the 1/sqrt(2) per-occurrence convention of an all-three-pair Schwarz
sum; that distinction must be respected in the next whole-system experiment.
The comparator is called `within-default`, because its public local solver may
choose exact/direct or elimination paths on easy components rather than
approximate Cholesky. Its warnings are retained, not suppressed.

Pins remain CMG `90e1fe0b0c14065155532711246ede6678bb4935` and
within/schwarz-precond `b7779cbab7a3116be56aae4389fde1f6e6a99a9f`.
CMG's direct threshold is fixed at 8 to exercise multilevel actions. Within's
local configuration is its pinned default. Neither is tuned after a holdout.

## Reproduction

```sh
RAYON_NUM_THREADS=1 cargo test --locked -p multiway-mg --all-features \
  --example issue4_pair_local
cargo build --locked --release -p multiway-mg --all-features \
  --example issue4_pair_local
RAYON_NUM_THREADS=1 target/release/examples/issue4_pair_local output smoke
python3 scripts/summarize_issue4_pair_local.py \
  output/pair-local.tsv output/SUMMARY.md
```

The permanent GitHub Actions job uses Rust 1.85, saves the exact source SHA,
CPU/toolchain/thread metadata, warnings, raw TSV, checksums and summary, and
uploads partial artifacts on failure. Rust testing is performed in Actions.

The smoke matrix uses 32 levels per factor for each of paths, hubs, weak
communities, dense graphs and six-order edge-weight variation. Four methods,
three builds and cumulative RHS prefixes 1, 4, 16 and 32 give 240 rows and
1,920 measured RHS solves. `calibration` selects 64 and 256 levels per factor;
exact and spectral references are omitted above 256 total vertices. It is a
scaling probe, **not** the comprehensive calibration/holdout required by #4.
An optional third argument selects one method for isolated process runs.
The normal summary deliberately requires the complete matrix, not a selection.

## Measurement boundaries

Input fixture and RHS generation are outside timings. Canonical pair graph
construction is timed separately and charged to every route. Within's public
Solver constructor includes its own design/domain preparation; this extra
boundary is visible in setup and is not claimed to be a pure kernel-only cost.

Every method has the same retained RHS buffer, input/output range projection,
and outer mutex. CMG uses a retained typed workspace. The first action is
charged as workspace initialization, including any opaque lazy within scratch;
this intentionally charges one action in setup for every route. Workspace time
is a **subset** of setup time, not a quantity to add again. Apply microbenchmarks
and dense quality diagnostics are separate and excluded from solver totals.

One discarded warm-up build/solve per route makes this a hot-process comparison;
process startup, cold allocator and first global thread-pool startup are not
claimed as measured production costs. Measured builds do not reuse solver
state from those warm-ups. Each measured build is reused across 32 distinct
RHS vectors, with rotated method order across three repeats. Repeated-prefix
rows are correlated and must not be treated as independent repetitions.

Solve time includes fresh Krylov allocations and independent certification
against the submitted rectangular operator. Exact B/B' counts for solver and
certificate are separate. Certification uses ||B'(y-Bx)||/||B'y|| <= 1e-8;
the iterative flag alone cannot authorize success. Failed attempts retain their
time and operation counts and cannot produce a performance winner. Constructor
or reference failures fail the run, with any partial evidence preserved.

The summary reports paired min/median/max total-time ratios, not confidence
intervals. It also reports the strict integer crossover under S+n*T using
median setup and average-per-RHS cost from the 32-RHS prefix. This is explicitly
a conditional model, not a measured extrapolation. It distinguishes a genuine
long-run amortization point, no win, and a finite early-RHS win window.

## Memory and workspace limits

`principal_solver_bytes` and `known_workspace_bytes` are only identified
categories, not complete memory. Within's opaque retained state is `NA`, never
zero. CMG's hierarchy can share the graph buffers with the common graph, so
summing those two columns double-counts shared storage. Dense reference peaks,
executor pools, allocation capacity/metadata, and complete outer-solver lifetime
must be measured separately before a memory-based routing conclusion.

Actions records process peak RSS around the already-built executable, excluding
compilation. For an all-method run that peak includes all routes and dense
spectral diagnostics, so it is not a per-solver peak.

The recovered three-way `PairCmgSchwarzPreconditioner` is hosted by the generic
`schwarz-precond::LocalSolver` interface. That interface supplies flat f64
scratch, not a mutable typed `CmgWorkspace`. The narrow adapter therefore pools
preallocated typed workspaces and reports fallback workspace allocations. It
retains generic Schwarz scheduling/gather/scatter rather than copying them.
This is not a claim that the full issue #5 allocation-free state redesign is
done. The pair-local harness additionally retains one common lock for fairness;
its absolute application time is not an unwrapped kernel microbenchmark.

## Admission and remaining work

CI gates complete coverage, independently certified residuals, finite positive
range spectra, symmetry/linearity, accounting identities and multilevel CMG
exercise. It never gates on noisy hosted-runner timings or a preferred winner.

Still required by issue #4: large and mixed/disconnected domains; actual
three-way Schwarz MLSMR/PCG comparisons; frozen-coarse-map integration;
selected-pair/component routes; full lifetime memory; thread counts beyond one;
changing-weight replay; broad calibration and a **fresh** frozen holdout. Only
then can the 20% outer-work target and positive end-to-end economics determine
whether CMG has a broad, selective, coarse-only or no local-solver advantage.
