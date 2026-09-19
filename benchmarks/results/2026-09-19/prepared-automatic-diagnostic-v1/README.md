# M6k complete automatic cost diagnostics

Measured source `0caa55192cb8be2d427999d98b2b720913252dda`; separate frozen
[diagnostic policy](../../../policies/prepared-automatic-diagnostic-v1.json).
Mac smoke/development and exact-source Linux smoke each preserve 2,160 isolated
processes, 1,080 exact reference/profile pairs and 29,520 certified columns.
Across all three: 6,480 processes, 3,240 exact pairs, 88,560 certified columns,
zero failed processes, timeouts, protocol errors or RSS-budget violations.
This policy has nine arms and excludes the frozen v1 identity performance arm;
its earlier identity failures remain preserved, not resolved by this increment.

`analysis.json` verifies 360 unchanged M6j scalar signatures per profile (1,080
total), retaining numerical, work, payload, routing, error and input fields.
Only new schema/diagnostic/layout fields, inline record size and clocks are
excluded from that cross-protocol comparison. All layouts also preserve exact
math/work records within each route; layout payloads remain explicit. Reference/
profile comparisons retain every payload and the shared 6,608-byte inline record.
Automatic TLS is 592 bytes; existing kernel profiler TLS is 6,960 bytes, both
fixed capacities outside requested heap scopes. Full process RSS is separate.

## Diagnostic attribution

These are sum-weighted shares within instrumented automatic scalar driver time
across the 40 development cases at each width. One sample per arm; no observer
correction, timing ratios or competitive inference.

| Region | K=1 | K=8 | K=32 |
|---|---:|---:|---:|
| Screening | 35.05% | 12.01% | 3.89% |
| Structural preparation | 22.19% | 7.35% | 2.39% |
| Coarse factor | 12.44% | 4.22% | 1.36% |
| Local solve | 27.77% | 74.41% | 90.43% |
| Original certificate | 0.44% | 1.18% | 1.52% |

Family behavior differs: screening is the largest one-RHS region in six
families; local solve dominates nested and pair-dominant; coarse factor leads
worker-firm-occupation; direct factor leads ragged. All-row/all-image grouping
accounts for less than 0.42% of its own instrumented one-RHS driver inventory on
this bounded matrix, but this does not establish a grouped speedup or larger-case
cost. Smoke chiefly measures small direct factors and is not evidence for a
hierarchy win. Exact per-cell regions and all outer costs remain in the summaries
and archived raw journal; shares are not savings obtainable by deleting a stage.

Next: preregister a separate uninstrumented layout experiment with complete cold
costs, matched scalar/all-row MAP controls, rotated paired repetitions and larger
development cases. Preserve numerical policy while comparing layouts. Then
consider separate terminal-depth/admission and screening candidates, requiring
fresh charged comparisons and original certificates. M7–M10 remain open.

## Preservation and qualification

`receipts.json` records checksums, source/build provenance and portable archive
paths. All three copies independently revalidate to exactly identical summaries;
every deterministic archive member is checked against its source hash. Raw
binaries/logs/journals live under `$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6k/`.
The first local collection stopped after its reference build because the CLT
Python lacked tomllib; no solver attempts occurred. The next launch hit the
fresh-directory guard. Both failures and the build-only directory are preserved;
Homebrew Python collected the final artifacts. No solver measurement was retried.

Required Rust1.85/release and 115 Python tests pass. See the
[allocation/protocol qualification](../automatic-diagnostic-qualification/README.md).
Source and PR checks pass; final evidence-head checks precede guarded merge.
Default selection, calibration, holdout and competitive qualification are absent.
