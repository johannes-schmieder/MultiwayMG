# Dense reconstruction evidence

Retained candidate source `55388b14028386effa35e7faaa3a6aa7a9e02b5e`, tree `50c9adc72b7452b2cd906dd5d914801670846dc6`. Contiguous column slices expose independent output FMAs without changing matrix storage, arithmetic order, rank, work, routing or allocation. The rejected general iterator and its partial development run remain below.

## Application-only results

Ratios are old/stream elapsed time within the same executable, with identical inputs and opaque call boundaries. Above one favors streaming. All five paired repetitions at every dimension and all warmups are retained in the full summaries. These intervals include both dense transforms but exclude factorization, preparation and complete-solver work.

| Dimension | Mac old/stream | Generic Linux x86 old/stream |
| --- | ---: | ---: |
| 3 | 0.710481 | 0.982925 |
| 6 | 0.831472 | 1.059136 |
| 9 | 1.018137 | 1.084582 |
| 16 | 1.150799 | 1.102331 |
| 31 | 1.292060 | 1.118045 |
| 32 | 1.389053 | 1.140628 |
| 33 | 1.386981 | 1.120972 |
| 63 | 1.544704 | 1.118665 |
| 64 | 1.590115 | 1.117705 |
| 65 | 1.635321 | 1.130584 |
| 127 | 1.666234 | 1.129926 |
| 128 | 1.697770 | 1.124014 |
| 129 | 1.669339 | 1.128537 |
| 255 | 1.719492 | 1.122523 |
| 256 | 1.725139 | 1.176889 |
| 257 | 1.736604 | 1.119908 |
| Equal-dimension aggregate | 1.377653 | 1.110297 |

Mac dimensions3/6 regress; generic x86 dimension3 also regresses. No per-size selector or native compiler flag was introduced to hide these negatives. Cross-platform output/modal/work fingerprints match for all192 records; both full micro summaries recompute exactly. Real eigensystem references and complete driver certification provide separate correctness evidence.

The exact measured ARM executable uses four two-lane vector FMAs per eight-output reconstruction chunk with a scalar tail; the forward transform retains its scalar dependent chain. Generic x86 retains libm `fma` calls, now with independent unrolled reconstruction calls. This does not establish how libm dispatches internally. Both disassemblies, binary/archive hashes and inspection receipts are preserved.

Whole-harness wall/CPU/RSS include preparation and verification within the test process and remain separate from application intervals. Build and collector wrapper costs are in the external logs. Printed CPU zeros are below resolution.

- Mac: wall 1.798809750s; resources `{"method": "darwin_time_l", "peak_rss_bytes": 3129344, "scope": "isolated_process_including_startup_teardown", "system_seconds": 0.0, "user_seconds": 1.59}`.
- Linux: wall 8.910775734s; resources `{"method": "gnu_time_v", "peak_rss_bytes": 4141056, "scope": "isolated_process_including_startup_teardown", "system_seconds": 0.0, "user_seconds": 8.9}`.

## Complete-solver and allocation regression

All three prescribed collections pass:45,360 processes and432,000 measured certified columns, with zero measured/warmup failures. All45,360 corresponding M6l schedule/input/math/work/payload/layout/routing records match exactly. Six Linux/macOS/Windows debug/release allocation logs match all39 controls and four zero-allocation denials. Requested/admitted capacities are unchanged; process RSS is recorded separately.

All17 full-cost comparisons, all/hard, single/repeated RHS, memory and family results are in [complete-layouts.md](complete-layouts.md). On Mac development, automatic all-image versus automatic scalar is1.234817x. Against grouped global MAP it is0.554050x overall (about1.81x slower),0.400390x atK1 and0.584871x for repeated RHS. Hard-family overall/K1/repeated ratios are0.586466/0.430759/0.617416. Every whole-family aggregate remains below grouped MAP. These are within-source layout/control comparisons, not paired measurements of the dense source change. Smoke uses direct dense terminals and supplies no recursive-MG speed claim.

