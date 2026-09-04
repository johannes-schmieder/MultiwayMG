# Issue #3 selective-cycle holdout v3

## Verdict

**NEGATIVE RESULT.** The frozen v3 policy fails one or more predeclared gates.

The fixtures, smoother order, structural limits, complete-cycle thresholds,
and scientific gates were committed before seeds `900`–`909` were evaluated.
Reference admissibility is conditional: the retained generating map is an exact
fiber partition, but is not assumed to be the globally optimal hard map.

## Case matrix

| Case | Family | Reference admissible | Automatic | Smoother | Source | κ baseline | κ reference | κ automatic | Recovery | vs one-shot | Probe factor | PCG residual |
|---|---|---:|---:|---|---|---:|---:|---:|---:|---:|---:|---:|
| cover-latin-seed-900 | cover-latin | Yes | Yes | `symmetric-map` | `bootstrap-final` | 3.097 | 1.844 | 1.564 | 1.223 | 0.000 | 0.361 | 7.872e-12 |
| cover-latin-seed-901 | cover-latin | Yes | Yes | `symmetric-map` | `bootstrap-final` | 2.720 | 1.664 | 1.316 | 1.330 | 0.000 | 0.240 | 1.174e-11 |
| cover-weak-chain-seed-902 | cover-weak-chain | No | No | `none` | `none` | — | — | — | — | — | — | — |
| cover-weak-chain-seed-903 | cover-weak-chain | No | No | `none` | `none` | — | — | — | — | — | — | — |
| cover-nearly-nested-seed-904 | cover-nearly-nested | Yes | Yes | `all-pairs-cmg` | `bootstrap-final` | 3.624 | 1.523 | 1.400 | 1.059 | 0.000 | 0.286 | 1.239e-11 |
| cover-nearly-nested-seed-905 | cover-nearly-nested | Yes | Yes | `all-pairs-cmg` | `bootstrap-final` | 3.871 | 1.600 | 1.465 | 1.059 | 0.000 | 0.317 | 4.175e-11 |
| cover-dominant-pair-seed-906 | cover-dominant-pair | Yes | Yes | `symmetric-map` | `bootstrap-final` | 2.667 | 1.639 | 1.304 | 1.327 | 0.000 | 0.233 | 7.925e-12 |
| cover-dominant-pair-seed-907 | cover-dominant-pair | Yes | Yes | `symmetric-map` | `bootstrap-final` | 2.405 | 1.517 | 1.253 | 1.297 | 0.000 | 0.202 | 8.734e-11 |
| cover-communities-seed-908 | cover-communities | No | No | `none` | `none` | — | — | — | — | — | — | — |
| cover-communities-seed-909 | cover-communities | No | No | `none` | `none` | — | — | — | — | — | — | — |

## Aggregate gates

- Reference-admissible fixtures: **6 of 10**; required at least 4.
- Reference-inadmissible fixtures: **4 of 10**; required at least 2.
- Conditional automatic acceptance: **6 of 6** = `1.000`; required `0.80`.
- Median cycle-consistent reference recovery: `1.260`; required `0.60`.
- Accepted bootstrap maps improving one-shot by at least 10%: **0**; required 2.
- Accepted bootstrap regressions worse than 10%: **0**; required zero.
- Maximum accepted true residual: `8.734e-11`; limit `1.0e-08`.
- Maximum probe underestimate versus dense radius: `0.000`; limit `0.03`.
- Maximum accepted two-level tuple complexity: `1.949`; limit `1.95`.
- Selected smoother counts: MAP `4`, pair-CMG `2`.
- Selected source counts: bootstrap `6`, protected structural baseline `0`.

## Determinism and correctness

The authoritative workflow executes the holdout twice and byte-compares the
matrix and true-residual traces. Every accepted row must independently pass
the matrix-free complete-cycle probe, dense quotient-space spectral analysis,
hard structural gates, and traced PCG with the original Gramian.

## Failed gates

- only 0 accepted bootstrap maps improve one-shot by at least 0.10; need 2

## Interpretation

A reference-inadmissible rejection is an intended fail-closed outcome, not a
forced regression. Acceptance on such a case is allowed only when an
alternative automatic hard map passes the same independent complete-cycle and
correctness checks. The report does not use those cases in the conditional
reference-recovery median.

Descriptive setup timings are retained separately and are not byte-compared or
used in any routing or scientific decision.
