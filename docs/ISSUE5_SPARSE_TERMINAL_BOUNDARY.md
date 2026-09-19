# Sparse terminal memory and execution boundary

M6n retains exact candidate visitation while removing a comparison sort. Its
complete development automatic route still trails grouped MAP by about 1.80x.
Further instruction-level experiments do not replace the remaining M6 requirement:
separately admitted fixed sparse terminals above the 256-coordinate dense cap.
This is an implementation sequence, not an implemented or qualified terminal.

## Inspected dependency boundary

The owner-controlled within fork is pinned at `fad1d462d44e7d5b5226370021d69dab4e854669`.
Its `SchwarzPreconditioner::subdomains()` and each entry's
`apply_weighted_into_with_scratch` already expose the local arithmetic. A fixed
serial fold can use caller-owned scratch with inner parallelism disabled. This
removes outer atomics, pooled scratch locks, thread-local buffers and O(PV)
accumulators from that new entry point. Existing APIs remain compatible. The
public within handle hides its concrete entries, so a narrow wrapper belongs in
the authorized fork, not copied local elimination code in MultiwayMG.

The generic local-solver trait permits implementations to allocate or ignore its
parallelism hint. A generic serial outer executor cannot promise otherwise.
Within's concrete kernels must be qualified separately. Current setup also uses
`into_par_iter`, so a serial apply API alone does not bound construction workers.

The pinned registry `approx-chol` 0.5.0 factor's `for_each_block` allocates an
n-element permutation vector when its optional permutation is present. Both
solve-into and solve-in-place reach this branch. This does not mean every current
within pair solve allocates: connected factors may avoid it. Disconnected and
interleaved factors must be tested, and either receive explicit permutation
scratch or have a proven, documented eligibility restriction.

Factor blocks, fallbacks and permutation arrays have private capacities. Public
n/original-n/step counts and serialization lengths cannot establish exact retained
payload or construction peak. Serialization adds storage; post-build RSS is not
pre-admission. The within builder's Schur complement, exact-pivot failure and
approximate fallback lifetimes need bounds at actual allocation sites. No factor
fork, vendoring, pin change or upstream message has been performed by this audit.

## Delivery sequence and gates

1. Add a serial, caller-owned, immutable-owner-bound within action workspace.
   Use checked sizes and fallible reservation, a fixed entry/reduction order,
   and exact local slices including ground/cover augmentation. Validate static
   shapes and budget before allocation or output mutation. Define local-failure
   output semantics explicitly; if output is transactional, charge its V-vector.
   Test first/repeated actions, failure/recovery, uncovered/empty domains and
   disconnected/interleaved inputs against existing local arithmetic. Distinguish
   generic outer scratch from opaque local-factor allocations and total memory.
   Qualify the fork and move both within/schwarz pins together only after merge.
2. Add factor capacity statistics, permutation scratch, explicit setup execution
   and checked Schur/fill reservations through a narrowly reviewed dependency
   change. Decide its exact delivery/pin before implementation. Bound all admitted
   routes, including failed exact-to-approximate overlap and every live ancestor.
   Exercise equality/one-byte-under limits, overflow, reservation failure and
   cleanup. Preserve an immutable upstream comparator. Do not call an opaque
   estimate a complete memory budget.
3. Integrate a distinct generation-bound terminal policy above 256. Apply a fixed
   action with no RHS-dependent inner stopping. Qualify symmetry, full numerical
   range preservation and positive energy on that range, including additional
   nullity and extreme positive weights. Structural factor-shift projection alone
   is not the full range projector; component count is not numerical rank. Test
   the actual recursive cycle and independently certify against the original
   operator. Charge screening, rejection/fallback and old/new generation overlap.
   Only then compare complete setup/application costs on the frozen development
   matrix and advance the M6 gate.

The modified Golub–Kahan solver is metric-preconditioned; treating its inverse
Gramian action as ordinary right preconditioning gives an incorrect conditioning
objection. No mathematical invalidity of the current additive method is inferred
from these API gaps. Preserve fixed configurations, all negative evidence, dense/
MAP fallback and the accepted M7 explicit-pool, M8 independent-panel, M9 fresh
numerical replay and M10 untouched-holdout requirements.
