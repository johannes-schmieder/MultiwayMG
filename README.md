# MultiwayMG

MultiwayMG is an experimental Rust package for solving the linear-algebra
problem created by **three or more high-dimensional categorical fixed effects**.
Its first target is exactly three large categorical intercept effects, where
each observation selects one level from each factor.

The project is motivated by high-dimensional regression, but its numerical
object is more general: a weighted multipartite incidence Gramian.

## The problem

Consider the weighted additive model

```text
y_i = alpha[a_i] + gamma[b_i] + delta[c_i] + error_i.
```

After collapsing observations with the same factor tuple, let `B` be the
sparse tuple-by-level incidence matrix. Every row of `B` contains one active
entry in each of the three factor blocks. With positive tuple weights `W`,
fixed-effect absorption requires repeated solutions of

```text
min_x ||sqrt(W) (y - Bx)||_2
```

or compatible solutions of the singular normal equations

```text
Gx = B'Wy,             G = B'WB.
```

This solve may be repeated for an outcome, many regressors, randomized right-
hand sides, or a sequence of PPML weight frames.

### Why ordinary CMG cannot solve the full three-way system

For two categorical fixed effects, changing the sign of one factor converts
the normal matrix into a weighted bipartite graph Laplacian. That is why the
existing [`CMG`](https://github.com/johannes-schmieder/CMG) package is a natural
fast solver for the two-way case.

For three factors, the full Gramian contains three positive pairwise
cross-blocks. No assignment of factor signs makes all three simultaneously
Laplacian. The complete operator is generally neither a graph Laplacian nor an
SDDM matrix, so submitting it directly to ordinary CMG would be mathematically
incorrect.

General alternating-projection and Krylov methods remain valid, but difficult
weakly coupled designs can require substantial iteration and data movement.
MultiwayMG asks whether the special multiway incidence structure supports a
stronger multilevel method.

## Goal outcome

The intended production outcome is a reusable, deterministic numerical library
that can solve or strongly precondition weighted three-way incidence systems
with:

- operator and cycle work close to linear in the number of unique tuples;
- a bounded-complexity hierarchy and slowly growing outer iteration counts;
- reusable state and fused kernels for many right-hand sides;
- exact symbolic replay with fresh numerical state for changing weights;
- explicit handling of disconnected components and rank deficiency;
- bounded, reported memory use; and
- true residual checks against the submitted incidence operator.

A narrower result is also useful. If a universal automatic three-way hierarchy
is not competitive, the package can still provide a selective coarse
correction, a stronger pairwise preconditioner, and a tested rule for when to
reject multigrid and use another solver.

MultiwayMG is not a regression estimator. It does not define samples, estimate
finite regressors, compute covariance matrices, or decide whether a candidate
solve is scientifically acceptable.

## Relationship to CMG

MultiwayMG uses CMG for exactly the matrix class CMG is designed to solve.
Marginalizing over the third factor produces three weighted factor-pair
systems:

```text
factor 1 -- factor 2
factor 1 -- factor 3
factor 2 -- factor 3
```

After a sign switch, each is a bipartite graph Laplacian. Fixed CMG cycles can
therefore act as symmetric pair corrections or smoothers. MultiwayMG adds the
missing global layer: factor-respecting coarsening and a hierarchy that captures
modes involving all three factors jointly.

```text
CMG
  ^
  |
MultiwayMG
```

Generic graph improvements belong in CMG. Multiway incidence topology,
three-way aggregation, global coarse correction, and cycle admission belong in
MultiwayMG.

## Relationship to fereg

[`fereg`](https://github.com/johannes-schmieder/fereg) is the intended first
downstream estimator.

```text
CMG
  ^
  |
MultiwayMG
  ^
  |
fereg
```

MultiwayMG will own tuple topology, pair corrections, hierarchy construction,
workspaces, and algebraic diagnostics. fereg will continue to own sample
construction, regression right-hand sides, finite-regressor algebra,
normalization, covariance estimation, memory admission, solver fallback, Stata
results, and final certification in the original observation-space fixed-effect
operator.

The current two-way-CMG-plus-small-nuisance route should remain preferred when
the third factor is small. MultiwayMG targets the harder regime in which all
three dimensions are genuinely high-dimensional.

## Numerical approach

For a unique tuple `e = (a_e, b_e, c_e)` with weight `w_e`,

```text
(Bx)_e = x1[a_e] + x2[b_e] + x3[c_e].
```

MultiwayMG combines:

1. **Pairwise graph corrections.** The three pair marginals are solved or
   preconditioned by fixed graph methods such as CMG.
2. **A true three-way coarse hierarchy.** With hard factor-respecting
   interpolation

   ```text
   P = diag(P1, P2, P3),
   ```

   exact Galerkin coarsening remains in the same matrix class:

   ```text
   G_c = P'GP = (BP)'W(BP).
   ```

   Fine tuples mapping to the same coarse tuple are merged by summing weights.
3. **Fail-closed cycle screening.** A proposed map is accepted only after hard
   component, dimension, and tuple-complexity gates and a matrix-free probe of
   the complete two-grid cycle. Rejection is a normal result, not a silent
   identity preconditioner.

Projected PCG is used for controlled Gramian experiments. Modified LSMR on
`sqrt(W)B` is the more rank-robust production candidate.

## Research status

### Issue #2: oracle feasibility — completed

Known-good factor maps were tested through dense quotient-space spectra,
explicit two-grid operators, and multilevel resolution sequences. A valid
three-way coarse space adds important global information beyond exact pairwise
solves, stays symmetric and positive, and can keep iteration counts stable with
hierarchy depth.

### Issue #3: automatic coarse-space construction — completed with a mixed result

Issue #3 developed compatible relaxation, relaxed-signature bootstrap matching,
witness-driven repair, matrix-free complete-cycle screening, selective MAP/CMG
smoothing, and recursive hierarchy construction.

The useful automatic method found so far is the **bounded pair-neighborhood
structural matcher plus fail-closed complete-cycle screening**. On the frozen v3
one-level holdout it accepted every reference-admissible case and rejected every
reference-inadmissible case. Accepted solves had true residuals below `7e-11`.

The extra witness/bootstrap machinery did not materially improve the protected
structural baseline. Complete-cycle witness splitting also missed its value-add
target: no tested row improved dense condition number by ten percent, and the
best gain was about 5.5 percent.

On the frozen recursive matrix, the bootstrap planner accepted only one of
eight fixtures. The simpler recursive pair-neighborhood route constructed
accurate hierarchies on all eight and often matched or exceeded the generating-
map reference, although cumulative tuple complexity reached about `3.44`.

The resulting decision is:

- pair-neighborhood maps are the default structural candidate;
- the exact intended cycle is the final admission authority;
- symmetric MAP is tried first, with all-pair fixed CMG only after MAP
  rejection;
- no accepted cycle means an explicit no-hierarchy result;
- compatible relaxation, bootstrap, and repair remain research diagnostics,
  not automatic defaults.

See `docs/ISSUE3_FINAL_RESULTS.md` and
`docs/ADR_0001_ISSUE3_AUTOMATIC_COARSENING.md`.

### Issue #4: pair-solver economics — completed with no current CMG advancement

Issue #4 compared the current fixed-CMG pair action with the pinned public
`within` local solver in four increasingly controlled settings:

1. identical connected pair domains with Jacobi and exact controls;
2. complete three-way Schwarz systems under modified LSMR and projected PCG;
3. a frozen balanced size ladder through 72 levels per factor and 32 RHS; and
4. non-finest pair solves inside the recursive issue-3 hierarchy while keeping
   the fine `within` smoother and three-way maps fixed.

CMG showed genuine local and scale-dependent work reductions. The strongest
balanced size-ladder point, planted clones with 48 levels per factor, used about
21.5 percent less LSMR work and 24.2 percent less PCG work than `within`.
However, **no finest-level family/size/solver cell produced a fully charged CMG
timing win through 32 RHS**, the crossover was nonmonotone, and size or terminal
metadata did not yield a stable selector.

Coarse-only use did not rescue the current method. In the oracle-map controlled
calibration, none of fourteen comparable solver cells reduced outer work by at
least 20 percent. Two weak-chain cells were faster at 32 RHS, but both used more
outer work than `within`. The automatic planner admitted only one of eight
revealed recursive fixtures, where coarse CMG changed outer work by zero percent
and was slower when fully charged.

The issue-4 decision is therefore:

- retain `within` as the pair-local production-shaped baseline;
- retain symmetric MAP as the preferred cheap smoother where admitted;
- retain pair-CMG, terminal diagnostics, and all evidence harnesses as explicit
  research controls;
- do not introduce a CMG routing rule from the observed calibration cases; and
- do not spend a fresh holdout when no calibrated candidate met the joint work-
  plus-economics advancement gate.

The frozen issue-3 MAP/CMG research portfolio remains reproducible, but its CMG
fallback is not a downstream production endorsement. A materially redesigned
CMG candidate must create a new calibration signal before a fresh holdout.

See `docs/ISSUE4_FINAL_RESULTS.md` and
`docs/ADR_0002_ISSUE4_PAIR_SOLVER_POLICY.md`.

## Current implementation

The workspace includes:

- deterministic validation and collapse of repeated weighted triples;
- matrix-free `B`, `B'`, `sqrt(W)B`, and `B'WB` kernels;
- incidence components and structural-null-space projection;
- exact factor-respecting Galerkin coarsening;
- weighted Jacobi, symmetric MAP, exact pair-Schwarz, and pair-CMG references;
- rank-revealing dense terminals;
- symmetric two-grid and recursive hierarchy operators;
- projected PCG, traced true residuals, and modified LSMR;
- dense quotient-space spectral diagnostics;
- compatible-relaxation, bootstrap, repair, and complete-cycle probes;
- fail-closed structural and cycle-screened portfolios;
- identical-domain and whole-system pair-solver economics harnesses;
- frozen size-ladder and coarse-level CMG evidence with permanent validators;
- deterministic evidence matrices, frozen policies, checksums, and preserved
  negative results.

This remains a research package. It has not yet demonstrated an end-to-end
production speed advantage over mature alternatives on large real-data-shaped
systems.

## Next steps

The next primary milestone is
[#5 — prepared topology, reusable workspaces, and changing-weight replay](https://github.com/johannes-schmieder/MultiwayMG/issues/5).
It will separate immutable topology from numerical generations, remove steady-
state allocations, fuse repeated-RHS kernels, report complete memory lifetimes,
and replay all weight-dependent hierarchy state exactly. This work should use
`within`/MAP as the local baseline while retaining CMG as a controlled
comparator.

A private, certified fereg integration remains tracked by
[#6](https://github.com/johannes-schmieder/MultiwayMG/issues/6). It must preserve
fereg's sample policy, memory admission, fallback, and independent observation-
space certificate.

PPML and broader `Q > 3` generalization remain later milestones after the
three-way fixed-topology production path is understood.

## Workspace

```text
crates/multiway-incidence
    Tuple topology, components, matrix-free operators, structural kernel,
    and exact hard coarsening. No CMG or fereg dependency.

crates/multiway-mg
    Aggregation, diagnostics, pair solvers, terminals, two-grid and recursive
    cycles, Krylov drivers, and research evidence executables.
```

## Validation

The repository pins its Git dependencies and validates Rust 1.85 with:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked --workspace --no-default-features
cargo doc --locked --workspace --all-features --no-deps
```

The programs under `crates/multiway-mg/examples/` are research diagnostics.
Their spectra, iteration counts, structural work, and certified residuals are
meaningful; small hosted-runner timings are not production benchmarks.

MultiwayMG is licensed under GNU GPL version 3 only.
