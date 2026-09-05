# Issue 4 coarse-only CMG automatic-map calibration

Comparable certified rows: 32. Numerical/accounting gate: PASS.
Maximum true relative residual among comparable rows: 4.806902e-11.

This is calibration on the already-revealed recursive issue-3 fixtures, not an issue-4 holdout.
The automatic map plan and the fine `within` smoother are identical across methods; only non-finest smoothers change.
Ratios below one favor coarse CMG.

| Case | Depth | Solver | Hybrid/within outer work | Hybrid/within solve time | Hybrid/within fully charged time | First measured charged win | Coarse CMG components | Max pair vertices | Max CMG levels |
|---|---:|---|---:|---:|---:|---|---:|---:|---:|
| recursive-nearly-nested-depth-2-seed-804 | 2 | mlsmr | 1.000 | 1.074 | 1.025 | none-through-32 | 3 | 32 | 3 |
| recursive-nearly-nested-depth-2-seed-804 | 2 | pcg-traced | 1.000 | 1.297 | 1.090 | 1 | 3 | 32 | 3 |

Rejected automatic plans (not compared): recursive-communities-depth-3-seed-807, recursive-dominant-pair-depth-2-seed-806, recursive-latin-depth-2-seed-800, recursive-latin-depth-3-seed-801, recursive-nearly-nested-depth-3-seed-805, recursive-weak-chain-depth-2-seed-802, recursive-weak-chain-depth-3-seed-803.

Cells with at least 20% outer-work reduction at 32 RHS: 0/2.
Cells with a fully charged timing win at 32 RHS: 0/2.

A routing rule must not be selected from these cases and then described as holdout-validated; any such rule requires a fresh preregistered issue-4 holdout.
