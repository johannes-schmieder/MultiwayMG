# Issue #3 final results: automatic three-way coarse spaces

## Question

Issue #3 asked whether MultiwayMG could discover a compact hard
factor-respecting coarse space automatically, recover a substantial fraction of
the oracle benefit established by issue #2, and do so robustly enough to justify
bootstrap aggregation and repair in an eventual production route.

The answer is mixed:

- **Yes:** a simple bounded pair-neighborhood matcher, combined with hard
  structural gates and a fail-closed complete-cycle screen, is a reliable
  automatic research route on the frozen matrices.
- **No:** the additional relaxed-signature bootstrap and both witness-driven
  repair schemes did not materially improve on that structural baseline.

This is a completed research result, including negative evidence. It is not a
claim of production speed superiority.

## Implemented methods

The issue developed a complete automatic-coarsening research stack:

1. diagonal-energy projection onto the complement of a hard aggregate space;
2. deterministic compatible-relaxation test vectors and contraction reports;
3. exact-context and bounded pair-neighborhood candidate maps;
4. relaxed-signature bootstrap matching;
5. protected structural-baseline arbitration;
6. compatible-witness split/promotion repair;
7. matrix-free complete-cycle power probing;
8. complete-cycle witness split repair;
9. MAP-first, all-pair-CMG-fallback cycle screening;
10. recursive hierarchy planning and construction;
11. true PCG residual traces, dense small-system spectra, structural work, and
    deterministic repeated-output checks.

All accepted maps preserve factor boundaries and exact incidence components.
All numerical decisions use deterministic seeds and structural quantities, not
elapsed time.

## Compatible relaxation

For `D = diag(G)` and hard interpolation `P`, the issue implemented the exact
`D`-orthogonal coarse projector

```text
Pi_D = P (P' D P)^(-1) P' D.
```

The compatible error `(I - Pi_D)e` satisfies `P'De = 0`. Tests verify
idempotence, the `D`-norm Pythagorean identity, component preservation, and
structural-shift removal.

Compatible relaxation successfully distinguished deliberately poor weak-chain
and nearly nested maps under a conservative Jacobi smoother. It also exposed a
key limitation: a strong pair-CMG smoother can hide coarse-map defects, and a
larger coarse space can make the complement artificially easy. Compatible
relaxation is therefore a useful diagnostic, not the sole admission authority.

## Preserved falsification results

### Frozen cycle v2

The v2 policy required one symmetric-MAP two-grid cycle to pass on eight of ten
seeded graph-cover fixtures. Duplicate runs were byte-identical, but only four
cases passed. Four exact generating maps—both weak-chain and both weak-community
cases—also failed the fixed MAP cycle threshold despite accurate solves.

This showed that a single universal MAP cycle is not robust to every graph-cover
fiber mode. The result and checksums are preserved under
`benchmarks/results/2026-09-04/issue3-cycle-v2-*`.

### MAP-to-CMG fallback development

All-pair fixed CMG rescued two nearly nested automatic maps but did not rescue
the weak-chain or community generating-map failures. Smoother selection alone
was therefore not a general repair.

## Frozen selective-cycle v3 holdout

The v3 policy was committed before evaluating new seeds `900`–`909`. It kept the
hard structural and complete-cycle thresholds unchanged, preferred MAP, and
used all-pair CMG only after MAP rejection.

The result was scientifically clean:

- reference-admissible fixtures: **6**;
- reference-inadmissible fixtures: **4**;
- automatic acceptance on reference-admissible fixtures: **6/6**;
- automatic rejection on reference-inadmissible fixtures: **4/4**;
- median cycle-consistent reference improvement recovered: approximately
  **1.29**;
- maximum accepted true relative residual: approximately **6.4e-11**;
- maximum two-level tuple complexity: approximately **1.95**;
- maximum matrix-free probe underestimation versus dense analysis:
  approximately **0.017**.

