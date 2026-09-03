# MultiwayMG

MultiwayMG is an experimental Rust package for solving the linear-algebra
problem created by **three or more high-dimensional categorical fixed effects**.
Its first target is the case of exactly three large intercept fixed-effect
dimensions, where every observation selects one level from each factor.

The project is motivated by high-dimensional regression, but the numerical
object is more general: a weighted multipartite incidence Gramian.

## The problem this package is trying to solve

Consider a weighted additive three-way model

```text
y_i = alpha[a_i] + gamma[b_i] + delta[c_i] + error_i.
```

Let `B` be the sparse incidence matrix whose row for observation `i` contains
one `1` in each of the three factor blocks, and let `W` contain positive
weights. Absorbing the fixed effects requires repeated solutions of

```text
min_x ||sqrt(W) (y - Bx)||_2
```

or, equivalently, compatible solutions of the singular normal equations

```text
Gx = B'Wy,            G = B'WB.
```

This is often the dominant numerical task in a regression with several large
fixed-effect dimensions. A regression command must usually solve the same
operator for an outcome and many regressors, and a PPML estimator must solve a
sequence of related weighted systems.

### Why ordinary graph CMG is not enough

With two categorical fixed effects, a sign change in one factor turns the
normal matrix into a weighted bipartite graph Laplacian. That special identity
makes the existing [`CMG`](https://github.com/johannes-schmieder/CMG) package a
natural fast solver.

For three factors, the full matrix has three positive pairwise cross-blocks.
No assignment of factor signs can make all three cross-blocks nonpositive at
once, so the complete three-way Gramian is generally neither a graph Laplacian
nor an SDDM matrix. It therefore cannot simply be submitted to ordinary CMG.

Existing general approaches—alternating projections, generic Krylov methods,
and one-level pairwise Schwarz preconditioning—remain valid, but difficult
three-large-FE systems can still require substantial iteration and data
movement. MultiwayMG asks whether the multiway incidence structure supports a
stronger multilevel method.

## Goal outcome

The intended outcome is a reusable numerical library that can make absorption
of three genuinely large categorical fixed effects competitive with the best
existing general methods while preserving deterministic, certified behavior.
A successful production version should provide:

- setup and operator work close to linear in the number of unique weighted
  tuples;
- a bounded-complexity hierarchy whose iteration count grows slowly with
  problem size on difficult structured systems;
- reusable state and fused kernels for many right-hand sides;
- exact symbolic replay with fresh numerical state for changing weight frames;
- robust handling of disconnected components and rank deficiency;
- true residual diagnostics against the submitted incidence operator; and
- a clean API that downstream regression packages can use without importing
  estimator-specific behavior.

The research project is deliberately allowed to produce a narrower result. If
a full automatic three-way hierarchy is not broadly competitive, a useful
outcome may still be a selective pair-CMG solver, a global coarse correction
for Schwarz-LSMR, or a well-tested incidence-operator package that identifies
where each method pays.

## Relationship to CMG

CMG remains the specialized solver for weighted graph Laplacians and SDDM
matrices. MultiwayMG **uses CMG; it does not redefine CMG's matrix class**.

For each factor pair, MultiwayMG marginalizes over the third factor. After a
sign change, that pair system is a genuine weighted bipartite graph Laplacian.
CMG can therefore apply a fixed linear correction to the three pair systems:

```text
factor 1 -- factor 2
factor 1 -- factor 3
factor 2 -- factor 3
```

Those pairwise graph corrections act as a strong smoother or Schwarz
preconditioner. MultiwayMG adds the missing global layer: a hierarchy that
coarsens all three factor spaces together and captures error modes that cannot
be represented adequately by any single pair problem.

The intended dependency direction is:

```text
CMG
  ^
  |
MultiwayMG
```

Generic graph improvements belong in CMG. Multiway incidence topology,
three-way aggregation, global coarse correction, and the hybrid cycle belong
in MultiwayMG.

## Relationship to fereg

[`fereg`](https://github.com/johannes-schmieder/fereg) is the intended first
downstream estimator. It already uses CMG for two-way graph solves and
Schwarz-LSMR for more general fixed-effect designs.

MultiwayMG is being developed separately so that the numerical research has a
clear matrix contract and can be tested independently of Stata and regression
semantics. A future integration will follow this direction:

```text
CMG
  ^
  |
MultiwayMG
  ^
  |
fereg
```

MultiwayMG will own tuple topology, pair graph corrections, the three-way
hierarchy, workspaces, and algebraic solve diagnostics. fereg will continue to
own sample construction, regression right-hand sides, finite-regressor algebra,
fixed-effect normalization, memory admission, fallback behavior, covariance
estimation, Stata results, and the final original-observation-space
certificate.

The initial fereg route will be private, OLS-only, and restricted to exactly
three categorical intercept effects. It will not replace fereg's current
automatic solver until controlled calibration and fresh holdout evidence show a
real end-to-end advantage.

## Numerical approach

For every unique tuple `e = (a_e, b_e, c_e)` with positive weight `w_e`,

```text
(Bx)_e = x1[a_e] + x2[b_e] + x3[c_e].
```

MultiwayMG combines two complementary ideas:

1. **Pair-CMG corrections.** Each of the three factor-pair marginals is solved
   approximately by a fixed CMG cycle and combined symmetrically.
2. **A true three-way hierarchy.** Levels are aggregated only within their own
   factor. Mapping every fine tuple through the three parent maps produces
   another weighted three-way tuple problem, so the operator class is preserved
   exactly under hard Galerkin coarsening:

   ```text
   G_c = P'GP = (BP)'W(BP),       P = diag(P1, P2, P3).
   ```

A symmetric V-cycle uses weighted-Jacobi or pair-CMG smoothing, exact tuple
restriction/prolongation, and a rank-revealing terminal. Projected PCG is used
for controlled Gramian experiments; modified LSMR on `sqrt(W)B` is the more
rank-robust production candidate.

## Current step

The first research MVP and the primary oracle-hierarchy feasibility gate are
complete.

The oracle study materializes the complete numerical range of small singular
three-way Gramians, including extra rank deficiency, and measures the spectrum
of each fixed preconditioner. Across six manufactured families:

- an oracle Jacobi V-cycle kept the preconditioned condition number below about
  `1.46`;
- an oracle pair-CMG/coarse hybrid kept it below about `1.006`;
- the oracle hybrid converged in three or four projected-PCG iterations; and
- every reported preconditioner remained positive and symmetric on the
  numerical range and passed a recomputed original-Gramian residual check.

These results establish that a good factor-preserving coarse space can add
substantial information beyond exact pairwise corrections. They are an
idealized ceiling, not evidence that automatic aggregation can yet discover the
same space or that the current implementation wins in wall-clock time. See
[`docs/ORACLE_RESULTS.md`](docs/ORACLE_RESULTS.md) and
[`docs/SPECTRAL_ANALYSIS.md`](docs/SPECTRAL_ANALYSIS.md).

The current milestone is
[issue #3](https://github.com/johannes-schmieder/MultiwayMG/issues/3):
**compatible-relaxation and bootstrap aggregation**. It will measure the gap
between automatic and oracle hierarchies, identify slow errors missed by a
proposed map, repair bad aggregates deterministically, and reject inadequate
hierarchies before solving.

Further work is tracked in GitHub issues:

- [#4 — pair-CMG versus approximate-Cholesky pair solvers](https://github.com/johannes-schmieder/MultiwayMG/issues/4)
- [#5 — prepared topology, reusable workspaces, and changing-weight replay](https://github.com/johannes-schmieder/MultiwayMG/issues/5)
- [#6 — certified experimental integration into fereg](https://github.com/johannes-schmieder/MultiwayMG/issues/6)

## Current implementation and evidence

The package currently includes:

- deterministic validation and collapse of repeated three-way tuples;
- matrix-free `B`, `B'`, `sqrt(W)B`, and `B'WB` kernels;
- incidence-component discovery and projection of the two structural shift
  directions per connected component;
- exact hard factor-respecting Galerkin coarsening;
- deterministic exact-context and bounded pair-neighborhood aggregation;
- stable weighted-Jacobi smoothing from the three-way bound `G <= 3D`;
- symmetric MAP and exact pair-Schwarz small-problem references;
- recursive symmetric V-cycles with a scale-invariant rank-revealing terminal;
- pairwise CMG corrections for all three factor pairs;
- a symmetric hybrid of pair-CMG smoothing and three-way coarse correction;
- projected PCG and modified LSMR drivers;
- independent normal-equation residual certification;
- dense quotient-space spectral diagnostics; and
- tests covering disconnected components, nesting-induced extra rank
  deficiency, numerical symmetry, positive action, weight-scale invariance,
  exact Galerkin identities, and an executable oracle spectral acceptance gate.

The earlier non-oracle matrix also showed strong iteration reductions on a
small weak-chain case: diagonal PCG required 85 iterations, pair-CMG required
9, the three-way V-cycle required 6, and the hybrid required 3. These results
establish mathematical and software feasibility, not a production speed
advantage.

The present implementation still allocates temporary vectors in important
paths, builds several solver structures, uses simple structural aggregation
rules, and has not yet been compared fairly with `within`'s mature
approximate-Cholesky Schwarz solver on large identical problems. See
[`docs/RESULTS.md`](docs/RESULTS.md),
[`docs/FEASIBILITY.md`](docs/FEASIBILITY.md), and
[`docs/ROADMAP.md`](docs/ROADMAP.md).

## Workspace

```text
crates/multiway-incidence
    Matrix class, tuple topology, components, kernels, and exact hard
    factor-respecting coarsening. It does not depend on CMG or within.

crates/multiway-mg
    Aggregation, rank-revealing terminals, V-cycles, pair-CMG, projected PCG,
    modified LSMR, and dense research spectral analysis.
```

## Development and validation

The repository pins Git dependencies and validates both the complete and
minimal feature sets with Rust 1.85:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked --workspace --no-default-features
cargo doc --locked --workspace --all-features --no-deps
cargo run --locked --release -p multiway-mg --example feasibility --all-features
cargo run --locked --release -p multiway-mg --example feasibility_matrix --all-features
cargo run --locked --release -p multiway-mg --example scaling_probe --all-features
cargo run --locked --release -p multiway-mg --example oracle_spectral_matrix --all-features
```

The feasibility programs are research diagnostics. Their iteration counts,
spectra, and certified residuals are meaningful; very small hosted-runner wall
times should not be interpreted as production benchmarks.

The minimum supported Rust version is 1.85. MultiwayMG is licensed under GNU
GPL version 3 only.
