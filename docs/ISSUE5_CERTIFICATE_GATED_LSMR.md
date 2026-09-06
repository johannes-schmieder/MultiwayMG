# Original-certificate-gated prepared LSMR (M4d)

The [preserved serial v1 baseline](../benchmarks/results/2026-09-06/prepared-serial-v1/README.md)
shows that native modified-LSMR stopping can miss the requested original normal-
equation tolerance. Native and original metrics differ; accepting the native
flag would be incorrect. This increment provides a separate prepared route that
continues resumable native candidates when the original certificate rejects them.
The native route, its tolerances and its measured negative results stay available.

## Public boundary

`solve_prepared_least_squares_with_certificate_gate` uses an existing
`PreparedLsmrWorkspace` and returns borrowed coefficients plus a
`PreparedGatedLsmrReport`. The report contains:

- `solve`: the existing native diagnostics, final original certificate, separate
  `accepted` flag and native/final-certificate action counts;
- `gate`: candidate checks and vetoes, all outer projection attempts, and
  additional original-operator candidate-certificate actions.

Inspect `result.report.solve.accepted`. A successful function return can still
contain a rejected candidate. The scalar batch companion
`solve_prepared_least_squares_with_certificate_gate_batch_into` accepts 1–32
column-major RHS/output columns and reports. It uses the same static validation
helper as the native batch. Validate before mutation, retain completed prefixes,
leave failed/unprocessed output suffixes unchanged, and leave their reports at
`None`. Both `last_work()` and `last_gate_work()` retain the failed admitted
attempt. Static rejection preserves those inventories.

The workspace remains bound to one exact immutable hierarchy. Changed weights
need a newly prepared numerical owner/workspace. No hidden rebind, automatic
fallback, repeated solve from zero or tolerance change occurs in this route.

## Recurrence and acceptance

Both dependency pins name the qualified owner fork
`cb20b27a7137804202be39976415618686a144de` (fork PR #2), extending the M3 merge
`2e7d5ec`. Its optional `LsmrCandidateGate` runs before a resumable native
terminal audit, because that audit overwrites the active bidiagonalization vector.
A veto skips that destructive audit and continues the existing rotations, basis
and local reorthogonalization history. Escalation remains in its normal position.
The old allocating and serial APIs never invoke the optional gate.

MultiwayMG's gate copies the candidate into existing coefficient scratch,
projects structural factor-shift modes using its existing exact-owner projection
scratch, and certifies `||B'W(y-Bx)|| / ||B'Wy||` against the submitted fine frame.
A finite certificate above the declared tolerance vetoes the stop; invalid
arithmetic returns the original typed error. The extra work counters include
attempted actions even when projection, incidence or certificate scaling fails.
Counter overflow fails closed. No positive weight or numerical direction is
thresholded to force a pass.

Zero/initial-normal-zero exits, exhausted iteration budgets, exact breakdown and
escalation can terminate without a resumable gate call. Every returned native
candidate therefore receives a fresh, independent final projection/certificate.
Native diagnostics retain their meaning even when final acceptance is false.
The final audit is deliberately retained in this correctness increment; any
future reuse of a cached passing certificate needs its own lifetime argument
and measured verification.

## Work and memory

Native weighted incidence/adjoint/hierarchy work and the final certificate stay
in `report.solve.work`. Add `report.gate.candidate_certificate` when reporting
all certificate work. `projection_applications` counts all outer projections,
including the final one; hierarchy-internal projections remain part of hierarchy
work. Native solves have zero candidate checks/vetoes and also populate their
new `last_gate_work()` final-projection count. Existing native reports are unchanged.
The v1 benchmark's projection counter represents tracked PCG projections only;
LSMR final projection is charged in its elapsed solve phase, not that v1 counter.

The gate reuses the existing candidate, projection and certificate arrays. No
new vectors, history, shared ownership or thread pools are introduced. The
retained outer payload remains `8*(5E + 9V + 2kV) + 72C` plus complete hierarchy
scratch, with k=min(window,E,V). Inline counters/references are excluded from
array-capacity reports as before. Both MGS windows remain. In the permanent
heterogeneous two-transition/window-8 allocator case, native and gated workspaces
each reserve and release the same 40 arrays exactly (48 before M5a scratch reuse).

## Verification and remaining scope

The fork's source `17159ca9b52070c0ca67150551cb19521e7ed73b` passed local pinned
Rust 1.85 checks, debug/release all/minimal workspace tests, original upstream
bit comparisons and allocation gates. Source CI `34060520381`, PR workspace CI
`34060522557` and inherited CI `34060522625` passed, including three operating
systems, Python/coverage, MSRV and API compatibility. Fork PR #2 merged as
`cb20b27`; its post-merge workspace CI `34061005439` passed.

Downstream tests cover every supplied recursive regression case, cold/zero RHS,
fixed-configuration repeatability, exact candidate/final work, native route reuse,
and all widths 1,2,4,8,16,17,32. Continued coefficients match a native reference
run for the same uninterrupted iteration count bit for bit. Separate fork tests
also verify exact action counts, full warm-start candidate composition and
escalation after vetoes. Numerical/input/owner failures preserve typed errors and
completed batch prefixes; iteration limits still allow explicit rejection.

The permanent allocator executable covers first/repeated complete gated solves,
multiple vetoes, zero/mixed columns, selected failures and recovery without
allocations, and unchanged exact release. All local pinned Rust 1.85 checks and 46 Python validator tests passed.
Source/PR CI passed and PR #44 merged as `ebd9dd6`. The separately frozen
[three-route comparison](ISSUE5_GATED_SERIAL_COMPARISON.md) now qualifies
development coverage with all extra checks charged. M4 completed through PR #45. No v1 results, calibration seeds or campaign holdout are retuned.
