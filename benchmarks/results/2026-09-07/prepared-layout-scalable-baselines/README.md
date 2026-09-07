# Existing-solver regression after scalable baselines (M6g)

Source `5660b97693443e7d2b05610b17d2d1a1a56ca67f` (tree
`88f8c80a6d22246d706d35488f976add218567af`) was committed before collection
under unchanged [layout policy v2](../../../policies/prepared-layout-v2.json).
The supplied-map solver exercises the generalized static owner boundary through
its unchanged hierarchy owner. It does not time the new
[baseline actions](../../../../docs/ISSUE5_SCALABLE_BASELINES.md).

| Artifact | Processes including warmups | Certified measured columns | Exact prior M6f records |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 2,520 |
| Mac development | 2,520 | 24,000 | 2,520 |
| Mac expanded, one RHS | 360 | 300 | 360 |
| Linux smoke | 2,520 | 24,000 | 2,520 |

All 7,920 processes pass accounting, repeatability, certification and RSS gates.
All 72,300 measured columns certify; no process errors, rejected columns,
timeouts, RSS failures or measured retries occurred. All 6,336 within-source
layout comparisons are exact. Every process matches the corresponding M6f
input, dimensions, route, repeat, numerical/work/fingerprint record, retained
payload, grouping admission and level/layout inventory.

Complete setup/solve/certificate/output timing, prefixes, payload scopes and RSS
remain in the summaries. Within-binary layout ratios do not compare revisions.
This is existing-solver regression, not baseline timing or automatic policy
qualification. Expanded remains one RHS at V=12,288 and at most 73,726 tuples;
no out-of-cache, parallel, calibration or holdout result is claimed.

The [receipts](receipts.json) identify source/tree/policy, exact executables,
current/prior manifests and deterministic archives. Complete raw/binary artifacts
are preserved under `$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-baseline-5660b97`.
Copies independently revalidate and every archive member is rehashed.
8 derived timing geomeans differ under local recomputation within the existing archival-only relative tolerance of 1e-14. Receipts record every difference; raw numerical/certificate/work/payload comparisons remain exact.
This is local preservation, not an independent off-machine backup.

Required Rust1.85/90 Python checks and all twelve release groups pass, including
independent baseline algebra, large complete solves, exact allocation/failure
gates and 120 actual protocol checks. Separate historical baseline diagnostics
preserve 14 native certificate rejects and 18 vetoes across 48 MAP cases; all
gated results certify. See the [per-case records](../baseline-certificate-qualification/records.json).
Those negatives are separate from this all-pass supplied-map regression.

Source workflows `34107489091`/`34107489103` and PR workflows
`34107513260`/`34107513287` passed. Linux evidence comes from source run
`34107489091`. Self-review covers shared recurrence/certificate arithmetic,
static ownership, positive fixed actions, live arrays and negative-result scope;
it is not external review. Final evidence-head checks and PR60 merge remain.

M6 remains open for complete component-local construction, admitted terminals
and charged fallback. M7–M10 follow. Scalar stays default; no automatic or
competitive promotion follows from this baseline execution increment.
