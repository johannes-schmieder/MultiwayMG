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

## Checked within setup (M6r)

The open item above is now delivered. Three merges carry it:

| Repository | Change | Merge |
| --- | --- | --- |
| approx-chol #3 | `CliqueTreeSampler` exact scratch reservation (`reservation_bytes`, `try_reserve`) | `05c2e58` |
| approx-chol #4 | `build_with_limit_reported`: accounting even when a build fails | `43f470a` |
| within #6 | `Preconditioner::build_checked`: serial setup under one live-payload ledger | `ffc1a56` |

MultiwayMG moves both within/schwarz-precond pins together to `ffc1a56`.

### What within's checked setup bounds

`build_checked` charges every array of within's construction before allocating
it, including reallocation overlap:

- weight permutation, active levels and pair lists;
- dense or sparse cross-tabulations, transposes and component splits;
- exact Schur rows and assembly, and sampled Schur edges with reserved sampler
  scratch;
- explicit Laplacians, inverse diagonals and partition weights;
- the result's shared blocks.

Each factor is built by approx-chol's checked builder against the remaining
budget. The peak of a failed exact attempt is folded in before within rebuilds
from the sampled complement; that is why fork PR #4 exists.

A refused setup stops before the refused allocation. A refused reservation is a
typed error. The limit never changes a successful result.

The scope is intercept-only designs, which is every three-way problem here.
Slope designs are refused before reserving.

Upstream qualification covers 88 builds over 18 inputs and 5 configurations. It
includes pivot-cancellation designs that reach the exact-failure fallback.
Results are:

- byte-for-byte equal to the parallel build on the wire;
- admitted at exactly their peak, and refused below it;
- every reservation failure is reported;
- isolated-allocator peak equal to the ledger peak with zero slack, in debug and
  release, on Linux, macOS and Windows CI.

### MultiwayMG

`WithinApproxCholPreconditioner::build_checked(problem, options, limit)` builds the
frozen comparator through that path. A refusal is the typed
`MultiwayError::WithinSetupBudgetExceeded`. Its memory report now carries
`within_retained_bytes()` (the complete retained requested payload, previously
`None`) and `within_setup_peak_bytes()`. `known_retained_bytes_estimate` is
unchanged. The unchecked build reports both new quantities as `None`.

Tests show that within's serial actions from the two routes are bit-identical on
two problems under both Schur modes. They also check the report and the inclusive
limit, refusing one byte under the peak.

On all 80 frozen development recipe runs ([evidence](../benchmarks/results/2026-10-20/within-setup-admission/README.md)):

- every checked setup is admitted at its peak and refused one byte below it;
- serial actions are bit-identical to the `Solver::new` route;
- comparator actions do not differ.

Setup peaks reach 2.58 MB, with 1.63 MB retained. Peak is 1.01 to 2.18 times the
retained payload. The complete retained payload is 1.9 to 75 times the factor
inventory, so M6q's factor count was a weak lower bound. The cross-tabulations
are likely most of the rest, since they scale with tuple count, but no
per-category breakdown was measured.

### Still open

- The parallel default path remains unaccounted.
- Pooled parallel-apply buffers are allocated on first use and not reported.
  The M6o serial workspace has its own exact payload.
- Allocator overhead and the wrapper's own level copies are excluded.
- Fixed-size shared blocks are charged but allocated infallibly.

Step 2 of the sparse-terminal plan is otherwise complete. Step 3 remains:

- a generation-bound terminal policy above 256 using the checked setup and the
  [certified range projector](ISSUE5_RANGE_ADMISSION.md);
- actual-cycle screening and independent original-operator certification;
- charged rejection, fallback and overlap;
- only then, a complete development cost comparison.

No routing, default, calibration or holdout change. Automatic MG still trails
grouped MAP by about 1.8× on development inputs.
