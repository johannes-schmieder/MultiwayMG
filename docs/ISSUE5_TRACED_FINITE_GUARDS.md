# Issue 5: fail-closed traced-PCG diagnostics

## Confirmed failure

The shared traced-PCG driver at `453227e` could return a converged zero solution
at iteration zero for an all-NaN RHS. Its norm routine reduced absolute values
with `f64::max`, then returned zero when the resulting scale was zero. Without a
finite-value guard, all NaNs could reach that shortcut. Both ordinary and
workspace-backed entry points had the same behavior because they share one
core; the flaw preceded the workspace dispatch refactor.

Tests-only head `7df0e7a` reproduced this in Rust 1.85 Actions run `33961838512`,
after formatting and strict Clippy passed. The failed assertions printed an
actual successful result with zero residual, zero operator applications,
`converged: true`, and `rhs_projection_norm: NaN`.

## Repair and boundaries

Both projected-RHS and recomputed-residual norms now validate every input value
before invoking the unchanged norm arithmetic. An unrepresentable norm returns
`PcgBreakdown` with iteration context. The RHS projection norm, effective
stopping tolerance, relative residual, and final returned solution values are
also checked. Consequently an overflowed diagnostic cannot authorize convergence
or escape in a successful traced result.

These checks add explicit linear scans over vectors. They do not change the
valid finite arithmetic, recurrence, preconditioner dispatch, operator counts,
trace order, tolerances, or workspace ownership. There is no performance claim
and no routing change. Finite inputs whose diagnostics overflow may now be
rejected earlier; this is intentional fail-closed behavior, not a new scaling
algorithm for extreme-range projection arithmetic.

Non-finite initial states and unrepresentable initial diagnostics are rejected
before any preconditioner application, leaving an empty hierarchy workspace
empty. Subsequent valid solves can reuse it. Both public traced APIs retain one
shared core. This change does not redesign the separate untraced PCG driver or
replace original-operator residual certification.

Regression tests cover NaN and positive/negative infinity in all or one RHS
entry, workspace non-mutation and subsequent reuse, overflowing initial norms,
projection diagnostics and stopping tolerances, and checked-norm bitwise
agreement on ordinary, zero, signed-zero, subnormal, and large finite vectors.
The existing hierarchy and PCG trace-equivalence tests and all permanent
scientific/evidence gates remain required under GitHub Actions.
