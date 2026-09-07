# Certificate-vector complete-layout regression (M5h)

Source `26bfc035984502ed227cf228432ec286342476ed` was committed before
collection under the unchanged [layout policy v2](../../../policies/prepared-layout-v2.json).
This qualifies [certificate vector reuse](../../../../docs/ISSUE5_CERTIFICATE_LIVENESS.md)
against frozen M5g input/numerical/work records. Separate old/new collections
are not paired timing evidence.

| Artifact | Processes including warmups | Certified measured columns | Exact outer-workspace savings per process |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 384 bytes |
| Mac development | 2,520 | 24,000 | 3,072 bytes |
| Mac expanded, one RHS | 360 | 300 | 98,304 bytes |
| Linux smoke | 2,520 | 24,000 | 384 bytes |

All 7,920 processes pass full accounting, certification, repeatability and RSS
gates. All 72,300 measured columns certify, with no process errors, timeouts,
rejected columns or measured retries. The 6,336 within-source scalar/layout
comparisons preserve exact results/work/fingerprints. Every process also matches
the corresponding M5g input, route, repetition, numerical/work/fingerprint and
layout inventory. Only outer_workspace/total payload falls by 8V bytes. Hierarchy
workspace, grouping admission and every other payload field remain exact.

The independent allocator gates confirm two certificate arrays, 24 complete
PCG arrays and 30 complete native/gated-LSMR arrays for the two-transition fixture,
including image layouts. Cycle stays14. Both adjoints are still charged to each
certificate; all candidate and final certificates remain fresh. The discarded
reference array is the only numeric storage removed. No target cache, tolerance,
iteration or local-reorthogonalization change is hidden in these results.

Full timing/setup/solve/certificate/output costs, RHS prefixes and scoped memory
remain in the summaries. Within-binary layout ratios do not compare M5h with
M5g. The exact retained-byte/allocation reduction proves no RSS or elapsed-time
gain by itself. Scalar remains default.

The [receipts](receipts.json) identify source/tree/policy, exact executable,
current/prior manifests and deterministic archives. Full artifacts live under
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-certificate-26bfc03`.
Copied artifacts independently revalidate; every archived member is rehashed.
This local copy is not an independent off-machine backup.

Required Rust1.85 checks and 90 Python validators passed before freeze. Release
all/minimal complete solvers/allocators, private failure precedence and 120
actual release protocol comparisons pass. Source workflows
`34081515304`/`34081515332` and PR workflows `34081517554`/`34081517611`
passed; Linux evidence comes from source run `34081515304`. Self-review covered
live-vector boundaries, exact finite/norm arithmetic, error precedence, counted
failed actions, static owner/input guards and successful recovery. This is
self-review, not independent external review. Final evidence-head CI and merge
remain before the serial optimization milestone closes.

Supplied-map development inputs do not qualify automatic construction or large
multi-RHS/thread scaling. Expanded covers only one RHS at V=12,288 and at most
73,726 tuples. Campaign calibration and holdout remain untouched. No competitive
qualification is claimed.
