# Issue #3 observed-seed smoother fallback development

## Scope

This matrix deliberately reuses the already observed frozen-v2 seeds
`700`–`709`. It is **training evidence only**, not a new holdout. The
frozen v2 negative verdict and its original symmetric-MAP policy remain
unchanged.

The tested fallback rule is predeclared within this development run:

1. keep the frozen hard structural gates and candidate maps;
2. prefer an accepted one-sweep symmetric-MAP two-grid cycle;
3. only when no MAP candidate passes, evaluate an all-pair fixed-CMG
   two-grid cycle against the same `0.50` complete-cycle factor gate;
4. within one smoother tier, select by estimated factor, then coarse
   tuple count, coarse dimension, and stable map-source order.

## Oracle smoother check

- Symmetric-MAP oracle cycles accepted: **6 of 10**.
- All-pair-CMG oracle cycles accepted: **6 of 10**.

## Deterministic fallback selections

| Case | Family | Accepted | Map | Smoother | Probe factor | Condition number | Recovery | Coarse tuples |
|---|---|---:|---|---|---:|---:|---:|---:|
| cover-communities-seed-708 | cover-communities | No | — | — | — | — | — | — |
| cover-communities-seed-709 | cover-communities | No | — | — | — | — | — | — |
| cover-dominant-pair-seed-706 | cover-dominant-pair | Yes | `primary-bootstrap-final` | `symmetric-map` | 0.265 | 1.361 | 1.268 | 478 |
| cover-dominant-pair-seed-707 | cover-dominant-pair | Yes | `one-shot-pair-neighborhood` | `symmetric-map` | 0.280 | 1.389 | 1.306 | 476 |
| cover-latin-seed-700 | cover-latin | Yes | `primary-bootstrap-final` | `symmetric-map` | 0.240 | 1.316 | 1.437 | 239 |
| cover-latin-seed-701 | cover-latin | Yes | `primary-bootstrap-final` | `symmetric-map` | 0.229 | 1.297 | 1.366 | 236 |
| cover-nearly-nested-seed-704 | cover-nearly-nested | Yes | `primary-bootstrap-final` | `all-pairs-cmg` | 0.267 | 1.365 | 1.061 | 430 |
| cover-nearly-nested-seed-705 | cover-nearly-nested | Yes | `primary-bootstrap-final` | `all-pairs-cmg` | 0.298 | 1.425 | 1.058 | 418 |
| cover-weak-chain-seed-702 | cover-weak-chain | No | — | — | — | — | — | — |
| cover-weak-chain-seed-703 | cover-weak-chain | No | — | — | — | — | — | — |

## Aggregate diagnostics

- Accepted cases under the fallback rule: **6 of 10**.
- Selected smoother counts: `all-pairs-cmg` 2, `symmetric-map` 4.
- Median accepted oracle-improvement recovery: `1.287`.
- Maximum accepted true PCG residual: `7.980e-11`.
- Maximum accepted two-level tuple complexity: `1.934`.

## Interpretation

This development matrix answers only whether the v2 failure is plausibly
a smoother-selection problem. A positive result justifies implementing a
first-class MAP-to-pair-CMG cycle portfolio and freezing a new policy with
new unseen seeds. It does not count toward the issue #3 scientific gate.
