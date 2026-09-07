# Stable grouped tuple kernels and MAP

M5c, 2026-09-06. This increment implements the separately admitted primitives
prioritized by [M5b diagnostics](ISSUE5_KERNEL_PROFILING.md). Scalar defaults and
complete prepared solver routes remain in place. Grouped hierarchy integration,
layout timing and a selector are subsequent work; these primitives do not close
M5 or establish a performance improvement.

## Representation and admission

`PreparedTupleGrouping` borrows one exact immutable `PreparedThreeWayTopology`.
It retains one `usize` offset array of length V+3 and one 2E tuple-ID array for
factors 1 and 2. Factor 0 uses implicit contiguous canonical ranges. Stable
counting placement preserves ascending original tuple IDs within every row.
Canonical 12E-byte AoS tuples remain the only tuple representation, and grouping
contains no numerical weights or copies of components/observation groups.

IDs use u32 when the maximum ID fits (including E=2^32 on a 64-bit target),
otherwise checked usize storage. `GroupedTupleRow` exposes contiguous, narrow
and wide immutable row views; its `for_each` dispatches outside each row loop.
No per-row allocation or compulsory grouping occurs on scalar execution.

On a 64-bit target, requested retained grouping is 8(V+3)+8E bytes for narrow
IDs or 8(V+3)+16E for wide IDs. Construction temporarily holds one reused
8*max(n1,n2) cursor. Checked array sizes respect isize limits. Setup admission
charges actual borrowed topology capacity, requested new arrays/cursor and the
caller's explicitly declared other live payload, including older groups/frames.
Actual retained capacities are reported separately. This requested-array budget
excludes stack, inline roots, allocator metadata and excess new capacity; it is
not an allocator quota or RSS cap. Construction errors publish no partial owner.

## Explicit arithmetic alternatives

`ThreeWayOperatorView::with_grouping` returns a zero-allocation
`ThreeWayGroupedOperatorView` after exact topology identity validation. Immutable
borrows then maintain the relationship. New weight frames can share the same
grouping and always supply their own current weights/roots. Equal-but-foreign
structural owners reject. The original scalar view remains available for
independent certification.

- Stable row gathers implement adjoint, weighted adjoint, RHS and Gramian.
  Each output starts at +0.0, visits original ascending tuple IDs, and preserves
  the scalar weight products and `(x0+x1)+x2` association.
- `apply_gramian_with_image` computes exactly the same weighted image once into
  a caller-owned E-element slice, then gathers it by row. All dimensions are
  checked before either image or output changes. The fully overwritten image
  retains no generation-specific cache and costs 8E bytes per scalar workspace.
- Incidence, weighted incidence and compensated energy use the original scalar
  tuple kernels. Residual uses grouped Gramian followed by the original subtraction.
- `PreparedSymmetricMap::with_grouping` returns `PreparedGroupedSymmetricMap`.
  It shares the existing frame-bound three-vector/projection workspace and all
  static/finite validation and transactional publication. Forward/reverse rows
  start from their RHS/rounded middle, subtract **each tuple coupling separately**
  in original order, and divide by the diagonal. Couplings retain increasing
  factor `mul_add` order and the complete middle is rounded before reverse work.
  Summing couplings first and subtracting once would violate this contract.

Low-level grouped operators propagate nonfinites like scalar actions. MAP keeps
its finite-input/arithmetic checks and returns typed errors without publishing
failed output. Neither view treats a numerical stopping flag as acceptance.

## Verification boundary

Tests cover every valid 2x2x2 support against independent dense incidence algebra,
ragged/disconnected/skewed supports, additional nullity, nonmonotone Galerkin
replay, signed zeros, subnormals and extreme finite scales. Both stored widths
are exercised without allocating billions of rows. Grouped MAP matches ordinary
prepared arithmetic and the independently frozen pre-M5 loop, including poisoned
scratch and rounded middle bits; symmetry and nonnegative action checks remain.

Exact-owner/dimension checks precede mutation; same topology/new weights can be
used concurrently through immutable views. Compile-fail examples prove topology
and grouping lifetimes. Every grouping reservation has injected failure coverage,
with panic/unwind recovery and checked size/index-width boundaries. The exact
payload-budget boundary rejects before reservations.

The isolated allocator executable measures three grouping setup allocations,
one released cursor and two retained arrays, then exact release. It measures
zero allocations for view construction and all first/repeat32 grouped actions,
MAP, static errors, numerical errors and recovery. MAP still allocates only its
four existing setup arrays. Linux/macOS/Windows debug/release and minimal/all
feature CI include this new contract. No timing win is inferred from it. Required local Rust 1.85 checks and 73
Python tests pass. Targeted release runs also pass grouped operator/Galerkin,
both-width, prepared/frozen MAP and complete allocator contracts.

## Next integration and measurement

Integrate explicit fine-only/all-nonterminal grouping into complete supplied-map
hierarchies, including every level's retained grouping, construction overlap and
optional tuple-image workspace. Keep scalar original certificates and compare
full PCG/gated LSMR coefficients, work and acceptance before collecting timing.
Freeze source/recipe for complete uninstrumented paired comparisons, retaining
all failures and balanced RHS widths. Use aggregate tuple complexity from M5b
when admitting layouts; decide no implicit default from nominal byte counts.
