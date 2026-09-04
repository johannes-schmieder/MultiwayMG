# Issue #2 final oracle two-grid and V-cycle results

## Verdict

**The oracle feasibility gate passes.** On all nine one-level structural families, adding the exact factor-preserving coarse correction improved the corresponding Jacobi, symmetric-MAP, exact-pair, and pair-CMG smoothers. Every admitted preconditioner was symmetric and positive on the complete numerical range, every traced PCG solve passed its recomputed original-Gramian residual, and 2–5-level resolution sequences retained stable iteration counts with tuple complexity below 3.

This establishes that a good hard factor-respecting coarse space is intrinsically capable of supplying the missing global three-way correction. It does not establish that automatic aggregation can recover the oracle maps or that pair-CMG is the best production smoother after complete large-system cost is charged.

## One-level spectral matrix

| Family | Jacobi κ | MAP κ | Pair-CMG κ | Two-grid Jacobi κ | Two-grid MAP κ | Two-grid pair κ | Pair PCG iters | Two-grid pair iters |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| disconnected-ragged-depth | 3.659 | 1.035 | 1.661 | 1.333 | 1.000 | 1.000 | 8 | 1 |
| dominant-pair-weak-third | 159.5 | 27.88 | 2.943 | 1.491 | 1.001 | 1.007 | 10 | 4 |
| hub-power-law | 7.148 | 1.396 | 2.536 | 1.414 | 1.001 | 1.003 | 9 | 4 |
| latin-square | 4.265 | 1.092 | 1.795 | 1.487 | 1.001 | 1.007 | 10 | 4 |
| nearly-nested | 536.7 | 90.57 | 2.984 | 1.488 | 1.001 | 1.007 | 12 | 4 |
| planted-communities | 75.25 | 24.60 | 1.987 | 1.499 | 1.001 | 1.008 | 9 | 4 |
| tensor-grid | 3.779 | 1.045 | 1.683 | 1.511 | 1.001 | 1.011 | 8 | 4 |
| weak-chain | 891.6 | 294.5 | 2.059 | 1.610 | 1.003 | 1.014 | 12 | 4 |
| weight-dynamic-range | 1,589,289 | 274,941 | 1,367 | 1.650 | 1.013 | 1.021 | 20 | 4 |

The difficult weight-dynamic-range case spans twelve orders of magnitude in positive tuple weights. Its condition number fell from approximately `1,589,289` under diagonal scaling and `1,367` under pair-CMG to `1.021` with the pair-CMG two-grid cycle.

The exact coarse correction alone is intentionally reported as semidefinite: it leaves a unit stationary error mode and is never passed to PCG as if it were a complete positive preconditioner. This is the expected, honest representation of coarse-only information.

## Improvement counts

- `two-grid-jacobi` improved on Jacobi in **9 of 9** families; median condition-number ratio `0.0199`, maximum ratio `0.3999`.
- `two-grid-symmetric-map` improved on symmetric MAP in **9 of 9** families; median condition-number ratio `0.0407`, maximum ratio `0.9666`.
- `two-grid-exact-pair` improved on exact pair Schwarz in **9 of 9** families; median condition-number ratio `0.4922`, maximum ratio `0.6020`.
- `two-grid-pair-cmg` improved on pair-CMG in **9 of 9** families; median condition-number ratio `0.4922`, maximum ratio `0.6020`.
- Exact pair Schwarz and pair-CMG differed by at most `0.000e+00` in condition number on these small direct-terminal pair systems. This is a reference equivalence, not evidence about approximate large-pair CMG.
- A single selected pair plus MAP had a median condition-number ratio of `13.13` relative to all-three-pair CMG. Selected-pair correction is therefore not a generally sufficient substitute in this matrix.
- Maximum active-method true residual: `9.986e-11`. Maximum full-action symmetry defect: `6.092e-16`. Minimum admitted preconditioner energy: `2.223e-07`.

## Multilevel resolution sequences

