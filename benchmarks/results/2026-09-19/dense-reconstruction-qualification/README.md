# M6m dense reconstruction source qualification

All Rust1.85 required checks pass, including strict Clippy, all/minimal workspace
tests, warning-free rustdoc and both automatic examples. All135 Python tests pass.
Release all/minimal gates cover dense references, automatic/baseline/hierarchy/
PCG/LSMR/cycle quality and allocation contracts. Both actual debug/release probes
pass2,250 reference/profile processes:1,125 exact pairs,225 legacy comparisons,
900 layout comparisons,26 malformed inputs and10 CLI failures per build.

`checks.json` retains commands, timings, statuses, log/source hashes and protocol
summaries. The test-only old application body was compared to verified M6l base
`4fdce09` and is unchanged apart from replacing the receiver name and omitting
optional profiling. It uses the same caller-owned scratch as the candidate.

These are correctness/allocation gates, not a speedup claim. The frozen
[application experiment](../../../policies/dense-reconstruction-v1.json) and
[complete qualification plan](../../../../docs/ISSUE5_DENSE_RECONSTRUCTION.md)
require preserved platform samples, assembly inspection and charged automatic
regression before retaining the candidate. No rank, storage, numerical policy,
default, calibration or holdout changes.

The plain-slice revision repeats all required/135 Python and release/protocol
gates successfully; see `checks-slices.json`. The first iterator version's
checks are retained above, separately from its rejected performance evidence in
[the result record](../dense-reconstruction-v1/README.md). New measurements follow
a fresh source freeze; no prior negative records are replaced.

## Retained source and cross-platform closure

Measured source55388b14028386effa35e7faaa3a6aa7a9e02b5e passes all36 source/PR
jobs in35446761648/35446761654 and35446763608/35446763630. Full job receipts are
`slices-source-ci.json`; the rejected revision's green36-job receipts remain in
`iterator-source-ci.json` and do not establish a performance win.

`slices-cross-platform-allocations.json` and the six raw platform/build logs
match all39 M6l controls and four zero-allocation denials, including6,240 certified
columns per configuration. Provider digests and artifact provenance are in the
[measurement evidence](../dense-reconstruction-v1/README.md). Final evidence-head
CI and guarded self-review remain a separate merge gate; self-review is not
independent external approval.
