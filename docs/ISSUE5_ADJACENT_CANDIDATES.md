# Complete-row adjacent candidates (M6f)

The explicit `try_adjacent_pairs` policy addresses a coverage limitation of the
legacy top-k proposal set. Each complete neighbor row is sorted by descending
pair mass with canonical target-ID ties, then contributes disjoint adjacent
pairs. Odd rows leave their final entry unpaired. This changes the proposed
map, not the positive-weight operator. The existing top-k constructor and all
frozen automatic policies keep their original behavior. Neither constructor
returns a quality-qualified hierarchy.

## Order and bounded storage

Both policies share the M6a canonical pair-mass reduction, finite arithmetic,
proposal ordinals, ordered overlap collapse, degree normalization, threshold,
global per-factor greedy matching and fallible dense-parent validation. Every
paired target shares an exact incidence component. No factor boundary or
positive tuple is removed. The setup report records the explicit coverage enum.

For target factor q and neighbor r, the adjacent bound is
`min(floor(E/2), Vr*floor(Vq/2))`. Sum the two neighbor bounds and take the
largest target-factor sum Q: Q is at most E. The capped product is computed
with saturation before the finite minimum, which preserves this bound exactly
even when the uncapped product exceeds usize. The three target factors emit
at most 3Q contributions in total. No top-k neighbor truncation occurs.

The shared flat arrays and lifetimes remain as documented in
[M6a](ISSUE5_BOUNDED_CANDIDATES.md): three retained u32 parent arrays, source
tuple IDs, pair masses, proposals and mate scratch. Requested exclusive setup
payload is `4V + max((I+16)*E + 24Q + 4M, M)` on the qualified 64-bit platforms.
Borrowed topology/current frame and declared other live owners are charged
once before any reservation. This array bound excludes new allocator excess,
inline roots, sort stack, allocator metadata and RSS. It is not a process quota.

The unit Latin control with E=16,384 and V=384 illustrates the tradeoff:

| Quantity | Legacy k=4 | Adjacent pairs |
| --- | ---: | ---: |
| Selected pairs | 6 | 192 |
| Coarse coefficients after one map | 378 | 192 |
| Shared proposal capacity Q | 1,536 | 16,384 |
| Emitted contributions | 4,608 | 49,152 |

Adjacent candidates request 356,352 more scratch bytes in this control and
reach the 256-coordinate dense cap after one transition. This is a coverage
result, not a speedup: the full uniform Latin operator is already easy for MAP.
Latin has the two structural shift modes, not additional null directions.
Full setup, screening, depth, solve time and memory must justify extra candidates.

## Qualification and preserved negatives

An independent BTreeMap implementation checks complete parents on full tensors,
nested factors, disconnected nested-plus-singleton input, hubs and singletons,
with unit/decimal/dyadic weights and affinities 0/0.02/0.2/1. It independently
constructs rows, adjacent proposals and matching. Ragged odd-degree bounds,
compact/wide source order, exact/insufficient budgets, checked overflow, exact
frame identity, all seven reservation errors/unwinds and recovery are tested.
The isolated allocator gate checks both policies: ten setup allocations, three
retained parent arrays, zero reallocation/hidden sort arrays, allocation-free
budget rejection and exact release after numerical overflow.

End-to-end qualification deliberately forces up to two transitions even on
small problems; it does not implement the later terminal-first route. Successive
provisional frames generate new adjacent maps and agree with independent maps;
complete final replay agrees with provisional weights. Actual recursive screens
match independent ordinary suffix/full-history probes bit for bit on 62 tails
from 11 problems and two weight generations. These comprise eight historical
issue-3 fixtures, Latin-128, nested-plus-singleton additional nullity and a
single tuple. The campaign holdout is untouched.

The preselected development screen uses two starts, eight steps, tail length
four, damping one, maximum estimated factor 0.8, maximum observed factor 1.05
and maximum structural defect 1e-10. Seven of 22 problem/weight combinations
reject: both generations of each weak-chain fixture, both generations of the
depth-three nearly nested fixture, and the changed-weight nested-plus-singleton
control. The last includes a measured observed factor above one and normalized
structural defect above the gate. These rejections remain evidence; criteria
are not adjusted after seeing them. See the
[diagnostic records](../benchmarks/results/2026-09-07/adjacent-candidate-qualification/records.json).

Forced diagnostic solves run even after screen rejection and are explicitly
separate from automatic acceptance. All 66 random/manufactured/zero PCG solves
and 66 gated-LSMR solves certify against the original operator. A successful
solve does not override a rejected screen or prove spectral quality. There is
no default-policy or competitive promotion in this increment.

All required Rust1.85 formatting, strict Clippy, all/minimal tests and rustdoc
checks pass, with 90 Python validators. Release all/minimal numerical/full-solver/
allocator tests, private failure/boundary gates and 120 actual protocol checks
pass. The unchanged four-artifact supplied-map regression will check existing solver behavior; it
does not invoke or time adjacent candidate setup. Complete component-local
construction, charged failure/fallback, parallelism, RHS panels, changing-weight
policy and competitive calibration/holdout remain open.
