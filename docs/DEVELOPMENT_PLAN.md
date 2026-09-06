# Competitive standalone three-way solver

Accepted 2026-09-06. Implementation ledger for issue #5. Milestone identifiers
M0–M10 here are engineering increments, distinct from the historical roadmap.

## Scope and decisions

Deliver a standalone CPU solver for exactly three categorical intercept factors,
positive weights, disconnected components and additional rank deficiency. Optimize
complete fixed-weight and changing-weight solves across 1–32 RHS. Qualification
covers Apple Silicon and BU SCC CPUs; retain Windows correctness/allocation CI.
Fixed configurations must repeat deterministically. Different thread counts and
architectures must agree numerically and independently certify; bitwise equality
across them is not required. Estimator integration, GPU/distributed execution,
PPML estimator logic, Q > 3 and package publication are outside this campaign.

A narrow owner-controlled `within` fork is authorized for reusable modified-LSMR
storage and explicit execution. Keep both `within` and `schwarz-precond` pinned
to the same exact fork revision; preserve upstream as a separate executable
baseline. Do not mix incompatible copies of the operator trait. Preserve the
recurrence and local reorthogonalization window before algorithm experiments.

## Milestone ledger

| Increment | Deliverable and acceptance boundary | State |
| --- | --- | --- |
| M0 | Commit this plan, performance protocol, immutable baseline identity, resource limits and evidence contract before optimization. | Complete: PR #34, main `1702699` |
| M1 | Reject zero damping and unrepresentable Jacobi coefficients; make independent LSMR certification fail closed, retain native diagnostics separately; extreme/ordinary regressions and adjacent numerical audit. | Complete: PR #35, main `0cfb9f3` |
| M2 | Frame-bound matrix-free operator views sharing existing arithmetic, exact-owner checks, static validation before mutation, zero-allocation operators, adjoint/Galerkin tests. | Complete: PR #36, main `f6a634a` |
| M3 | Narrow pinned within fork: caller-owned LSMR workspace, mutable action adapters, explicit execution, allocating wrappers over the same recurrence; equivalence and allocation gates. | Complete: PR #37, main `1ca043d` |
| M4 | Complete prepared serial supplied-map solve: numerical MAP hierarchy, PCG and LSMR, certificate workspace, bounded scalar RHS reuse, ownership/memory report; compare fresh construction and dense references. | Complete: PRs #38–#45, main `4fa6401` |
| M5 | Measure and reduce serial memory traffic: grouped indices, remove empty MAP passes, scratch liveness arena, fused prolong-add, direct dense assembly, replace expensive tree-based setup where measured. | In progress: M5a merged; M5b opt-in kernel attribution |
| M6 | Scalable automatic construction: bounded structural candidates before numerical setup, component-local depths, bounded dense/sparse terminals and bottom-up actual-cycle screening. | Planned |
| M7 | Explicit bounded CPU pool, deterministic reductions, edge-balanced grouped kernels, threading caps and charged thread scaling. | Planned |
| M8 | Fused 2/4/8 RHS panels, independent convergence, bounded panel concurrency and calibrated structural selector; optional pair-CSR smoother only if charged evidence supports it. | Planned |
| M9 | Complete current-weight replay, all numerical quantities rebuilt, quality screening, reuse prefix/rebuild suffix once, fail-closed fallback, overlap memory admission and fresh/replay comparison. | Planned |
| M10 | Freeze candidate, dependencies and selector; qualify untouched holdout on Mac/SCC plus Windows correctness CI; publish measured limits, examples and release-candidate checkpoint. | Planned |

A milestone may require multiple focused PRs. Update its state and evidence at
each merge; never equate primitives with complete integration or performance
qualification. Start a small fully charged end-to-end benchmark at M4 and rerun
it after every subsequent performance milestone. If a gate fails, document the
failure and repair the relevant layer before moving on; do not weaken a frozen
gate after observing its holdout.

## Ownership and public boundaries

1. `PreparedThreeWayTopology` owns canonical tuples, components and optional
   observation groups; retain no numerical weights there.
2. `ThreeWayOperatorView` borrows an exact immutable numerical frame. Share the
   scalar kernels with `ThreeWayProblem`; views do not copy topology or weights.
