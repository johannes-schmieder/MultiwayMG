# Existing-solver regression after provisional replay (M6d)

Source `3e9028f41cea793fc6c04518b0880bc63bd45cb3` was committed before
collection under unchanged [layout policy v2](../../../policies/prepared-layout-v2.json).
The existing supplied-map solver does not invoke the new
[provisional constructor](../../../../docs/ISSUE5_PROVISIONAL_REPLAY.md).
Separate Rust tests qualify its numerical, lifetime, staged-memory and failure
boundaries. This regression checks existing complete solver behavior.

| Artifact | Processes including warmups | Certified measured columns | Exact prior M6c records |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 2,520 |
| Mac development | 2,520 | 24,000 | 2,520 |
| Mac expanded, one RHS | 360 | 300 | 360 |
| Linux smoke | 2,520 | 24,000 | 2,520 |

All 7,920 processes pass accounting, certification, repeatability and RSS gates.
All 72,300 measured columns certify, with no errors, timeouts, rejected columns,
RSS failures or measured retries. The 6,336 within-source layout comparisons
are exact. Every process matches its M6c input hash, dimensions, route, repeat,
numerical/work/fingerprint record, retained payload categories, grouping admission
and level/layout inventory. These regressions do not establish a cross-revision
timing gain or provisional-construction performance.

Complete setup/solve/certificate/output times, prefixes, scoped memory and RSS
remain in the summaries. Within-binary layout ratios do not compare M6d with
M6c. Scalar remains default. Expanded is one RHS at V=12,288 and at most 73,726
tuples; no out-of-cache, thread-scaling, calibration or campaign-holdout
qualification is claimed.

The [receipts](receipts.json) identify source/tree/policy, exact executables,
current/prior manifests and deterministic archives. Full raw/binary artifacts
are preserved under `$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-provisional-3e9028f`.
Copies independently revalidate and every archived member is rehashed. Seven
derived timing geomeans differ by one ULP under local recomputation, within the
existing archival-only relative tolerance of 1e-14. The receipts list every
difference; raw numerical/certificate/work/payload records remain exact. This is local preservation, not an independent
off-machine backup.

Required Rust1.85 checks and 90 Python validators pass. Release all/minimal
provisional/full-solver/allocator/component checks, private reservation and
numeric failures, budgets and 120 actual protocol comparisons pass. Additional
nullity is tested explicitly with nested-factor rank assertions and independent
nonconstant null vectors. The full cyclic Latin-square control has only the
structural factor shifts; M6d corrects the earlier interpretation without
altering frozen raw numerical evidence. Initial test scaffolding failures and
final passing logs are preserved; no measured run was repeated because of them.

Source workflows `34092590894`/`34092590882` and PR workflows
`34092595142`/`34092595090` passed; Linux evidence comes from source run
`34092590894`. Self-review covered exact owners, shared finishing, raw-weight
semantics, staged live payload, release on failure, rank controls and evidence
scope. This is self-review, not independent external review. Final evidence-head
CI and PR57 merge remain at this checkpoint.

M6 remains open for component-local automatic construction, bounded terminals
and actual recursive screening; M7–M10 follow. The eliminated prefix replay and
predecessor lifetime are implementation properties, not a demonstrated
competitive solver or 25% RSS qualification.
