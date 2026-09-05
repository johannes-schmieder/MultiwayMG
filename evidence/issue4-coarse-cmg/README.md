# Issue 4 coarse-only CMG calibration evidence

This directory freezes calibration evidence for testing pair-CMG only on non-finest levels of the existing three-way hierarchy while keeping the fine `within` smoother fixed.

## Provenance

Final read-only GitHub Actions run: `33937258900`.
Exact benchmark/source head: `4959cd69e43a906dd2b7c770d28d1cb4fc8007e5`.
Uploaded artifact: `issue-4-coarse-cmg-output`, artifact ID `9960594295`.
Artifact digest: `sha256:3033802c218646cfdff69bb9a070f495609de641ec8717ed578aa1c3cc3a963a`.

The raw automatic/oracle TSVs and complete runner metadata remain in that Actions artifact. `checksums.sha256` records the hashes of those raw files, while the two Markdown files preserve the validated summaries.

The run used Rust 1.85.0 (`4d91de4e48198da2e33413efdcd9cd2cc0c46688`), single-thread settings (`RAYON_NUM_THREADS=1`, `OMP_NUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`), CMG commit `90e1fe0b0c14065155532711246ede6678bb4935`, and `within`/`schwarz-precond` commit `b7779cbab7a3116be56aae4389fde1f6e6a99a9f`.

## Interpretation

Automatic admission is sparse: seven of the eight already-revealed recursive issue-3 fixtures are rejected by the current automatic planner. The one admitted fixture shows no outer-work benefit from replacing non-finest `within` smoothers with pair-CMG, and no 32-RHS charged timing win.

The oracle-map calibration isolates the local-solver choice by holding the already-revealed issue-3 map sequence fixed. One communities hierarchy is excluded symmetrically because the all-`within` baseline itself violates the outer SPD/certification gate. Across the remaining seven fixtures / fourteen solver cells, pair-CMG produces zero cells with at least 20% lower outer work. Two weak-chain cells have charged timing wins, but both use *more* outer work than the all-`within` baseline. Therefore no observed coarse-level case satisfies the joint issue-4 advancement criterion.

Together with the finest-level size-ladder evidence, this does not support freezing a CMG routing rule or spending a fresh holdout. A future CMG attempt should first improve local setup/application economics or local spectral quality enough to create a material calibration candidate.
