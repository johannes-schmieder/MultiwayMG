# Issue #3 selective-cycle holdout policy v3

## Why v3 is necessary

The frozen v2 experiment required eight of ten graph-cover fixtures to accept a
one-sweep symmetric-MAP two-grid cycle. The repaired implementation reproduced
the experiment byte for byte but accepted only four cases. Four exact generating
fiber maps—both weak-chain and both weak-community cases—also failed the same
complete-cycle threshold. The result is preserved in
`docs/ISSUE3_CYCLE_V2_NEGATIVE.md` and is not retuned away.

An observed-seed development experiment then tested all-pair fixed CMG as a
fallback. It rescued the two nearly nested automatic maps, but did not rescue
the weak-chain or community generating maps. The one universal cycle
architecture is therefore replaced by a predeclared selective portfolio, not by
a weaker threshold.

## Frozen fixtures

V3 uses requested seeds `900`–`909`, declared before numerical evaluation:

| Family | Seeds |
|---|---|
| Latin cover | 900, 901 |
| Weak-chain cover | 902, 903 |
| Nearly nested cover | 904, 905 |
| Dominant-pair cover | 906, 907 |
| Weak-community cover | 908, 909 |

A requested seed may advance only to the first structurally valid cover, using
the same deterministic rule as v2. Structural validity is independent of
solver results.

## Automatic method

The hard-map candidate set and all structural thresholds remain unchanged:

1. build the relaxed-signature bootstrap candidate with bounded witness
   enrichment and monotone split repair;
2. retain the bounded pair-neighborhood map as a protected structural baseline;
3. reject any map exceeding the coarse-dimension, tuple-reduction, component,
   or two-level tuple-complexity limits;
4. screen all eligible maps with one symmetric-MAP two-grid cycle;
5. select the best accepted MAP candidate by estimated energy factor, coarse
   tuple count, coarse dimension, and stable source order;
6. only when no MAP candidate passes, repeat complete-cycle screening for the
   same deterministic maps with one all-pair fixed-CMG two-grid cycle;
7. reject the hierarchy when neither smoother tier passes.

The pair fallback deliberately repeats deterministic candidate construction in
the first API version. That duplicated setup is measured and reported. Sharing
prepared candidates belongs to issue #5 and cannot change the v3 scientific
selection rule.

## Reference interpretation

The retained fiber map is an exact generating partition: recoarsening recovers
the base weighted problem. It is **not assumed to be the globally best hard
coarse space** for the chosen cycle.

A fixture is reference-admissible when its generating map passes the unchanged
complete-cycle threshold under either predeclared smoother, with MAP evaluated
first. Reference-inadmissible cases remain scientifically useful: an automatic
rejection is valid, while an alternative automatic map may still be accepted
if its own complete cycle independently passes every structural, dense, probe,
and residual check.

This interpretation avoids the false requirement that an automatic method must
accept a fixed number of systems even where the reference cycle itself is
inadequate.

## Cycle-consistent recovery

For an accepted automatic cycle, oracle-improvement recovery is computed with
its selected smoother:

```text
recovery = (baseline_smoother_condition - candidate_condition)
           / (baseline_smoother_condition - generating_map_condition)
```

The baseline is the corresponding no-coarse smoother and the reference cycle
uses the same smoother. Recovery is reported only when the denominator is
materially positive and the fixture is reference-admissible.

The structural one-shot comparison is also made with the automatic cycle's
selected smoother. A bootstrap-selected map counts as materially better only
when its condition number is at least ten percent below the pair-neighborhood
map under that same complete-cycle construction.

## Frozen scientific gates

The v3 result passes only if all of the following hold:

- at least four fixtures are reference-admissible and at least two are
  reference-inadmissible;
- the automatic portfolio accepts at least 80 percent of reference-admissible
  fixtures;
- median cycle-consistent recovery among accepted reference-admissible fixtures
  is at least 60 percent;
- at least two accepted bootstrap-selected maps improve on the one-shot
  structural baseline by at least ten percent in condition number;
- no bootstrap-selected accepted map is more than ten percent worse than the
  one-shot baseline;
- every accepted cycle has a recomputed relative residual at most `1e-8`;
- probe underestimation relative to the dense exact cycle radius is at most
  `0.03`;
- two-level tuple complexity is at most `1.95`;
- repeated outputs are byte-identical.

Rejection on a reference-inadmissible case is not a failure. Acceptance on such
a case is permitted only through the same independent complete-cycle and
correctness gates; it is not counted in the conditional oracle-recovery median.

The policy is machine-readable at
`benchmarks/policies/issue3-cycle-portfolio-v3.tsv`.
