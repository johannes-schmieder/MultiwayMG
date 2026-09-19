# M6j scalar compatibility after grouped automatic integration

Measured source `49fcf604d386b86b977dfacf55562c6d2abff334` was committed
before collection. The unchanged [automatic v1 policy](../../../policies/prepared-automatic-v1.json)
invokes the original scalar API; this evidence does not measure the new explicit
grouped entry point and makes no grouped speedup or default-selection claim.

| Artifact | Processes including warmups | Certified measured columns | Exact M6i signatures |
| --- | ---: | ---: | ---: |
| Mac smoke | 8,400 | 80,000 | 8,400 |
| Mac development | 8,400 | 79,925 | 8,400 |
| Linux smoke | 8,400 | 80,000 | 8,400 |

All **25,200** scheduled process records match M6i source `bc0267c` exactly
in input hashes/dimensions, schedule, status, numerical fingerprints/certificates,
work, payload, routing/rejection details and the 5,936-byte inline record ABI.
Only phase/total/overhead times are omitted from probe-signature comparison.
Resources and provenance are separately recorded, not asserted equal across
source builds. [Comparison receipts](comparison.json) include journal hashes.
The summaries preserve every measured time, CPU precision limitation and RSS.

As before, the development identity control rejects 75 columns in 35 measured
processes and seven warmups. Its complete certification gate remains false;
no incomplete identity aggregate is presented as a speedup. All automatic,
component-MAP, global-MAP and global-diagonal measured columns certify. The
previous negative automatic economics remain part of the record. No timeout,
launch/protocol error, RSS-budget failure or measured retry occurred.

[Preservation receipts](receipts.json) identify exact source/tree/binary/policy,
raw journal, summaries and deterministic archives. Raw copies independently
revalidate, and every archive member is checked against its preserved file.
Full binaries, process stdout/stderr, manifests, journals and logs remain outside
the repository at `$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6j` (current
machine `/Users/johannes/Git/MultiwayMG-assessments/2026-09-19-m6j`). Linux comes
from exact-source Rust CI35432625825. Platform timings remain separate.

[Cross-platform allocator receipts](../automatic-grouped-qualification/cross-platform.json)
compare all 39 controls and four zero-allocation group denials to local records
in debug/release on Linux, macOS and Windows, from exact-source allocation
CI35432625827. Each LSMR-enabled configuration certifies 6,240 columns; all
11 original scalar allocation records remain exactly M6h. This is requested
heap qualification, not a measurement of parallel allocation or grouped speed.

Required Rust1.85/105 Python and release/scientific checks pass. Source CI
35432625825/35432625827 and PR CI35432628126/35432628025 pass. Final evidence-head
checks and guarded merge follow. Next, separately instrument complete automatic
costs before a committed matched grouped comparison and larger development
controls. M6 terminal/admission work and M7–M10 remain open; calibration and
holdout are untouched.