The frozen policy nevertheless failed one declared value-add gate: it required
at least two bootstrap-selected maps to beat the protected pair-neighborhood
baseline by ten percent in condition number. The observed count was **zero**.
The structural baseline was selected on five of the six accepted cases; the
learned map was selected once and improved condition number by only about
0.2 percent.

The v3 evidence, traces, gate status, and checksums are preserved under
`benchmarks/results/2026-09-04/issue3-cycle-v3-*`.

## Complete-cycle witness split repair

A final observed-seed development experiment used the slowest complete-cycle
witness rather than a smoother-only witness. It split the aggregate carrying
the most unresolved diagonal energy and admitted each split only when the
matrix-free complete-cycle factor improved by at least two percent, subject to
a coarse-dimension ratio of `0.65`, tuple reduction of `0.05`, two-level tuple
complexity of `1.95`, and at most eight splits.

Results across twenty method/case rows:

- rows admitting at least one split: **5/20**;
- rows improving dense condition number by at least ten percent: **0/20**;
- best observed condition-number gain: approximately **5.5 percent**;
- maximum true PCG residual: approximately **9.95e-11**;
- maximum final two-level tuple complexity: approximately **1.949**.

This mechanism is technically valid but did not justify inclusion in automatic
routing.

## Frozen recursive v1 holdout

The recursive policy was also frozen before evaluation and run twice with
byte-identical outputs.

The bootstrap/cycle-screened hierarchy accepted only **1/8** fixtures. It did
not materially outperform the simpler recursive structural route and violated
its cumulative acceptance objectives.

In contrast, the recursive one-shot pair-neighborhood route constructed and
solved all **8/8** requested hierarchies. Its reference-recovery ratios were:

- Latin, depth 2: approximately `1.32`;
- Latin, depth 3: approximately `1.30`;
- weak chain, depth 2: approximately `1.47`;
- weak chain, depth 3: approximately `1.66`;
- nearly nested, depth 2: approximately `1.00`;
- nearly nested, depth 3: approximately `0.92`;
- dominant pair, depth 2: approximately `1.30`;
- weak communities, depth 3: approximately `1.02`.

Every solve retained true residual accuracy. However, cumulative tuple
complexity reached approximately **3.444** and dimension complexity
approximately **2.573**, beyond the frozen bootstrap planner's production-shaped
budgets on some fixtures.

This means recursive structural coarsening is numerically promising, while
production admission requires explicit setup, memory, and amortization policy.
It does not support adding the current bootstrap planner.

## Final decision

The automatic research baseline is:

1. construct the bounded pair-neighborhood hard map;
2. enforce exact component preservation and hard dimension/tuple gates;
3. probe the exact intended complete cycle;
4. prefer a symmetric-MAP two-grid cycle;
5. evaluate all-pair fixed CMG only after MAP rejection;
6. select only a fully accepted candidate;
7. return an explicit no-hierarchy result when no candidate passes.

For recursive research, apply the same structural map level by level while
reporting cumulative dimension and tuple complexity. Do not treat recursive
admission as production-ready until issue #5 supplies exact memory and prepared
state accounting.

The following remain diagnostic or experimental, not automatic defaults:

- compatible-relaxation scores;
- relaxed-signature bootstrap matching;
- compatible-witness split repair;
- complete-cycle witness split repair.

A future method may revisit them on materially different real-data-shaped
families, but the existing frozen evidence does not justify their setup cost or
complexity.

## What issue #3 establishes

Issue #3 did not produce the originally hoped-for bootstrap value-add. It did
produce something operationally useful and scientifically clearer:

- automatic hard coarsening can be reliable without reproducing arbitrary
  oracle labels;
- complete-cycle quality, not smoother-compatible quality, must be the final
  admission authority;
- selective rejection is a correct solver outcome;
- pair-neighborhood maps compose recursively and can be stronger than the
  generating reference map;
- additional adaptive machinery must earn its cost against a strong structural
  baseline rather than only against diagonal smoothing.

The next questions are local-solver economics and production engineering, not
another retuning of bootstrap thresholds on these holdouts.
