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

## Milestone 2 — broader evidence and diagnostics

- [x] Freeze an initial six-family synthetic matrix.
- [x] Cover planted clones, noisy clones, Latin patterns, a weak chain,
      additional nesting-induced rank deficiency, and disconnected components.
- [x] Record setup time, solve time, Krylov iterations, and independently
      certified residuals.
- [ ] Add explicit operator, smoother, pair-CMG, and coarse-cycle work counters.
- [ ] Add two-grid error-propagation spectral diagnostics for small problems.
- [ ] Add hub, power-law degree, weak-community, and extreme-weight families.
- [ ] Add larger sparse scaling cases where dense-terminal and timer noise are
      negligible.
- [ ] Compare exact pair solves, current `within` approximate Cholesky, and CMG
      on identical pair subdomains.
- [ ] Decide whether pair-CMG is independently competitive after complete setup
      and apply costs are charged.

## Milestone 3 — adaptive coarse spaces

- [x] Build a bounded sparse candidate graph from shared pair-marginal
      neighborhoods as a structural fallback.
- [ ] Generate deterministic relaxed test vectors.
- [ ] Build richer sparse same-factor candidate graphs from pair marginals and
      compact neighborhood sketches.
- [ ] Rank candidates by test-vector affinity, predicted tuple contraction, and
      energy inflation.
- [ ] Add compatible-relaxation quality measurement.
- [ ] Split or promote bad aggregates.
- [ ] Add bounded bootstrap slow-mode repair.
- [ ] Evaluate energy correction before allowing richer interpolation.

## Milestone 4 — production engineering

- [ ] Caller-owned reusable V-cycle and CMG workspaces.
- [ ] Fused tuple kernels for multiple RHS vectors.
- [ ] Deterministic parallel setup and application.
- [ ] Exact retained and peak memory reports.
- [ ] Component-local hierarchy depths and terminals.
- [ ] Prepared topology with changing numerical weights.
- [ ] Failure injection and allocation-bound tests.

## Milestone 5 — fereg experiment

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
