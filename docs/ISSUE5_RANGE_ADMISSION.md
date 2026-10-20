# Exact incidence kernel and full-range terminal admission (M6p)

M6o left full numerical-range preservation as an open gate. The M6o external audit
kept a valid connected V=12/rank-7 support where two-sided structural projection
around the `within` action leaks 0.252389 (unit) and 0.176210 (dyadic) into the
Gramian kernel. Structural factor-shift projection is the range projector only
when every component's kernel is exactly the two shift directions. Component
count is not numerical rank.

This increment supplies the missing range statement. It certifies the exact kernel
and projects onto its complement. It does not integrate a sparse terminal, change
routing or defaults, or touch the dependency pins.

## Topology-only exact kernel

For positive weights, `ker(B^T W B) = ker(B)`, so the complete kernel depends only
on the prepared topology and is shared by every weight frame on it.
`IncidenceKernelCertificate::try_certify` in `multiway-incidence` works in four steps:

1. Eliminates tuple rows over the prime field `2^61 - 1` in canonical tuple
   order with a fixed smallest-column pivot rule. Pivots give a minor that is
   nonzero modulo the prime, hence nonzero over the integers: a lower bound on
   rational rank. A component stops eliminating once it reaches `V_c - 2` pivots.
   The two exactly verified factor shifts already bound its rank from above, so
   the kernel is *structural*.
2. For a component with fewer pivots, back-substitutes one modular kernel vector
   per free coordinate.
3. Reconstructs each entry as a rational with numerator and denominator at most
   `2^30 - 1`, scales to coprime 64-bit integers, and checks every vector exactly
   (in `i128`) against every incidence row of its component. The free-coordinate
   pattern (own entry nonzero, other free entries zero) proves independence.
4. Concludes that the two bounds meet, so the retained integer basis spans
   exactly the *extended* kernel.

Any failed reconstruction, overflow or verification returns `KernelUncertified`.
No floating-point rank decision substitutes for it. All arrays reserve fallibly
after a ledger checks live requested payload, including old-plus-new overlap
during doubling growth, against `IncidenceKernelLimits`. Growth is
budget-independent, so the peak is a property of the input and the
equality/one-byte-under boundary is exact. Certification is deterministic.

`try_into_projector` consumes the certificate and orthonormalizes each component
kernel in a fixed order: the closed-form shifts, or the verified integer basis
converted exactly to `f64`. It uses two passes of modified Gram–Schmidt. It
checks orthogonality and incidence residual against a declared tolerance and
retains both bases. `IncidenceRangeProjector::project` is allocation-free and
deterministic. `multiway-mg::apply_range_projected` applies `P M P` with
caller-owned scratch and validates every dimension before mutation.
`RangeProjectedAction` is a diagnostic allocating wrapper over it.

For symmetric `M`, `P M P` is symmetric, maps into the exact range and annihilates
the exact kernel. It is positive on the range exactly when `M` is. This is the
treatment the sparse-terminal plan required: an exact full-range projector, not
a broader structural one.

## Qualification

- 400 random supports match an independent fraction-free Bareiss rational rank
  per component. More than 50 components fall in each kernel class. Every retained
  vector satisfies every row and has full rank.
- The five M6o audit topologies are classified exactly. Tensor and Latin have nullity 2.
  Nested (`(a,b,a)`) and sparse-connected have nullity 5, and two-components has 6,
  matching the audit oracle.
- The certified projector is symmetric to 1e-14, idempotent, has trace `V - nullity`
  and leaves every incidence-row image `B^T e_t` unchanged.
- Exact budget boundaries, every reservation failure and unwind, binding mismatch
  and invalid tolerances are all rejected cleanly.
- In the allocation harness, retained bytes equal net allocator payload, a rejected
  certification releases everything, and 64 repeated projections and wrapped
  applications allocate nothing.
- Dense oracle, `within` comparator, 45 topologies with unit, dyadic and 2^±30 weights:
  the full-range wrapper is symmetric with leakage at most 1e-12 and has no negative
  quotient direction. Unit/dyadic weights are positive on the range at the
  oracle tolerance. At 2^±30 the quotient energies span more than a dense `f64`
  eigensolver resolves, so only negativity beyond 64 eps times the largest energy
  is excluded. This is a stated resolution limit, not a positivity proof.
- The M6o negative is preserved: two-sided structural projection on the
  sparse-connected support still leaks 0.252389/0.176210 and fails the gate.

## Development evidence

On all 80 frozen smoke/development recipe runs, exact nullity agrees with the
dense oracle (see the [evidence](../benchmarks/results/2026-10-20/range-admission/README.md)).
Twelve runs have additional nullity, six of them on a component above 256 coordinates.
Two-sided structural projection is admitted on 68 runs and fails exactly those 12
(leakage up to 2.01e-2). Certified full-range projection is admitted on all 80.
Certification peaks at 73 KB of requested payload. Three of the twenty development
topologies (communities balanced, nested balanced and unbalanced) have such a
large component, which structural projection alone cannot admit.

The dense oracle's own range basis has an exact-kernel component of up to
7.7e-10 on one well-conditioned smoke topology. Its 1e-10 classification is
therefore near its accuracy limit there, and the certified projector is the
authoritative statement. Both observations are recorded rather than tuned away.

## Limits and next boundaries

- Modular elimination fill is bounded only by the declared payload budget. A
  component whose echelon form or extended basis (`nullity * V_c` integers plus
  `f64`) exceeds it is refused, not approximated. Large extended kernels cost
  dense per-component storage and `O(nullity * V_c)` projection work per side.
- Rational reconstruction caps entries at `2^30 - 1`. All development bases have
  entries of magnitude one; larger bases fail closed.
- The certificate binds a borrowed prepared owner. A sparse terminal must also
  bind the exact coarse topology it projects on.
- The terminal action itself still needs factor-memory admission. Within's
  approx-chol permutation scratch, opaque factor capacities, parallel setup and
  unchecked Schur/fallback reservations are unchanged and remain the
  [sparse-terminal plan's](ISSUE5_SPARSE_TERMINAL_BOUNDARY.md) step 2. That step
  needs a decision on delivering an owner-controlled `approx-chol` fork before
  implementation.
- Integration remains step 3: a distinct generation-bound terminal policy above
  256, actual recursive-cycle screening, independent original-operator
  certification, charged rejection/fallback/overlap, and only then a complete
  development cost comparison. No speedup, default or competitive claim follows
  from this increment.
