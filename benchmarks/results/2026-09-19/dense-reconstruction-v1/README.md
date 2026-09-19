# Dense reconstruction evidence

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

The next revision replaces only the general iterator with a contiguous slice.
It must collect fresh measurements and preserve this rejected revision. No
matrix/rank/routing/storage policy, scalar default, calibration or holdout changes.
