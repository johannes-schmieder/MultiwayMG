# Complete prepared projected PCG (M4c)

The prepared PCG driver shares the existing **untraced** projected-PCG recurrence
with the ordinary allocating entry point. It uses six caller-owned coefficient
vectors and explicit mutable projection/preconditioner actions. Dot products,
FMA updates, projection order, recomputation schedule and stopping/breakdown logic
are unchanged. Traced research PCG remains a separate diagnostic implementation.

The ordinary convenience wrapper now creates this storage and projection scratch
once, so it no longer allocates a new projection/residual vector during every
iteration. It reserves the complete six-vector shape even for zero RHS; its setup
and memory behavior therefore changes. Competitive comparisons must continue to
use the separately frozen original MultiwayMG executable, not relabel the current
in-process ordinary wrapper as an immutable baseline.

`PreparedPcgWorkspace` borrows one exact numerical hierarchy and owns complete
outer/hierarchy/projection/RHS/certificate storage. The least-squares entry point
constructs `B'Wy` from canonical tuple targets and runs native projected PCG.
It then independently certifies `||B'W(y-Bx)|| / ||B'Wy||` using the shared fail-
closed certificate. Returned native stopping information and independent
`accepted` status remain separate. Extra unidentified directions can cause PCG
breakdown; this API does not turn structural projection into a rank certificate
or implement an automatic fallback. Rectangular LSMR remains the rank-robust
candidate route for subsequent selector work.

Scalar batch execution supports the same declared RHS counts up to 32 with
column-major arrays. Static validation precedes mutation. Each completed column
receives a candidate and report whose acceptance must be checked. A numerical
failure preserves the completed prefix and leaves failed/unprocessed columns
unchanged; the first empty report names the failed column. Work counts retain
actual RHS adjoints, fine Gramian actions including residual recomputation,
complete hierarchy applications, outer projections and independent certificate
work, including admitted failures.

## Payload and verification

Outer requested scratch is `8*(E + 9V) + 72C` bytes: six recurrence vectors,
one coefficient RHS, the three certificate arrays and component projection.
Add complete hierarchy application scratch. Setup admission counts all immutable
direct owners plus requested new arrays and caller-declared other live state.
Retained reports include actual capacities and heap descriptors, not process RSS
or opaque factorization/allocator overhead. Numerical breakdown construction is
not claimed universally allocation-free; successful repeated solves are.

All eight recursive fixtures match ordinary coefficients and native diagnostics
bit for bit and pass the independent certificate on the declared cases. Tests
also cover RHS1,2,4,8,16,17,32, zero RHS, exact-owner and static rejection,
failure-prefix preservation/recovery, iteration-cap nonacceptance and a connected
terminal with additional nullity. All six new storage reservation boundaries are
injected with failure while previous storage remains usable; poisoned storage
and a failed mutable action recover on the next valid solve.

An independent local probe compiles the **unaltered pre-refactor PCG source** at
`347210d612b3b374c5b52bc7d2f30d41ef244447` beside the current library. All 144
comparisons match coefficient and scalar diagnostic bits: eight recursive
fixtures, zero/two general RHS, iteration caps 1/3/1000 and recomputation intervals
1/25. The frozen source SHA-256 is
`d87bab88a36bcd4d2d6709b3c4c02a049acf3aa9eb30f0eb47b727f86e9ccc76`.
The probe, inputs/source and raw TSV are preserved outside the repository at
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/pcg-equivalence`.
This is a local recurrence comparison, not cross-platform bitwise qualification.
The permanent scientific/platform CI continues to qualify supported configurations.

The permanent allocation executable verifies first solve and every declared RHS
count without allocation, one-byte-short budget rejection, exact complete payload
and release of 42 arrays in a two-transition case. Native/independent stopping
remain distinct and no performance qualification is claimed.

M4 still requires a reproducible complete-cost serial benchmark/evidence surface.
Memory-traffic optimization, scalable automatic construction, CPU/RHS scheduling,
changed-weight quality admission and competitive qualification remain M5–M10.
