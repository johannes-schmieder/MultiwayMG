# Existing-solver regression after incremental admission (M6b)

Source `6a212e0ab19eea9f253f5588260e62681c9a89bb` was committed before
collection under unchanged [layout policy v2](../../../policies/prepared-layout-v2.json).
This checks the existing complete prepared solver after the new
[incremental structural builder](../../../../docs/ISSUE5_INCREMENTAL_HIERARCHY.md).
The measured solver uses supplied maps and does not invoke the builder.
Its separate Rust tests directly exercise incremental construction, admission,
failed-prefix recovery, allocation and complete current-weight solves.

| Artifact | Processes including warmups | Certified measured columns | Exact prior M6a records |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 2,520 |
| Mac development | 2,520 | 24,000 | 2,520 |
| Mac expanded, one RHS | 360 | 300 | 360 |
| Linux smoke | 2,520 | 24,000 | 2,520 |

All 7,920 processes pass accounting, certification, repeatability and RSS gates.
All 72,300 measured columns certify, with no errors, timeouts, rejected columns,
RSS failures or measured retries. The 6,336 within-source layout comparisons
are exact. Every process matches its M6a input hash, dimensions, route, repeat,
numerical/work/fingerprint record, every retained payload category, grouping
admission and level/layout inventory. These are regression results, not a
cross-revision timing gain or incremental-setup performance measurement.

Complete setup/solve/certificate/output times, prefixes, scoped memory and RSS
remain in the summaries. Within-binary layout ratios do not compare M6b with
M6a. Scalar remains default. The expanded profile is one RHS at V=12,288 and
at most 73,726 tuples; no large/out-of-cache, thread-scaling, calibration or
campaign-holdout qualification is claimed.

The [receipts](receipts.json) identify source/tree/policy, exact executables,
current/prior manifests and deterministic archives. Full raw/binary artifacts
are preserved under `$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-incremental-6a212e0`.
Copies independently revalidate, saved summaries reproduce exactly, and every
archived member is rehashed. This is local preservation, not an independent
off-machine backup. An initial archival helper stopped on an old input-path
prefix before copying any artifact; its correction did not rerun measurements.

Required Rust 1.85 checks and 90 Python validators pass. Release all/minimal
complete solvers/allocators, every builder reservation failure and 120 actual
protocol comparisons pass. Eight historical recursive fixtures times three
current-weight generations times 32 RHS produce 768 exact certified comparisons
per PCG/gated-LSMR route between incremental and supplied construction. Those
fixtures predate and are separate from the untouched campaign holdout.
Source workflows `34086907166`/`34086907201` and PR workflows
`34086939140`/`34086939147` passed; Linux evidence is from source run
`34086907166`. Self-review covered ownership, checked sizing/admission,
failed publication, component maps, report meanings, numerical identity and
exact release. This is self-review, not independent external review. Final
evidence-head CI and PR55 merge remain at this checkpoint.

M6 remains open for component-local construction, bounded numerical terminals,
bottom-up actual-cycle screening and the complete automatic driver; M7–M10
follow. The partition design keeps inverse recoding in temporary setup storage
so it can be released before numerical workspace allocation.
