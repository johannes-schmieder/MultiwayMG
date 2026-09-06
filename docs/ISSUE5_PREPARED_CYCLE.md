# Prepared serial fixed V-cycle (M4c, cycle increment)

`PreparedMapHierarchy` borrows the complete immutable current-frame replay and
owns its dense terminal factorization. It creates no ordinary problems, duplicate
fine topology/weights/components, or per-level numerical MAP owners. All MAP and
projection actions borrow their exact current frames. The numerical hierarchy is
immutable and has no owning clone or implicit changed-weight rebuild.

The ordinary and prepared paths now execute one shared fixed V-cycle recurrence:
one pre-MAP sweep, original-level residual, factor-map restriction, recursive
coarse correction, prolongation, one post-MAP sweep and structural projection.
This preserves the existing symmetry, factor boundaries and arithmetic ordering.
There is no adaptive inner stopping or timing-based route in the preconditioner.

The serial prepared route admits at most 64 levels and a **whole terminal** of
at most 256 coefficients before dense assembly. These are explicit execution
bounds. A large disconnected terminal still rejects even if its individual
components are small; component-local dense/sparse terminals remain M6 work.
The existing research hierarchy retains its old public construction behavior.

The prepared terminal assembles directly in native column-major storage, then
uses the same finite-checked, iteration-bounded spectral factorization and rank
threshold as the ordinary dense reference. This necessary part of direct dense
assembly moves forward from M5; ordinary dense convenience assembly remains
unchanged. No thresholding of positive tuple weights is introduced.

## Workspace and memory

`PreparedHierarchyWorkspace` borrows one exact numerical hierarchy. It owns every
traversal, per-level projection/MAP and terminal modal vector. Static identity,
input/output dimensions and finite RHS are checked before any scratch mutation.
Every active value is initialized on each call. Any numerical failure leaves
caller output unchanged; a later valid call recovers with the same capacities.
There is no apply-time allocation, hidden preparation, lock or implicit pool.

The retained payload report counts fine topology, coarse topology/maps, fine
frame, all coarse frames, terminal factors, complete application workspace and
caller-declared other live arrays exactly once. Heap descriptor capacities and
inactive/spare capacities are included. Caller inputs, outer Krylov/certificate
storage, RHS panels and other workers must be added explicitly. Inline roots,
allocator metadata, construction temporaries and RSS are separate.

The dense dimension cap bounds nalgebra's setup allocations; those internal
allocations are not made fallible by this API. Direct matrix reservation and all
application-workspace arrays are fallible. The retained report does not claim an
exact factorization peak or an allocator quota. End-to-end benchmarks must charge
and measure setup, including these temporary allocations and peak RSS.

## Qualification and remaining M4 work

All eight existing recursive fixtures match ordinary V-cycle output bit for bit
with original and changed weights, across multiple RHS. Tests cover symmetry,
nonnegative quadratic forms, terminal-only and extra-nullity references, exact
hierarchy/frame provenance, numerical failure/recovery and execution limits.
Errors and unwinds are injected at all 31 application-workspace reservation
boundaries of a two-transition case; poisoned traversal arrays are overwritten.

The permanent allocation executable measures terminal retained/released bytes,
all 31 workspace arrays, exact complete workspace payload, zero-allocation
first/repeat32 calls, static/numerical failures and recovery. Existing research
scientific gates continue to exercise the shared ordinary recurrence.

M4 remains open: integrate caller-owned outer PCG/LSMR, the independent original-
operator certificate, scalar reuse across bounded 1–32 RHS and complete solve
payload/economics reporting. Later memory, automatic construction, parallelism
and competitive qualification gates remain unchanged.
