# Changelog

## Unreleased

### Added

- Initial `multiway-incidence` matrix class and exact hard coarsening.
- Matrix-free incidence, adjoint, weighted incidence, Gramian, energy, and dense
  reference kernels.
- Incidence components and projection of structural factor-shift modes.
- Symmetric three-way V-cycles with weighted-Jacobi smoothing.
- Scale-invariant rank-revealing dense terminals.
- Exact-context and bounded pair-neighborhood aggregation.
- Pairwise CMG preconditioning and symmetric pair/coarse hybrid cycles.
- Projected PCG, traced true residuals, and rectangular modified LSMR.
- Independent original-operator normal-equation residual certification.
- Dense complete-range decomposition and quotient-space spectral diagnostics.
- Symmetric MAP, exact dense pair-Schwarz, selected-pair, and scheduled oracle
  reference methods.
- Exact stationary-error and symmetric two-grid analysis.
- Two- through five-level oracle hierarchy generators and frozen issue #2
  feasibility matrices.
- Setup, apply, retained-memory, scratch-memory, and deterministic evidence
  diagnostics.
- Exact diagonal-energy compatible projection and deterministic compatible-
  relaxation histories.
- Explicit compatible-relaxation acceptance criteria and rejection reasons.
- Relaxed-signature bootstrap matching with bounded candidate neighborhoods.
- Protected pair-neighborhood structural-baseline arbitration.
- Monotone compatible-witness aggregate split/promotion repair.
- Matrix-free complete-cycle power probing and fail-closed cycle criteria.
- Cycle-screened map portfolios and recursive hierarchy planning.
- Selective symmetric-MAP-first, all-pair-CMG-fallback cycle screening.
- Distinct retention of learned and protected structural maps for fair cycle
  comparison.
- Complete-cycle witness-driven split repair with exact structural and
  improvement budgets.
- Validated supplied-map recursive cycle construction.
- Frozen issue #3 one-level and recursive policies, matrices, true-residual
  traces, gate status files, checksums, and preserved negative results.
- Final issue #3 decision record selecting the structural pair-neighborhood
  baseline plus complete-cycle screening while retaining bootstrap and repair as
  experimental diagnostics.

### Research conclusions

- Oracle factor-preserving coarse spaces can provide excellent multilevel
  conditioning beyond exact pairwise corrections.
- Complete-cycle quality, not smoother-compatible quality alone, must be the
  final hierarchy admission authority.
- A universal one-sweep symmetric-MAP cycle is insufficient on some weak-chain
  and weak-community graph-cover modes.
- The bounded pair-neighborhood matcher is a strong automatic baseline and
  composes recursively on the frozen matrices.
- Relaxed-signature bootstrap and both witness-repair schemes did not materially
  improve that baseline under the declared issue #3 gates.
- Recursive structural hierarchies are numerically promising but can exceed
  provisional cumulative tuple and dimension complexity budgets.

### Validation

- Rust 1.85 formatting, strict Clippy, all-feature and minimal-feature tests,
  and warning-free rustdoc.
- Dense Galerkin, kernel, component, rank, symmetry, positivity, permutation,
  and deterministic-repeatability checks.
- True original-Gramian residual checks for every accepted research solve.
