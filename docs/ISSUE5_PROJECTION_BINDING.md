# Issue 5: structural projection workspace identity

## Source audit and repaired contract

At `fe3dc2f`, the merged projection workspace checked only coefficient dimension
and component count. Contrary to the handover, the private partition binding had
not landed. Equal-sized component partitions can differ; the public workspace
contract now fails closed rather than treating those two counts as identity.

Each `IncidenceComponents` construction owns a private `Arc<()>` identity.
Ordinary component/problem clones share it. A projection workspace clones that
identity when created, and both workspace-backed projection and defect methods
require `Arc::ptr_eq` after their existing dimensional checks and before
clearing scratch or changing caller values. A mismatch returns the dedicated
`IncidenceError::WorkspaceBindingMismatch`.

The conservative contract rejects all independently constructed decompositions,
including metadata-equal ones. Reconstructing the same topology is not the same
as cloning its existing component decomposition. This avoids hash collisions,
global counters, address reuse after owner destruction, and repeated partition
comparisons. The workspace's strong reference keeps the token alive, and
ordinary clones remain compatible even after the original owner is dropped.
The token carries no topology payload, weights, numerical state, or mutable
operator cache.

`IncidenceComponents` retains value equality of its actual component metadata.
`StructuralProjectionWorkspace` equality also requires compatible binding, in
addition to equal dimensions and scratch values. Neither equality operation is
used to replace the explicit application-time validation.

## Memory and numerical boundaries

The existing `retained_bytes()` API reports the capacity-based payload of the
workspace's exclusively owned component scratch array. It excludes the inline
object, allocator metadata, and the shared identity token's reference-counting
metadata; it is not a complete ownership-graph or peak-lifetime memory report.
There is one shared token per independently constructed decomposition, not per
application. Binding clones and validation allocate no additional token.

Projection formulas, accumulation order, compensated sums and defect arithmetic
are unchanged. Current scratch is cleared before use, so the missing identity
check was a missing rejection contract, not evidence by itself that numerical
projections had been corrupted. The change prevents accepting an incompatible
workspace as future prepared/generation-safe infrastructure is built.

This contract intentionally does **not** apply to
`CycleScreenedMapHierarchyWorkspace`: its anonymous vector scratch remains
cross-instance and cross-size reusable. The hierarchy and traced-PCG bitwise
regressions must continue to pass.

## Regression and repository cleanup

The adversarial fixtures have identical coefficient dimensions and numbers of
components, but different coefficient-to-component partitions. Tests cover
rejection before output/scratch mutation for both projection and defect,
subsequent valid reuse, ordinary component/workspace clones after owner drop,
metadata equality versus private compatibility, and preserved capacity.

The obsolete `.github/workflows/issue5-workspace-binding.yml` and
`.github/workflows/issue5-workspace-binding-cleanup.yml` are removed. They were
one-time source-edit orchestration accidentally retained on main, not permanent
validation. No permanent Rust or scientific CI workflow is changed. Rust 1.85
GitHub Actions is the authoritative acceptance gate.
