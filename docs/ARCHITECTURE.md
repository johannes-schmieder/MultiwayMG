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

## Pair solver layer

Each of the three factor-pair marginals is a weighted bipartite graph. After a
sign change of the second factor block, the local Gramian becomes a genuine
graph Laplacian. This permits either the existing `within`
approximate-Cholesky/block-elimination local solver or a fixed CMG cycle to
supply pairwise Schwarz corrections.

The current MultiwayMG `PairCmgPreconditioner` is a correctness/research path,
not yet the performance authority for issue #4. It retains CMG hierarchies and
reusable CMG workspaces, but the surrounding three-way apply path still copies
the compatible global RHS, the hybrid cycle allocates temporary residual and
correction vectors, pair workspaces are mutex-backed, and the three pair
corrections are traversed serially. A wall-clock comparison against `within`
must therefore first use a narrow production-shaped, scratch-buffered CMG local
solver adapter so that the experiment compares algorithms rather than wrapper
maturity. The broader workspace/state redesign remains issue #5.

## Iterative drivers

- projected PCG is the controlled Gramian research driver used for spectral and
  symmetry experiments;
- modified LSMR on the rectangular weighted incidence operator is the more
  rank-robust production candidate; and
- true residual diagnostics against the submitted operator remain authoritative
  for accepted research solves.

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

## Current milestone boundary

- Issue #4: determine pair-solver economics on identical domains—broad CMG win,
  selective/component win, finest-level-only win, or no local-solver win are all
  valid outcomes.
- Issue #5: prepared topology, allocation-free caller-owned workspaces, repeated
  RHS execution, exact memory accounting, and changing-weight numerical replay.
- Issue #6: private certified OLS integration into `fereg` after the preceding
  numerical and engineering gates.

The three-way hierarchy remains the distinctive research contribution; CMG is
not required to win every local pair solve for MultiwayMG to succeed.