3. `PreparedHierarchyTopology` owns a level inventory and owner-indexed maps.
   Avoid self-referential owning Rust structs. External one-transition borrowing
   maps remain useful, but complete hierarchy construction needs an owning
   structural inventory and explicit validated indices between its levels.
4. `ThreeWayNumericalHierarchy` borrows that structure and the fine frame, and
   owns current coarse weights, diagonals, smoother state and terminal factors.
   Never expose an API that silently combines different numerical generations.
5. An `ExecutionPlan` and caller-owned `SolveWorkspace` separate thread/RHS
   scheduling from shared immutable solver data. Return explicit native status,
   independent certificate, work counts and a scoped memory report.
6. Keep convenience APIs as allocating wrappers over shared numerical cores.
   Validate owners, dimensions and workspace capacity before public output writes.
   Numerical failures must not publish a successful result or partial candidate.

## Memory layout and CPU hypotheses

Let E be unique tuple count, V total coefficient count, P worker count and K
panel width. Preserve canonical lexicographic `Vec<[u32; 3]>` (12E bytes). Do not
pay for duplicated tuple layouts by default. Evaluate two additional factor
permutations with 32-bit tuple IDs when E fits, otherwise checked 64-bit IDs.
Factor 0 groups are implicit in canonical order. Group offsets cost roughly
8(V + 3) bytes; two 32-bit permutations cost 8E. Count capacity, construction
scratch and old/new overlap separately from logical payload.

Compare three exact Gramian kernels: serial tuple gather/scatter; coefficient
rows gathering grouped tuples; and two-pass tuple-image then grouped reduction
with E*K temporary values. Row-owned output avoids floating-point atomics and
P*V reduction buffers. Benchmark extra index traffic against rereading RHS and
weights; none is assumed universally fastest. Compiler vectorization on stable
Rust precedes explicit SIMD experiments.

Symmetric MAP retains its mathematical factor order and barriers. Rows within
one factor may run independently. Remove the first forward and last reverse
empty coupling scans. An optional pair-CSR MAP route shares represented pair
weights between forward/reverse actions and is considered only when summed
unique pair edges <= 1.5E and memory admission passes. Pair CSR is a smoother
representation, not a replacement for the original tuple operator/certificate.

Flatten reusable scratch after a liveness audit: share pre/post residual storage,
fuse prolongation with addition, avoid duplicate compatible RHS buffers and
unneeded fills, and retain temporary transactional output only where necessary.
For repeated candidate certificates, evaluate computing the original `B'Wy`
reference and its norm once per exact RHS/weight-frame scope. It must preserve
reference arithmetic and fail closed on any owner/target change; all preparation
and failed certificate work remains charged. Reusing a passing final candidate
certificate is a separate lifetime/immutability question, not an assumed free
optimization. Both are M5 hypotheses prompted by the new continuation route.

Build terminal matrices directly in the dense library's native layout instead
of row vectors plus flattened and transposed copies. Account for LSMR's local
reorthogonalization history explicitly (two windows of eight coefficient vectors
already cost 16V values per lane); never remove it merely to claim low memory.

## Scalable hierarchy construction

The legacy automatic path creates dense one-level coarse factors during each
candidate screen, before later hierarchy budgets can reject it. The new builder
must first construct bounded structural map candidates, enforce tuple/dimension
and memory admission, then assemble a map sequence. Build numerical state from
the bottom upward and screen the actual recursive cycle at each accepted level.
Share structural candidate work across smoother trials. Use component-local
depths so small components terminate without holding back difficult components.

Cap dense terminals at 256 coefficient coordinates per component. Larger
terminals require a separately admitted fixed generation-bound sparse within/
Schwarz action, with symmetry/range/positivity checks for PCG use. No RHS-dependent
inner stopping in a nominally fixed PCG preconditioner. Reject/fallback when the
sparse terminal or complete cycle fails. Component count is not numerical rank.
Define this as a new policy; preserve all frozen issue-3/issue-4 policies.

## Parallelization and RHS execution

Use one explicit caller-bounded Rayon pool. Support serial execution, parallel
work within a single RHS, or independent RHS/panels whose inner actions are
serial. Avoid unrestricted nested pools, including inside the dependency fork.
Partition by tuple work rather than row count. Split exceptional hub rows into
fixed segments and combine partials in a documented order. Deterministic
reductions and fixed partitioning must repeat at a fixed configuration.

