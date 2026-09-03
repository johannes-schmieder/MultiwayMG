# Roadmap

## Milestone 0 — mathematical core

- [x] Deterministic tuple validation and duplicate collapse.
- [x] Matrix-free weighted incidence and Gramian kernels.
- [x] Incidence components and known structural-kernel projection.
- [x] Exact factor-respecting Galerkin coarsening.
- [x] Dense scale-invariant rank-revealing reference terminal.

## Milestone 1 — complete first research version

- [x] Stable weighted-Jacobi smoother.
- [x] Recursive symmetric three-way V-cycle.
- [x] Exact shared-context aggregation.
- [x] Bounded pair-neighborhood fallback aggregation.
- [x] Deterministic adaptive hierarchy policy with per-level diagnostics.
- [x] Pairwise CMG smoother.
- [x] Symmetric pair-CMG plus coarse hybrid.
- [x] Projected PCG research driver.
- [x] Rectangular modified-LSMR driver and independent certificate.
- [x] Full-feature and minimal-feature GitHub Actions on Rust 1.85.
- [x] Planted and six-family release feasibility probes.
- [x] Raw first-stage evidence and interpretation committed to the repository.

## Milestone 2 — oracle hierarchy and quotient-space spectral gate

- [x] Materialize the complete numerical range of small singular Gramians.
- [x] Detect additional nullity beyond the two generic factor shifts.
- [x] Materialize fixed preconditioner actions and report full/quotient symmetry,
      range leakage, positive-action defects, and preconditioned spectra.
- [x] Add symmetric MAP and exact dense pair-Schwarz reference ceilings.
- [x] Construct two- through four-level manufactured oracle hierarchies.
- [x] Cover planted weak communities, Latin patterns, a weak chain, nearly
      nested structure, disconnected components, and a complete hierarchy.
- [x] Validate spectral predictions against projected-PCG iterations and true
      original-Gramian residuals.
- [x] Preserve a compact machine-readable matrix and detailed interpretation.
- [x] Establish that an oracle Jacobi hierarchy keeps condition numbers below
      about `1.46` in the first matrix.
- [x] Establish that the oracle pair-CMG/coarse hybrid keeps condition numbers
      below about `1.006` and converges in three or four PCG iterations.

The primary issue #2 go/no-go gate is passed. The near-unit oracle hybrid
spectra are deliberately treated as an idealized ceiling because the current
refinement expands every parent tuple into a complete child tensor.

## Milestone 3 — adaptive coarse spaces

Tracked by issue #3.

- [x] Build a bounded sparse candidate graph from shared pair-marginal
      neighborhoods as a structural fallback.
- [ ] Generate deterministic relaxed test vectors suitable for hierarchy setup.
- [ ] Define and validate compatible projection in a weighted norm.
- [ ] Measure compatible-relaxation contraction for proposed hard maps.
- [ ] Build richer sparse same-factor candidate graphs from pair marginals and
      compact neighborhood sketches.
- [ ] Rank candidates by test-vector affinity, predicted tuple contraction, and
      energy inflation.
- [ ] Split or promote bad aggregates.
- [ ] Add bounded bootstrap slow-mode repair.
- [ ] Add sparse and adversarial oracle refinements and deliberately imperfect
      maps to quantify the automatic-to-oracle gap.
- [ ] Evaluate energy correction before allowing richer interpolation.

## Milestone 4 — pair solver and production engineering

Pair-solver comparison is tracked by issue #4; reusable numerical state is
tracked by issue #5.

- [ ] Compare exact pair solves, the current `within` approximate-Cholesky
      local solver, and CMG on identical pair subdomains.
- [ ] Decide whether pair-CMG is broadly useful, selectively useful, or only an
      oracle-quality reference after complete setup/application costs are
      charged.
- [ ] Caller-owned reusable V-cycle and CMG workspaces.
- [ ] Fused tuple kernels for multiple RHS vectors.
- [ ] Deterministic parallel setup and application.
- [ ] Exact retained and peak memory reports.
- [ ] Component-local hierarchy depths and terminals.
- [ ] Prepared topology with changing numerical weights.
- [ ] Failure injection and allocation-bound tests.

## Milestone 5 — fereg experiment

Tracked by issue #6.

- [ ] Private route for exactly three categorical intercept FEs in OLS.
- [ ] Collapse weighted RHS values by tuple.
- [ ] Preserve fereg's original observation-space certificate and fallback.
- [ ] Benchmark against MAP, CG, Schwarz-LSMR, and two-way CMG plus nuisance.
- [ ] Calibrate only after a frozen matrix and fresh holdout.

## Milestone 6 — PPML and generalization

- [ ] Replay symbolic aggregate maps under changing IRLS weights.
- [ ] Rebuild all numerical coarse state and pair CMG state per generation.
- [ ] Add quality invalidation for stale aggregation.
- [ ] Explore const-generic `Q > 3` operator support only after the three-way
      method is understood.