| Family | Depth | Diagonal iters | Pair-CMG iters | Oracle Jacobi | Pair finest | Pair first two | Pair all levels | MAP all levels | Tuple complexity |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| community | 2 | 19 | 10 | 10 | 5 | 4 | 4 | 3 | 1.312 |
| community | 3 | 20 | 12 | 10 | 4 | 4 | 4 | 3 | 1.328 |
| community | 4 | 21 | 12 | 10 | 4 | 4 | 4 | 3 | 1.332 |
| community | 5 | 22 | 12 | 9 | 4 | 4 | 4 | 3 | 1.333 |
| latin | 2 | 13 | 10 | 10 | 4 | 4 | 4 | 3 | 1.312 |
| latin | 3 | 14 | 10 | 10 | 4 | 4 | 4 | 3 | 1.328 |
| latin | 4 | 14 | 11 | 9 | 4 | 4 | 4 | 3 | 1.332 |
| weak-chain | 2 | 19 | 11 | 10 | 4 | 4 | 4 | 3 | 1.312 |
| weak-chain | 3 | 21 | 12 | 10 | 5 | 4 | 4 | 3 | 1.328 |
| weak-chain | 4 | 21 | 12 | 10 | 4 | 4 | 4 | 3 | 1.332 |
| weak-chain | 5 | 22 | 12 | 10 | 4 | 4 | 4 | 3 | 1.333 |

Across all scheduled families and depths, maximum tuple complexity was `1.333`, maximum dimension complexity was `1.969`, and maximum final true residual was `9.435e-11`.

Pair-CMG only on the finest level captured nearly all of the numerical benefit of retaining pair-CMG at every level in these oracle refinements. Symmetric MAP at every level was typically the strongest and materially lighter retained-state option. These are high-value hypotheses for the production architecture, not automatic routing decisions.

## Setup and apply diagnostics

The phase-separated timing matrix records coarsening, smoother construction, pair graph construction, CMG construction, pair workspace construction, terminal construction, complete setup, and median fixed preconditioner application. Hosted-runner nanosecond timings are descriptive only.

| Method | Median setup | Median apply | Median retained state |
|---|---:|---:|---:|
| diagonal | 0.17 µs | 0.03 µs | 9.7 KiB |
| symmetric-map | 0.08 µs | 10.83 µs | 9.4 KiB |
| pair-cmg-all | 84.69 µs | 2.85 µs | 30.1 KiB |
| exact-first-coarse | 40.95 µs | 2.54 µs | 14.7 KiB |
| oracle-jacobi | 34.83 µs | 7.01 µs | 14.6 KiB |
| oracle-map-all-levels | 32.86 µs | 30.61 µs | 14.3 KiB |
| oracle-pair-finest | 111.84 µs | 13.07 µs | 34.4 KiB |
| oracle-pair-first-two | 118.05 µs | 13.10 µs | 36.6 KiB |
| oracle-pair-all-levels | 113.14 µs | 13.04 µs | 36.6 KiB |

## Acceptance gates

- [x] Oracle coarse cycles materially improve a predeclared majority over both Jacobi and pair-CMG; in fact, every corresponding comparison improved in all nine one-level families.
- [x] Every admitted preconditioner is numerically symmetric and positive on the complete numerical range.
- [x] Tuple complexity remains below the provisional limit of 3 through five supplied levels.
- [x] PCG iteration counts are stable: every family/schedule spread across the resolution sequence is at most two iterations.
- [x] Every returned solve passes a recomputed original-Gramian residual, with full per-iteration traces retained.
- [x] Coarse-only and other incomplete actions are exposed honestly instead of hidden behind a diagonal fallback.
- [x] Phase setup timing, principal retained memory, and apply scratch estimates are reported.
- [x] CI repeats the deterministic matrices and byte-compares their outputs.

## Scientific conclusion

Issue #2 is resolved positively. The binding research problem is no longer whether a factor-preserving three-way hierarchy can work when given a good coarse space. It can. The next scientific risk is whether an automatic algorithm can discover a sufficiently small, low-tuple-complexity approximation to that oracle space at acceptable setup cost.

## Limitations and handoff

The matrices are still small enough for dense quotient-space analysis, and the oracle refinements deliberately encode a known hierarchy. The result does not establish production runtime superiority. In particular, the small pair systems make CMG coincide with exact pair solves. Issue #4 must compare approximate CMG with the existing approximate-Cholesky pair solver on large identical domains. Issue #3 should use compatible relaxation and bounded bootstrap repair to measure the automatic-to-oracle gap. Issue #5 should convert the most promising schedule—likely MAP or pair-CMG only at the finest level—into allocation-free prepared state for repeated right-hand sides and changing weights.
