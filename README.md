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

A symmetric V-cycle uses weighted-Jacobi, symmetric MAP, or pair-CMG smoothing,
exact tuple restriction/prolongation, and a rank-revealing terminal. Projected
PCG is used for controlled Gramian experiments; modified LSMR on `sqrt(W)B` is
the more rank-robust production candidate.

## Current step

The first research MVP and the complete oracle two-grid/multilevel feasibility
milestone are finished.

The final oracle study now includes explicit energy-coordinate error operators,
coarse-only and complete two-grid actions, true PCG residual traces, selected
factor-pair experiments, level-specific smoother schedules, phase-separated
setup diagnostics, and principal memory reports. It covers nine one-level
families and exact two- through five-level resolution sequences.

The central result is positive:

- adding the exact factor-preserving coarse correction improved Jacobi,
  symmetric MAP, exact pair Schwarz, and pair-CMG in **all nine** one-level
  families;
- every admitted action was symmetric and positive on the complete numerical
  range;
- every returned solve passed a recomputed original-Gramian residual, with
  full per-iteration traces retained;
- hierarchy tuple complexity remained at most about `1.333` through five
  supplied levels, and iteration spreads within every family/schedule sequence
  were at most two;
- in the twelve-order-of-magnitude weight case, the condition number fell from
  about `1.59 million` under diagonal scaling and `1,367` under pair-CMG to
  about `1.021` for the pair-CMG two-grid cycle; and
- pair-CMG on only the finest level captured nearly all the benefit of retaining
  pair-CMG on every level, while all-level symmetric MAP was usually the
  strongest and much lighter retained-state oracle schedule.

This resolves the question posed by issue #2: **a good hard
factor-respecting coarse space can supply the missing global three-way
correction.** The unresolved problem is discovering a comparably effective
space automatically and cheaply on realistic sparse systems.

See [`docs/ISSUE2_FINAL_RESULTS.md`](docs/ISSUE2_FINAL_RESULTS.md) for the
complete findings, [`docs/ISSUE2_METHODS.md`](docs/ISSUE2_METHODS.md) for the
protocol, and `benchmarks/results/2026-09-03/issue2-*.tsv` for the raw matrices,
residual histories, setup diagnostics, and checksums.

The current milestone is
[issue #3](https://github.com/johannes-schmieder/MultiwayMG/issues/3):
**compatible-relaxation and bootstrap aggregation**. Its diagnostic foundation
is already present. The next step is bounded witness-driven aggregate repair,
followed by a direct automatic-to-oracle gap analysis.

Further work is tracked in GitHub issues:

- [#4 — pair-CMG versus approximate-Cholesky pair solvers](https://github.com/johannes-schmieder/MultiwayMG/issues/4)
- [#5 — prepared topology, reusable workspaces, and changing-weight replay](https://github.com/johannes-schmieder/MultiwayMG/issues/5)
- [#6 — certified experimental integration into fereg](https://github.com/johannes-schmieder/MultiwayMG/issues/6)

## Current implementation and evidence

The package currently includes:

- deterministic validation and collapse of repeated three-way tuples;
- matrix-free `B`, `B'`, `sqrt(W)B`, and `B'WB` kernels;
- incidence-component discovery and projection of structural shift directions;
- exact hard factor-respecting Galerkin coarsening;
- deterministic exact-context and bounded pair-neighborhood aggregation;
- stable weighted-Jacobi smoothing from the three-way bound `G <= 3D`;
- symmetric MAP and exact pair-Schwarz small-problem references;
- recursive symmetric V-cycles with a scale-invariant rank-revealing terminal;
- pairwise CMG corrections for all three factor pairs and selected-pair research
  portfolios;
- explicit exact coarse corrections and symmetric two-grid cycles;
- fixed level-specific oracle smoother schedules;
- projected PCG, traced true-residual PCG, and modified LSMR drivers;
- independent normal-equation residual certification;
- dense quotient-space spectral and stationary-error diagnostics;
- phase-separated pair/coarse/hierarchy setup timing and memory reports;
- deterministic byte-comparison gates for research matrices; and
- tests covering disconnected components, nesting-induced extra rank
  deficiency, numerical symmetry, positive action, weight-scale invariance,
  exact Galerkin identities, and multilevel schedule behavior.

These results establish mathematical and software feasibility, not a production
speed advantage. The present implementation still allocates temporary vectors
in important paths, builds several solver structures, and has not yet been
compared fairly with `within`'s mature approximate-Cholesky Schwarz solver on
large identical pair domains.

See [`docs/RESULTS.md`](docs/RESULTS.md),
[`docs/FEASIBILITY.md`](docs/FEASIBILITY.md),
[`docs/COMPATIBLE_RESULTS.md`](docs/COMPATIBLE_RESULTS.md), and
[`docs/ROADMAP.md`](docs/ROADMAP.md).

## Workspace

```text
crates/multiway-incidence
    Matrix class, tuple topology, components, kernels, and exact hard
    factor-respecting coarsening. It does not depend on CMG or within.

crates/multiway-mg
    Aggregation, rank-revealing terminals, two-grid and V-cycle research
    operators, pair-CMG, projected PCG, modified LSMR, and diagnostics.
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
cargo run --locked --release -p multiway-mg --example issue2_two_grid_matrix --all-features -- output
cargo run --locked --release -p multiway-mg --example issue2_resolution_matrix --all-features -- output
cargo run --locked --release -p multiway-mg --example issue2_setup_cost_matrix --all-features -- output
python3 scripts/summarize_issue2_completion.py output output/ISSUE2_FINAL_RESULTS.md
```

The feasibility programs are research diagnostics. Their iteration counts,
spectra, and certified residuals are meaningful; very small hosted-runner wall
times should not be interpreted as production benchmarks.

The minimum supported Rust version is 1.85. MultiwayMG is licensed under GNU
GPL version 3 only.
