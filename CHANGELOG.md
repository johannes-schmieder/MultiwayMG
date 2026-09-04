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
- Component-local fixed-CMG Schwarz adapter and a public pinned `within`
  comparator with setup phases, warnings, and explicit known-memory boundaries.
- Exact MLSMR incidence, adjoint, preconditioner, and certification work counts.
- Identical-domain pair-local economics harness with Jacobi/exact/CMG/within,
  repeated RHS prefixes, true residual certification, CMG terminal metadata,
  and conditional setup-amortization reporting.
- Permanent issue #4 GitHub Actions numerical/accounting smoke gate; timing
  results are descriptive, never preferred-winner pass criteria.

### Fixed

- Pair-adapter structural projection now uses component-local scaling to avoid
  intermediate overflow without erasing unrelated tiny disconnected components.
- Invalid and nonfinite pair-adapter calls clear output; reusable projection
  workspaces survive ordinary numerical/input failures.
- Compensated pair-marginal accumulation preserves representable small positive
  tuple masses alongside large weights.
- The pair-local dynamic-weight generator explicitly spans all prescribed
  powers from 1e-3 to 1e3 and has a coverage regression test.

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
- Issue #4 remains open: small pair-local economics are not broad three-way
  production evidence. Jacobi controls and direct/iterative terminal metadata
  must accompany any apparent CMG timing advantage.

### Validation

- Rust 1.85 formatting, strict Clippy, all-feature and minimal-feature tests,
  and warning-free rustdoc.
- Dense Galerkin, kernel, component, rank, symmetry, positivity, permutation,
  and deterministic-repeatability checks.
- True original-Gramian residual checks for every accepted research solve.
- Issue #4 algebra, true-residual, complete-matrix, failed-attempt accounting,
  crossover arithmetic, extreme-scale projection, and invalid-input regressions.
