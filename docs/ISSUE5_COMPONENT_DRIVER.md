# Terminal-first automatic component batches

M6h integrates the M6a–M6g prepared primitives into an explicit, one-shot,
exactly-three-way CPU driver. The `lsmr` feature exports
`solve_prepared_automatic_batch_into`, its batch/options types and a bounded
progress record. This is component integration; automatic-policy economics and
competitive qualification remain separate open gates. No default is promoted.

## Execution and acceptance

The caller supplies one immutable current weight frame, canonical column-major
targets, global coefficient outputs, one report slot per column, explicit
construction/screening/LSMR options, and a complete payload budget. K is 1–32.
Options are checked before output, report or progress mutation. The driver then
processes all K columns for one component before releasing its numerical state:

1. One-tuple components use the analytic minimum-norm value `y/3` in each factor.
   If every component is a singleton, no component permutation or inverse is
   constructed.
2. Components with at most 256 coefficients assemble a native column-major
   Gramian directly from original weights and recoded keys. They share the
   existing rank-revealing dense factorization and application arithmetic.
3. Larger connected inputs borrow their existing topology/frame. Disconnected
   large components materialize one canonical local root and fresh local frame.
   Roots consume their key array and reuse proved connectivity, avoiding a second
   union/find and copied-key input. Current weights move into the local frame.
4. An explicitly enabled hierarchy attempt respects depth, total tuple,
   coefficient and live-memory limits before numerical setup. It uses bounded
   candidates, one-transition provisional replay, full fresh replay, a bounded
   dense terminal and bottom-up screening of the actual recursive cycles.
   Rejected attempts release their storage before the chosen identity,
   inverse-diagonal or symmetric-MAP baseline is constructed.
5. Large routes share the existing certificate-gated LSMR recurrence and one
   reusable workspace across columns. A local solver certificate rejection also
   rejects that attempt.
6. After component storage and the temporary inverse have been released, each
   assembled column is projected and independently certified against the entire
   submitted operator. Rejected columns receive one full-problem baseline
   attempt. A component or initial certification error instead sends all columns
   through this final fallback. There are no timed decisions or tolerance retries.

`Ok(())` and progress stage `Complete` mean that every column has a completed
certificate, **not that every column was accepted**. Consume only a report with
`accepted == true`. Reports retain initial and final certificate values and
whether the final fallback ran. A returned execution error can leave partial
component coefficients or RHS scratch; unreported columns must not be consumed.
Static validation preserves all submitted outputs/reports/progress. Earlier
prepared scalar batch APIs retain their existing stronger failure contract.

## Memory and ownership

The bound includes the original topology/frame, all submitted target/output/report
slices, declared additional live caller storage, component permutations/inverse,
local roots and frames, candidate/build/replay scratch, current dense factors,
screen scratch, solver workspace and independent global certificates. Pass unused
caller capacity, retained old frames and unrelated live arrays through
`additional_live_payload_bytes`. Requested sizes are checked before fallible
reservation; retained capacities are checked after construction.

Only one component's numerical factor and Krylov workspace are live at once.
The small dense apply uses three fixed 256-element stack arrays (6 KiB), with no
per-column heap allocations. At dimension n, the pinned nalgebra 0.33.3 dense
factorization's requested heap peak is `8 * (2*n*n + 3*n - 2)` bytes. This follows
the matrix/tridiagonal/eigenvector lifetimes and is independently measured at
n=6,130,255,256. Its internal allocations remain infallible; the driver pre-admits
their requested payload but cannot convert process-level allocation failure into
a recoverable Rust error.

Array-payload admission is not a process RSS limit. Inline descriptors, fixed
stack scratch, error strings, allocator metadata/rounding and other runtime state
are excluded. Future complete-cost measurements must include total time and RSS,
including failed attempts. No forest of retained component factors is built;
persistent changed-weight policy remains M9.

Progress uses aggregate work/count fields and only the last typed rejection and
last rejected quality tail. It does not accumulate an unbounded per-component
event vector. Counters include failed candidate, screen and native/gated solve
work; hierarchy application counts do not expand internal traversal counts.
The maximum requested payload includes refused requests; maximum admitted payload
records admitted bounds and checked retained capacities.

## Qualification

Independent component-root and restricted-RHS references cover canonical recoding,
interleaved original IDs, nested factors, new positive weights, wrong owners,
length errors and inverse release. Private tests exercise every new root/frame
reservation with injected errors and unwinding, exact/one-byte-short admission,
overflow and recovery. Compile-fail examples enforce root/layout and frame/root
lifetimes.

Complete-driver tests cover K=1,2,4,8,16,17,32 prefixes, all-singleton input, dense
references, giant-plus-small disconnected input, one through three automatic
transitions, changed weights, depth/budget/screen rejection, rank-truncation
recovery, static-validation preservation, extreme numerical failure and explicit
nonconverged final certificates. Every accepted result is independently checked
against the original full problem.

`automatic_allocation_peaks` is an isolated, harness-free executable using pinned
test-only `allocation-counter` 0.8.1. Its positive controls distinguish cumulative
allocation from simultaneously live bytes and include realloc overlap. It runs
only serial measurements without nesting or unwinding through the counter.
Complete call setup, execution, fallback and drop are measured together; caller
arrays and fine input owners are prepared outside the measured region and added
to the bound. Tests require zero net retained allocations, bounded observed peaks
and identical allocation records across K and repeated calls on fixed routes.
Many-small and multiple-large disconnected controls check sequential lifetimes.
The existing `stats_alloc` executable remains separate. Both run in debug/release
and minimal/all-feature allocation CI on Linux, macOS and Windows.

The unchanged supplied-map regression checks shared-kernel compatibility and
does not measure this automatic driver. Full automatic-versus-baseline setup,
screen, rejection, solve and RSS economics are still required to close M6.
Parallel execution (M7), RHS panels (M8), persistent changing-weight policy (M9)
and competitive Mac/SCC qualification (M10) remain open. Calibration and the
campaign holdout have not been consumed.


All required Rust1.85 checks, 90 Python checks and seven release groups pass,
including 120 protocol comparisons. The [qualification records](../benchmarks/results/2026-09-19/component-driver-qualification/README.md)
preserve eleven debug/release-identical peak controls and 1,980 certified columns
per configuration. Self-review covers owner lifetimes, shared arithmetic,
admission/failure semantics and measurement scope; it is not external review.
Source freeze precedes the unchanged supplied-map compatibility regression.
