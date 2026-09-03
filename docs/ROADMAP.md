# Roadmap

## Milestone 0 — mathematical core

- [x] Deterministic tuple validation and duplicate collapse.
- [x] Matrix-free weighted incidence and Gramian kernels.
- [x] Incidence components and known structural-kernel projection.
- [x] Exact factor-respecting Galerkin coarsening.
- [x] Dense rank-revealing reference terminal.

## Milestone 1 — first solver prototype

- [x] Stable weighted-Jacobi smoother.
- [x] Recursive symmetric three-way V-cycle.
- [x] Shared-context automatic matching.
- [x] Pairwise CMG smoother.
- [x] Symmetric pair-CMG plus coarse hybrid.
- [x] Projected PCG research driver.
- [x] Rectangular modified-LSMR driver and independent certificate.
- [x] GitHub Actions and manufactured feasibility probe.

## Milestone 2 — evidence and diagnostics

- [ ] Freeze a broader synthetic family matrix.
- [ ] Record setup, apply, and Krylov work separately.
- [ ] Add two-grid error-propagation spectral diagnostics for small problems.
- [ ] Compare exact pair solves, current `within` approximate Cholesky, and CMG.
- [ ] Add adversarial nesting, disconnected, hub, and heterogeneous-weight cases.
- [ ] Decide whether pair-CMG is independently competitive.

## Milestone 3 — adaptive coarse spaces

- [ ] Generate deterministic relaxed test vectors.
- [ ] Build sparse same-factor candidate graphs from pair marginals and sketches.
- [ ] Rank candidates by test-vector affinity and predicted tuple contraction.
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
