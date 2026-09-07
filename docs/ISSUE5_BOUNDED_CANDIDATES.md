# Bounded structural candidates (M6a)

M5's serial prepared layout/liveness work is complete through PR53. This first
M6 increment proposes a factor-respecting map from an exact current weight frame
using bounded flat arrays. It builds no coarse numerical frame, smoother or
dense factor. A returned candidate is explicitly unscreened; it is not an
accepted automatic hierarchy. The legacy automatic route remains available.

## Layout, order and admission

For each target factor, process the other two neighbor factors in increasing
order. Reuse one source-ID array, using u32 IDs when the maximum ID fits and usize otherwise,
sorted in place by (neighbor, target, original
tuple ID). Reduce duplicate pair masses in original canonical tuple order into
one flat array. At each neighbor retain at most the explicit degree cap k using
descending mass and canonical-ID ties, then emit level-pair proposals in ID
order. Selecting proposals never deletes an operator tuple or positive weight.

Each proposal contains two u32 IDs, a floating-point contribution and a usize
ordinal (24 bytes on the qualified 64-bit platforms). Sort by pair and ordinal,
then collapse in the same order as the finite legacy matcher. The ordinal avoids
an allocating stable sort and an unspecified floating-point order. Reuse the
contribution field for affinity; greedy matching retains its canonical ties.
Plain pair/overlap accumulation and normalization must stay finite. A valid
positive operator can overflow the sum of overlap contributions from two
marginals; this new candidate rejects explicitly instead of hiding infinity in
clamping. Complete original weights remain untouched, and the failure reports
work already performed. This is a declared stricter preparation boundary.

With E tuples, V coefficients, and M the largest factor count, target factor q
uses effective cap `kq=min(k,Vq)`. Each neighbor r supplies at most
`min(Vr*choose(kq,2), floor(E*(kq-1)/2))` proposals. Let Q be the largest sum
of these two bounds over q. On a 64-bit target, new arrays are:

| Array | Requested bytes | Lifetime |
| --- | ---: | --- |
| Three parent arrays | 4V | Retained structural result |
| Original tuple-ID permutation | 4E compact, 8E wide | Reused across all six marginals |
| Flat pair masses | 16E | Reused across all six marginals |
| Pair proposals with ordinal | 24Q | Reused across three factors |
| Mate IDs | 4M | Reused across three factors |
| Dense-parent validation | At most M | After flat scratch is dropped |

The complete conservative peak is borrowed topology + current frame + declared
other live owners + `4V + max((I+16)*E+24Q+4M,M)`, where I is the source-ID width. Checked byte/count arithmetic and
whole-stage admission precede the first new array reservation. Actual retained
parent capacities are reported separately. New excess capacity, inline roots,
stack/sort frames, allocator metadata and process RSS are not measured by this
payload bound. This is not an allocator quota. Even this flat proposal bound can
be expensive for a large k; automatic policy must admit its requested candidates
before trying them and charge any rejection.

`FactorAggregation::try_new` provides fallible dense-label validation; `new`
uses the same path. Invalid huge labels no longer size an array from an arbitrary
u32 value. A missing dense label must occur by the fine factor count, so validation
scratch can be bounded accordingly. Valid parent numbering and arithmetic are
unchanged. Partial validation/allocation errors publish no map.

## Scientific and engineering gates

Compare complete parent maps against the independent legacy tree implementation
on unit, dyadic and decimal weights; caps2/4/16 and affinities0/0.02/0.15/1;
ragged disconnected blocks, hubs, a Latin-square extra-nullity fixture and a
single tuple. Verify exact current-frame identity, deterministic reports, exact
Galerkin coarse tuples/weights and unmodified originals. A separate 65,536-tuple,
V=12,288 fixture checks finite legacy identity and a 49,152-proposal cap4 bound.
These are correctness tests, not timings or competitive evidence.

Private failure gates cover every one of the seven flat reservations, error and
unwind, full admission before the first reservation, integer/byte overflow and
recovery. Parent validation separately injects each of its three reservations.
The isolated allocator gate expects ten setup arrays, three retained parent
arrays, no hidden sort allocations/reallocations, zero allocations on insufficient
budget, and full release on numerical failure. Failed candidate errors retain
actual tuple/pair/proposal work and the requested peak when sizing succeeded.

Run required Rust1.85 checks and all Python validators, plus release scientific/
allocation gates, before source freeze. Then rerun the small complete prepared
solve regression and preserve all costs against M5h. No candidate-setup timing
or automatic/default-layout policy is selected at this checkpoint.

## Next construction stages

Append bounded structural transitions without rebuilding earlier levels; admit
coefficient/tuple complexity and complete live overlap before numerical setup.
Introduce component-local depths with actual removal of completed components
from deeper work, preserving factor labels and charging partition/gather/scatter
and any required topology copies. Cap dense terminals at256 coordinates per
component or use a separately admitted fixed sparse/Schwarz terminal. Build and
screen the actual recursive cycle bottom-up. No RHS-dependent inner stop may
enter a nominally fixed PCG preconditioner. These M6 stages are not implemented
by this candidate primitive. Calibration/holdout remain untouched.

Before source freeze, review reduced source tuple IDs from always-usize to
checked u32 when the largest ID fits (including E=2^32 on a 64-bit target). This
saves another 4E setup bytes on current inputs. A forced-wide small fixture
verifies identical visit order, and index-width/array-overflow boundaries reject
before reservation. Previous passing check logs are retained; repeat final
checks after this actual memory-layout refinement.

Final compact-index Rust1.85 format, strict Clippy, all/minimal workspace tests
and warning-free docs pass, together with all 90 Python evidence tests. Release
all/minimal complete solvers, candidate/allocator gates, private compact/wide and
reservation tests, fallible-parent tests and 120 actual layout-protocol
comparisons pass. M5h post-merge `34082605701`/`34082605728` passed. Commit
this source before the unchanged four-artifact v2 existing-solver regression;
all corresponding M5h numerical/work/payload records must match exactly. This
regression does not measure the new candidate builder or qualify an automatic
solver. Candidate setup timing follows complete builder integration.


Frozen source `2d67bc4` passes the [four-artifact existing-solver regression](../benchmarks/results/2026-09-07/prepared-layout-bounded-candidates/README.md):
7,920 complete processes and 72,300 certified measured columns. Every
corresponding M5h input/numerical/work/fingerprint/payload/layout record matches
exactly, with no process, rejection, timeout, RSS failure or measured retry.
Exact binaries/raw outputs and independently revalidated archives are preserved;
every archive member is rehashed. This supplied-map regression exercises shared
parent validation but does not time the new candidate stage or qualify an
automatic hierarchy. Source `34084223619`/`34084223515` and PR
`34084226689`/`34084226778` passed. Final evidence-head CI and PR54 merge
remain before incremental structural admission begins. M6–M10 remain open.
