# Frame-bound matrix-free operator views (standalone M2)

`ThreeWayWeightFrame::operator_view()` returns a borrowed `ThreeWayOperatorView`
for that exact immutable numerical frame. It exposes B, B', sqrt(W)B, B'sqrt(W),
B'WB, normal-equation RHS, residual and compensated energy actions in caller
storage. It neither copies numerical/symbolic arrays nor discovers components.
View creation, first/repeated actions, identity checks and dimension rejection
allocate nothing. These serial operators require no scratch workspace.

Views and ordinary `ThreeWayProblem` now delegate to one internal `OperatorData`
implementation. The scalar methods were extracted without changing tuple order,
arithmetic association, accumulation or dimension checks; original error-context
strings are retained. Allocating problem convenience methods and dense reference
materialization also delegate to that core. There is no second numerical operator
implementation to diverge as later kernels are optimized.

## Identity, lifetimes and mutation boundaries

A view contains only a borrowed frame. Copies borrow the same owner. `binding()`
and `validate_for(frame)` let downstream state validate exact numerical identity;
equal separately constructed frames and changed frames both reject. Old and new
views may coexist and execute concurrently in separate caller buffers. An old
view remains valid for its original frame, never implicitly updates to a new one,
and cannot outlive or permit replacement of its borrowed owner. Structural owner
identity follows from the immutable frame's own exact topology borrow.

No external frame argument is required on ordinary action calls: the view already
names the complete submitted operator. The view cannot accidentally combine one
frame's roots with another frame's weights. A coarse replay's borrowed frame can
produce a view while retaining all replay/map/parent lifetimes.

All dimensions validate before caller output changes, including residual's RHS,
coefficient and output lengths. Rejections preserve NaN payloads and signed zeros
in untouched buffers. Low-level arithmetic retains the original propagation of
non-finite values and representational overflow. The view is not a convergence,
rank or numerical-finiteness certificate; M1's fail-closed solver boundary remains
necessary. No projection workspace, complete hierarchy or solver attachment is
claimed by this increment.

## Memory and verification

Exclusive heap payload is zero. Inline reference size is not heap payload; the
borrowed topology/frame arrays are charged once by their owning inventory. This
is not a process-RSS claim. No construction reservation failure is possible for
a view, and no artificial scratch allocation or global identity is introduced.

`multiway-incidence/tests/operator_views.rs` checks all valid supports in a 2x2x2
universe against explicitly materialized dense incidence algebra, including
connected/disconnected and additionally rank-deficient supports; weighted and
unweighted adjoints; Gramian, RHS, energy and residuals; wrong dimensions before
mutation; equal/changed/foreign frame owners; concurrent old/new views; and
Galerkin identity through compensated coarse replay with nonmonotone maps.

`multiway-mg/tests/support/operator_view_allocations.rs` is included in the
permanent isolated allocation executable. It measures construction, first and
64 repeated actions, identity checks and rejected dimensions with live allocator
instrumentation. Existing scientific and workspace suites also exercise the
owned API after extraction. Rustdoc compile-fail tests enforce the owner lifetime.

Next: reusable rank-robust LSMR execution (M3), then complete prepared supplied-map
serial solves (M4). This increment changes no automatic routing or frozen policy
and makes no performance claim beyond measured allocation absence.
