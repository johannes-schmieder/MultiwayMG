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
| M2 | Frame-bound matrix-free operator views sharing existing arithmetic, exact-owner checks, static validation before mutation, zero-allocation operators, adjoint/Galerkin tests. | Implemented; PR qualification pending |
| M3 | Narrow pinned within fork: caller-owned LSMR workspace, mutable action adapters, explicit execution, allocating wrappers over the same recurrence; equivalence and allocation gates. | Planned |
| M4 | Complete prepared serial supplied-map solve: numerical MAP hierarchy, PCG and LSMR, certificate workspace, bounded scalar RHS reuse, ownership/memory report; compare fresh construction and dense references. | Planned |
| M5 | Measure and reduce serial memory traffic: grouped indices, remove empty MAP passes, scratch liveness arena, fused prolong-add, direct dense assembly, replace expensive tree-based setup where measured. | Planned |
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
