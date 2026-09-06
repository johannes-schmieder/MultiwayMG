# Reusable LSMR dependency (standalone M3)

Both `within` and `schwarz-precond` now name the owner-controlled fork
`https://github.com/johannes-schmieder/within` at
`2e7d5ec935b4846b369430ff00deb59f90e7d2d5` (fork PR #1). Its dedicated
`multiwaymg` branch begins at frozen upstream b7779cb; upstream main and its
later statistical-screen changes are preserved separately and not imported.

The fork adds caller-owned modified-LSMR buffers, mutable operator actions and
borrowed coefficient results. The new workspace driver uses serial internal
kernels and no implicit Rayon pool. Operator actions can own MAP/hierarchy
scratch without locks or interior mutability. All numerical state read by each
solve is initialized again; shape-compatible changed operators may reuse only
storage, never stale basis/recurrence/operator images. Local reorthogonalization
and native stopping/audit logic are unchanged. The allocating entry points share
one recurrence and retain their legacy parallel threshold behavior.

The new API supplies the outer storage boundary required for M4. It does not
itself attach a MultiwayMG hierarchy to an immutable numerical frame. That exact
owner validation remains the complete prepared solver's responsibility. Native
convergence remains a candidate; MultiwayMG's independent fail-closed certificate
is still required. Controlled parallel reductions and scheduling remain M7.

## Memory and integration

For m tuple observations, n coefficients and k=min(window,m,n), requested storage
is `8*(3m + 6n + 2kn)` bytes. Warm-start capacity is reserved even for cold solves.
Both MGS windows are retained. Construction and retained-capacity queries use
checked arithmetic; reservations are fallible. This scope excludes callers,
operators, MAP/hierarchy scratch, allocator overhead and process RSS.

The permanent MultiwayMG allocation executable composes an immutable frame
operator, actual caller-owned symmetric MAP scratch, forked LSMR workspace and
an independent original-operator certificate across 32 RHS. It exercises the
real dependency interface without introducing a new public automatic solver
route. Topology, numerical setup, RHS/certificate buffers and workspaces are
constructed before the measured calls; their costs must be charged in future
end-to-end economics. This is not a performance qualification.

## Qualification and immutable baselines

Fork source `e8a3561463018762e16396e3ee0f8af8699e40ff` passed:

- pinned local Rust 1.85 formatting, strict workspace Clippy, full all/minimal
  workspace tests, and warning-free rustdoc;
- exact-source workspace CI 34053279313 and PR workspace CI 34053282380, including
  Linux/macOS/Windows debug/release all/minimal allocation tests;
- inherited PR CI 34053282377: coverage floor, Python tests, platform tests, MSRV
  and public API compatibility.

Direct tests execute the original upstream crate at b7779cb as a development-only
comparison and match bits on the declared small full/rank-deficient, under/over-
determined, warm-start, iteration-cap, escalation and reorthogonalization cases.
An isolated n=12,000, window-8 first/repeat test measured zero internal solve
allocations and exact release of 2,400,000 bytes. All 11 reservation boundaries
have injected failure tests; invalid inputs reject before mutable actions and
subsequent valid solves recover after callback/computed-residual errors.

The original within performance baseline stays
`b7779cbab7a3116be56aae4389fde1f6e6a99a9f`, and the original MultiwayMG baseline
stays `989d72081bbe3eb1fb56db1cc920cd094595e312`. Future competitive comparisons
run them in separate executables as required by `PERFORMANCE_PROTOCOL.md`.
The current in-process `within` comparator now executes the pinned fork, so it
must not be described as the immutable upstream executable. Historical recorded
issue-4 evidence and ADR 0002 remain intact; unchanged small-case numerical
results do not establish unchanged complete setup or memory costs.

No copied elimination/Cholesky implementation, statistical-screen changes,
release publication, or fereg integration is included.
