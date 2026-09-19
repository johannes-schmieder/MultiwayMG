# Existing-solver regression after component integration (M6h)

Source `654ddd16b6a66442c74fe98bc5d5fb12a25d6514` (tree
`5a1ccefa11d2e23f46dff678bf782c9bcf38a2d2`) was committed before collection
under unchanged [layout policy v2](../../../policies/prepared-layout-v2.json).
This supplied-map regression checks the shared RHS/dense arithmetic used by the
[component driver](../../../../docs/ISSUE5_COMPONENT_DRIVER.md). It does not
time automatic construction or establish an automatic default.

| Artifact | Processes including warmups | Certified measured columns | Exact M6g records |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 2,520 |
| Mac development | 2,520 | 24,000 | 2,520 |
| Mac expanded, one RHS | 360 | 300 | 360 |
| Linux smoke | 2,520 | 24,000 | 2,520 |

All 7,920 processes pass accounting, repeatability, certification and RSS gates.
All 72,300 measured columns certify; no process errors, rejected columns,
timeouts, RSS failures or measured retries occurred. All 6,336 within-source
layout comparisons are exact. Every process matches its corresponding M6g input,
dimensions, route, repeat, numerical/work/fingerprint record, retained payload,
grouping admission and level/layout inventory.

Complete setup/solve/certificate/output timing, prefixes, payload scopes and RSS
remain in the summaries. Within-binary layout ratios do not compare revisions.
These runs use the installed Command Line Tools and explicit SDK path;
no cross-revision speedup is claimed.
Expanded remains one RHS at V=12,288 and at most 73,726 tuples; no out-of-cache,
parallel, calibration or campaign holdout result is claimed.

The [receipts](receipts.json) identify source/tree/policy, exact executables,
current/prior manifests and deterministic archives. Raw/binary artifacts are
preserved under `$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6h/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-component-654ddd1`.
Copies independently revalidate and every archived member is rehashed.
1 derived timing geomeans differ under local recomputation within the existing archival-only relative tolerance of 1e-14; receipts record every difference. Raw numerical/work/payload comparisons remain exact.
This is local preservation, not an independent off-machine backup.

Required Rust1.85/90 Python checks and seven release groups pass, including
seven complete-driver scientific tests, all new reservation/lifetime/budget
gates, eleven peak controls and 120 actual protocol comparisons. The six
Linux/macOS/Windows debug/release peak records exactly match the local reference.
See [component qualification](../component-driver-qualification/README.md).

Source workflows `35426610749`/`35426610758` and PR workflows
`35426643488`/`35426643483` passed. Linux evidence comes from source run
`35426610749`. Self-review covers ownership, shared arithmetic, live admission,
failure reporting and evidence scope; it is not independent external review.
Final evidence-head checks and PR61 merge remain.

M6 remains open for complete automatic-versus-baseline economics. M7–M10 follow.
Scalar remains default and calibration/campaign holdout remain untouched.
