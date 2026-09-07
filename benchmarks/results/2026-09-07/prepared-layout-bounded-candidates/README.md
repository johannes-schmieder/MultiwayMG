# Existing-solver regression after bounded candidates (M6a)

Source `2d67bc40cbc489a70494e1ef9edac4eba39b7c93` was committed before
collection under unchanged [layout policy v2](../../../policies/prepared-layout-v2.json).
This checks the existing complete prepared solver after the new
[bounded-candidate primitive](../../../../docs/ISSUE5_BOUNDED_CANDIDATES.md)
and shared fallible parent validation. The measured solver still uses supplied
maps; it does not invoke or measure the new candidate builder. Candidate quality,
allocation and ordering gates are separate Rust tests tied to the same source.

| Artifact | Processes including warmups | Certified measured columns | Exact prior M5h records |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 2,520 |
| Mac development | 2,520 | 24,000 | 2,520 |
| Mac expanded, one RHS | 360 | 300 | 360 |
| Linux smoke | 2,520 | 24,000 | 2,520 |

All 7,920 complete processes pass accounting, certification, repeatability and
RSS gates. All 72,300 measured columns certify, with no process errors, timeouts,
rejected columns or measured retries. The 6,336 within-source scalar/layout
comparisons are exact. Every process also matches its M5h input hash, dimensions,
route, repeat, numerical/work/fingerprint, every retained payload category,
grouping admission and level/layout inventory. No solver memory, arithmetic or
work change is hidden in this construction increment.

Complete setup/solve/certificate/output times, prefixes, scoped memory and RSS
remain in the summaries. Within-binary layout ratios do not compare M6a with
M5h, and separate old/new collections imply no paired timing gain. These results
are not candidate-setup timing or automatic-hierarchy qualification. Scalar
remains default.

The [receipts](receipts.json) identify source/tree/policy, exact executable,
current/prior manifests and deterministic archives. Full raw/binary artifacts
live under `$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-candidates-2d67bc4`.
Copied artifacts independently revalidate, saved summaries reproduce exactly,
and every archived member is rehashed. This is local preservation, not an
independent off-machine backup.

Required Rust1.85 checks and 90 Python validators pass. Release all/minimal
complete solvers/candidates/allocators, compact/wide and reservation-failure
checks, fallible-parent checks and 120 actual protocol comparisons pass. Source
workflows `34084223619`/`34084223515` and PR workflows
`34084226689`/`34084226778` passed; Linux evidence comes from source run
`34084223619`. Self-review covered exact source/ordinal tie ordering, proposal
bounds, whole-stage admission, failed-work reports, compact index limits,
parent-label validation, prefix ownership boundaries and complete release.
This is self-review, not independent external review. Final evidence-head CI
and PR54 merge remain at this checkpoint.

The candidate is an unscreened structural proposal. Incremental structural
admission, component-local depths, recursive quality and the complete automatic
driver remain open. Expanded solver regression is one RHS at V=12,288 and at
most 73,726 tuples; it is not a large/out-of-cache or thread-scaling gate.
Campaign calibration/holdout remain untouched and competitive qualification
remains false.
