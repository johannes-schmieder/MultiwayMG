# Issue #3 recursive complete-cycle holdout results

## Verdict

**The frozen recursive automatic-coarsening gate does not pass.**

The holdout uses the predeclared seeded multi-level graph covers and policy
in `benchmarks/policies/issue3-recursive-cycle-v1.tsv`. Each automatic level
is proposed by conservative bootstrap/repair, screened through its actual
symmetric-MAP two-grid cycle, and admitted only after cumulative hierarchy
dimension and tuple budgets remain valid.

## Case matrix

| Case | Depth | One-shot accepted | One-shot recovery | Automatic accepted | Automatic depth | Automatic recovery | Oracle κ | Automatic κ | Dimension complexity | Tuple complexity | PCG iterations |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| recursive-communities-depth-3-seed-807 | 3 | True | 1.020 | False | 0 | — | 7.668 | — | 1.000 | 1.000 | NA |
| recursive-dominant-pair-depth-2-seed-806 | 2 | True | 1.297 | False | 1 | — | 1.634 | — | 1.510 | 1.941 | NA |
| recursive-latin-depth-2-seed-800 | 2 | True | 1.322 | False | 1 | — | 1.677 | — | 1.510 | 1.941 | NA |
| recursive-latin-depth-3-seed-801 | 3 | True | 1.300 | False | 1 | — | 1.775 | — | 1.500 | 1.947 | NA |
| recursive-nearly-nested-depth-2-seed-804 | 2 | True | 1.000 | True | 2 | 1.000 | 1.407 | 1.314 | 1.750 | 2.107 | 8 |
| recursive-nearly-nested-depth-3-seed-805 | 3 | True | 0.920 | False | 0 | — | 1.950 | — | 1.000 | 1.000 | NA |
| recursive-weak-chain-depth-2-seed-802 | 2 | True | 1.469 | False | 0 | — | 1,344 | — | 1.000 | 1.000 | NA |
| recursive-weak-chain-depth-3-seed-803 | 3 | True | 1.661 | False | 0 | — | 2,026 | — | 1.000 | 1.000 | NA |

## Aggregate diagnostics

- Accepted automatic hierarchies: **1 of 8**.
- Median accepted automatic oracle-recovery fraction: **1.000**.
- Cases improving recursive one-shot recovery by at least 0.10: **0**.
- Maximum accepted true residual: `9.688e-11`.
- Maximum accepted dimension complexity: `2.573`.
- Maximum accepted tuple complexity: `3.444`.
- Selected level-source counts: `Some(StructuralBaseline)` 2.

## Scientific gates

- [PASS] All eight supplied oracle hierarchies reach their requested depth and pass residual/complexity checks.
- [FAIL] At least 6 of 8 automatic hierarchies are accepted.
- [PASS] Every accepted automatic hierarchy reaches the exact requested terminal depth.
- [PASS] Median accepted automatic oracle-recovery fraction is at least 0.60.
- [FAIL] Automatic hierarchy improves recursive one-shot recovery by at least 0.10 in at least 2 cases.
- [PASS] No accepted automatic hierarchy regresses more than 0.10 below an accepted recursive one-shot hierarchy.
- [PASS] Every accepted traced PCG solve converges.
- [PASS] Every accepted final true residual is at most 1.0e-08.
- [FAIL] Every accepted hierarchy respects cumulative dimension and tuple budgets.
- [PASS] Every reported accepted level satisfies the frozen 0.50 complete-cycle factor gate.

## Interpretation

A passing result demonstrates that the one-level acceptance rule composes into
a bounded recursive hierarchy on unseen synthetic graph covers. It remains a
research feasibility result: production runtime, allocation-free workspaces,
large approximate pair solvers, and fereg's independent observation-space
certificate are separate milestones.
