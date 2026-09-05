# Issue 4 final results: current pair-CMG does not pass the local-solver advancement gate

Date: 2026-09-05. Status: **completed**. This is a calibrated negative
advancement decision, not a fresh holdout and not a claim that CMG is generally
ineffective as a graph solver.

## Decision

The current fixed-CMG implementation is **not selected** as a production-shaped
pair-local solver for MultiwayMG:

- not universally on the finest three-way level;
- not by any stable size, topology, terminal, or component rule supported by the
  measured evidence;
- not as a finest-level-only portfolio;
- not on the tested non-finest levels of the recursive three-way hierarchy; and
- not as an automatic route requiring a new qualification holdout.

The public pinned `within` approximate-Cholesky/block-elimination route remains
the pair-local comparison baseline. Symmetric MAP remains the preferred cheap
three-way smoother where the complete-cycle gate admits it. The component-local
CMG adapter, reports, and benchmark harnesses remain valuable research controls
and make a future redesigned CMG candidate directly comparable.

A fresh issue-4 holdout was deliberately **not spent**. No calibrated candidate
met the joint advancement rule strongly enough to justify freezing a routing
policy before holdout evaluation.

## Question and advancement rule

Issue #4 separated the local pair solver from the genuinely three-way coarse
space. Each factor pair is a valid weighted bipartite graph Laplacian after a
sign change, so either fixed CMG or `within` can provide a pair correction. The
question was not whether CMG can solve those graphs. It was whether using the
current fixed CMG cycles inside MultiwayMG improves the complete three-way solve
on identical domains after charging setup, application, failed attempts, and
certification.

The provisional advancement rule required a structurally interpretable regime
with both:

1. a material outer-work reduction, normally at least 20 percent; and
2. positive fully charged setup-plus-solve economics at the relevant repeated-
   RHS width.

A favorable elapsed-time cell without lower work, or lower work without a
fully charged timing win, was not sufficient. Timing was descriptive rather
than a CI winner gate, but a candidate that was slower in every repeated-RHS
calibration did not qualify for a fresh holdout.

## Evidence sequence

### 1. Certified pair-local smoke

The first identical-domain matrix compared Jacobi, an exact pseudoinverse, one
fixed CMG cycle, and pinned `within` on five connected pair-graph families. It
contained 240 rows and 1,920 measured RHS solves, with cumulative prefixes 1,
4, 16, and 32. Every row passed the independent residual and accounting gate;
the maximum relative normal residual was `8.307600e-11`.

The local evidence was deliberately mixed:

| Pair graph | Within time / CMG time at 32 RHS | CMG / within outer work | Interpretation |
|---|---:|---:|---|
| Weak communities | 2.855 | 0.318 | Clear local CMG signal in this small matrix. |
| Dense | 1.274 | 0.639 | CMG beat `within`, but Jacobi was cheaper. |
| Hubs | 9.796 | 0.119 | Apparent win came from a one-level iterative terminal; Jacobi was cheaper. |
| Path | 0.131 | 9.850 | `within` was decisively better. |
| Six-order weight variation | 0.508 | 1.831 | `within` was better by 32 RHS. |

Terminal reasons prevented a false multilevel interpretation of the hub result,
but did not produce a safe routing rule. See
[`ISSUE4_PAIR_LOCAL_RESULTS.md`](ISSUE4_PAIR_LOCAL_RESULTS.md) and the preserved
artifact under
[`benchmarks/results/2026-09-04/issue4-pair-local-smoke/`](../benchmarks/results/2026-09-04/issue4-pair-local-smoke/).

### 2. Whole-system three-way comparison

The next harness assembled diagonal, all-pair fixed-CMG Schwarz, and pinned
`within` Schwarz on the same complete `ThreeWayProblem` objects under modified
LSMR and traced projected PCG. The issue-3 coarse hierarchy was omitted so that
local-solver effects were not confounded with the global three-way correction.

At the first scale, CMG used more outer work than `within` on every family. A
larger calibration profile revealed a real but selective scale crossover on
balanced dense-coupling designs:

| Larger calibration case | CMG / within LSMR work | CMG / within PCG work |
|---|---:|---:|
| Planted clones | 0.809 | 0.784 |
| Noisy clones | 0.852 | 0.871 |
| Latin square | 0.852 | 0.833 |
| Weak chain | 2.276 | 2.417 |
| Disconnected Latin | 1.211 | 1.250 |
| Unbalanced cycle | 1.513 | 1.618 |

The balanced cases established that CMG's relative spectral quality can improve
with component scale. They did not establish economic superiority: fully
charged four-RHS timing still favored `within` in every larger case. Moreover,
CMG's largest advantages over diagonal occurred on weak-chain and unbalanced
systems where `within` was stronger still. A scalar “hard problem implies CMG”
rule was therefore rejected. See
[`ISSUE4_WHOLE_SYSTEM_RESULTS.md`](ISSUE4_WHOLE_SYSTEM_RESULTS.md).

### 3. Frozen balanced size ladder

A dedicated ladder then crossed planted-clone, noisy-clone, and Latin-square
families with 12, 18, 24, 36, 48, and 72 levels per factor, both outer solvers,
two independent builds, and RHS prefixes through 32. The 864 reported rows
represent 6,912 underlying solves and passed the numerical/accounting gate.

The strongest point was planted clones with 48 levels per factor:

- CMG / `within` LSMR work: `0.785`, a 21.5 percent reduction;
- CMG / `within` PCG work: `0.758`, a 24.2 percent reduction.

