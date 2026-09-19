# M6k local diagnostic qualification

Rust1.85 required and release checks pass; `checks.json` preserves commands,
statuses and log hashes. The final executable reports both fixed profiler TLS
capacities. The final debug/release `protocol.json` records 1,125 exact observer
pairs per build, 225 legacy scalar and 900 cross-layout comparisons, 26 malformed
inputs and ten CLI controls, with binary hashes. Complete streams and build logs
remain in `$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6k/`.

`records.json` preserves all 39 active-profiler allocation controls, exactly
matching M6j in debug/release: 6,240 certified columns/configuration and four
zero-allocation grouping denials. The 592-byte automatic TLS is fixed inline
capacity outside the requested heap; the existing kernel profiler TLS and
6,608-byte executable record are separately reported. This serial allocator
qualification does not establish parallel allocator behavior or speedups.

Required checks include all/minimal Rust features, strict Clippy, warning-free
docs and all 115 Python tests. Final targeted format/Clippy, profiler release,
ten adversarial diagnostic tests and final debug/release protocol qualification
cover the final six-field diagnostic output. Initial five-field development
streams remain preserved separately and do not enter final qualification.

All 39 records and four zero-allocation denials also match on Linux, macOS and
Windows in debug/release (`cross-platform.json`, source run35436270925).
Linux debug/release actual probes each pass all 1,125 pairs, 225 legacy and 900
layout comparisons plus malformed/CLI controls (`linux-protocol.json`). Source
Rust/allocation CI35436270926/35436270925 and PR CI35436272117/35436272105 pass.
Committed-source diagnostic evidence is preserved in
[`prepared-automatic-diagnostic-v1`](../prepared-automatic-diagnostic-v1/README.md).
