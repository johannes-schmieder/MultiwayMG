# Architecture

## Dependency direction

```text
CMG --------------------+
                        |
schwarz-precond --------+--> multiway-mg --> downstream estimators
                              ^
                              |
                       multiway-incidence
```

`multiway-incidence` owns the matrix class and remains independent of graph
solvers. `multiway-mg` owns the three-way numerical research layer. Downstream
packages such as `fereg` own estimator semantics, routing, result construction,
and final observation-space certification.

## Structural and numerical state

`ThreeWayTopology` stores factor counts and canonical unique tuples.
`ThreeWayProblem` adds the current positive weights, diagonal information, and
component metadata. This separation is the foundation for issue #5's prepared
state and changing-weight replay: symbolic topology may eventually be reused,
but numerical hierarchies, pair conductances, smoothers, and terminals must
never be silently reused across incompatible weight generations.

## Incidence kernels

The incidence layer provides:

- deterministic tuple validation and duplicate collapse;
- matrix-free `B`, `B'`, `sqrt(W)B`, and `B'WB`;
- component discovery and structural factor-shift projection; and
- exact hard factor-respecting Galerkin coarsening by tuple remapping and
  deterministic duplicate merging.

## Automatic three-way coarse space

Issue #3 established the current research baseline.

1. Build a bounded pair-neighborhood hard aggregation within each factor.
2. Reject maps that cross exact incidence components or violate declared
   dimension, tuple-reduction, or tuple-complexity budgets.
3. Build and probe the intended complete two-grid cycle.
4. Prefer a fixed symmetric-MAP cycle.
5. Evaluate an all-pair fixed-CMG cycle only after MAP rejection.
6. Accept only a fully screened cycle; otherwise return an explicit
   no-hierarchy result.

Compatible relaxation, relaxed-signature bootstrap matching, and both
witness-repair mechanisms remain research diagnostics. The frozen issue #3
evidence did not show a material value-add over the protected structural
pair-neighborhood baseline.

Recursive pair-neighborhood construction is also retained as a research path.
It is numerically promising, but cumulative dimension and tuple complexity must
be charged before production admission.

The issue-3 MAP/CMG portfolio is a frozen scientific result. Its availability in
the research API does not imply that CMG passed the later pair-solver economics
gate. A production integration must apply ADR 0002 rather than infer routing
policy from the historical experimental fallback.

## Pair solver layer

Each of the three factor-pair marginals is a weighted bipartite graph. After a
sign change of the second factor block, the local Gramian becomes a genuine
graph Laplacian. This permits either the existing `within`
approximate-Cholesky/block-elimination local solver or a fixed CMG cycle to
supply pairwise Schwarz corrections.

Issue #4 completed the comparison of those local actions in four settings:

- identical connected pair domains with Jacobi and exact controls;
- assembled complete three-way Schwarz systems under modified LSMR and traced
  projected PCG;
- a balanced component-size and repeated-RHS ladder; and
- recursive three-way hierarchies with the fine `within` smoother and map
  sequence fixed while only non-finest local solvers changed.

The current fixed-CMG action is mathematically valid and can reduce Krylov work
on selected larger balanced pair domains. The strongest size-ladder point used
about 21.5 percent less LSMR work and 24.2 percent less PCG work than `within`.
However, the crossover was nonmonotone, did not admit a stable size/topology/
terminal selector, and produced no fully charged finest-level timing win through
32 RHS. Coarse-only CMG produced no material outer-work reduction on the
controlled oracle-map calibration; the two coarse timing wins used more outer
work.

The selected pair-local policy is therefore:

1. keep the pinned public `within` route as the production-shaped pair-local
   baseline;
2. keep symmetric MAP as the preferred cheap complete-cycle smoother when its
   declared gate passes;
3. keep component-local CMG, exact terminal metadata, and all comparison
   harnesses as explicit research controls; and
4. require a materially redesigned CMG candidate to create a new calibration
   signal and pass a fresh frozen holdout before any automatic route is added.

No current production path should route by elapsed time, by the observed
calibration fixture label, or by a post hoc size/terminal threshold. See
`ADR_0002_ISSUE4_PAIR_SOLVER_POLICY.md` and `ISSUE4_FINAL_RESULTS.md`.

The surrounding three-way apply path still has production-engineering work:
shared immutable state, caller-owned scratch, fused multiple RHS, exact lifetime
memory, parallel traversal, and changing-weight replay. Those concerns now
belong to issue #5. They should be optimized around the `within`/MAP baseline
while retaining CMG as a controlled comparator; they do not reopen the completed
issue-4 selection decision without a materially new CMG algorithm.

## Iterative drivers

- projected PCG is the controlled Gramian research driver used for spectral and
  symmetry experiments;
- modified LSMR on the rectangular weighted incidence operator is the more
  rank-robust production candidate; and
- true residual diagnostics against the submitted operator remain authoritative
  for accepted research solves.

Neither outer driver may accept a local or multilevel route solely from an
internal convergence flag. Original-operator certification and downstream
estimator fallback remain mandatory.

## Integration boundary for fereg

A future fereg adapter should initially support exactly three categorical
intercept fixed effects in OLS:

```text
retained observations
    -> canonical tuple collapse
    -> MultiwayMG topology and screened hierarchy
    -> bounded RHS solves
    -> scatter fitted tuple contributions to observations
    -> fereg's unchanged observation-space FE certificate
    -> accept, polish, or fall back
```

MultiwayMG should own topology, pair solvers, three-way hierarchy construction,
workspaces, and algebraic diagnostics. `fereg` should continue to own sample
policy, finite-regressor algebra, normalization, covariance estimation, memory
admission for the full command, user-visible routing, and final certification.

The initial private adapter should use the issue-4 `within`/MAP local policy and
must not expose current pair-CMG as an automatic user route. CMG may remain an
explicit benchmark/control option during qualification.

## Current milestone boundary

- Issue #4 is complete: the current fixed CMG remains an explicit research
  control rather than the selected local pair solver; `within`/MAP form the
  baseline policy.
- Issue #5 is the current primary milestone: prepared topology, allocation-free
  caller-owned workspaces, repeated-RHS execution, exact memory accounting,
  thread scaling, and changing-weight numerical replay.
- Issue #6 is private certified OLS integration into `fereg` after the numerical
  generation, memory, and engineering gates are ready.

The three-way hierarchy remains the distinctive research contribution; CMG is
not required to win local pair solves for MultiwayMG to succeed.
