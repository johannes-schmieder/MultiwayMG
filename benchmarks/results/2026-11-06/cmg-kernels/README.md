# CMG scalar-kernel qualification

This separate fixed study qualifies the changed upstream CMG revision
`e5613e81fb3b03c78de98658b372646fa802aabb`. Earlier CMG consolidation
results remain unchanged in the adjacent directory. All planned observations
and independent scientific checks are retained; none were repeated or excluded.

All 40 fixture/RHS cells pass max(5%, 1 ms). The largest ratio is 1.05160;
that sub-millisecond increase is covered by the declared 1 ms allowance.
All 54 independent spectral rows are numerically unchanged, including the
12 pair-CMG rows. Rust 1.85 formatting, Clippy, all/minimal-feature tests, docs
and retained examples pass. Hosted CI passes all 15 jobs; allocation checks
pass on macOS, Linux and Windows. The within/Schwarz pin remains unchanged.

The receipt binds the exact runtime source and all payload hashes. This evidence
does not itself publish a release or freeze CMG main.
