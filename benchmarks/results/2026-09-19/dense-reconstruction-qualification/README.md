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
