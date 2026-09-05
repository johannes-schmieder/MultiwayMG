# Issue 4 checkpoint: certified pair-local economics

> **Historical checkpoint.** This document records the state after PR #12 on
> September 4, 2026. Issue #4 was completed on September 5, 2026; the final
> synthesis and policy are in [`ISSUE4_FINAL_RESULTS.md`](ISSUE4_FINAL_RESULTS.md)
> and [`ADR_0002_ISSUE4_PAIR_SOLVER_POLICY.md`](ADR_0002_ISSUE4_PAIR_SOLVER_POLICY.md).
> Statements below that issue #4 “remains open” are preserved as historical
> descriptions of this checkpoint, not the current project status.

Date: 2026-09-04. Status: substantial infrastructure merged in PR #12;
**issue #4 remains open**. No production routing decision is authorized.

## What is now implemented

The component-local fixed-CMG adapter uses the existing generic Schwarz
executor, while the comparator uses the public pinned `within` preconditioner.
The pair-local experiment compares Jacobi, an exact pseudoinverse, one fixed
CMG cycle and `within-default` on identical connected weighted bipartite graphs.
It separates common graph construction, local setup, workspace initialization,
application cost and certified repeated-RHS solve time. Failed attempts retain
their cost. Unknown retained memory is explicitly unknown, not zero.

The permanent GitHub Actions gate checks algebra, failure behavior, complete
matrix coverage, accounting identities and true residuals. It does not require
CMG to win. The issue #3 coarse-space policy and dependency pins are unchanged.

During review, the adapters were hardened against nonfinite inputs and
intermediate overflow; projection uses component-local scaling and retains
workspaces after ordinary input failures. Compensated pair-marginal sums
preserve representable small positive tuple masses. The dynamic-weight fixture
was corrected to cover all prescribed powers from 1e-3 to 1e3, with a regression
test. Earlier four-order dynamic-fixture timings are not the evidence below.

## Frozen evidence and provenance

- Implementation PR: #12, merged as `2ff28724b65c031070a0e04a97c790c81cd4a590`.
- Reviewed source head: `ad608f856ac5e61f94b80d74550597a8fae02c11`.
- Tested PR merge: `236c6c65c8ee70a6c294b5e7f992dc160e64db1e`.
- Tested/merged source tree: `e013333ede421b667a775c658eb091dd0a18938c`.
- GitHub Actions run: `33919335599`; all six jobs passed.
- Artifact: `9954398642`, `issue-4-pair-local-output`.
- Original artifact ZIP SHA256:
  `0cd28a983a0cd20966fdd79481256e1c843344641d6ebf0339a4ee3928fa756b`.
- Raw TSV SHA256:
  `db3d0aded48fffe2158431a7bfea0ff08bdd081141fd54f7c3dc2e071567a70e`.

The byte-preserved artifact files and source manifest are archived under
[`benchmarks/results/2026-09-04/issue4-pair-local-smoke/`](../benchmarks/results/2026-09-04/issue4-pair-local-smoke/).
The archived [generated summary](../benchmarks/results/2026-09-04/issue4-pair-local-smoke/SUMMARY.md)
contains all prefixes, paired repeat ranges and conditional crossover models.
The [protocol](ISSUE4_PAIR_LOCAL_PROTOCOL.md) defines measurement boundaries.

The run used Rust 1.85.0 on a hosted AMD EPYC 7763 Linux VM, with Rayon,
OpenMP and OpenBLAS thread counts fixed to one. There were five graph families,
32 levels per factor (64 vertices), four methods, three independently built
repeats and cumulative RHS prefixes 1, 4, 16 and 32: **240 rows and 1,920 measured
RHS solves**. The cumulative prefixes are correlated, not additional independent
replicates. The maximum independently recomputed relative normal residual was
**8.307600e-11**, below the admission tolerance 1e-8. All rows were certified;
there were no build warnings.

## Measured findings

