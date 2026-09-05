# ADR 0002: Keep `within` as the pair-local baseline

- **Status:** Accepted
- **Date:** 2026-09-05
- **Decision owners:** MultiwayMG numerical research maintainers
- **Related issues:** #4, #5, #6
- **Supersedes:** no earlier ADR

## Context

Every three-way tuple contributes to three valid factor-pair graph systems.
After a factor sign change, each local system is a weighted bipartite graph
Laplacian. MultiwayMG can therefore build pairwise Schwarz corrections from
several mathematically valid local actions, including:

- the pinned public `within` approximate-Cholesky/block-elimination solver;
- fixed CMG cycles;
- exact pair pseudoinverses on small reference domains; and
- cheap positive backgrounds such as Jacobi or symmetric MAP.

Issue #2 established that the genuinely three-way factor-respecting coarse space
adds important global information beyond pair corrections. Issue #3 retained a
MAP-first complete-cycle portfolio with all-pair fixed CMG evaluated only after
MAP rejection. Issue #4 was created to determine whether current fixed CMG
should be the broad, selective, finest-level, coarse-level, or reference-only
pair solver in a production-shaped MultiwayMG path.

The issue-4 evidence proceeded from identical pair domains to complete
three-way systems, a frozen balanced size ladder, and a controlled experiment
that replaced only non-finest pair solvers inside the recursive hierarchy. All
reported comparison rows were independently certified against the submitted
operator and charged setup, solve, failed-route, and certificate work.

CMG produced some genuine local and scale-dependent outer-work improvements.
The strongest balanced size-ladder point reduced LSMR work by about 21.5 percent
and PCG work by about 24.2 percent relative to `within`. However:

- no finest-level family/size/solver cell produced a fully charged timing win at
  any measured RHS prefix through 32;
- the work crossover was nonmonotone and did not admit a stable selector from
  size, topology, or terminal metadata;
- on weak-chain, disconnected, unbalanced, path, and dynamic-weight controls,
  `within` often remained substantially stronger;
- replacing only non-finest `within` solvers with CMG yielded no material outer-
  work reduction across the controlled oracle-map calibration; and
- the two coarse-level timing wins both used more outer work than `within`.

No calibrated candidate therefore met the joint advancement gate of material
work reduction plus positive fully charged economics.

## Decision

### Pair-local policy

Use the pinned public `within` route as the pair-local comparison and production-
shaped baseline for the next MultiwayMG milestones. Prefer symmetric MAP as the
cheap complete-cycle smoother where it passes the declared issue-3 cycle gate.
Do not route ordinary pair components to the current fixed-CMG implementation
by default.

### CMG status

Retain current pair-CMG as an explicit research and diagnostic route:

- preserve the component-local adapter;
- preserve exact CMG terminal, hierarchy, warning, work, and memory-boundary
  reports;
- preserve pair-local, whole-system, size-ladder, and coarse-level harnesses;
- preserve negative evidence and validators; and
- continue to permit explicit CMG experiments and comparisons.

This retention is not a production endorsement and does not authorize an
automatic size-, topology-, component-, or terminal-based routing rule.

### Frozen issue-3 portfolio

Do not rewrite or reinterpret the frozen issue-3 scientific evidence. Its
MAP-first / CMG-after-MAP-rejection portfolio remains available for reproducible
complete-cycle research. Downstream production integration must not treat that
historical fallback as evidence that CMG passed issue #4.

A future production cycle should use `within`/MAP local actions unless a new CMG
implementation independently passes the requalification rule below.

### Holdout policy

Do not spend a fresh issue-4 holdout for the current implementation. A holdout
qualifies a frozen candidate; it is not a substitute for creating one during
calibration. Since no candidate passed the joint gate, the correct outcome is a
negative advancement decision.

### Ownership of remaining engineering work

Move prepared topology, allocation-free workspaces, exact retained/peak memory,
thread scaling, repeated-RHS kernels, and changing-weight numerical replay to
issue #5. These are important production-engineering requirements, but they do
not reopen the current pair-solver selection: they cannot by themselves turn a
method with no stable work-plus-time candidate into a qualified route.

Issue #5 should retain CMG as a comparator while optimizing around the
`within`/MAP local baseline. Issue #6 may integrate MultiwayMG privately into
fereg only after generation safety, memory admission, and original-space
certification interfaces are ready.

## Requalification rule

A future materially changed CMG local solver may supersede this ADR only after:

1. defining the algorithm and all routing inputs before a new comparison;
2. comparing it with the pinned `within` baseline on identical domains and
   complete three-way systems;
3. charging setup, workspaces, failed routes, certification, and relevant
   repeated-RHS widths;
4. demonstrating a material and structurally interpretable outer-work reduction;
5. demonstrating positive fully charged economics in the same regime;
6. freezing a deterministic pre-solve policy that does not use elapsed time;
7. passing a fresh preregistered holdout and cross-platform qualification; and
8. updating this ADR or replacing it with an explicitly superseding decision.

A redesigned CMG need not win universally. A selective route is acceptable if
the selector is auditable, calibrated before the holdout, and safely rejects
other cases.

## Consequences

### Positive

- The next engineering milestone has a single mature pair-local baseline rather
  than an unqualified adaptive portfolio.
- MultiwayMG preserves its distinctive three-way coarse-space contribution
  without making success depend on CMG winning every local graph solve.
- Negative results remain reproducible, and future CMG improvements have ready-
  made identical-domain and whole-system comparators.
- No fresh holdout is contaminated by selecting a rule after observing its
  outcomes.
- Downstream fereg integration can distinguish a scientific three-way hierarchy
  experiment from a local-solver routing experiment.

### Negative

- The CMG dependency and research adapter remain in the repository even though
  they are not the selected local baseline.
- Some balanced large pair domains may leave Krylov-work reductions unrealized
  until CMG setup/application economics improve materially.
- The issue-3 research portfolio and the production policy differ, so
  documentation must continue to state that distinction clearly.

### Neutral

- This ADR does not change CMG itself, delete any experimental route, or claim
  that `within` is universally optimal outside the tested MultiwayMG context.
- It does not choose the final outer solver for fereg. Modified LSMR remains the
  rank-robust production candidate, while projected PCG remains a controlled
  Gramian diagnostic.
- It does not weaken the complete-cycle, structural, memory, or original-
  operator certification requirements.

## Evidence

The detailed synthesis and provenance are in
[`ISSUE4_FINAL_RESULTS.md`](ISSUE4_FINAL_RESULTS.md). The underlying checkpoints
are:

- [`ISSUE4_PAIR_LOCAL_RESULTS.md`](ISSUE4_PAIR_LOCAL_RESULTS.md);
- [`ISSUE4_WHOLE_SYSTEM_RESULTS.md`](ISSUE4_WHOLE_SYSTEM_RESULTS.md);
- [`evidence/issue4-size-ladder/SUMMARY.md`](../evidence/issue4-size-ladder/SUMMARY.md);
- [`evidence/issue4-coarse-cmg/AUTOMATIC_SUMMARY.md`](../evidence/issue4-coarse-cmg/AUTOMATIC_SUMMARY.md); and
- [`evidence/issue4-coarse-cmg/ORACLE_SUMMARY.md`](../evidence/issue4-coarse-cmg/ORACLE_SUMMARY.md).