Panels use coefficient-major, lane-minor storage and widths 2, 4 or 8. Fuse
operator, projection and transfer traversal across independent Krylov lanes;
do not silently introduce block Krylov. Track convergence per lane, retire
certified lanes, charge unused lane work, and handle scalar tails. Compare fused
panels with parallel scalar solves under the same memory/thread budget.

Choose routes from frozen offline constants and available structural/numerical
features (E, V, grouped degree distribution, pair counts, RHS width, admitted
bytes and worker budget). No elapsed-time online routing. Freeze the simple
cost model and supported regimes before the holdout. Test Mac threads 1,2,4,8,
16,24; SCC 1–16 before declared 28/32-core and socket-placement experiments.

## Changing weights

Recompute every coarse weight, pair conductance/degree, smoother coefficient,
rank diagnostic and factor from the current frame. Screening may reuse a valid
prefix; rebuild the first rejected suffix once, screen again, then reject to
the baseline if needed. Support changes require a new prepared topology. Charge
screening, rejected builds, overlap and fallback. Test gradual changes, shocks,
component-local degeneration and formerly good maps becoming poor; compare
with fresh builds and verify no retained-memory growth over repeated frames.

## Delivery discipline

For every increment: inspect/sync main and preserve unrelated work; implement
small reviewable changes; run Rust 1.85 required and relevant scientific checks;
commit and push a focused `codex/` branch; open a milestone PR; review its final
ownership, numerical, cost and evidence boundaries; wait for exact-head and PR
CI; merge only green reviewed work; verify the actual resulting main before
the next increment. Pin dependency changes in a separate integration PR after
the fork's own checks. Identify measured source separately from later results
commits. A self-review is not independent external review.

Keep this ledger, `PERFORMANCE_PROTOCOL.md`, `ROADMAP.md`, README, relevant ADRs
and result documents synchronized. Update the external vault's project status
and reverse-chronological timeline at milestone closure. Raw logs/builds remain
outside the vault. Final release publication and fereg changes require separate
work; this campaign ends at a verified standalone candidate and honest evidence.

## Checkpoint receipts

- M0: PR #34 merged to `170269976c2ce98b968c765d3fdd8a5db16aad13`.
  Reviewed source `bc6be61e40c32ae62251f3d53395c98b0265a751`; push CI
  34050885584/34050885611 and PR CI 34050891406/34050891411 all passed.
- M1: boundary contracts and regression scope are documented in
  [`NUMERICAL_BOUNDARIES.md`](NUMERICAL_BOUNDARIES.md). Source qualification
  passed locally on Rust 1.85. PR #35 merged as `0cfb9f348bad536b5496a850e656fca3306a3a43`;
  reviewed source `1d7fff0cc837d02743dac177918ea515601df4ba`, push workflows
  34051329176/34051329194 and PR workflows 34051333405/34051333432 all passed.

- M2: shared scalar operators and exact numerical ownership are documented in
  [`ISSUE5_OPERATOR_VIEWS.md`](ISSUE5_OPERATOR_VIEWS.md). Complete solver and
  numerical hierarchy integration remain M3/M4 work.

- M2: PR #36 merged as `f6a634af127f9f89b11606339cfd4f9aad8281e6`.
  Source `a0bf566bdbdc976cd818c048a41fdc4e58649fd6`; push workflows
  34051783143/34051783162, PR workflows 34051806535/34051806542 and
  post-merge workflows 34051949685/34051949697 all passed.
- M3: owner-controlled `johannes-schmieder/within` PR #1 targets its dedicated
  `multiwaymg` branch at frozen upstream b7779cb, preserving upstream main.
  The prepared path uses explicit serial internal execution; existing allocating
  wrappers retain legacy Rayon thresholds. Controlled parallelism remains M7.
  Joint dependency-pin integration requires qualified fork source.

- M3 fork PR #1 merged as `2e7d5ec935b4846b369430ff00deb59f90e7d2d5` after
  reviewed source `e8a3561463018762e16396e3ee0f8af8699e40ff` passed exact-source
  workspace CI 34053279313, PR workspace CI 34053282380 and inherited full CI
  34053282377. See [`ISSUE5_LSMR_DEPENDENCY.md`](ISSUE5_LSMR_DEPENDENCY.md).

