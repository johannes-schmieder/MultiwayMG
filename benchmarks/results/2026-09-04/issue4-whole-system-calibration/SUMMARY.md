# Issue 4 whole-system Schwarz smoke matrix

Rows: 288. Numerical/accounting gate: PASS.

This matrix removes the three-way coarse hierarchy and asks only whether the local pair solver changes work on the complete three-way system. Hosted-runner timing is descriptive and never a CI winner gate.

## CMG versus within: outer work

Ratios are pair-CMG/within. Values below one favor pair-CMG. LSMR counts weighted B and B' applications; traced PCG counts original-Gramian applications, so ratios are comparable only within a solver.

| Case | LSMR work min / median / max | PCG work min / median / max | CMG multilevel pair components |
|---|---:|---:|---:|
| planted-clones | 0.778 / 0.809 / 0.840 | 0.750 / 0.784 / 0.818 | 3 / 3 |
| noisy-clones | 0.852 / 0.852 / 0.920 | 0.833 / 0.871 / 0.909 | 3 / 3 |
| latin-square | 0.852 / 0.852 / 0.852 | 0.833 / 0.833 / 0.833 | 3 / 3 |
| weak-chain | 2.241 / 2.276 / 2.407 | 2.417 / 2.417 / 2.500 | 3 / 3 |
| disconnected-latin | 1.211 / 1.211 / 1.211 | 1.250 / 1.250 / 1.250 | 6 / 6 |
| unbalanced-cycle | 1.513 / 1.513 / 1.513 | 1.588 / 1.618 / 1.647 | 6 / 6 |

## Four-RHS charged timing

Each value is setup plus four solves for one outer solver. The range is the two repeat ratios within/CMG; greater than one favors CMG. These tiny hosted-runner timings are diagnostics, not qualification.

| Case | LSMR within/CMG min / max | PCG within/CMG min / max |
|---|---:|---:|
| planted-clones | 0.864 / 0.921 | 0.909 / 0.947 |
| noisy-clones | 0.905 / 0.919 | 0.890 / 0.926 |
| latin-square | 0.682 / 0.714 | 0.678 / 0.751 |
| weak-chain | 0.385 / 0.394 | 0.374 / 0.386 |
| disconnected-latin | 0.412 / 0.420 | 0.389 / 0.423 |
| unbalanced-cycle | 0.338 / 0.456 | 0.363 / 0.370 |

## Boundaries

Maximum true relative residual: 9.896386e-11. Maximum warnings on a build: 0. Sequential fallback workspace allocations: zero by gate.

The within retained-state number is only the wrapper categories exposed by the current API; it excludes the opaque inner preconditioner. Pair-CMG retained bytes are estimates of known retained categories, not process peak RSS.

This is still a small deterministic calibration matrix. It is not the broad topology/size calibration, thread study, changing-weight experiment, coarse-hierarchy comparison, or fresh holdout required to close issue #4.
