# Issue 4 coarse-only CMG oracle-map calibration

Comparable certified rows: 224. Numerical/accounting gate: PASS.
Maximum true relative residual among comparable rows: 9.930245e-11.

This is calibration on the already-revealed recursive issue-3 fixtures, not an issue-4 holdout.
The revealed issue-3 oracle map sequence and the fine `within` smoother are identical across methods; only non-finest smoothers change.
Ratios below one favor coarse CMG.

| Case | Depth | Solver | Hybrid/within outer work | Hybrid/within solve time | Hybrid/within fully charged time | First measured charged win | Coarse CMG components | Max pair vertices | Max CMG levels |
|---|---:|---|---:|---:|---:|---|---:|---:|---:|
| recursive-dominant-pair-depth-2-seed-806 | 2 | mlsmr | 1.000 | 1.043 | 1.042 | none-through-32 | 3 | 32 | 3 |
| recursive-dominant-pair-depth-2-seed-806 | 2 | pcg-traced | 1.000 | 1.188 | 1.184 | none-through-32 | 3 | 32 | 3 |
| recursive-latin-depth-2-seed-800 | 2 | mlsmr | 1.000 | 1.112 | 1.106 | none-through-32 | 3 | 32 | 3 |
| recursive-latin-depth-2-seed-800 | 2 | pcg-traced | 1.003 | 1.204 | 1.195 | 1 | 3 | 32 | 3 |
| recursive-latin-depth-3-seed-801 | 3 | mlsmr | 0.987 | 1.315 | 1.305 | none-through-32 | 6 | 64 | 3 |
| recursive-latin-depth-3-seed-801 | 3 | pcg-traced | 0.985 | 1.438 | 1.422 | 1 | 6 | 64 | 3 |
| recursive-nearly-nested-depth-2-seed-804 | 2 | mlsmr | 1.000 | 1.134 | 1.130 | none-through-32 | 3 | 32 | 3 |
| recursive-nearly-nested-depth-2-seed-804 | 2 | pcg-traced | 1.000 | 1.109 | 1.105 | none-through-32 | 3 | 32 | 3 |
| recursive-nearly-nested-depth-3-seed-805 | 3 | mlsmr | 1.000 | 1.210 | 1.202 | none-through-32 | 6 | 64 | 4 |
| recursive-nearly-nested-depth-3-seed-805 | 3 | pcg-traced | 0.997 | 1.180 | 1.173 | none-through-32 | 6 | 64 | 4 |
| recursive-weak-chain-depth-2-seed-802 | 2 | mlsmr | 1.018 | 0.999 | 0.998 | 32 | 3 | 32 | 3 |
| recursive-weak-chain-depth-2-seed-802 | 2 | pcg-traced | 1.008 | 0.868 | 0.868 | 1 | 3 | 32 | 3 |
| recursive-weak-chain-depth-3-seed-803 | 3 | mlsmr | 1.024 | 1.170 | 1.168 | none-through-32 | 6 | 64 | 4 |
| recursive-weak-chain-depth-3-seed-803 | 3 | pcg-traced | 1.034 | 1.154 | 1.151 | none-through-32 | 6 | 64 | 4 |

Baseline-inadmissible hierarchies (excluded from solver ratios because the all-`within` hierarchy itself failed the outer SPD/certification gate): recursive-communities-depth-3-seed-807.

Cells with at least 20% outer-work reduction at 32 RHS: 0/14.
Cells with a fully charged timing win at 32 RHS: 2/14.

A routing rule must not be selected from these cases and then described as holdout-validated; any such rule requires a fresh preregistered issue-4 holdout.
