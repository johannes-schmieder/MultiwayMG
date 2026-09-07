# Fused cycle prolongation (M5f)

M5e's full layout comparison is merged. This increment removes the separate
prolongation materialization and addition passes from the shared fixed V-cycle.
`FactorAggregation::prolong_add` performs `solution[i] += coarse[parent[i]]`
with the original factor offsets and coefficient-wise addition. Both dimensions
are validated before writes. Ordinary overwrite prolongation remains unchanged.
Prepared and ordinary MAP hierarchies use the same fused cycle recurrence;
projection, smoothing, residual order and all numerical checks are unchanged.

The exact logical vector-traffic reduction is one eight-byte temporary store
and one eight-byte temporary load per fine coefficient at each nonterminal level per cycle. The
residual is still needed for the following operator action, so retained arrays,
capacities and allocations do not change. These are removed source-level vector
accesses, not a measured DRAM-bandwidth reduction. The profiler now attributes
the addition to prolongation instead of exclusive cycle time. Original certificates, solver work
counts, output transaction boundaries and default layout remain unchanged.

Adversarial transfer tests compare bits to an independent two-pass materialized
reference for identity, ragged/relabelled, many-to-one and halving maps, signed
zeros, subnormal values and extreme finite values including overflow. Shape
failures leave output untouched. Complete recursive solver, changed-frame,
rank-deficient, poison/recovery and allocator contracts remain required.

Frozen source `a95c84e` now passes the unchanged five-layout policy on Mac
smoke/development/expanded and Linux smoke. All 7,920 processes and 72,300
measured columns certify, with no failed process, timeout or RSS violation.
Every input/numerical/work/fingerprint/payload/layout record matches M5e exactly.
See the [complete regression](../benchmarks/results/2026-09-06/prepared-layout-fused-transfer/README.md)
for exact receipts, full costs and preserved raw archives. These separately
collected regressions do not support an old/new timing claim. Final evidence-head
CI/review and merge remain at this checkpoint.
Traversal arena consolidation and target-scoped certificate-reference reuse
remain separate M5 audits. Prepared symbolic sorting and direct dense assembly
are already implemented; the remaining large setup cost informs M6 construction
and tuple-complexity admission.

All required local Rust1.85 checks and 88 Python tests pass. Release transfer,
complete hierarchy/solver and zero-allocation gates pass in all/minimal feature
configurations. These checks precede the measured source commit.
