# Factor memory and checked construction (M6q)

Step 2 of the [sparse-terminal plan](ISSUE5_SPARSE_TERMINAL_BOUNDARY.md) required
explicit factor capacities, caller-owned permutation scratch and checked construction
reservations, delivered through a narrowly reviewed dependency change. The owner
authorized an `approx-chol` fork for this
(`johannes-schmieder/approx-chol`). Its `multiwaymg` branch starts at the published
0.5.0 source `36f07b1`, and upstream `main` is untouched.

## Delivered dependency chain

| Repository | Change | Merge |
| --- | --- | --- |
| approx-chol #1 | Caller-owned permutation scratch, checked `retained_heap_bytes()`, `SolveError` discriminant repair | `68fb926` |
| approx-chol #2 | `Builder::build_with_limit`: every construction array charged to a live-payload ledger | `68ad1b9` |
| within #5 | approx-chol pinned to `68ad1b9`; permutation scratch from dead local buffers; `local_factor_retained_heap_bytes()` | `504fbb7` |
| MultiwayMG (this increment) | `within`/`schwarz-precond` pins moved together to `504fbb7`; comparator reports factor capacity | — |

The recovered fork commit `60e6cfc` inserted a new error variant before an existing
one. That shifted a published discriminant and failed the inherited
`cargo semver-checks` gate (`enum_no_repr_variant_discriminant_changed`). The failure
was reproduced; appending the variant passes all 196 checks.

## What is now bounded

- **Factor application.** No factor layout allocates during a successful within
  local solve. Permuted factors take scratch from regions of the two existing local
  buffers that are dead at that point, so the outer serial workspace is unchanged.
  Interleaved factors that require a permutation match the allocating 0.5.0 solve
  bit-for-bit.
- **Retained factor memory.** Checked capacity inventories exist per factor and
  per within preconditioner. The comparator memory report exposes the latter as
  `within_local_factor_bytes()`. It is factor arrays only, so it is a lower bound
  on within's total retained state and is not added to the existing estimate.
- **Factor construction.** `build_with_limit` charges the requested capacity of
  every construction array before allocation, including reallocation overlap,
  sort scratch and a failed exact block's dense copy. It refuses an exceeded
  inclusive limit with a typed error before allocating, and never changes a
  successful factor. On 155 generated builds, an isolated counting allocator
  found the real live-heap peak equal to the ledger peak with zero slack, in
  debug and release on Linux, macOS and Windows. All builds are wire-identical to
  registry 0.5.0. The unaccounted `build` is within noise of 0.5.0 (1.012×
  geometric mean); the checked path costs 1.032× on one container
  (informational, not a gate).

## Unchanged and still open

`schwarz-precond` is byte-identical between the previous pin `69110db` and
`504fbb7`. The automatic route and every frozen protocol's numerics therefore
cannot change. Only the `within-comparator` path now uses the fork, whose
unaccounted build reproduces 0.5.0 bit-for-bit.

within's own construction is not yet admitted: pair cross-tabulations, exact and
sampled Schur complements, cover assembly, the coordinate maps and its parallel
`into_par_iter` setup. Exposing `build_with_limit` through within with an explicit
serial construction path is the next increment. Only after that does step 3, a
generation-bound sparse-terminal policy above 256 using the
[certified range projector](ISSUE5_RANGE_ADMISSION.md), become admissible.

The inherited within `SemVer (public API)` check is now red on the fork's
`multiwaymg` base as well as on #5. It compares against crates.io `within` 0.4.0,
which was released from a later upstream line than this fork branch. Fixing it
needs a separate decision: rebase the fork or change its baseline.

No routing, default, calibration or holdout change. Automatic MG still trails
grouped MAP by about 1.8× on development inputs.