The table reports paired median total-time speedups at **32 RHS**. Setup and
certification are included. A speedup above one favors CMG. The work ratio is
CMG/within for actual B and B' applications, excluding separately recorded
certificate applications. These are descriptive small-fixture measurements,
not confidence intervals or a scaling claim.

| Pair graph | Within time / CMG time | CMG / within operator work | Jacobi time / CMG time | Interpretation |
|---|---:|---:|---:|---|
| Weak communities | 2.855 | 0.318 | 1.227 | Most promising local candidate in this matrix; beats both controls. |
| Dense | 1.274 | 0.639 | 0.655 | Beats within, but Jacobi remains cheaper. |
| Hubs | 9.796 | 0.119 | 0.668 | One-level diagonal iterative terminal, not a multilevel gain; Jacobi is cheaper. |
| Path | 0.131 | 9.850 | 1.334 | Within is much cheaper; CMG performs substantially more outer work. |
| Six-order weight variation | 0.508 | 1.831 | 1.969 | CMG improves on Jacobi, but within is cheaper at 32 RHS. |

The attractive hub headline must not be interpreted as evidence for native
multigrid: its CMG hierarchy has one level and no direct factor, with terminal
reason `FullContraction`. The terminal is iterative diagonal action. The dense
case has two levels and an iterative `StagnatedVertexReduction` terminal. Weak
communities have two levels and a direct terminal; paths and the dynamic-weight
fixture have three levels and a direct terminal. Those distinctions are saved
in the raw evidence and generated summary.

The varying-weight case also illustrates why setup amortization is not always
an eventual win for the more elaborate method. CMG has lower setup here but a
slower per-RHS solve. Under the explicitly conditional S+n*T model fitted to the
32-RHS prefix, its winning window is only n=1 or 2; there is no long-run CMG win.
The observed one-RHS median favors CMG by 1.515, whereas the four-RHS median is
0.818. The modeled n=2 result is an interpolation, not an observed measurement.

## What this does and does not establish

The implementation and evidence pipeline are ready for larger comparisons.
The results do **not** show broad CMG superiority, a completed three-way solver
speed advantage, or a valid deterministic routing rule. Weak-community domains
are a sensible priority for the next investigation, but selection must be based
on declared structural/numerical properties and qualified on fresh data, not on
these elapsed times.

The process peak RSS was 4,408 KiB for the entire already-built smoke executable.
It includes all routes and dense diagnostics; it cannot be attributed to one
solver or used as a solver-memory comparison. The pinned within API does not
expose complete retained bytes. CMG graph sharing also prevents naive summing
of all reported byte categories. Complete lifetime accounting remains open.

All graphs here are tiny connected pair domains in a hot process. Setup is
rebuilt for each measured repeat, but discarded warm-up builds mean that process
startup and the first global thread-pool startup are outside the measurement.
No large-domain scaling, disconnected mixture, multi-thread or changing-weight
result has been measured by this checkpoint. This matrix is not a held-out
qualification set. The review was performed in the implementing session, not by
an independent reviewer.

## Remaining issue #4 plan

1. Freeze a broader topology/size/component calibration matrix, with declared
   Jacobi, exact-where-feasible, within and stationary-CMG controls. Preserve
   both positive and negative families and terminal metadata.
2. Compare the actual three-way Schwarz adapters under modified LSMR and
   projected PCG, then integrate them with the unchanged issue #3 coarse maps.
   Distinguish local-solver effects from the true three-way coarse correction.
3. Measure single-/multi-thread setup, apply, repeated-RHS and complete retained
   and peak memory. Then compare selected-pair/component and fixed-cycle
   portfolios, including changing positive weights without claiming untested
   symbolic reuse.
4. Freeze any proposed structural admission rule before a fresh holdout. Only
   then evaluate the issue's outer-work and end-to-end economics gates and decide
   between broad, selective, coarse-only or no CMG local-solver advantage.

GitHub issue #4 remains the planning authority. Issue #5 owns the broader
allocation-free state and changing-weight production redesign.
