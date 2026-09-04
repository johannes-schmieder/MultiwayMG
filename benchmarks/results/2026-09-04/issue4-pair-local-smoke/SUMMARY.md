# Issue 4 pair-local smoke evidence

Rows: 240. Numerical/accounting gate: PASS.

This is an identical-domain, single-pair experiment, not a three-way end-to-end speed claim. Timing is descriptive; it is never a CI pass criterion.

## CMG versus within default

Speedup is within/CMG (greater than one favors CMG). The range is min/median/max of three paired repeat ratios, not a confidence interval. Operator-work ratio is CMG/within for B and B' calls, excluding separately recorded certification calls.

| Fixture | RHS | Total speedup min / median / max | Operator-work ratio |
|---|---:|---:|---:|
| dense-32 | 1 | 1.919 / 1.991 / 2.107 | 0.636 |
| dense-32 | 4 | 1.459 / 1.462 / 1.485 | 0.636 |
| dense-32 | 16 | 1.297 / 1.303 / 1.304 | 0.641 |
| dense-32 | 32 | 1.269 / 1.274 / 1.280 | 0.639 |
| dynamic-32 | 1 | 1.432 / 1.515 / 1.800 | 1.824 |
| dynamic-32 | 4 | 0.781 / 0.818 / 0.849 | 1.824 |
| dynamic-32 | 16 | 0.545 / 0.552 / 0.565 | 1.824 |
| dynamic-32 | 32 | 0.506 / 0.508 / 0.516 | 1.831 |
| hubs-32 | 1 | 7.099 / 7.440 / 8.182 | 0.121 |
| hubs-32 | 4 | 9.099 / 9.302 / 9.336 | 0.120 |
| hubs-32 | 16 | 9.335 / 9.992 / 10.039 | 0.120 |
| hubs-32 | 32 | 8.922 / 9.796 / 10.201 | 0.119 |
| path-32 | 1 | 0.529 / 0.797 / 1.018 | 9.400 |
| path-32 | 4 | 0.220 / 0.327 / 0.444 | 9.400 |
| path-32 | 16 | 0.119 / 0.165 / 0.182 | 9.675 |
| path-32 | 32 | 0.114 / 0.131 / 0.136 | 9.850 |
| weak-32 | 1 | 3.510 / 3.592 / 3.639 | 0.333 |
| weak-32 | 4 | 2.793 / 3.161 / 3.226 | 0.321 |
| weak-32 | 16 | 2.863 / 2.894 / 2.942 | 0.318 |
| weak-32 | 32 | 2.854 / 2.855 / 2.859 | 0.318 |

## Jacobi control and CMG terminal

A win over within alone does not justify CMG when Jacobi is cheaper. A one-level iterative terminal is diagonal iteration, not a demonstrated multilevel gain. These are paired median total-time ratios at 32 RHS.

| Fixture | Jacobi / CMG time | CMG levels | CMG terminal | Direct factor |
|---|---:|---:|---|---|
| dense-32 | 0.655 | 2 | StagnatedVertexReduction | false |
| dynamic-32 | 1.969 | 3 | Direct | true |
| hubs-32 | 0.668 | 1 | FullContraction | false |
| path-32 | 1.334 | 3 | Direct | true |
| weak-32 | 1.227 | 2 | Direct | true |

## Conditional RHS crossover model

S+n*T uses median charged setup and median time per RHS from the 32-RHS prefix. This assumes future RHS cost resembles that prefix; it is not a measured extrapolation or a routing rule. Strict integer wins exclude ties. Compare against the observed prefixes above.

| Fixture | CMG setup (ms) | Within setup (ms) | CMG / within per RHS (ms) | Modeled CMG-winning RHS counts |
|---|---:|---:|---:|---|
| dense-32 | 0.083 | 0.293 | 0.1916 / 0.2386 | n >= 1 |
| dynamic-32 | 0.024 | 0.113 | 0.0741 / 0.0344 | only 1 <= n <= 2; no long-run win |
| hubs-32 | 0.015 | 0.100 | 0.0168 / 0.1656 | n >= 1 |
| path-32 | 0.028 | 0.097 | 0.1190 / 0.0108 | never |
| weak-32 | 0.029 | 0.145 | 0.0420 / 0.1181 | n >= 1 |

## Numerical and measurement boundaries

Maximum independently recomputed relative normal residual: 8.307600e-11. Build warnings across measured builds: 0. Certified rows with a false recurrence flag: 0.

Known memory columns are not total process memory. CMG's graph storage can be shared with the common graph; do not add those columns as disjoint allocations. The pinned within preconditioner is opaque, so its principal retained bytes are NA. Process peak RSS, when present, includes all routes and dense diagnostics in that process, not an attributable solver peak. See docs/ISSUE4_PAIR_LOCAL_PROTOCOL.md.

Issue #4 remains open: large domains, fresh holdout, complete memory, multi-threading, changing weights, whole-system Schwarz and frozen coarse-hierarchy comparisons are not resolved here.