- M3: PR #37 merged as `1ca043dfbaacb7f1ec25d919b17d1e8850f6dba0`.
  Reviewed source `fa2b39ff5ef94b5aec5e5c63ea6a1c3fb88c19ff`; source workflows
  34054067320/34054067276 and PR workflows 34054069461/34054069466 passed.
  Post-merge workflows 34054356919/34054356938 passed on the identical tree.
  The fork's merged pin also passed post-merge workflow 34053690619.
- M4 is split into reviewable increments without changing its completion gate:
  M4a owns supplied-map topology and complete current-frame numerical replay;
  M4b shares projection/MAP arithmetic with frame-based caller scratch; M4c
  integrates the fixed serial cycle, PCG/LSMR, certification and repeated RHS.
  The complete M4 milestone remains open until the entire vertical slice passes.

- M4a: reviewed source `8d9c4bf16644b9a9ce817650047cc81901c98f8e` passed
  source workflows 34054714391/34054714389 and PR workflows
  34054716149/34054716213; PR #38 merged. See
  [`ISSUE5_PREPARED_HIERARCHY.md`](ISSUE5_PREPARED_HIERARCHY.md).
- M4b: prepared projection/MAP share ordinary scalar kernels and bind caller
  scratch to the exact current frame. The integrated allocation test now uses
  prepared MAP directly, eliminating its independently constructed ordinary
  problem. See [`ISSUE5_PREPARED_MAP.md`](ISSUE5_PREPARED_MAP.md).

- M4a merged as `d898a08dfeaca10507bcce23bb872c937edb7f7c`; post-merge workflows
  34054976958/34054976971 passed on the same qualified tree.
- M4b PR #39 merged as `bb33c259beae9a98f92dcb3b8025c7d177b3d9a7`.
  Reviewed source `c629c2d7e3c8ab8bc1c4255d29f98ade3811e01b` passed push workflows
  34055195957/34055195967 and PR workflows 34055198201/34055198223;
  post-merge workflows 34055570517/34055570575 passed.
- M4c fixed-cycle increment shares the ordinary recurrence, adds a whole-terminal
  cap of 256 coefficients and depth cap of 64 levels, and counts complete retained
  application payload. Prepared native dense assembly moves forward from M5 to
  avoid manufacturing an ordinary problem just to factor the terminal. M4 remains
  open for outer drivers, certificate, bounded RHS reuse and complete economics.
  See [`ISSUE5_PREPARED_CYCLE.md`](ISSUE5_PREPARED_CYCLE.md).

- M4c cycle PR #40 merged as `347210d612b3b374c5b52bc7d2f30d41ef244447`.
  Reviewed source `aa98a53bc376665c14a24c7d561f1250b76596d9` passed source workflows
  34055873544/34055873557 and PR workflows 34055875700/34055875819;
  post-merge workflows 34056313663/34056313711 passed on the identical tree.
- M4c complete LSMR increment integrates the exact current-frame cycle, shared
  original-operator certificate, native/accepted diagnostic separation, complete
  payload admission/work counts and bounded scalar RHS1–32 reuse. PCG and a
  complete-cost serial benchmark surface remain required before M4 closes.
  See [`ISSUE5_PREPARED_LSMR.md`](ISSUE5_PREPARED_LSMR.md).

- M4c LSMR PR #41 merged as `cb0b3cf2fd41385c91690c515013ee591496c263`.
  Reviewed source `8f0dbb5671ec97745bcbf66ea159d057a380e0c6` passed source workflows
  34056669213/34056669197 and PR workflows 34056671815/34056671806;
  post-merge workflows 34057003019/34057003032 passed on the identical tree.
- M4c prepared PCG shares the untraced recurrence, complete hierarchy/certificate
  storage, independent acceptance, actual work and bounded RHS reuse. A local
  frozen-source comparison passes 144 coefficient/diagnostic bit comparisons;
  full platform/scientific CI remains required. Complete-cost serial economics
  reporting is the remaining M4 closure item. See
  [`ISSUE5_PREPARED_PCG.md`](ISSUE5_PREPARED_PCG.md).

