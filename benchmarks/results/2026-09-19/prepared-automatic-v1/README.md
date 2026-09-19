# Complete automatic construction development evidence (M6i)

Frozen source `bc0267c48512607bab5a61ddf41c65e4ec88d36d`, tree
`c26b1a21bd1b5b0fafa1b4e17702cbec3379a03c`. The
[policy](../../../policies/prepared-automatic-v1.json) was committed before
any comparative timing. See the [contract](../../../../docs/ISSUE5_AUTOMATIC_BENCHMARK.md),
[receipts](receipts.json), [derived analysis](analysis.json) and full platform
summaries. This is bounded development evidence, not competitive qualification.

## Result

All 25,200 scheduled processes are retained. Mac and Linux smoke each certify
80,000 measured columns. Mac development certifies 79,925 of 80,000: automatic,
component-disabled MAP, global MAP and global diagonal certify every measured
column, while global identity has 75 rejected column results in 35 measured
processes plus seven failed warmups. Therefore the complete all-control
development certification gate is false; no identity aggregate is reported.
No timeout, launch/protocol error, RSS violation or measured retry occurred.

Paired full-process speedup relative to whole-problem MAP (higher is better):

| Artifact | Automatic | Components, hierarchy disabled | Global diagonal | Global identity |
| --- | ---: | ---: | ---: | ---: |
| smoke | 1.1011x | 1.1045x | 1.0095x | 0.9824x |
| development | 0.5409x | 1.0031x | 1.0679x | incomplete |
| linux-smoke | 1.3741x | 1.3737x | 1.1889x | 1.1389x |

These are equal-family/equal-width geomeans, with shapes and weight regimes
balanced inside each group. Platforms remain separate. Both smoke matrices
primarily exercise direct terminals; they do not establish a hierarchy win.

Automatic construction is 1.8488x slower than MAP on Mac development overall.
Its speedups are 0.3751x for one RHS and 0.5749x across repeated RHS. At K32
the balanced speedup is still 0.6913x. No entire development family improves
over MAP: family geomeans range from 0.3668x (uniform) to 0.9935x (ragged).
The hierarchy-disabled route is 1.0031x overall; diagonal is 1.0679x.
The current explicit automatic policy is not a suitable default.

## Numerical and work findings

Across the 280 K-specific development cells, one representative automatic
record per cell gives 266 large-component hierarchy attempts: 203 accepted
hierarchies and 63 quality rejections with local MAP fallback. There are 98
dense and 392 singleton component executions, and zero final global fallback
columns. Counts include each K configuration once, not all six process repeats.
Each configuration repeats its numerical, work, rejection and payload record
exactly. The nine underlying large-component input configurations rejected by
screening recur at all seven K values; they are not removed or retuned.

The coarse correction often saves iterative actions without saving time. For
example, the chain family geometric ratio of automatic/global-MAP weighted
incidence action counts is 0.3690, but its complete-process speedup is only
0.6224x. These action counts omit construction/screen work, which is separately
recorded and included in total time. They are not a substitute for time or
whole-cycle cost. All nested development hierarchies reject screening; their
outer solve action counts match fallback MAP.

The identity negatives are all nested/balanced/heterogeneous, across every K.
For K1, the native report says converged at iteration1000, while the independent
original certificate is 1.4247615402301577e-8 and rejects at the frozen 1e-8
threshold. This is why native convergence does not determine acceptance.
All failed work remains charged: 42.473667087 seconds across failed measured
and warmup identity processes. No tolerance or iteration limit was changed.

Both smoke artifacts have 77 final-global-fallback columns per representative
automatic record summed across cells; component-disabled execution has the
same fallback. Every final result certifies. Direct terminal completion is not
assumed to imply original acceptance.

## Memory, timing and preservation

Mac development maximum observed process RSS is 10,321,920 bytes for automatic
and 8,437,760 for global MAP, both below the declared 1GiB budget. These are
separate maxima, not a paired memory-overhead qualification. Exact input/caller
capacities and requested/admitted array bounds are in every record. Allocator
metadata, fixed stack and runtime are included in process RSS but excluded
from array bounds; no timing-binary allocator peak is invented.

Local complete collection/validation elapsed 67.54 seconds for smoke and
889.37 seconds for development. Paired per-process measurements exclude harness
generation/build/validation and include full decode/setup/failure/solve/certify/
output/destruction. Every automatic internal stage is charged inside its whole
driver interval; internal stage times remain unavailable. CPU user/system
figures retain OS rounding, and no CPU speedup is inferred from printed zeros.

Copies were independently revalidated from raw logs and committed source; every
member of each deterministic archive was checked against its copied file.
All three recomputed summaries match exactly, with no archival rounding
exception required. Archives contain exact executables and full outputs:

| Artifact | Archive bytes | Receipt |
| --- | ---: | --- |
| mac-smoke | 7,557,644 | `da0a10ae3c485fe0e037a82e80c2e38ceacd9a2633ec5b2f196f99e652c7847b` |
| mac-development | 8,544,962 | `f0373cba817bb681aad3036542eeca3f05d52917e6fba42d872da6676e199219` |
| linux-smoke | 7,466,787 | `41ff1dcd9cd06bca392b0091aa1a76f8bbdde3af1b5b0465e9514d374d90e5f0` |

Portable location: `$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6i/automatic-benchmark/`.
Current machine: `/Users/johannes/Git/MultiwayMG-assessments/2026-09-19-m6i/automatic-benchmark/`.
These are local preserved copies, not an external backup. Source Linux artifact
comes from Rust CI [35429720359](https://github.com/johannes-schmieder/MultiwayMG/actions/runs/35429720359).
Source allocation CI35429720334 and PR CI35429722277/35429722256 also pass.
All required Rust1.85 checks, 105 Python tests, twelve release/scientific groups,
120 existing layout protocol comparisons and 241 new protocol checks per debug/
release configuration pass. Final evidence-head CI and merge remain.

## Next increment

Keep v1 and every negative unchanged. M6j will expose the already qualified
grouped row/tuple-image layouts to automatic construction and its matched
baseline controls, with complete group/image admission and exact scalar
references. Add separate diagnostic attribution where needed; instrumented
times will not serve as comparative evidence. Charge setup and rejected
groups before any admission recommendation. Test larger development cases and
terminal/application costs before deciding a structural policy; do not infer
an automatic route from the smoke win or tune against campaign holdout.

M6 remains open, including remaining terminal/admission decisions. M7 bounded
parallelism follows this cost diagnosis; M8 panels, M9 changing-weight policy
and M10 competitive qualification remain open. Development seeds10002/10003,
calibration and campaign holdout are untouched.
