# Issue 4 balanced size ladder

Rows: 864. Numerical/accounting gate: PASS.

This is calibration around an observed crossover, not a holdout. Timing is descriptive and never a CI pass criterion.

## 32-RHS outer-work ladder

Ratios are pair-CMG/within; below one favors pair-CMG. Reported values are paired medians of two independently built repeats. LSMR and PCG work units differ and are compared only within an outer solver.

| Family | Levels/factor | Tuples | LSMR work ratio | PCG work ratio | Direct / iterative pair terminals | Max CMG levels |
|---|---:|---:|---:|---:|---:|---:|
| planted-clones | 12 | 288 | 1.159 | 1.196 | 1 / 2 | 3 |
| planted-clones | 18 | 648 | 1.349 | 1.424 | 0 / 3 | 3 |
| planted-clones | 24 | 1152 | 1.335 | 1.397 | 1 / 2 | 4 |
| planted-clones | 36 | 2592 | 0.826 | 0.805 | 0 / 3 | 3 |
| planted-clones | 48 | 4608 | 0.785 | 0.758 | 0 / 3 | 3 |
| planted-clones | 72 | 10368 | 0.858 | 0.838 | 0 / 3 | 3 |
| noisy-clones | 12 | 324 | 1.093 | 1.107 | 1 / 2 | 3 |
| noisy-clones | 18 | 729 | 1.106 | 1.122 | 1 / 2 | 4 |
| noisy-clones | 24 | 1296 | 1.127 | 1.148 | 0 / 3 | 3 |
| noisy-clones | 36 | 2916 | 0.912 | 0.901 | 0 / 3 | 4 |
| noisy-clones | 48 | 5184 | 0.930 | 0.921 | 0 / 3 | 2 |
| noisy-clones | 72 | 11664 | 1.080 | 1.091 | 0 / 3 | 2 |
| latin-square | 12 | 144 | 1.204 | 1.247 | 2 / 1 | 3 |
| latin-square | 18 | 324 | 1.183 | 1.217 | 0 / 3 | 3 |
| latin-square | 24 | 576 | 1.176 | 1.200 | 0 / 3 | 3 |
| latin-square | 36 | 1296 | 0.852 | 0.833 | 0 / 3 | 5 |
| latin-square | 48 | 2304 | 0.856 | 0.838 | 0 / 3 | 3 |
| latin-square | 72 | 5184 | 0.920 | 0.909 | 0 / 3 | 3 |

## Observed fully charged economics

The final column is the first measured RHS prefix where the paired-median within/CMG setup-plus-solve ratio exceeds one. It is an observed prefix, not an interpolated break-even. `none through 32` means within remained faster at every measured prefix.

| Family | Levels | LSMR within/CMG at 32 RHS | First observed LSMR CMG win | PCG within/CMG at 32 RHS | First observed PCG CMG win |
|---|---:|---:|---|---:|---|
| planted-clones | 12 | 0.559 | none through 32 | 0.539 | none through 32 |
| planted-clones | 18 | 0.522 | none through 32 | 0.512 | none through 32 |
| planted-clones | 24 | 0.504 | none through 32 | 0.522 | none through 32 |
| planted-clones | 36 | 0.700 | none through 32 | 0.799 | none through 32 |
| planted-clones | 48 | 0.800 | none through 32 | 0.861 | none through 32 |
| planted-clones | 72 | 0.622 | none through 32 | 0.632 | none through 32 |
| noisy-clones | 12 | 0.625 | none through 32 | 0.603 | none through 32 |
| noisy-clones | 18 | 0.637 | none through 32 | 0.633 | none through 32 |
| noisy-clones | 24 | 0.612 | none through 32 | 0.666 | none through 32 |
| noisy-clones | 36 | 0.548 | none through 32 | 0.570 | none through 32 |
| noisy-clones | 48 | 0.785 | none through 32 | 0.789 | none through 32 |
| noisy-clones | 72 | 0.936 | none through 32 | 0.924 | none through 32 |
| latin-square | 12 | 0.576 | none through 32 | 0.486 | none through 32 |
| latin-square | 18 | 0.602 | none through 32 | 0.588 | none through 32 |
| latin-square | 24 | 0.601 | none through 32 | 0.606 | none through 32 |
| latin-square | 36 | 0.521 | none through 32 | 0.544 | none through 32 |
| latin-square | 48 | 0.569 | none through 32 | 0.602 | none through 32 |
| latin-square | 72 | 0.476 | none through 32 | 0.491 | none through 32 |

## Boundaries

Maximum true relative residual: 1.039452e-10. Sequential fallback allocations are zero by gate.

The ladder was chosen after seeing the earlier smoke/calibration crossover, so it can characterize that crossover but cannot qualify a policy. A routing rule must be frozen only after this analysis and tested on a fresh holdout.

## Provenance

This summary was produced by GitHub Actions run `33934987774` at head `6f413c04c45d0a0fa26e147441d001e7b9eae119`. The uploaded artifact `issue-4-size-ladder-output` had GitHub artifact digest `sha256:c78dc6b720f4294832559261d6ca33b5f6868fb8a56a80d982400d46acd5dd1d`. The raw `size-ladder.tsv` SHA-256 is `95817e755a3f464c6f60d44a31585562cb68129440694b977a0706df7eefee38`.