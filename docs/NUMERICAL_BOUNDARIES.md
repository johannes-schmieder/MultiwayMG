# Numerical acceptance boundaries (standalone M1)

## Reproduced failure and resulting behavior

At baseline `989d72081bbe3eb1fb56db1cc920cd094595e312`, one tuple with
weights/target `(1e155, 1e154)` or `(1e200, 1e110)` produced finite LSMR
coefficient candidates and a native convergence flag but a NaN independent
normal-equation certificate. This was invalid certification, not evidence that
the fitted values themselves were wrong. The original weighted products can
overflow even when the weighted rectangular input is representable.

LSMR now rejects non-finite certificate inputs, residuals, weighted products,
accumulated gradients/references, norms and ratios with typed `NumericalFailure`.
A nonzero weighted product or residual ratio rounding to zero also rejects.
Exact zero reference and zero gradient retain a zero certificate; zero reference
with nonzero gradient cannot publish an infinite diagnostic as a certificate.
`converged()` remains the documented native flag. `is_certified(tolerance)` tests
the separate original-operator certificate and rejects invalid tolerances.
Finite certificate values above tolerance remain inspectable failed candidates.

The external recurrence, local reorthogonalization and reported native diagnostics
are unchanged. Successful native diagnostics are not relabeled independent
certificates. The guard is conservative: some valid finite problems return an
error because this original-space arithmetic is unrepresentable in f64. It does
not claim a new scale-invariant certifier or arbitrary-range solver.

## Adjacent audit

- Jacobi now enforces its documented open interval `0 < omega < 2/3`. Its scaled
  inverse diagonal must be finite and strictly positive. Tiny positive weights
  are not deleted; an unrepresentable preconditioner is rejected.
- Ordinary `ThreeWayProblem` construction now rejects overflowing weighted
  degrees, matching the prepared frame boundary. Unused levels retain their
  original distinct error.
- Dense terminal construction validates matrix, eigendecomposition, positive
  finite spectral threshold and retained inverse eigenvalues. Eigensolver setup
  has a 10,000-iteration limit, with the existing machine-epsilon convergence
  tolerance and rank threshold rule unchanged. Unrepresentable state returns a
  typed error instead of relying on a debug assertion or publishing bad factors.
- Ordinary untraced PCG now checks RHS/projection norms, stopping tolerance,
  recurrence beta, candidate solution and norms at acceptance/return. The traced
  workspace path already has dedicated finite guards and remains unchanged.

These constructor/driver checks do not make the low-level incidence and smoother
kernels checked-arithmetic APIs. Such kernels retain their explicit dimensional
contracts; finite inputs can still overflow in an application. Complete prepared
solves must certify and fail closed at their own numerical boundaries.

## Validation

`tests/numerical_boundaries.rs` exercises both reproduced LSMR failures, ordinary
and representable extreme controls, zero RHS, invalid acceptance tolerances,
Jacobi endpoints and inverse overflow/underflow, weighted-degree overflow,
dense extreme scales and ordinary-PCG invalid norms. Private certificate tests
exercise NaN/Infinity, zero denominator, product underflow and norm overflow.

Run the authoritative Rust 1.85 checks from `AGENTS.md`, including all/minimal
features, plus the permanent scientific and three-platform allocation CI.
This is a correctness milestone; no timing or competitive-performance claim.
