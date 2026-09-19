# Serial Schwarz workspace qualification (M6o)

This qualifies a dependency API and its integration, not an admitted sparse
terminal or complete-solver speedup. Both dependencies pin the reviewed fork
merge `69110db2733a29bce7b6151466ece95c6249713f` (source `b69d07d`, tree
`b8f17126eeb750e6775a7bede3b7975dd0bca153`). All 13 fork source/PR jobs pass,
and the guarded merge verified the actual clean fork tree. Its four post-merge
jobs also pass.

Eight local Rust1.85 required/release groups pass. Three provider-digest-verified
ZIPs preserve Linux/macOS/Windows debug/release contracts. All 16 concrete-factor
records agree, with 288/360/632 outer bytes by case, one reservation, zero first/
repeated/static-rejection allocations and exact drop. Four generic feature/build
records per platform agree: 288,016 bytes for n=12,000, one reservation and zero
first/repeated/static/local-error/recovery allocations for the allocation-free
control solver. Exact ZIP/file digests, source metadata and every raw log are
included. Factor-owned bytes and all-factor permutation allocations are excluded.

The initial process-wide allocator harness observed delayed global/setup-worker
activity. Both failing logs are preserved. Explicit OS-thread joins before the
first action remove that interference; no action warmup was added. The original
small connected fixture label "tensor" was corrected to "latin" without changing
its tuples; original logs are unchanged. Required final-source runs use the
corrected name.

`range-audit/` preserves the external mathematical audit and both revisions of
its exploratory fixture list. A connected 12-coordinate/rank-seven support has
numerical-range leakage 0.252389/unit and 0.176210/dyadic even after two-sided
structural projection makes the action symmetric. Quotient energy is positive.
It does not satisfy the accepted full-range terminal gate; it is not evidence
against previously certified LSMR solutions. All other audited cases are retained.

M6n final/merge/post-merge receipts close PR67 at `172f0d8`; its 36 final and18
post-merge jobs pass. M6o downstream source/PR and pin-integration qualification
will be recorded separately. Raw external workspace:
`$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6o`.

`mg-local-checks.json` records all eight downstream required and eight release
groups, exact boundary-file/log/binary hashes and original external log locations.
Both actual protocol builds pass 1,125 reference/profile, 225 legacy and900 layout
comparisons, with malformed/CLI rejection. The 160-RHS integration test certifies
on the real paired dependency pins in debug and release. Exact-source/PR CI and
guarded merge remain required.