- M4c PCG PR #42 merged as `3bb2d3f9424f39754463cf85d155ba48b9ec3a04`;
  post-merge workflows `34057809493` and `34057809500` passed. The final M4
  [serial evidence increment](ISSUE5_SERIAL_BENCHMARK.md) freezes bounded
  canonical-input smoke/development recipes, complete cold-process and RHS-prefix
  costs, reserved payload/RSS/work scopes and adversarial evidence validation.
  Commit source and recipe before timed collection; retain all negative results.
  M4 stays open until this evidence surface is checked and merged.

- M4 serial source `9b4f1740ce5e9bbb399059fe1615adbd10820ab7` was committed
  before Mac smoke/development and Linux smoke collection. All three complete
  artifacts pass the evidence gate and fixed-configuration repeatability. Each
  has 504 processes and 4,800 measured RHS columns. PCG certifies 2,400/2,400 in
  every run; LSMR certifies 1,840/2,400 in both smokes and 1,465/2,400 in Mac
  development. All rejections are native normal-equation stops missing the
  independent 1e-8 original-operator tolerance. No timeout or process error.
  Preserve the [v1 negative baseline](../benchmarks/results/2026-09-06/prepared-serial-v1/README.md).
- M4 now requires one additional correctness/completion increment before M5:
  a separate certificate-aware LSMR route that vetoes early native stops before
  the dependency's terminal audit clobbers recurrence vectors. Continue the same
  Krylov recurrence after a veto; keep the legacy/native route unchanged. Charge
  every candidate certificate, handle exact breakdown/iteration limits, retain
  independent final certification and zero-allocation storage. Use a separately
  declared development comparison, not retuned or overwritten v1 results.

- M4 serial evidence PR #43 merged as `285f1e9d07c2880f6975a39b302178b0cb24349c`.
  Final source/PR workflows `34059873790`/`34059873640` and
  `34059875980`/`34059875996` passed; post-merge `34060024104`/`34060024117` passed.
- M4d dependency candidate gate qualified and merged in within PR #2 as
  `cb20b27a7137804202be39976415618686a144de`; its post-merge CI passed. Both
  downstream pins now move together to that revision. The [separate prepared
  certificate-gated route](ISSUE5_CERTIFICATE_GATED_LSMR.md) reuses all arrays,
  preserves native APIs, records every extra check/veto and retains fresh final
  independent acceptance. After its source/PR qualification, freeze a distinct
  three-route comparison (PCG, native LSMR, gated LSMR) on the unchanged v1
  development inputs, with five rotated repetitions and all candidate work.
  Record full gated coverage before closing M4 or moving to M5 optimization.

- M4d prepared certificate gate PR #44 merged as
  `ebd9dd6ef25a7a8d24de90682ce333089ae7601f` after source workflows
  `34061787307`/`34061787345` and PR workflows `34061788640`/`34061788626` passed.
- M4e [gated serial comparison v2](ISSUE5_GATED_SERIAL_COMPARISON.md) adds the
  gated route on unchanged v1 inputs/tolerances. Keep the native negative
  control in full coverage, require complete PCG/gated coverage separately,
  and charge all candidate/final certificate/projection work in probe schema 2.
  Policy/source must be committed before Mac smoke/development collection.

- M4d post-merge workflows `34062454335` and `34062454329` passed. The v2
  comparison code passes the required local Rust 1.85 checks and the extended
  evidence suite before its recipe/source commit. No v2 timings have been
  collected at this checkpoint.

- M4e measured source `3c1273eb00698c59b27eb79d1539c137fe3632a9` passes the
  [frozen v2 coverage gate](../benchmarks/results/2026-09-06/prepared-serial-gated-v2/README.md):
  PCG and gated LSMR each certify 2,400/2,400 columns on Mac smoke/development
  and Linux smoke. Native control retains 560 smoke and 935 development rejects.
  All fixed-config numerical/work/payload repeats pass, with no process errors,
  timeouts or RSS-budget failures. Original Mac native/PCG numerical results
  match v1 in all 504 corresponding processes per profile. Gated/native LSMR
  retain identical array capacities. Source workflows `34063265592`/`34063265572`
  and PR workflows `34063297969`/`34063297964` passed. Final evidence-head CI and
  merge are the remaining M4 boundary; no competitive qualification is claimed.

- M4e PR #45 merged as `4fa6401c3421bd61173e047291ca4e534940f1e7` after
  final-source workflows `34063604680`/`34063604715` and PR workflows
  `34063606533`/`34063606524` passed. Post-merge `34063748407`/`34063748392`
  also passed. M4 is complete. M5a begins with empty MAP scan removal and
  proven scratch reuse, preserving the M4 executable for paired comparisons.

