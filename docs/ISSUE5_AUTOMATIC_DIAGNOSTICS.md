# Complete automatic cost diagnostics (M6k)

M6j [PR63](https://github.com/johannes-schmieder/MultiwayMG/pull/63) merged as
`e07239dcf72bef50a2ffd0dc91e07beb61d2d2cc`, with verified reviewed tree
`cfd4bd8d22de2f25062007859fa7215539e4e41b`. All final source/PR checks and
post-merge CI35433990078/35433990086 pass. Its grouped ownership qualification
and 25,200 exact scalar compatibility records remain unchanged. M6i/M6j scalar
development economics remain negative; this increment diagnoses costs before
new layout or numerical-policy performance trials.

## Collector and ownership

`automatic_profiling`, enabled only with `lsmr` and `profiling`, provides a
current-thread `collect` callback and disjoint region guards. It retains a fixed
17-entry inline inventory with no per-event history, heap array or worker pool.
The qualified 64-bit TLS size is reported explicitly (592 bytes on the local
build), separately from array admission and the executable's inline record size.
The existing kernel profiler's fixed TLS capacity is reported separately too: its
hooks are enabled by this feature even while its collector is inactive. These
are fixed type capacities, not measured live allocations after early CLI/decode
failure; process RSS includes runtime overhead.
Inactive spans skip clock reads; builds without `profiling` compile out hooks.

Overlapping spans, unfinished guards, counter overflow or inconsistent total
cost invalidate the report. Nested collection refuses to invoke its callback.
Checked generations prevent stale guards from recording into a later collection;
unwinding disables collection and permits later reuse. Guards cannot cross
threads. No timing affects construction, layout, screening, convergence or
fallback decisions. The existing 16-category kernel profiler is unchanged and
has independent scope; its inclusive times cannot be added to these regions.

The 17 regions cover partition/recoding, local root, local frame, direct dense
factor, direct columns, structural preparation, hierarchy/local/global grouping,
full numerical replay, coarse factor, screening, local workspace, local columns,
original certificate, global workspace and global columns. Structural preparation
includes candidates, admission and provisional replay; full replay counts the
subsequent complete numerical owner. Screening includes its scratch setup and
destruction. Local columns aggregate hierarchy and fallback-baseline execution.
A completed region includes any failed work; it is not an accepted-owner count.
Bookkeeping/destruction outside regions remains in unattributed callback time.
Disjoint region sum plus unattributed time must equal the complete callback.

The hooks preserve existing arrays, arithmetic and caller/report semantics.
Integration tests compare inactive and active results exactly across layouts,
weights, component scopes, rejected screens, global recovery and grouping denial.
All isolated allocator controls execute with active hooks under `profiling`;
their allocation records must still match the uninstrumented M6j records.
These serial TLS controls do not establish parallel allocator behavior.

## Separate executable and policy

`prepared_automatic_diagnostic` accepts `ROUTE LAYOUT`, supporting all five v1
routes and scalar/fine-row/all-row/fine-image/all-image. It keeps the same strict
`MG3AUT1` canonical input boundary, bounds, current weights, numerical settings
and original certificates. Output schema2 adds explicit layout/group inventory,
last grouping failure, disjoint regions and profiler scope. Its inline record
has the same size in reference and instrumented builds. Failed decode/CLI calls
have no invented driver profile; failed driver calls retain attempted regions
and any completed columns. Global LSMR controls allocate row grouping without a
dead Gramian image. The frozen `prepared_automatic_benchmark` and v1 policy are
unchanged.

The [diagnostic policy](../benchmarks/policies/prepared-automatic-diagnostic-v1.json)
uses seed10001, ten families, two shapes, two weights and K=1,8,32. Nine distinct
arms comprise five automatic layouts and scalar/all-row component-MAP/global-MAP.
Fine/all and image/row baseline aliases have the same fine row storage, so the
collection need not repeat their equivalent paths. Actual protocol qualification
still checks every route/layout, identity/diagonal controls, recursive and weak
fallback cases, numerical errors and malformed inputs.

Each profile has 1,080 independent input/route/layout pairs and 2,160 isolated
processes: one reference and one instrumented executable per pair. There is one
fresh diagnostic sample and no warmup or speedup aggregation. Rotate arms and
alternate build order as declared; report perturbed per-case phase attribution,
not competitive or grouped performance. Smoke/development dimensions remain the
bounded v1 recipes; this does not consume calibration/holdout or establish
out-of-cache/SCC performance. Larger development cases and a matched grouped
full-cost policy are separate follow-ups.

The collector requires committed clean Rust1.85 source with no compiler overrides.
It preserves both binaries, build logs, complete source/build/hardware/affinity
metadata, raw stdout/resources and an append-only attempt journal. Timeouts and
launch/protocol failures retain charged process time. Validation reconstructs
all inputs/schedules, checks each raw hash and parser result, and requires exact
reference/profile numerical/work/payload/layout/routing signatures. The common
v1 numerical/outer-cost checks are reused through a checked schema projection;
the original observer flag and every new field are independently validated first.
No instrumented result enters the authoritative timing validators.

Complete reference/profile pairs may report regions; incomplete pairs cannot
fabricate phase shares. Every accepted column must independently satisfy the
original certificate in the executable. Both builds' reported certificate/status
consistency is checked even when its partner failed. Full process RSS includes
profiler/runtime overhead; OS CPU precision limits remain unchanged. Setup,
failed attempts and certification stay visible. A profile share is diagnostic
attribution, not a measured speedup from eliminating that region.

Local Rust1.85 required/release qualification passes, including 115 Python tests,
four collector tests and two integration tests. Final debug/release probes each
pass 1,125 reference/profile pairs, 225 frozen scalar comparisons, 900 layout
comparisons, 26 malformed inputs and ten CLI checks. Active allocation records
exactly match all 39 M6j controls and four zero-allocation denials. See the
[qualification receipts](../benchmarks/results/2026-09-19/automatic-diagnostic-qualification/README.md).
The [committed-source evidence](../benchmarks/results/2026-09-19/prepared-automatic-diagnostic-v1/README.md)
now validates 6,480 Mac smoke/development and Linux smoke processes, 3,240 exact
observer pairs and 88,560 certified columns with no failures. All 1,080 same-route
scalar records match M6j. Every layout preserves math/work; both TLS capacities
are explicit (592/6,960 bytes), and the shared inline record is 6,608 bytes.
Cross-platform allocator records match Linux/macOS/Windows debug/release.

One-RHS development scalar driver attribution is dominated by screening (35.05%)
and structural preparation (22.19%); coarse factor is 12.44%, local solve 27.77%.
At K32, local solve is 90.43%. These perturbed, sum-weighted diagnostic shares
identify work to examine, not removable overhead or speedups. Family differences
remain in full records. Proceed with a separate matched, uninstrumented grouped
layout experiment before numerical-policy trials. Source and PR CI pass; final
evidence-head checks/merge follow. M6 admission/terminal and M7–M10 remain open.
