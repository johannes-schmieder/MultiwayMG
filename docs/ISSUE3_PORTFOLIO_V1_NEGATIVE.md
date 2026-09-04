# Issue #3 portfolio v1: preserved negative holdout

## Why this result is retained

The first frozen issue #3 portfolio policy is intentionally preserved rather
than retuned or overwritten. Its declared seeds and thresholds are recorded in
`benchmarks/policies/issue3-portfolio-holdout.tsv`.

The holdout executable completed successfully. Two independent executions
produced identical outputs:

```text
issue3-portfolio-holdout.tsv: 41 rows in both runs, identical
issue3-portfolio-traces.tsv:  667 rows in both runs, identical
```

The earlier CI failure occurred in the surrounding compare/commit workflow, not
because the numerical program was nondeterministic. A dedicated comparison
confirmed byte-identical matrix and residual-trace contents.

## Scientific failure

The policy nevertheless fails as a final automatic-hierarchy acceptance rule.
It uses weighted-Jacobi compatible relaxation as the primary screen and
symmetric-MAP compatible relaxation as a secondary screen, both with a frozen
maximum factor of `0.85` per sweep.

That smoother-only rule rejects supplied oracle maps on holdout cases where the
complete symmetric-MAP two-grid cycle is excellent.

### Nearly nested holdout

For one supplied oracle map:

```text
primary Jacobi compatible factor:       about 0.929
secondary MAP compatible factor:        about 0.954 in the diagonal norm
complete MAP two-grid condition number: about 1.011
```

The policy rejects the map despite the complete cycle being nearly an exact
inverse on the numerical range.

### Weak-chain holdout

For the supplied oracle map:

```text
primary Jacobi compatible factor:       about 0.979
secondary MAP compatible factor:        about 1.015 in the diagonal norm
complete MAP two-grid condition number: about 1.003
```

The protected one-shot structural map also has a complete two-grid condition
number around `1.003`, but the smoother-only screens reject it. The primary
bootstrap path then rematches into a map that violates both the coarse-dimension
and unique-tuple-contraction budgets, and correctly fails closed.

## Diagnosis

Compatible relaxation measures the smoother on a projected complement of a
proposed coarse space. It is useful for generating slow witnesses and exposing
some missed modes, but it is not equivalent to measuring the complete cycle

```text
E = I - M^{-1} G,
```

where coarse correction and smoothing interact.

A map may have a slowly damped compatible component under the smoother alone
while the complete symmetric two-grid composition corrects that component very
effectively. Making the smoother-only threshold looser after seeing this
holdout would not solve the conceptual problem and would invalidate the frozen
experiment.

## Design consequence

The issue #3 architecture now separates three authorities:

1. **Structural gates are hard.** Coarse dimension, unique-tuple reduction,
   tuple complexity, factor boundaries, and exact components cannot be bypassed.
2. **Compatible relaxation generates witnesses.** Conservative Jacobi remains
   useful for bootstrap enrichment, aggregate attribution, and monotone repair.
3. **The complete cycle determines final numerical acceptance.** A new
   matrix-free `G`-energy probe estimates the dominant error factor of the
   actual fixed cycle intended for use.

The revised policy will receive a new version and a new unseen holdout. The v1
seeds 600--609 remain development evidence and will not be reused as an unseen
test.

## Status

The v1 result is a successful falsification of an inadequate acceptance rule.
It is not counted as passing issue #3's scientific gate.
