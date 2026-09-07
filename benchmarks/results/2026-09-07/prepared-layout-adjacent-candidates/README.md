# Existing-solver regression after explicit adjacent candidates (M6f)

Source `31b202e10c65cee77a079fb574f1408e778901db` (tree
`7ed7b3f00443b4ff1b7e7b6945cfae125b2689ef`) was committed before collection
under unchanged [layout policy v2](../../../policies/prepared-layout-v2.json).
The supplied-map solver does not invoke the new
[adjacent candidate policy](../../../../docs/ISSUE5_ADJACENT_CANDIDATES.md).
Separate tests qualify its parents, bounds, actual cycles, failure recovery and
certified diagnostic solves, including preserved quality-screen rejections.

| Artifact | Processes including warmups | Certified measured columns | Exact prior M6e records |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 2,520 |
| Mac development | 2,520 | 24,000 | 2,520 |
| Mac expanded, one RHS | 360 | 300 | 360 |
| Linux smoke | 2,520 | 24,000 | 2,520 |

All 7,920 processes pass accounting, repeatability, certification and RSS gates.
All 72,300 measured columns certify; no process errors, rejected columns,
timeouts, RSS failures or measured retries occurred. All 6,336 within-source
layout comparisons are exact. Every process matches the corresponding M6e
input, dimensions, route, repeat, numerical/work/fingerprint record, retained
payload, grouping admission and level/layout inventory.

Complete setup/solve/certificate/output timing, prefixes, payload scopes and RSS
remain in the summaries. The within-binary layout ratios do not compare M6f
with M6e. This is existing-solver regression, not adjacent-setup timing or an
automatic policy qualification. Expanded remains one RHS at V=12,288 and at most
73,726 tuples; no out-of-cache, parallel, calibration or holdout result is claimed.

The [receipts](receipts.json) identify source/tree/policy, exact executables,
current/prior manifests and deterministic archives. Complete raw/binary artifacts
are preserved under `$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-adjacent-31b202e`.
Copies independently revalidate and every archive member is rehashed.
11 derived timing geomeans differ under local recomputation within the existing archival-only relative tolerance of 1e-14. Receipts record every difference; raw numerical/certificate/work/payload comparisons remain exact.
This is local preservation, not an independent off-machine backup.

Required Rust1.85/90 Python checks and release all/minimal numerical/full-solver/
allocator/private/120 protocol gates pass. Independent ordered-map parents and
62 ordinary-suffix/full-history tail references agree. All 66 PCG and 66 gated-
LSMR forced diagnostic solves certify. Seven of 22 preselected quality screens
reject and remain in the separate [diagnostic records](../adjacent-candidate-qualification/records.json).
These negatives do not occur in this supplied-map benchmark; its all-pass result
does not override them. Both candidate policies have ten setup allocations,
three retained parent arrays, allocation-free budget rejection and exact release
on numerical failure. Every flat reservation error/unwind recovers.

Source workflows `34100575280`/`34100575335` and PR workflows
`34100640663`/`34100640606` passed. Linux evidence comes from source run
`34100575280`. Self-review covers Q<=E admission, legacy identity, finite order,
ownership, live arrays and negative-result scope; it is not external review.
Final evidence-head checks and PR59 merge remain at this checkpoint.

M6 remains open for complete component-local construction, admitted terminals
and charged fallback. M7–M10 follow. Scalar stays default, and the adjacent
policy receives no automatic/default/competitive promotion from this increment.
