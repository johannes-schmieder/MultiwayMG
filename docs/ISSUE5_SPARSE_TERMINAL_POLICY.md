# Generation-bound sparse terminals above 256 (M6s)

This is step 3 of the [sparse-terminal plan](ISSUE5_SPARSE_TERMINAL_BOUNDARY.md).
Steps 1 and 2 delivered the serial `within` workspace (M6o), the certified
full-range projector (M6p), factor memory (M6q) and `within`'s checked serial
setup (M6r). This increment integrates them as an explicit, separately admitted
terminal policy in the automatic driver, qualifies it, and measures its complete
cost. Frozen policies and defaults are unchanged.

## The terminal

`PreparedSparseTerminal` (feature `sparse-terminal`) is a fixed linear action
`P W P` for one exact numerical frame:

- `W` is one serial application of `within`'s additive pair Schwarz
  preconditioner, built by `Preconditioner::build_checked` for the level's
  current tuples and weights;
- `P` is the certified orthogonal projector onto the complement of the level's
  exact incidence kernel.

`W` is symmetric positive semidefinite, so `P W P` is symmetric, maps into the
exact range and annihilates the exact kernel. There is no RHS-dependent inner
stopping. Application is allocation-free: one caller-owned workspace holds
`within`'s serial array and a projected input vector.

Construction charges against one declared limit, before allocating:

- the kernel certificate and its projector (M6p ledgers);
- three level-code columns;
- a fixed 1,024-byte bound for the borrowed `within` design descriptors (264
  bytes measured);
- `within`'s checked setup, run against the remaining limit.

A refusal is typed and retains nothing. The terminal binds its frame by
identity; a hierarchy accepts it only for exactly its coarsest frame.

`PreparedMapHierarchy`'s terminal is now dense or sparse. The cycle kernel's
terminal scratch became an associated type, and the dense path's arithmetic and
tests are unchanged. The boxed terminal descriptor is charged as retained payload.

## The policy

`solve_prepared_automatic_batch_into_with_sparse_terminal` runs the existing
driver unchanged except where a large component's coarsening reaches the
hierarchy's transition cap above 256 coordinates:

- the frozen policy rejects that attempt as `HierarchyStagnated`;
- this policy admits a sparse terminal on the capped level, if it is within the
  declared `maximum_dimension`.

The hierarchy then goes through the same actual recursive-cycle screen,
certificate-gated LSMR solve, independent original-problem certification and
fallback as any other. Zero transitions put the sparse terminal on the component
itself. A non-reducing aggregation, a refused budget or an uncertified kernel
keeps its typed rejection. Policies with no hierarchy, a cap at or below 256, or
invalid tolerances are refused before any mutation. When coarsening reaches the
dense cap, the policy is bit-identical to the frozen route.

## Qualification

**Terminal.** 37 supports were tested:

- the M6o/M6p audit topologies;
- 30 random supports;
- two supports above 256 coordinates (nested-400 with additional nullity, and
  random-300).

Each support was tested with unit, dyadic and `2^±30` weights against the dense
oracle. Every terminal is symmetric, has range leakage of at most 1e-12, and
matches the exact nullity. It is positive definite on the range, except that at
`2^±30` only negativity beyond 64 eps of the largest energy is excluded (the
M6p oracle limit).

**Hierarchy.**

- A depth-zero hierarchy applies exactly its terminal, up to roundoff from the
  structural projection.
- A one-transition cycle with a sparse terminal is symmetric and positive.

**Limits.** Admission is inclusive: the exact peak is admitted and one byte
below is refused. The dimension cap, tolerance and frame identity are checked.

**Allocator proofs.**

- Terminal builds stay within the reported peak, with 808 bytes of slack: the
  descriptor bound.
- Retained payload is exact.
- Application allocates nothing.
- Refusals stay within their limit and leak nothing, in debug and release.

**Driver.**

- Both engagement routes certify every column independently.
- With coarsening that reaches the dense cap, scalar and all-image layouts are
  bit-identical to the frozen route.
- A budget refusing only the terminal falls back with a typed rejection.
- Mixed components and grouped layouts certify.

**Driver allocator proof.**

- On five routes, two of them with two large components, the observed peak plus
  the caller's owners fits the admitted payload, and nothing is retained.
- Repeated calls produce identical records.
- It caught one real undercharge: the boxed 472-byte terminal descriptor, now
  charged.

Both allocator proofs run in the three-platform allocation job.

## Complete cost

The [evidence](../benchmarks/results/2026-10-20/automatic-sparse-terminal/README.md)
covers all 80 frozen seed-10001 smoke/development recipes at K=1 and K=32:
2,400 measured cold processes, five routes, three rotated repetitions. It was
collected on a Linux container, not Mac/SCC.

**Correctness.** Every column of every route certifies, and repeated outputs are
bit-identical.

**Development driver time:**

| comparison | speedup |
| --- | --- |
| frozen automatic / global MAP | 0.324 (about 3.1× slower) |
| sparse t0 / frozen automatic | 3.11 |
| sparse t0 / global MAP, overall | 1.008 (parity) |
| sparse t0 / global MAP, K=1 | 0.448 |
| sparse t0 / global MAP, K=32 | 2.265 |

**By family**, t0 versus global MAP:

- wins on nested (2.46), pair-dominant (2.20), chain (2.03), communities (1.55)
  and ragged (1.40);
- loses on worker-firm-occupation (0.84), exporter-importer-product (0.72),
  hubs (0.58), uniform (0.44) and tensor (0.29).

**Shallow hierarchy (t1).** Its sparse terminal engages on 36 cells, but t1
stays as slow as the frozen route (0.40 versus global MAP).

**Memory.** Peak RSS for t0 is at most 11.1 MB, against 9.0 MB for global MAP.

So the sparse terminal makes the screened automatic route competitive with MAP
only when it replaces coarsening entirely. There the `within` action does the
work. The win is confined to repeated right-hand sides and harder families, and
there is a clear K=1 loss.

## Limits

- The policy is explicit and fixed; nothing selects it, and no default changes.
  A width- or family-aware selector would need a calibrated, frozen cost model
  (M8) and the untouched holdout (M10).
- `within`'s parallel default path is unaccounted. The terminal uses only the
  checked serial build and serial application.
- The design descriptor bound is fixed and verified by an allocator test, not
  computed from `within` internals.
- Evidence is one Linux container, scalar layout, K∈{1,32}. Mac/SCC, thread
  scaling (M7), panels (M8), changing weights (M9) and holdout qualification
  (M10) remain.
