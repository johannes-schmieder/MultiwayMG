# Single-arena complete-layout regression (M5g)

Source `c40f9e53fcd59b3d0466be6800795a1d16df6432` was committed before
collection under [layout policy v2](../../../policies/prepared-layout-v2.json).
This qualifies the [single arena](../../../../docs/ISSUE5_TRAVERSAL_ARENA.md)
against frozen M5f numerical/work records. It is a regression collection, not an
old/new paired timing experiment. Version1 evidence and policy remain immutable.

| Artifact | Processes including warmups | Certified measured columns | Scalar/row descriptor savings | Image descriptor savings |
| --- | ---: | ---: | ---: | ---: |
| Mac smoke, depth1 | 2,520 | 24,000 | 120 bytes | 144 bytes |
| Mac development, depth3 | 2,520 | 24,000 | 312 bytes | 336 bytes |
| Mac expanded, depth8, one RHS | 360 | 300 | 792 bytes | 816 bytes |
| Linux smoke, depth1 | 2,520 | 24,000 | 120 bytes | 144 bytes |

All 7,920 processes pass accounting, certification, repeatability and RSS gates;
all 72,300 measured columns certify. There are no process errors, timeouts,
rejected columns or measured retries. The 6,336 within-source scalar/layout
comparisons preserve exact numerical/work/fingerprint results. Every process
also matches the corresponding M5f input hashes, dimensions, route, repeat,
numerical/work/fingerprint and level inventory. Only hierarchy-workspace and
total payload fall by `24*(1+4*depth+image_enabled)` bytes. Every other payload
category and grouping-admission field is unchanged; the declared image descriptor
is 0 rather than 24 bytes. All numeric element lengths remain unchanged.

Independent allocator gates give two-transition cycle/PCG/LSMR totals of
14/25/31 arrays in every layout. Image storage adds 8E numeric bytes inside the
same arena and requires no extra reservation. This proves descriptor and
allocation reductions, not a peak-RSS or wall-clock improvement. Complete costs,
RHS prefixes, setup, solve/certificate/output times and measured RSS remain in
the summaries. Within-binary layout ratios do not compare M5g with M5f.
Scalar remains default.

The [receipts](receipts.json) identify exact source/tree/policy, executables,
current/prior manifests and deterministic archives. Full artifacts are preserved
under `$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-arena-c40f9e5`.
Copied artifacts independently revalidate, all saved summaries reproduce exactly,
and every archive member is rehashed. Local preservation is not an independent
off-machine backup.

Required Rust1.85 checks, 90 Python tests, release all/minimal solver/allocation
gates and 120 actual v2 probe comparisons passed before freeze. Historical v1
Mac/Linux artifacts still validate and reproduce. Source workflows
`34079679036`/`34079679035` and PR workflows `34079732692`/`34079732651`
passed; Linux evidence comes from source run `34079679036`. Self-review checks
whole-arena admission, safe disjoint slicing, ordinary storage-path comparison,
constant-time image boundary, output transaction/failure recovery and historical
policy dispatch. This is self-review, not independent external review. Final
evidence-head CI and merge remain at this checkpoint.

Inputs use supplied maps. Expanded is V=12,288 and at most 73,726 tuples with
one RHS. Large/out-of-cache multi-RHS, automatic construction, changed weights
and CPU scaling remain unqualified. Calibration/holdout remain untouched and
competitive qualification remains false.