All135 Python tests, required Rust1.85 and release scientific/protocol gates pass. All36 measured-source and PR CI jobs pass; final evidence-head CI remains a separate merge gate. No calibration/holdout, SCC scaling, Windows timing, default promotion or competitive qualification follows. The scalar default remains unchanged. Expanded reruns were not required: no numerical, work, capacity, routing, terminal-cap change or size-specific correctness concern arose.

## Preservation

`slices-preservation.json` records independent original/copy validation and byte-for-byte verification of every archive member. Linux derived geometric-mean archival roundoff uses the predeclared1e-14 relative bound; each difference is listed. Raw fields/hashes and scientific signatures remain exact. Provider ZIP hashes and sizes are independently checked before extraction. Archives are local preservation copies, not independent off-machine backups, under `$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6m/slices-preserved/` (current `/Users/johannes/Git/MultiwayMG-assessments/2026-09-19-m6m/slices-preserved/`).

The rejected iterator Linux smoke also completes 15,120 processes and 144,000 measured columns with 0 measured/0 warmup failures. Its15,120 M6l records match exactly. This additional complete smoke does not complete or replace the withdrawn Mac development collection.

## Rejected general-iterator experiment

The first source8655c351d022d48f8e51adc6406e56149ab12e8c is numerically exact
but rejected for performance. The general nalgebra column iterator leaves scalar
FMAs and per-element bookkeeping on ARM. Fixed old/new application ratios above1
favor the candidate; all warmups and five paired repetitions remain in the full
micro summaries. Complete Mac smoke preserves15,120 exact M6l records and144,000
certified measured columns. No speed claim follows from cross-revision times.

| Dimension | Mac old/new | Linux old/new |
| --- | ---: | ---: |
| 3 | 0.612287 | 0.928332 |
| 6 | 0.537075 | 0.972091 |
| 9 | 0.530310 | 0.985366 |
| 16 | 0.473174 | 0.982297 |
| 31 | 0.500200 | 1.000489 |
| 32 | 0.501589 | 0.997704 |
| 33 | 0.508459 | 0.990303 |
| 63 | 0.602984 | 0.999856 |
| 64 | 0.611693 | 1.023767 |
| 65 | 0.577156 | 0.995334 |
| 127 | 0.695811 | 1.003919 |
| 128 | 0.701159 | 1.000521 |
| 129 | 0.697585 | 0.997718 |
| 255 | 0.802731 | 1.003019 |
| 256 | 0.812053 | 0.996129 |
| 257 | 0.811271 | 0.997502 |
| Equal-dimension aggregate | 0.613554 | 0.991946 |

Thus the Mac application is about1.63x slower, with every size unfavorable;
Linux is near parity. The192 cross-platform output/modal/work records match
exactly after excluding clocks. The original Linux ZIP matches its provider
SHA256 and its summary recomputes exactly.

The unfinished development collection was withdrawn after the negative micro
and assembly diagnosis. Preserve2,154 complete journal entries and147.764251434
seconds of their process cost. The manifest remains collecting, all raw files
are unchanged, and a possibly interrupted in-flight attempt has unavailable
complete cost/output. There is no complete or selected-subset development
aggregate. See `iterator-comparison.json` for the explicit withdrawal receipt.

`iterator-preservation.json` records independently checked copies and archives
of both micro collections, complete Mac smoke, and partial development. Every
member matches its original size/hash. Complete copies validate exactly; the
partial collection is explicitly ineligible. Local preservation root:
`$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6m/iterator-preserved/`
(current `/Users/johannes/Git/MultiwayMG-assessments/2026-09-19-m6m/iterator-preserved`).
This is a local preservation copy, not an independent off-machine backup.

The retained revision above replaces only the general iterator with a contiguous slice. The original negative results remain unchanged; its completed Linux smoke is also archived and independently compared with M6l. No matrix/rank/routing/storage policy, scalar default, calibration or holdout changes.
