# Changelog

## Unreleased

### Added

- Initial `multiway-incidence` matrix class and exact hard coarsening.
- Matrix-free incidence, adjoint, weighted incidence, Gramian, energy, and dense
  reference kernels.
- Incidence components and projection of the two structural factor-shift modes
  per component.
- First symmetric three-way V-cycle with weighted-Jacobi smoothing.
- Scale-invariant rank-revealing dense spectral terminal.
- Exact shared-context and bounded pair-neighborhood aggregation.
- Deterministic adaptive hierarchy policy with per-level aggregation
  diagnostics.
- Pairwise CMG preconditioner and symmetric pair-CMG/coarse hybrid cycle.
- Projected PCG and rectangular modified-LSMR research drivers.
- Independent original-operator normal-equation residual certification.
- Numerical hardening tests for weight-scale invariance, disconnected kernels,
  arbitrary-input pair symmetry, exact Galerkin closure, and additional nested
  rank deficiency.
- Locked Rust 1.85 CI covering formatting, Clippy, full/minimal features,
  rustdoc, and release-mode feasibility probes.
- Planted and six-family feasibility executables plus committed raw evidence and
  interpretation.
- Recursive release-mode scaling probe through 768 coefficient coordinates and
  131,072 unique tuples, with repeated median solve timing and hierarchy
  complexity diagnostics.
- Difficult weak-chain scaling probe through 3,072 coefficient coordinates,
  designed to excite a slowly varying mode while comparing diagonal PCG,
  pair-CMG, the three-way V-cycle, the hybrid, and rectangular modified LSMR.
- Reference-counted immutable three-way problem state, making solver-level
  problem clones constant-time.
- Retained per-pair CMG RHS, solution, and cycle workspaces reused across
  preconditioner applications.
