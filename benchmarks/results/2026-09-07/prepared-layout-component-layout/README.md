# Existing-solver regression after flat component layout (M6c)

Source `1b4baaed8f67956f5f210171345f14b586b8e17c` was committed before
collection under unchanged [layout policy v2](../../../policies/prepared-layout-v2.json).
This checks the existing complete prepared solver after the new
[component permutation and temporary inverse](../../../../docs/ISSUE5_COMPONENT_LAYOUT.md).
The measured solver uses supplied maps and does not invoke the component layout.
Separate Rust tests exercise the new partition, recoding, local operators,
malformed-output behavior, identity and memory lifetime directly.

| Artifact | Processes including warmups | Certified measured columns | Exact prior M6b records |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 2,520 |
| Mac development | 2,520 | 24,000 | 2,520 |
| Mac expanded, one RHS | 360 | 300 | 360 |
| Linux smoke | 2,520 | 24,000 | 2,520 |

All 7,920 processes pass accounting, certification, repeatability and RSS gates.
All 72,300 measured columns certify, with no errors, timeouts, rejected columns,
RSS failures or measured retries. The 6,336 within-source layout comparisons
are exact. Every process matches its M6b input hash, dimensions, route, repeat,
numerical/work/fingerprint record, every retained payload category, grouping
admission and level/layout inventory. These regressions do not establish a
cross-revision timing gain or component-layout setup performance.

Complete setup/solve/certificate/output times, prefixes, scoped memory and RSS
remain in the summaries. Within-binary layout ratios do not compare M6c with
M6b. Scalar remains default. The expanded profile is one RHS at V=12,288 and
at most 73,726 tuples; no large/out-of-cache, thread-scaling, calibration or
campaign-holdout qualification is claimed.

The [receipts](receipts.json) identify source/tree/policy, exact executables,
current/prior manifests and deterministic archives. Full raw/binary artifacts
are preserved under `$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-components-1b4baae`.
Copies independently revalidate, saved summaries reproduce exactly, and every
archived member is rehashed. This is local preservation, not an independent
off-machine backup.

Required Rust 1.85 checks and 90 Python validators pass. Release component tests
in all/minimal configurations, private allocation-error/unwind/width/budget
checks, complete solvers/allocators and 120 actual protocol comparisons pass.
Component fixtures include ragged/interleaved labels, connected and separated
singletons, duplicate observations and disconnected Latin controls. Local
weighted Gramians exactly match the original restriction. Initial fixture and
lifetime-lint corrections are preserved beside passing final logs; no measured
run was repeated because of them.
Source workflows `34088832395`/`34088832402` and PR workflows
`34088867632`/`34088867657` passed; Linux evidence is from source run
`34088832395`. Self-review covered source-ID order/width, linear placement,
temporary inverse lifetime, checked memory admission, ownership, output failure
boundaries and exact release. This is self-review, not independent external
review. Final evidence-head CI and PR56 merge remain at this checkpoint.

M6 remains open for component-local structural/numerical construction, bounded
terminals, bottom-up actual-cycle screening and the complete automatic driver;
M7–M10 follow. The temporary inverse saving is a proved array-lifetime choice,
not a demonstrated competitive solve or 25% RSS qualification.

Qualification correction (2026-09-07): the cyclic Latin square has only structural
nullity. See the [explicit rank audit and new nested controls](../../../../docs/ISSUE5_PROVISIONAL_REPLAY.md).
This corrects the former additional-nullity description; raw frozen evidence is
unchanged. The new nested regression is part of M6d, not the older measured source.