That crossed the provisional work threshold, but **no family/size/solver cell
produced a fully charged CMG timing win at any measured prefix through 32 RHS**.
The effect was also not monotone: noisy clones lost the work advantage again at
72 levels. Direct-versus-iterative terminal classification did not separate
favorable from unfavorable cells. The evidence therefore did not support a
frozen pre-solve selector. See the frozen
[`size-ladder summary`](../evidence/issue4-size-ladder/SUMMARY.md).

### 4. CMG only on non-finest hierarchy levels

The final plausible role for the current implementation kept the fine `within`
smoother fixed and replaced only non-finest local pair solvers inside the
recursive three-way hierarchy. The three-way map sequence was identical across
methods.

The automatic planner admitted only one of the eight already-revealed issue-3
recursive fixtures. On that nearly-nested depth-2 case, coarse CMG changed
32-RHS outer work by exactly zero percent under both solvers and was slower when
fully charged. See the
[`automatic-map summary`](../evidence/issue4-coarse-cmg/AUTOMATIC_SUMMARY.md).

An oracle-map controlled calibration used the already-revealed issue-3 map
sequences to isolate the local non-finest solver. One communities depth-3
hierarchy was excluded symmetrically because the all-`within` baseline itself
failed the outer SPD/certification gate. Across the remaining seven fixtures
and fourteen solver cells:

- `0/14` achieved at least a 20 percent outer-work reduction;
- outer-work ratios ranged only from approximately `0.985` to `1.034`;
- `2/14` had a fully charged timing win at 32 RHS, both on weak-chain depth 2;
- both timing-winning cells used **more** outer work than `within`; and
- `0/14` met the joint advancement rule.

See the
[`oracle-map summary`](../evidence/issue4-coarse-cmg/ORACLE_SUMMARY.md).

## Final gate assessment

| Gate | Result | Basis |
|---|---|---|
| Algebra, symmetry, and independent residual certification | **PASS** | Pair-local, whole-system, size-ladder, and coarse-level matrices. |
| Honest setup, solve, failure, and operator-work accounting | **PASS** | Fail-closed validators and permanent regression tests. |
| Broad or stable ≥20% outer-work reduction versus `within` | **FAIL** | Isolated balanced-size cells only; nonmonotone and topology-dependent. |
| Positive fully charged economics at measured RHS widths | **FAIL** | No finest-level ladder win through 32 RHS; coarse timing wins lacked lower work. |
| Deterministic pre-solve routing rule | **FAIL** | Size, terminal reason, and simple difficulty controls do not separate outcomes. |
| Fresh holdout qualification | **NOT SPENT** | No calibrated candidate met the joint advancement rule. |
| Production memory/thread/reweighting qualification | **TRANSFERRED** | Owned by issue #5; it cannot rescue the current failed work-plus-time candidate. |

The negative decision is stronger than “more benchmarking is needed.” The
current fixed CMG local action was tested in the roles that could plausibly
matter for this architecture: alone on identical pair domains, assembled on the
whole three-way system, across a repeated-RHS size ladder, and only on coarse
hierarchy levels. None yielded a stable candidate satisfying the joint gate.

## Architectural consequences

1. **Keep the three-way hierarchy.** Issues #2 and #3 showed that a good
   factor-respecting coarse space contributes global information that pair
   solves alone cannot provide. That remains the distinctive MultiwayMG result.
2. **Keep `within` as the pair-local baseline.** It is the stronger current
   production-shaped local solver across the complete evidence sequence.
3. **Keep CMG as an explicit research control.** The adapter, terminal reports,
   and harnesses are useful for diagnosing graph difficulty and evaluating a
   future CMG redesign without changing production routing.
4. **Do not infer production endorsement from the issue-3 CMG fallback.** That
   portfolio is retained for frozen scientific reproducibility and complete-
   cycle research. A downstream automatic route must be requalified under the
   policy in ADR 0002.
5. **Do not spend a holdout after a failed calibration.** Any materially changed
   CMG algorithm must first create a new, predeclared calibration candidate;
   only then should its policy be frozen and evaluated on fresh fixtures.

The decision is recorded in
[`ADR_0002_ISSUE4_PAIR_SOLVER_POLICY.md`](ADR_0002_ISSUE4_PAIR_SOLVER_POLICY.md).

## Reproducibility and review

The issue-4 program was implemented and reviewed through PRs #12–#18. The final
coarse-level increment was squash-merged as
`5ed12cf88ad1d29328e6930ed98683485cf50ff6` after an independent post-open diff
review found and repaired a material validator gap: the original validator
could infer its fixture universe from observed rows. The permanent validator now
freezes all eight recursive fixtures and audits schema, coverage, hierarchy
shape, charged totals, prefix monotonicity, solver accounting, CMG terminal
coverage, fallback allocations, and symmetric baseline exclusion.

The strengthened validator revalidated both archived coarse-level raw TSVs and
regenerated byte-identical summaries. On the exact merge head, the full Rust CI
matrix and the permanent validator passed both before and after merge.

Key preserved evidence:

- pair-local run `33919335599`, artifact `9954398642`;
- whole-system larger calibration run `33934212856`, artifact `9959613244`;
- size-ladder run `33934987774`;
- coarse-level final calibration run `33937258900`, artifact `9960594295`;
- coarse artifact digest
  `sha256:3033802c218646cfdff69bb9a070f495609de641ec8717ed578aa1c3cc3a963a`;
- post-merge full CI run `33951031732`; and
- post-merge permanent-validator run `33951031743`.

## Next milestone

Issue #5 is now the primary development milestone: prepared immutable topology,
caller-owned allocation-free workspaces, exact memory accounting, repeated-RHS
kernels, and generation-safe numerical replay under changing positive weights.
That work should use the current `within`/MAP local baseline while retaining CMG
as an explicit comparator. A future private fereg integration remains issue #6
and must preserve fereg's original observation-space certification and fallback
contracts.
