# Quotient-space spectral analysis

Issue #2 asks a deliberately narrower question than automatic coarsening:

> If the factor aggregation maps are known to be good, can the resulting
> two-grid or multilevel method produce a strong, stable preconditioner for the
> weighted three-way incidence Gramian?

The `spectral` module and `oracle_spectral_matrix` example answer this on small
problems by materializing both the submitted Gramian and one complete fixed
preconditioner action.

## Complete numerical range

The three-way Gramian has at least two factor-shift null directions per
incidence component, but nesting and sparse tuple support may create additional
nullity. The analyzer therefore diagonalizes the dense Gramian and retains all
eigenvectors with

```text
lambda > relative_rank_tolerance * max(abs(lambda)).
```

The resulting Euclidean-orthonormal matrix `Q` spans the complete numerical
range. All quotient diagnostics use this basis, rather than assuming that the
known structural shifts describe the whole kernel.

## Materialized preconditioner

For a fixed linear preconditioner `M^{-1}`, the analyzer applies it to every
coordinate basis vector. It reports:

- the relative Frobenius symmetry defect of the full action;
- the symmetry defect of `Q' M^{-1} Q`;
- leakage of `M^{-1}Q` outside `range(Q)`;
- minimum and maximum eigenvalues of the symmetric quotient action;
- negative and near-zero preconditioner-energy directions.

Range leakage is informative rather than automatically fatal. For example, a
raw inverse-diagonal action need not preserve the singular Gramian's range;
projected PCG explicitly projects its preconditioned vectors. The quotient
matrix still describes that projected action.

## Preconditioned spectrum

Write the range-restricted Gramian as

```text
Q' G Q = Lambda,
```

where `Lambda` is positive diagonal in the selected eigenbasis, and let

```text
S = sym(Q' M^{-1} Q).
```

The analyzer forms

```text
H = Lambda^(1/2) S Lambda^(1/2).
```

When the preconditioner is symmetric positive definite on the range, `H` is
symmetric and has the same positive eigenvalues as the preconditioned operator
relevant to PCG. The report includes:

- minimum and maximum preconditioned eigenvalues;
- spectral condition number;
- energy-norm radius of one undamped stationary correction,
  `max |1 - lambda_i|`;
- optimal scalar Richardson damping over the observed eigenvalue interval; and
- the corresponding optimal energy-norm radius.

The exact dense pseudoinverse provides a calibration check: every retained
eigenvalue should equal one and the stationary radius should be zero up to
roundoff.

## Reference smoothers and local solves

The issue #2 matrix includes:

- three safe Jacobi damping values;
- one symmetric MAP/block-Gauss--Seidel correction;
- exact dense pairwise Schwarz as a small-problem local-solver ceiling;
- fixed pair-CMG Schwarz;
- an oracle Jacobi V-cycle; and
- an oracle pair-CMG/coarse hybrid.

Every row also runs projected PCG on a deterministic compatible right-hand side
and reports its recomputed relative residual. This links spectral predictions
to actual Krylov convergence without treating iteration counts as a substitute
for structural diagnostics.

## Oracle hierarchy construction

Each family starts from a small weighted base problem. A refinement replaces
every factor level by several children and every tuple by all child triples,
with deterministic heterogeneous positive weights summing to the parent tuple
weight. The exact parent maps are retained. Repeated exact coarsening therefore
recovers every earlier level, providing one to four known hierarchy maps.

The matrix currently covers:

- planted weakly coupled communities;
- a Latin-square incidence pattern;
- a weak chain;
- a nearly nested third factor;
- disconnected Latin-square components; and
- a four-level complete weighted hierarchy.

These oracle maps isolate multigrid mechanics from aggregate discovery. They do
not constitute evidence that automatic coarsening can recover the same spaces.

## Interpretation constraints

The dense analyzer is for small research problems. It allocates quadratic
matrices and uses cubic eigendecompositions. Its purpose is to falsify invalid
smoothers, detect missing range directions, measure the oracle convergence
ceiling, and guide later compatible-relaxation work.

A favorable oracle spectrum is necessary but not sufficient for a production
solver. Automatic aggregate quality, hierarchy setup, tuple complexity, pair
CMG cost, reusable workspaces, multiple right-hand sides, and realistic
large-problem comparisons remain separate gates.
