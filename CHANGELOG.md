# Changelog

## Unreleased

### Added

- Weights-free prepared incidence topology with deterministic observation groups,
  borrowed owner bindings, fallible arrays and checked setup-payload admission;
  see `docs/ISSUE5_PREPARED_TOPOLOGY.md`.

- Checked fixed-MAP hierarchy payload inventories and strict prepared traced-PCG
  payload-budget admission; see `docs/ISSUE5_PAYLOAD_ADMISSION.md`.

- Caller-owned outer traced-PCG vectors, projection and bounded trace storage,
  with borrowed results and one shared recurrence; see `docs/ISSUE5_PCG_STORAGE.md`.

- Complete prepared recursive MAP-cycle scratch, including nested projection,
  MAP and dense-terminal modal storage; explicit reprepare and checked memory
  boundaries with isolated allocator regression gates on three platforms.
- Caller-owned recursive MAP hierarchy traversal workspace with checked
  retained-heap accounting, transactional output, and cross-instance reuse.
  Extended to nested operator scratch below; see `docs/ISSUE5_WORKSPACES.md`.
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
- Whole-system three-way Schwarz harness comparing diagonal, all-pair fixed CMG,
  and pinned `within` under modified LSMR and traced projected PCG.
- Frozen larger mixed-topology calibration with balanced, weak-chain,
  disconnected, and unbalanced three-way families.
- Explicit CMG terminal reasons, hierarchy depths, direct-factor state,
  warnings, fallback allocations, retained-memory boundaries, and separate
  certificate work.
- Frozen balanced issue-4 size ladder across three families, six component
  sizes, two outer solvers, two independent builds, and RHS prefixes through 32.
- Controlled recursive hierarchy harness that keeps the fine `within` smoother
  and three-way maps fixed while replacing only non-finest pair solvers.
- Automatic-map and already-revealed oracle-map coarse-CMG calibration modes.
- Permanent coarse-CMG evidence validator workflow with frozen eight-fixture
  coverage and ten adversarial regression tests.
- Archived issue-4 size-ladder and coarse-level summaries, raw-evidence hashes,
  dependency pins, run identifiers, artifact digests, and preserved negative
  results.
- Final issue #4 synthesis and ADR selecting `within` as the pair-local baseline
  and MAP as the preferred cheap smoother while retaining CMG as an explicit
  research comparator.
- Reusable `StructuralProjectionWorkspace` scratch with allocation-free
  structural-range projection and defect evaluation entry points.
- Caller-owned `rhs_from_targets_into` and `residual_into` incidence kernels;
  the existing allocating convenience methods now delegate to these primitives.

### Fixed

- Pair-adapter structural projection now uses component-local scaling to avoid
  intermediate overflow without erasing unrelated tiny disconnected components.
- Invalid and nonfinite pair-adapter calls clear output; reusable projection
  workspaces survive ordinary numerical/input failures.
- Compensated pair-marginal accumulation preserves representable small positive
  tuple masses alongside large weights.
- The pair-local dynamic-weight generator explicitly spans all prescribed
  powers from 1e-3 to 1e3 and has a coverage regression test.
- The coarse-CMG evidence validator now freezes the complete issue-3 fixture
  universe rather than inferring it from observed rows, so omission of an entire
  fixture is a hard failure.
- Coarse-CMG validation now rejects mixed/duplicate plan states and audits exact
  schema, hierarchy shape, case/build invariants, charged-time identities,
  prefix monotonicity, solver work units, certificate work, terminal coverage,
  fallback allocations, and symmetric baseline exclusion.

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
- Fixed CMG can produce genuine local and size-dependent pair-solver work
  reductions; the strongest balanced ladder point reduced LSMR work by about
  21.5 percent and PCG work by about 24.2 percent relative to `within`.
- That CMG crossover is nonmonotone, does not admit a stable pre-solve selector,
  and produced no fully charged finest-level win through 32 RHS.
- On weak-chain, path, disconnected, unbalanced, and dynamic-weight controls,
  pinned `within` often remained substantially stronger even where CMG was a
  major improvement over diagonal.
- Replacing only non-finest `within` pair solvers with CMG produced no material
  outer-work reduction on the controlled recursive oracle-map calibration; two
  timing wins both used more outer work.
- Issue #4 is complete for the current implementation with no CMG advancement:
  `within` remains the pair-local production-shaped baseline, MAP remains the
  preferred cheap smoother where admitted, and CMG remains an explicit research/
  control route.
- A fresh issue-4 holdout was deliberately not spent because no calibrated
  candidate met the joint material-work and fully-charged-economics gate.
- Complete lifetime memory, thread scaling, allocation-free workspaces, fused
  kernels, and changing-weight replay belong to issue #5. A material CMG
  economic improvement from that work must re-enter calibration under ADR 0002
  before it changes routing policy.

### Validation

- Rust 1.85 formatting, strict Clippy, all-feature and minimal-feature tests,
  and warning-free rustdoc.
- Dense Galerkin, kernel, component, rank, symmetry, positivity, permutation,
  and deterministic-repeatability checks.
- True original-Gramian residual checks for every accepted research solve.
- Issue #4 algebra, true-residual, complete-matrix, failed-attempt accounting,
  crossover arithmetic, extreme-scale projection, and invalid-input regressions.
- Whole-system and balanced-size evidence gates with exact work units, terminal
  metadata, zero fallback allocations, and independent residual certification.
- Coarse-level frozen-fixture, charged-total, prefix-monotonicity, solver-
  accounting, terminal-coverage, and baseline-admission regression tests.
- Exact-head and post-merge full Rust CI plus permanent issue-4 validator runs
  for the final coarse-level evidence.
