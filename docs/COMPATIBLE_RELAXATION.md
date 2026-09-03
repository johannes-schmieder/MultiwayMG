# Projected compatible relaxation

## Purpose

The oracle spectral study established that a good factor-preserving coarse space
can nearly eliminate the global spectral gap left by exact pairwise solves. The
next problem is to determine whether a proposed automatic aggregation actually
captures the error that its chosen smoother cannot remove.

MultiwayMG uses a projected compatible-relaxation diagnostic for this purpose.
It is related to classical compatible relaxation, but it is expressed as an
explicit coarse-complement projection rather than as a designated set of
fixed coarse points.

The diagnostic answers:

> After removing all error representable by a proposed hard aggregation, how
> rapidly does a fixed smoother damp the remaining error?

Rapid contraction supports the proposed coarse space. Slow contraction exposes
a mode that the coarse space should represent or that the smoother must handle
more effectively.

## Hard factor aggregation

For a three-way problem with Gramian

```text
G = B' W B
```

let

```text
P = diag(P1, P2, P3)
```

be a hard factor-respecting interpolation. Every fine level belongs to exactly
one aggregate in the same factor. The range of `P` is the space of vectors that
are constant within each aggregate.

The package preserves this hard structure because it gives exact Galerkin
closure:

```text
Gc = P' G P = (B P)' W (B P).
```

## Diagonal-energy coarse projector

Let

```text
D = diag(G).
```

For a hard aggregation, the columns of `P` have disjoint support and
`P' D P` is positive diagonal. The `D`-orthogonal projection onto the coarse
space is

```text
Pi_D = P (P' D P)^(-1) P' D.
```

The compatible component of an error is

```text
e_f = (I - Pi_D) e.
```

Equivalently, within every aggregate the projector subtracts the
`D`-weighted mean. The resulting error satisfies

```text
P' D e_f = 0.
```

This formulation has several useful properties:

- it is exact for arbitrary positive degree heterogeneity;
- it is idempotent;
- retained and removed components are orthogonal in the `D` inner product;
- all factor-shift kernel vectors lie in `range(P)` when aggregates respect
  incidence components, so the compatible error also excludes those modes;
- the projector can be applied in linear time using one parent index and one
  aggregate diagonal mass per fine coordinate.

`DiagonalAggregationProjector` rejects a hard aggregate that crosses two exact
incidence components. Such a merge would destroy component-local shift
structure and is not an admissible coarse map.

## Homogeneous relaxation experiment

For a fixed linear smoother `M^{-1}`, one sweep is

```text
e <- e - omega M^{-1} G e
e <- (I - Pi_D) e.
```

The second line removes coarse-representable drift created by smoothing. The
experiment begins with deterministic hash-derived vectors, projects each into
the compatible complement, and normalizes it in the `D` norm. It then applies a
fixed number of sweeps.

The first implementation supports any MultiwayMG `Preconditioner`, including:

- weighted Jacobi;
- symmetric MAP;
- pair-CMG Schwarz;
- future fixed smoothers satisfying the same interface.

The experiment does not use estimator randomness, elapsed-time stopping, or an
RHS-dependent inner iteration count.

## Reported diagnostics

For every test vector, the report records:

- raw diagonal norm;
- diagonal norm removed by the initial coarse projection;
- diagonal-norm history;
- true Gramian-energy history;
- coarse drift removed after every sweep;
- initial and final factor-block diagonal norms;
- initial and final relative `P' D e` defect;
- initial and final weighted structural-shift defect;
- total diagonal contraction;
- total energy contraction when the initial energy is numerically meaningful.

The aggregate report gives worst and geometric-mean contractions, total
smoother and Gramian applications, and maximum final defects.

For `s` sweeps, a useful comparable rate is

```text
rho_D = contraction_D^(1/s)
```

and similarly for the energy contraction. The CI matrix reports both total
contractions and per-sweep factors.

## Interpretation

Compatible relaxation measures a specific pair:

```text
proposed coarse space + selected smoother.
```

A map can be adequate with pair-CMG and inadequate with Jacobi. Likewise, a
strong smoother can hide defects that would matter under a cheaper production
cycle. Hierarchy setup should therefore test the exact smoother family intended
for that level, or use a deliberately conservative reference smoother.

A low contraction factor does not by itself prove a good complete V-cycle. The
coarse operator, terminal, interpolation energy, tuple complexity, and
recursive interaction still matter. Conversely, a high compatible-relaxation
factor is direct evidence that the proposed map misses an error left by the
smoother.

## Current evidence matrix

The `compatible_relaxation_matrix` example compares:

- oracle parent maps;
- exact-context automatic matching;
- pair-neighborhood automatic matching;
- deliberately misaligned maps;
- complete child tensors and parity-sparse child tensors;
- weighted Jacobi, symmetric MAP, and pair-CMG smoothers.

Parity-sparse refinement is useful because every parent tuple generates only
one parity class of its child tensor. The exact parent map remains valid and
recoarsens to the original weighted problem, while sibling levels need not
share an exact two-factor context. This distinguishes a narrow exact-context
matcher from the broader pair-neighborhood construction.

The matrix is a diagnostic stage. It does not yet mutate aggregates or make an
automatic hierarchy acceptance decision.

## Next algorithmic step

After the diagnostic distinguishes strong and weak maps reliably, issue #3 will
add a deterministic quality gate and bounded repair loop:

1. evaluate a proposed map;
2. retain the slowest compatible error vectors;
3. attribute disagreement to aggregates and factor blocks;
4. split or promote problematic members;
5. rebuild only affected local matchings;
6. rerun compatible relaxation under a strict setup-work budget;
7. accept, stop the hierarchy, or fall back to the declared baseline.

Hard one-parent interpolation remains the first target so the exact weighted
coarse-tuple representation is preserved throughout the experiment.
