# Issue 5: prepared projection, MAP and terminal scratch

This slice removes temporary vector allocation from the prepared symmetric-MAP
and dense-terminal **application** APIs. Hierarchy integration and the measured complete-cycle allocation contract are
now documented in `ISSUE5_WORKSPACES.md`. No timing improvement,
complete solver workspace, weight replay, or total peak-memory claim is made.

## Projection: explicit preparation, strict application

`IncidenceComponents::try_projection_workspace()` is a fallible constructor.
`projection_workspace_required_bytes()` reports minimum scratch-array payload.
The existing convenience constructor and numerical projection formulas remain
unchanged.

`StructuralProjectionWorkspace::is_compatible_with(components)` checks exact
private construction identity as well as dimensions. `try_prepare_for` explicitly
resizes and rebinds scratch at a setup boundary. An already compatible call is
an allocation-free no-op. Otherwise scratch is cleared, and a successful
reservation precedes changing its dimensions and binding. A failed reservation
preserves the previous values and binding. Application and defect methods still
reject incompatible workspaces before touching values or scratch; they never
silently rebind. This API does not retain topology or weights in scratch.

## MAP

`SymmetricMapPreconditioner::application_workspace()` returns fallibly allocated
`SymmetricMapWorkspace`. `apply_with_workspace` validates input/output lengths,
all scratch lengths and the projection binding before any caller output mutation.
It uses four vectors (compatible RHS, forward, middle, staged solution) and one
projection workspace. The complete staged result is copied to output only after
both triangular sweeps and the final projection succeed.

Ordinary `apply` and the prepared API execute one numerical implementation. The
sweep order, tuple order, fused multiply-add expressions, division and projection
arithmetic remain unchanged. The extra staged solution enforces transactional
caller output. Independently built component decompositions require explicit
`workspace.try_prepare_for(&map)`; ordinary clones remain compatible. Rebinding
scratch is not reusing a numerical preconditioner under new weights.

## Dense terminal

`DensePseudoinverse::application_workspace()` prepares
`DensePseudoinverseWorkspace`. `solve_into_with_workspace` reuses its modal vector;
ordinary `solve_into` delegates to the same two modal loops. All modal entries
are overwritten before consumption. The workspace is anonymous: independent
terminals of equal dimension may share it. Different dimensions require explicit
`try_prepare_for(&terminal)` before application. Terminal construction and the
rank-revealing factorization are unchanged and still allocate.

## Memory and failure boundary

Required-byte queries count minimum exclusive vector payload. Retained-byte
reports use actual vector capacities with checked sums and products. They exclude
inline descriptors, immutable problems/factors, shared identity reference-count
metadata, allocator overhead and caller input/output. These are not process RSS
or peak-lifetime reports. A partial multi-vector MAP reservation failure may
retain additional capacity, but does not publish new lengths or binding.

No global pool, locks, unsafe code or mutable numerical operator is introduced.
Capacity-overflow tests exercise rejected reservation without attempting a huge
allocation. Full allocator-failure injection remains a separate engineering gate.

## Independent regression references

The test-only modules under `tests/support/pre_workspace_{map,dense}.rs` preserve
the allocating operator implementations from main `e60809b`. Only module imports,
private dimension-error construction and reference type names differ. These are
permanent regression oracles, not production alternatives or generation scripts.
MAP is compared bitwise on all eight already-revealed recursive fixtures and
signed, scaled and zero RHS vectors. Dense terminals, explicit rebinding, changed
sizes, binding/dimension rejection, capacity retention and post-rejection reuse
have additional tests. Existing hierarchy/PCG scientific gates remain required.

Rust 1.85 GitHub Actions is authoritative. ADR 0002 is unchanged and no fresh
holdout is consumed.