- M5a [serial scratch audit](ISSUE5_SERIAL_SCRATCH.md) removes two empty MAP
  tuple scans, reuses forward storage for rounded middle values, and reduces
  each cycle transition from seven traversal vectors to four. The fixed cycle,
  factor/tuple/projection order and every gate/certificate remain unchanged.
  Frozen-loop signed-zero/subnormal/poisoned-scratch tests pass alongside all
  required Rust 1.85 checks, numerical gates and exact allocation/release.
  A two-transition solve now reserves 40 LSMR or 34 PCG arrays (previously
  48/42); no first/repeated RHS action allocates. All 61 Python validators pass.
  The separate paired protocol fixes original v2 inputs and baseline binaries,
  retains native negatives, verifies exact numerical/work equivalence and
  the derived capacity savings, and reports all paired time/RSS ratios. Commit
  this source and recipe before collecting; no M5 timing exists at this checkpoint.

- M5a source `754f6d622d6040c968dd931d7e52973fa42c517a` passes the
  [frozen paired smoke/development gates](../benchmarks/results/2026-09-06/prepared-serial-m5a/README.md):
  complete numerical/work/fingerprint agreement and exact 1,656/21,816-byte
  hierarchy-workspace savings. Every candidate-route development cell has
  inner median speedup above one; balanced PCG/gated geomeans are 1.0610x/1.0629x.
  Linux source smoke also passes candidate coverage; all native negatives persist.
  Source workflows `34064382081`/`34064382036` and PR workflows
  `34064411985`/`34064412162` passed. Final evidence-head CI/merge remains.
  Next profile individual kernels and enlarged development dimensions before
  choosing grouped indices; continue flat scratch/fused transfer/certificate
  reference work as separately reviewed increments. No competitive claim.

The next [grouped-layout experiment design](ISSUE5_GROUPED_LAYOUT_DESIGN.md) specifies
exact owner binding, stable counting placement, u32/checked-wide indices, construction
cursors, three traffic alternatives and later thread/panel ownership. It is a design
for measurement, not an implemented layout or selected default.

- M5a PR #46 merged as `c5d623bc5323a92189664a16cce7b4238b431f0e` after
  final source workflows `34064935550`/`34064935556` and PR workflows
  `34064937694`/`34064937677` passed. Post-merge workflows
  `34065082134`/`34065082163` also passed.
- M5b [opt-in diagnostic profiling](ISSUE5_KERNEL_PROFILING.md) adds fixed
  current-thread nested attribution, actual level inventory and independently
  validated complete work/certificate accounting. Disabled builds contain no
  hooks. Active collection preserves solve bits/work and zero allocations.
  The frozen diagnostic policy covers existing smoke/development widths and a
  separately declared n4096, depth8, single-RHS expanded diagnostic. It is not
  authoritative performance evidence. Source/recipe must be committed before
  collection; no M5b timing exists at this checkpoint. Grouped layout remains
  a measured hypothesis; M5 and M6–M10 stay open.

- M5b source `42d6f527bdf09a2b38769d5724b88829cce97877` passes the
  [complete diagnostic gates](../benchmarks/results/2026-09-06/prepared-kernel-profile-v1/README.md):
  Mac smoke/development and Linux smoke each certify 4,800 measured columns;
  expanded RHS1 certifies 60/60. No invalid profiles, errors, timeouts or RSS
  failures. All 1,008 Mac smoke/development processes exactly match corresponding
  uninstrumented M5a input/numerical/work/fingerprint/payload records. Source
  workflows `34067385068`/`34067385070` and PR `34067398348`/`34067398347` passed.
  Final evidence-head CI/merge remains before the next increment.
- Diagnostic attribution prioritizes grouped MAP/Gramian above small transfer
  and certificate-reference savings. Expanded nonterminal tuple complexity is
  6.71x uniform / 5.58x communities / 1.81x chain; account for grouping at each
  level, compare explicit fine-only/all-nonterminal choices, and preserve scalar
  scatter. Coefficient halving alone is insufficient for M6 admission. These
  instrumented, cache-bounded development results select the next experiment,
  not a default layout or a competitive route. M5–M10 remain open.
