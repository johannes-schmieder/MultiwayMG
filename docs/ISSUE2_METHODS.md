# Issue #2: oracle two-grid and multilevel methodology

## Scientific question

Issue #2 isolates the capability of the proposed multilevel method from the
harder problem of automatic aggregate discovery:

> If MultiwayMG is supplied with a good hard factor-respecting hierarchy, do
> its smoothers and coarse corrections form symmetric positive
> preconditioners with stable convergence as the hierarchy deepens?

All maps in this study are manufactured oracle maps. The study therefore tests
multigrid mechanics, not the automatic coarsener.

## Complete numerical range

For every small research problem, MultiwayMG materializes

```text
G = B' W B
```

and computes a rank-revealing symmetric eigendecomposition. The retained
orthonormal basis `Q` includes every eigenvector with eigenvalue above the
configured relative threshold. This discovers the complete numerical range,
including additional nullity caused by nesting or sparse tuple support rather
than assuming that only the generic factor shifts are unidentified.

In this basis,

```text
Q' G Q = Lambda,
```

where `Lambda` is positive diagonal.

## Explicit stationary error operator

For a fixed linear preconditioner `M^{-1}`, the energy-coordinate action is

```text
H = Lambda^(1/2) Q' M^{-1} Q Lambda^(1/2).
```

One damped stationary error step is

```text
E_omega = I - omega H.
```

`analyze_stationary_error` reports:

- the symmetry defect of the full materialized preconditioner;
- leakage of preconditioned range vectors into the numerical null space;
- the symmetry defect of `E_omega`;
- the spectral radius of one and repeated steps;
- the induced Gramian-energy operator norm from an SVD;
- the complete one-step error eigenvalue list.

The energy norm and spectral radius are reported separately. They coincide for
a self-adjoint error operator, but keeping both detects nonnormality rather than
assuming it away.

## Exact coarse correction

For one hard factor map

```text
P = diag(P1, P2, P3),
```

the exact Galerkin coarse problem is

```text
Gc = P' G P = (B P)' W (B P).
```

The coarse correction is

```text
C = P Gc^+ P'.
```

`ExactCoarseCorrection` applies this operation through exact tuple remapping,
deterministic duplicate collapse, a rank-revealing dense terminal, and
factor-respecting prolongation. It is intentionally reported as a semidefinite
coarse-only action; it is not passed to PCG as if it were a complete positive
preconditioner.

## Symmetric two-grid cycle

For a fixed smoother action `S`, one pre- and one post-sweep, and an exact
coarse correction `C`, the implemented cycle is obtained algorithmically by:

```text
x = S r
x = x + C (r - G x)
x = x + S (r - G x)
```

with structural-range projection at every boundary. When `S` and `C` are
symmetric, the corresponding fixed cycle is symmetric and suitable for the
quotient-space spectral and PCG tests.

The matrix compares:

- weighted Jacobi at three safe damping values;
- symmetric factor MAP/block Gauss--Seidel;
- exact dense pair Schwarz;
- all-three-pair CMG;
- a single selected pair plus a positive Jacobi or MAP background;
- exact coarse correction alone;
- Jacobi, MAP, exact-pair, and pair-CMG two-grid cycles.

## Scheduled multilevel cycles

`ScheduledOracleHierarchy` applies one supplied aggregation per level and
allows a fixed smoother choice at each nonterminal level. The resolution study
compares:

- Jacobi on every level;
- symmetric MAP on every level;
- pair-CMG on the finest level only;
- pair-CMG on the first two levels;
- pair-CMG on every level.

This separates the numerical value and retained-state cost of deeper pair-CMG
hierarchies. It does not modify production automatic routing.

## True PCG residual traces

`solve_projected_pcg_traced` recomputes

```text
r_k = b - G x_k
```

against the original submitted Gramian after every iteration, projects it into
the structural range, and records the complete relative residual history. The
report also counts all Gramian and preconditioner applications. Recurrence-only
residual estimates are never used as the scientific certificate.

## Problem matrix

The one-level matrix includes:

- weakly coupled planted communities;
- a dominant factor pair with weak third-factor coupling;
- a weak chain;
- near nesting;
- Latin-square incidence;
- a rectangular tensor grid;
- hub/power-law degree structure;
- positive weights spanning twelve orders of magnitude;
- disconnected components with different local oracle depths.

The resolution matrix contains weak-chain and community sequences through five
exact hierarchy levels and Latin-square sequences through four levels.

## Setup and memory accounting

The setup-cost matrix records diagnostic nanosecond timings for:

- exact coarse tuple construction;
- non-CMG smoother construction;
- pair marginal/graph/component construction;
- CMG hierarchy construction;
- retained pair workspace construction;
- dense terminal construction;
- complete setup;
- median fixed preconditioner application.

It also reports principal retained-memory and serial apply-scratch estimates.
The pair-CMG categories use exact `retained_bytes` and workspace byte counts
provided by CMG where available; three-way problem, terminal, and temporary
vector categories are explicitly labelled estimates.

Hosted-runner timings are descriptive and are never used as a deterministic
route or correctness gate.

## Determinism and acceptance

CI executes the two deterministic spectral/trace matrices twice and compares
their bytes. Setup timing is run once because timing fields are intentionally
nondeterministic. The generated report fails CI unless:

- every admitted action is symmetric and positive on the complete numerical
  range;
- every traced solve meets the original-Gramian residual tolerance;
- the coarse correction materially improves a predeclared majority over both
  Jacobi and pair-CMG;
- tuple complexity remains below three;
- iteration spreads remain at most two over each resolution sequence;
- coarse-only incomplete actions are represented honestly;
- setup phases and memory categories are populated.

These are research gates for oracle feasibility, not production-routing
thresholds.
