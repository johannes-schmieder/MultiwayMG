# Issue #3 complete-cycle split-repair development

## Scope

This experiment reuses the already observed v3 seeds `900`–`909`.
It is calibration evidence only and cannot change the frozen v3 verdict.
The new repair starts from the pair-neighborhood baseline, finds the
slowest complete-cycle witness, splits the aggregate carrying the most
remaining diagonal energy, and admits the split only when the matrix-free
cycle factor improves by at least two percent while all structural budgets
remain satisfied.

## Aggregate result

- Rows admitting at least one split: **5 of 20**.
- Rows with at least ten percent dense condition-number improvement: **0 of 20**.
- Final cycles passing the unchanged complete-cycle gate: **10 of 20**.
- Maximum true PCG residual: `9.945e-11`.
- Maximum final two-level tuple complexity: `1.949`.

## Largest condition-number improvements

| Case | Smoother | Splits | Initial κ | Final κ | Improvement | Initial factor | Final factor | Stop |
|---|---|---:|---:|---:|---:|---:|---:|---|
| cover-communities-seed-909 | `all-pairs-cmg` | 1 | 3.014 | 2.848 | 5.5% | 0.668 | 0.649 | `InsufficientCycleImprovement { current_factor: 0.6486735563735909, candidate_factor: 0.6462198969476202, maximum_ratio: 0.98 }` |
| cover-latin-seed-900 | `symmetric-map` | 1 | 1.564 | 1.482 | 5.2% | 0.361 | 0.325 | `InsufficientCycleImprovement { current_factor: 0.32517065103911486, candidate_factor: 0.3209228001726101, maximum_ratio: 0.98 }` |
| cover-dominant-pair-seed-906 | `symmetric-map` | 2 | 1.304 | 1.257 | 3.6% | 0.233 | 0.204 | `InsufficientCycleImprovement { current_factor: 0.2042337921235011, candidate_factor: 0.20268879744102372, maximum_ratio: 0.98 }` |
| cover-nearly-nested-seed-905 | `all-pairs-cmg` | 1 | 1.465 | 1.449 | 1.1% | 0.317 | 0.310 | `InsufficientCycleImprovement { current_factor: 0.309992455957513, candidate_factor: 0.30957989369908934, maximum_ratio: 0.98 }` |
| cover-nearly-nested-seed-904 | `all-pairs-cmg` | 1 | 1.400 | 1.387 | 0.9% | 0.286 | 0.279 | `InsufficientCycleImprovement { current_factor: 0.27864802101245156, candidate_factor: 0.2774256621786343, maximum_ratio: 0.98 }` |
| cover-latin-seed-900 | `all-pairs-cmg` | 0 | 1.461 | 1.461 | 0.0% | 0.316 | 0.316 | `InsufficientCycleImprovement { current_factor: 0.315717558226623, candidate_factor: 0.31571619482242896, maximum_ratio: 0.98 }` |
| cover-latin-seed-901 | `symmetric-map` | 0 | 1.316 | 1.316 | 0.0% | 0.240 | 0.240 | `InsufficientCycleImprovement { current_factor: 0.2399042682560214, candidate_factor: 0.23980576537029294, maximum_ratio: 0.98 }` |
| cover-latin-seed-901 | `all-pairs-cmg` | 0 | 1.366 | 1.366 | 0.0% | 0.268 | 0.268 | `InsufficientCycleImprovement { current_factor: 0.2678276503839816, candidate_factor: 0.2677483582281184, maximum_ratio: 0.98 }` |
| cover-weak-chain-seed-902 | `symmetric-map` | 0 | 301.118 | 301.118 | 0.0% | 0.972 | 0.972 | `InsufficientCycleImprovement { current_factor: 0.9716360072257365, candidate_factor: 0.9715351979103894, maximum_ratio: 0.98 }` |
| cover-weak-chain-seed-902 | `all-pairs-cmg` | 0 | 12.128 | 12.128 | 0.0% | 0.917 | 0.917 | `InsufficientCycleImprovement { current_factor: 0.9174135098535015, candidate_factor: 0.9171770982543073, maximum_ratio: 0.98 }` |

## Decision use

A positive result—material improvements on multiple structurally distinct
cases—would justify integrating complete-cycle split repair into the
automatic candidate portfolio and freezing a new unseen holdout. A weak or
isolated result would support closing issue #3 with the protected structural
baseline as the accepted automatic method and bootstrap/repair as a negative
research result.
