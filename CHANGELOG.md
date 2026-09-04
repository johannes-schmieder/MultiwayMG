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
  using a 96-coordinate spectral terminal and designed to excite a slowly
  varying mode while comparing diagonal PCG, pair-CMG, the three-way V-cycle,
  the hybrid, and rectangular modified LSMR.
- Reference-counted immutable three-way problem state, making solver-level
  problem clones constant-time.
- Retained per-pair CMG RHS, solution, and cycle workspaces reused across
  preconditioner applications.
- Dense complete-range decomposition for small singular three-way Gramians,
  including additional numerical nullity beyond structural factor shifts.
- Materialized-preconditioner diagnostics for full and quotient symmetry,
  range leakage, positive action, preconditioned eigenvalue intervals,
  condition numbers, and stationary energy radii.
- Symmetric MAP/block-Gauss--Seidel and exact dense pair-Schwarz reference
  preconditioners.
- Two- through four-level first-stage oracle hierarchy generators spanning weak
  communities, Latin-square patterns, weak chains, nearly nested structure,
  disconnected components, and complete weighted systems.
- Projected compatible-relaxation diagnostics, diagonal-energy coarse
  projection, deterministic map-quality matrices, and explicit acceptance
  criteria.
- Explicit energy-coordinate stationary error operators with spectral-radius
  and induced-energy-norm diagnostics.
- Exact hard-space coarse correction and complete symmetric two-grid cycles for
  Jacobi, MAP, exact-pair, and pair-CMG smoothers.
- Canonical selected-pair CMG research portfolios with positive background
  smoothers.
- Scheduled supplied-map hierarchies supporting Jacobi, symmetric MAP, and
  pair-CMG on selected levels.
- True PCG residual traces recomputed against the submitted Gramian after every
  iteration, with Gramian and preconditioner application counts.
- Pair graph, CMG, workspace, coarsening, smoother, terminal, and complete setup
  timing diagnostics.
- Principal retained-state and serial apply-scratch memory reports, using exact
  CMG byte reports where available.
- A nine-family one-level matrix covering weak communities, a dominant pair,
  weak chains, near nesting, Latin and tensor patterns, hubs, disconnected
  ragged hierarchies, and twelve orders of magnitude in positive weights.
- Exact two- through five-level resolution sequences comparing all-level
  Jacobi/MAP with pair-CMG on the finest, first two, or every hierarchy level.
- Deterministic CI byte-comparison gates, machine-readable raw matrices, full
  residual histories, generated final findings, and SHA-256 evidence checksums
  resolving the oracle feasibility milestone in issue #2.
