# Existing-solver regression after actual recursive screening (M6e)

Source `ba98d54947b3c05b35b7a1dd38a190fcf06728a7` was committed before
collection under unchanged [layout policy v2](../../../policies/prepared-layout-v2.json).
The existing supplied-map solver does not invoke the new
[recursive quality screen](../../../../docs/ISSUE5_RECURSIVE_SCREEN.md).
Separate Rust tests qualify actual tails, summary/reference equality, owners,
work, failure and memory boundaries. This regression checks existing complete
solver behavior; it does not measure automatic screening costs.

| Artifact | Processes including warmups | Certified measured columns | Exact prior M6d records |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 2,520 |
| Mac development | 2,520 | 24,000 | 2,520 |
| Mac expanded, one RHS | 360 | 300 | 360 |
| Linux smoke | 2,520 | 24,000 | 2,520 |

All 7,920 processes pass accounting, certification, repeatability and RSS gates.
All 72,300 measured columns certify, with no errors, timeouts, rejected columns,
RSS failures or measured retries. The 6,336 within-source layout comparisons
are exact. Every process matches its M6d input hash, dimensions, route, repeat,
numerical/work/fingerprint record, retained payload categories, grouping admission
and level/layout inventory. These regressions do not establish a cross-revision
timing gain or screen performance.

Complete setup/solve/certificate/output times, prefixes, scoped memory and RSS
remain in the summaries. Within-binary layout ratios do not compare M6e with
M6d. Scalar remains default. Expanded is one RHS at V=12,288 and at most 73,726
tuples; no out-of-cache, thread-scaling, calibration or campaign-holdout
qualification is claimed.

The [receipts](receipts.json) identify source/tree/policy, exact executables,
current/prior manifests and deterministic archives. Full raw/binary artifacts
are preserved under `$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-screen-ba98d54`.
Copies independently revalidate and every archive member is rehashed.
8 derived timing geomeans differ under local recomputation within the existing archival-only relative tolerance of 1e-14. Receipts record every difference; raw numerical/certificate/work/payload comparisons remain exact.
This is local preservation, not an independent off-machine backup.

Required Rust1.85 checks and 90 Python validators pass. Release all/minimal
screen/full-solver/allocator/component checks, private reservation and numerical
failures, budgets, full 16-start/64-step history bounds and 120 actual protocol
comparisons pass. Actual tails and summary probes match independent ordinary
suffixes and full-history diagnostics bit for bit across five layouts, changed
weights, historical recursive fixtures, explicit nested additional nullity and
terminal-only cases. Two setup allocations retain three coefficient vectors and
one report array; repeated/rejected/failed screens allocate zero. Initial lifetime
and Clippy findings are preserved with final passing logs; no measured run was
repeated because of them.

Source workflows `34096124588`/`34096124774` and PR workflows
`34096127941`/`34096127889` passed; Linux evidence comes from source run
`34096124588`. Self-review covered recursive identity, unchanged root execution,
shared arithmetic, scratch liveness, bounded errors, completed reports and cost
scope. This is self-review, not independent external review. Final evidence-head
CI and PR58 merge remain at this checkpoint.

M6 remains open for candidate-policy comparison and complete component-local
automatic construction with bounded terminals and charged fallback. M7–M10
follow. Quality probes are empirical guards, not full-spectrum proofs or final
original-operator solve certificates. No default-policy or competitive claim is
introduced here.
