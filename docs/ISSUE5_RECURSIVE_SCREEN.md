# Bounded screening of actual recursive cycles

M6e probes the complete fixed recursive cycle at each numerical level, from the
terminal upward. It reuses the already constructed terminal factor and complete
cycle workspace. No one-level dense surrogate, repeated suffix factorization
or reconstructed hierarchy is used by the prepared screen. Options and criteria
are explicit; this increment does not select an automatic policy or claim a
competitive solver.

## Actual tail execution

`PreparedMapHierarchy::apply_tail_with_workspace` calls the same V-cycle recurrence
at the selected original level, with that level's operator-scratch suffix and
the existing terminal/image buffers. The full fine-result span provides a
transactional result prefix. Traversal storage after that full span covers every
dimension-nonincreasing suffix. No parent values live across separate calls, so
no arena-offset table or second traversal allocation is needed. Level zero uses
the existing root hot path unchanged.

Invalid levels, wrong dimensions, a different workspace owner and nonfinite RHS
values reject without publishing output. The same checks and finite-result
boundary apply on successful recursive execution. Private probe adapters use
that workspace's validated owner and existing projection/defect scratch. Shared
frame energy and Gramian arithmetic remain tied to the current numerical frame.

## Probe state and arithmetic

`PreparedCycleScreenWorkspace` owns two arrays: one arena of three fine-dimension
coefficient vectors and one bounded level-report array. It borrows the exact
immutable numerical hierarchy and takes a caller's complete cycle workspace
mutably only while screening. The cycle workspace can be reused after the screen
owner is dropped.

The power probe applies `I - damping*M^-1*G` to deterministic normalized range
starts. Its three vectors hold current error, gradient and correction/next error.
After projecting the correction, overwrite it with the next error using the same
fused multiply-add as the existing diagnostic. Retain gradient through the
Rayleigh dot, normalize next, and swap vector roles. Initial random starts reuse
the gradient vector. Existing deterministic fill, orientation, scaling, dot and
tail-geometric helpers are shared with the full-history diagnostic. The numerical
core uses the same compensated tuple energy and structural projections.

A fixed 64-f64 stack history preserves the diagnostic's ordered tail geometric
mean. No per-start histories or final witness vectors are retained. Explicit
options admit 1–16 starts, 1–64 iterations and a valid positive tail length;
finite positive damping/zero tolerance and finite valid criteria are required.
The count caps bound work and storage; they are not recommended automatic
settings. Even the historical 8-by-16 probe can cost too much for one RHS.
The later automatic policy must compare complete setup/screen/solve economics.

## Admission, failures and reports

Requested exclusive heap payload is `24*V_fine + sizeof(LevelReport)*levels`.
Before either fallible reservation, setup charges the complete hierarchy/cycle
payload inventory plus caller-declared other live state and both requested new
arrays. It checks actual retained capacities before publication. Old screens,
outputs and concurrent component/solver workspaces belong in caller other-live
state. The 512-byte factor history, other stack/inline roots, allocator metadata
and process RSS are outside this array-payload scope. Requested capacities do
not predict allocator-provided excess during construction.

Owner and option validation precede resetting reports or scratch. Reports are
stored bottom-up, with original level index, dimension, completed/annihilated
starts, worst tail estimate, maximum observed factor, final absolute Rayleigh,
structural defect and decision. Stop at the first failed quality criterion and
return `accepted=false`. The result borrows report storage; another screen cannot
overwrite a still-used result. The completed-prefix accessor preserves measured
levels after a later numerical failure. Invalid static input leaves the previous
admitted run's diagnostics unchanged.

A numerical failure returns a compact inline error, current level/start,
completed-tail count and attempted work. Proven level/start limits permit byte
indices rather than heap-boxing an error. Counters increment before entered
Gramian, complete-cycle, energy, explicit projection and defect calls, including
failed attempts. They do not expand internal cycle tuple work or measure elapsed
time. The driver must charge full attempted setup/screen/fallback time itself.
Nonfinite corrections, errors, energies, normalized values, Rayleigh values and
defects fail closed. Failure after the bounded 16 range-start attempts is explicit.
The original frame and fixed numerical hierarchy remain immutable and reusable.

## Qualification

Every recursive tail is compared with an independently constructed ordinary
suffix cycle, including random and zero RHS. Summary probes are compared bit for
bit with the existing full-history diagnostic at the same options, including
worst tail/observed factors, final Rayleigh, structural defect, annihilation and
Gramian/cycle/energy work. Tests cover all eight historical recursive fixtures,
a separate changed-weight generation, the explicitly rank-deficient nested
component case, terminal-only execution and five layout configurations. These
historical fixtures are not the untouched campaign holdout.

Private tests inject errors and unwinding at both reservations, test exact and
minus-one budgets and checked integer overflow, preserve old reports on invalid
inputs/owners, and recover after actual numerical failure. A 16-start, 64-step
configuration with damping 0.01 visits every history slot and checks its exact
cycle count. The isolated allocator gate checks two setup allocations, exact
retained/released bytes, no reallocations, and zero allocations for first/repeated
screening, quality rejection, numerical failure/recovery and malformed tail
calls. Borrow compile-failure tests enforce result lifetime.

Initial projection-adapter lifetime and Clippy findings are preserved in external
logs. All final required Rust1.85 checks and 90 Python validators pass, followed by
release all/minimal reference/full-solver/allocator/component tests, private
failure/budget/count-cap gates and all 120 actual protocol comparisons. Numerical quality probes remain
an empirical guard, not a proof over the full spectrum. Fixed-action mathematical
checks and every final original-operator certificate remain separate requirements.

## Next integration

Build component-local structural/numerical owners, use the one-transition
provisional constructor while extending maps, then screen these actual tails.
Qualify a bounded candidate policy that covers high-degree tied rows and admit
dense terminals per component or separately qualified fixed sparse terminals.
Charge failed constructions and screen results before baseline fallback. M6 and
M7–M10 remain open; scalar is still the default layout and no automatic-selector,
calibration, campaign-holdout or competitive claim is introduced here.


Frozen source `ba98d54` passes the [four-artifact supplied-map regression](../benchmarks/results/2026-09-07/prepared-layout-recursive-screen/README.md):
7,920 processes, 72,300 certified measured columns, 6,336 exact within-source
layout comparisons and exact agreement with every corresponding M6d input,
numerical, work, payload and layout record. No measured retries or failed gates
occurred. Source/PR workflows passed; copies and every archive member independently
revalidate. 8 derived timing geomeans differ under local recomputation within the existing archival-only relative tolerance of 1e-14. Receipts record every difference; raw numerical/certificate/work/payload comparisons remain exact.
Final evidence-head CI and PR58 merge remain. These measurements do not run
the new screen and cannot qualify its automatic construction cost.
