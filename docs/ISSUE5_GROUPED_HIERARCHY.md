# Explicit grouped complete hierarchies (M5d)

M5c supplied stable row primitives. M5d connects an explicitly selected prefix
of nonterminal levels to complete prepared cycles, PCG and native/gated LSMR.
It does not select a default layout or claim a speedup. The separate paired
layout experiment follows with a committed policy and measured source identity.

## Structural ownership and admission

`PreparedHierarchyGrouping` borrows an exact `PreparedHierarchyTopology` and
owns only the selected `PreparedTupleGrouping` objects and their descriptor
array. Prefix zero executes scalar kernels; prefix one groups the nonterminal
fine level; the full depth groups every nonterminal. Terminal-only hierarchies
accept zero. No per-unselected-level descriptor is reserved, and no numerical
weight or factor is cached in this object. New weight frames may share it.
Equal but foreign hierarchy owners reject before numerical factor construction.
Borrow checking prevents either the grouping or numerical hierarchy outliving
its borrowed owner.

Before the first reservation, admit the entire requested setup peak: fine and
owned coarse structure once, the selected descriptor array, all previously built
groups, the next group's arrays and its construction cursor, plus declared other
live arrays. Cursors die between levels, so their peak is a maximum over builds,
not a sum. Each actual build rechecks retained capacities and the final total.
Old groups, numerical frames, terminal factors and workspaces must be declared
when they overlap. This is array-payload admission; stack, allocator metadata,
allocator excess capacity during reservation, and RSS remain separate scopes.

## One shared image and unchanged scalar scratch

`GroupedGramianMode::RowGather` uses stable row gathers and no additional
numerical vector. `TupleImage` appends one maximum-E image vector to the existing
hierarchy workspace. A Gramian consumes its image before its caller recurses or
starts another action; therefore all cycle levels and outer PCG reuse that one
buffer. The shared mutable borrow enforces sequential access. Each operation
fully overwrites the used prefix before reading it. In these collapsed
hierarchies coarse tuple counts cannot exceed fine E.

The four live traversal buffers per transition remain unchanged. Scalar and
row-gather numerical capacities and allocation counts are identical. Image mode
adds exactly `8*max(E_selected)` bytes plus one `Vec<f64>` descriptor
(24 bytes on the tested 64-bit targets), with one additional allocation. It does
not multiply image storage by the measured 6.71x aggregate hierarchy tuple count.
`grouping_payload_bytes` is a separate borrowed-owner category in the complete
payload report; the numerical hierarchy's exclusive retained bytes still count
only its dense terminal factorization. Outer solver arrays remain unchanged.

## Arithmetic and certification

Selected levels use grouped MAP and grouped Gramian residuals. Fine PCG also
uses the grouped RHS and outer Gramian; fine LSMR uses the grouped weighted
adjoint. Stable tuple order, multiplication association, per-tuple MAP subtraction,
rounded middle vector, projections and Krylov recurrences are preserved. All
candidate and final certificates still use the original scalar tuple operator.
Batch execution retains independent scalar lanes and successful-prefix semantics.
No panel fusion, threading, automatic maps or terminal-policy changes occur here.

## Failure-path correction and gates

The stronger allocator fixture found two existing formatted numerical errors:
46 bytes from PCG's overflowing assembled RHS, then 100 bytes from the pinned
LSMR driver's unrepresentable residual norm. Prepared PCG now returns structured
`PcgNonFinite` / `PcgMetricBreakdown` data. The ordinary owning PCG API translates
those errors to its historical variant and text. A separately qualified narrow
within fork increment returns exact norm bits without formatting at the prepared
LSMR boundary; its owning wrapper likewise preserves the old diagnostic.
No new hot-path norm scan, successful arithmetic or numerical workspace is added.
The original failed logs are retained as development evidence.

Qualification includes existing recursive regression fixtures (not campaign
holdout seeds), every declared RHS width, all prefixes and both modes, changed
current weights, zero/mixed targets, exact output bits/reports/work and original
certificates. Gates also cover exact setup/retained/drop capacities, admission
before allocation, every reservation error/unwind, poisoned scratch, foreign
owners, dimensions, non-finite/overflowing input, successful batch prefixes and
recovery, plus lifetime compile-fail tests. Required Rust/Python checks and
release allocator/integration checks must pass before merge. Receipts follow.


The narrow norm correction is fork PR #3, merged at
`fad1d462d44e7d5b5226370021d69dab4e854669` after source `34071170642`
and PR `34071211981`/`34071211993` passed. Actual merged and reviewed trees
match. Both dependency pins use this revision. Local fork full Rust/release
allocation checks pass; downstream complete qualification is the next gate.


Local final Rust 1.85 format, strict all-target/all-feature Clippy, full and
minimal workspace tests, warning-free rustdoc and 73 Python validator tests
pass. Release all/minimal grouped integration and the full allocator executable
also pass. Two-transition scalar/row-gather PCG and LSMR use 34 and 40 setup
allocations; image mode uses 35 and 41. All first/repeated action, static failure,
RHS/metric/norm overflow and recovery windows allocate/deallocate/reallocate zero.
Fork post-merge `34071548162` passed. Exact-source/PR checks and the unchanged
v2 scalar regression run follow the source commit before this increment closes.


Source `d11917f152704973c673728d85b3eae1190f76b5` passed exact-source
`34071799728`/`34071799760` and PR `34071824688`/`34071824642` workflows.
The [scalar regression receipts](../benchmarks/results/2026-09-06/prepared-serial-m5d-scalar/README.md)
preserve all 2,268 Mac/Linux input/numerical/work/payload records exactly against
M5c, with full eligible-route certification and unchanged native negatives.
The first sandboxed local hardware inventory was blocked before any cases ran;
its retained log is separate from the complete successful measurements. Final
evidence-head CI/merge remains; grouped paired timing follows in M5e.
