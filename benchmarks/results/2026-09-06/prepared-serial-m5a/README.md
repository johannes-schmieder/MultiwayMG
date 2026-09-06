# M5a: serial passes and scratch reuse

Measured candidate `754f6d622d6040c968dd931d7e52973fa42c517a` and the paired recipe
were committed before collection. The baseline is the preserved M4 v2 executable
from `3c1273eb00698c59b27eb79d1539c137fe3632a9`. Both Mac paired profiles validate
in full: identical inputs, complete numerical diagnostics, coefficient fingerprints,
work counts and native-control negatives, with exactly the derived payload reduction.
PCG and gated LSMR each certify all 2,400 measured columns per source/profile.
The exact-source Linux v2 smoke also passes full candidate-route certification.
No process errors, timeouts or observed RSS-budget failures occurred.

M5a removes empty MAP factor scans and reuses dead scratch while preserving all
arithmetic and projections. Each transition loses four allocated arrays. MAP
scratch falls from four coefficient vectors to three; cycle traversal falls from
seven vectors to four. The exact fresh hierarchy-workspace reduction is 1,656
bytes in smoke and 21,816 bytes in development, including descriptor changes.
All other retained payload owners are unchanged. Complete two-transition LSMR
and PCG allocation tests now reserve/release 40/34 arrays, previously 48/42;
first/repeated scalar and every RHS width still allocate nothing.

## Paired complete costs

Each Mac profile contains 1,512 isolated processes: 252 warmups and 1,260 measured
processes, 14,400 measured columns total. Old/new order alternates inside each
case/route/repetition, with the frozen three-route rotation. Reported speedups are
old time divided by new time; above one is faster. A cell uses the median of five
paired ratios, and each route aggregate is the geometric mean over every one of
its 42 balanced cells. Inner cost includes decode, fresh preparation, all
solve/certificate/output work and destruction. External process costs additionally
include cold startup and serialization. Compilation is outside solve timing;
both build identities/logs and exact binaries are preserved.

| Profile | PCG inner speedup | Gated LSMR inner speedup | PCG process speedup | Gated LSMR process speedup |
| --- | ---: | ---: | ---: | ---: |
| Mac smoke | 1.0469x | 1.0533x | 1.0104x | 1.0123x |
| Mac development | 1.0610x | 1.0629x | 1.0527x | 1.0545x |

Every development candidate-route cell has an inner median ratio above one;
individual repetitions, minima/maxima and all native-control timings remain in
the complete summary. The table below reports width-32 median inner milliseconds
for both executables, alongside the median paired ratio. The ratio need not equal
the ratio of marginal medians.

| Family | Weights | Route | M4 ms | M5a ms | Paired speedup |
| --- | --- | --- | ---: | ---: | ---: |
| uniform | unit | pcg | 177.750 | 168.568 | 1.0557x |
| uniform | unit | lsmr-gated | 281.691 | 266.084 | 1.0621x |
| uniform | heterogeneous | pcg | 231.583 | 213.097 | 1.0867x |
| uniform | heterogeneous | lsmr-gated | 337.432 | 309.822 | 1.0897x |
| communities | unit | pcg | 143.541 | 135.370 | 1.0609x |
| communities | unit | lsmr-gated | 217.841 | 207.568 | 1.0577x |
| communities | heterogeneous | pcg | 165.278 | 155.472 | 1.0658x |
| communities | heterogeneous | lsmr-gated | 239.764 | 224.481 | 1.0665x |
| chain | unit | pcg | 167.560 | 155.207 | 1.0796x |
| chain | unit | lsmr-gated | 204.070 | 189.957 | 1.0709x |
| chain | heterogeneous | pcg | 189.687 | 176.872 | 1.0794x |
| chain | heterogeneous | lsmr-gated | 225.334 | 210.526 | 1.0682x |

Native LSMR still rejects 560 columns per smoke source and 935 per development
source. Those attempts remain in all raw evidence and coverage totals; they are
not time-to-certified-solution baselines. The M5a gate checks correctness and
memory; timing results are separate observations, not M10 qualification.

The Mac is an unpinned serial Apple M2 Ultra, 192 GiB, 24 logical CPUs. Development
has 8,271–16,571 tuples and only 384 coefficients. This cache-resident supplied-map
matrix does not establish the best large-V layout, automatic hierarchy or CPU
parallelization strategy. Sampling/profile experiments and larger declared
development cases must precede grouped-layout selection. No calibration or campaign
holdout was used. The roughly 6% inner improvement is useful engineering progress,
not a competitive-solver result.

## Reproducibility and next increment

[Receipts](receipts.json) identify every source/tree, policy, executable, manifest,
summary and archive hash. Full paired child artifacts, order traces, raw resources,
binaries and deterministic archives are outside Git at
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`.
Use the measured paired scripts to validate extracted Mac archives, and the v2
validator for the Linux archive. The exact-source Linux artifact is from
[CI run 34064382081](https://github.com/johannes-schmieder/MultiwayMG/actions/runs/34064382081),
`prepared-serial-gated-smoke`. Local preservation is not an independent backup.

Required local Rust 1.85 checks, independent frozen-loop tests, full numerical
and allocation/release gates and all 61 Python validators passed. Exact-source
workflows `34064382081`/`34064382036` and PR workflows
`34064411985`/`34064412162` passed before the evidence commit. Final evidence-head
CI, review and merge remain required. M5 remains open for measured kernel/layout
work, flat scratch, fused transfers and exact-RHS certificate-reference reuse;
M6–M10 remain open. Preserve this increment and the M4 binary as distinct controls.
