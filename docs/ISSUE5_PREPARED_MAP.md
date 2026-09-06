# Prepared structural projection and MAP (M4b)

Prepared topology now supplies caller-owned structural projection scratch. It
borrows the exact topology and uses its existing component labels and factor
sizes, with no rediscovery or copies. Projection is structural, so its scratch
may serve different frames on that same topology. An equal rebuilt topology is
rejected. Both the ordinary and prepared interfaces call the same scalar
projection and defect kernels, preserving compensated accumulation and ordering.

`PreparedSymmetricMap` borrows one exact immutable weight frame without owning
any additional numerical arrays. Its workspace owns four coefficient vectors and
one component-projection array. The forward/reverse factor sweeps share the
ordinary MAP kernel, including factor barriers and floating-point association.
Removing empty passes and reorganizing memory traffic remain measured M5 work.

MAP applications validate both vector dimensions, the exact frame binding and
finite input before scratch mutation. Every active scratch vector is initialized
on each application. A nonfinite projected RHS, sweep or projected solution
returns a typed numerical failure and leaves caller output unchanged. A later
valid call recovers using the same storage. Equal-valued or changed frames need
new explicit MAP scratch; there is no implicit generation rebinding.

The low-level prepared projection retains the ordinary projection's numerical
range. It is not an extreme-scale or numerical-rank certificate. The MAP wrapper
fails closed on nonfinite vector results; complete solve certification will use
the original operator independently. Native iteration convergence alone remains
insufficient.

## Memory and verification

MAP requested scratch is `32*V + 72*C` bytes for V coefficient coordinates and C
exact incidence components. Retained-capacity queries charge all five arrays;
borrowed topology/frame, inline descriptors, allocator metadata and process RSS
are separate. Constructors reserve fallibly. Tests inject failure at all five
MAP setup boundaries while a previous workspace remains usable.

Ordinary/prepared projection and MAP match bits on connected, disconnected,
unequal-factor and extra-rank-deficient cases with multiple weights/RHS. Tests
also check symmetry, nonnegative quadratic forms, repeatability, exact-owner and
dimension rejection before mutation, nonfinite and finite-overflow failures, and
valid recovery. The permanent isolated allocation executable covers first/repeat
32 applications, static/numerical failures and exact five-array release on every
supported platform/build/feature configuration.

This increment completes M4b. M4 remains open pending the fixed serial hierarchy,
PCG/LSMR drivers, original-operator certificate and repeated-RHS integration.
No performance qualification is claimed.
