# Canonical pair-order qualification

All eight required check groups pass: Rust1.85 format, strict Clippy, all/minimal
workspace tests, rustdoc,135 Python tests and both example unit contracts.
Eight release groups cover all/minimal scientific integrations and candidate
ordering/admission/failure tests, component-root/layout invariants and actual
debug/release protocols. Each protocol passes1,125 exact reference/profile pairs,
225 legacy comparisons,900 layout comparisons,26 malformed and10 CLI checks.
These protocol processes are correctness evidence, not timing comparisons.

`checks.json` retains commands, statuses, elapsed check times, raw-log hashes,
protocol binary hashes and the exact changed Rust source hash. External logs:
`$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6n` (current
`/Users/johannes/Git/MultiwayMG-assessments/2026-09-19-m6n`). The measured source freeze is
`f4a611d61287e2375c1812232f1df1ac451605a1`. `source-ci.json` records all 36 green
source/PR jobs. Six raw allocation logs and their three metadata records match
39 controls, four zero-allocation denials and 6,240 certified columns each;
`cross-platform-allocations.json` records exact hashes and M6m comparisons.

[Complete evidence](../canonical-pair-order-v1/README.md) preserves all 45,360
processes and 432,000 measured certificates, with exact prior scientific/work/
memory/routing signatures. Final evidence-head CI and guarded actual-main
verification remain required; measured-source receipts do not stand in for them.
