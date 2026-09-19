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
`/Users/johannes/Git/MultiwayMG-assessments/2026-09-19-m6n`). No new performance
measurements precede the clean source freeze. Full-cost automatic regression,
three-platform allocation comparison and final exact-head CI remain required.
