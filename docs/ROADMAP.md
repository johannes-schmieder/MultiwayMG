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

## Milestone 2 — complete oracle two-grid and multilevel feasibility gate

Tracked by issue #2. **Completed.**

### Quotient-space and stationary analysis

- [x] Materialize the complete numerical range of small singular Gramians.
- [x] Detect additional nullity beyond the two generic factor shifts.
- [x] Materialize fixed preconditioner actions and report full/quotient symmetry,
      range leakage, positive-action defects, and preconditioned spectra.
- [x] Form the explicit energy-coordinate stationary error operator.
- [x] Report spectral radius and induced energy norm separately.
- [x] Add one- and repeated-sweep stationary diagnostics.
- [x] Validate the exact pseudoinverse as the zero-error reference.

### Smoother and two-grid matrix

- [x] Compare three safe Jacobi damping values.
- [x] Add symmetric MAP/block-Gauss--Seidel.
- [x] Add exact dense pair Schwarz as a small-system quality ceiling.
- [x] Compare all-three-pair CMG.
- [x] Compare a selected pair with positive Jacobi and MAP backgrounds.
- [x] Add exact coarse-only correction and expose its semidefinite nature.
- [x] Add complete symmetric Jacobi, MAP, exact-pair, and pair-CMG two-grid
      cycles.
- [x] Show material coarse-correction improvement in all nine one-level
      families for every corresponding baseline.

### Multilevel and adversarial coverage

- [x] Add supplied-map smoother schedules using pair-CMG only on the finest
      level, on the first two levels, or on every level.
- [x] Add all-level Jacobi and all-level symmetric-MAP schedules.
- [x] Run exact two- through five-level resolution sequences.
- [x] Cover planted weak communities.
- [x] Cover one dominant factor pair with weak third-factor coupling.
- [x] Cover weak chains and nearly nested systems.
- [x] Cover Latin-square and rectangular tensor-grid patterns.
- [x] Cover hub/power-law degree structure.
- [x] Cover disconnected components with different local oracle depths.
- [x] Cover positive tuple weights spanning twelve orders of magnitude.
- [x] Keep tuple complexity below three and iteration spreads within two over
      every resolution sequence.

### Certification, cost, and reproducibility

- [x] Record a recomputed original-Gramian residual after every PCG iteration.
- [x] Record Gramian and preconditioner application counts.
- [x] Record exact coarse, pair graph, CMG, workspace, smoother, and terminal
      setup phases.
- [x] Report principal retained-memory and serial apply-scratch estimates.
- [x] Use exact CMG retained/workspace byte reports where available.
- [x] Execute deterministic matrices twice in CI and byte-compare the outputs.
- [x] Preserve raw matrices, residual traces, setup diagnostics, generated
      findings, and SHA-256 checksums in the repository.
- [x] Fail CI unless every predeclared scientific acceptance gate passes.

The final result is positive: a good hard factor-preserving coarse space supplies
the missing global three-way correction. Pair-CMG on only the finest level
captured almost all the benefit of pair-CMG on every level in the oracle
sequences, while all-level symmetric MAP was usually stronger and retained far
less state. These are hypotheses for production design, not routing rules.

See `docs/ISSUE2_METHODS.md`, `docs/ISSUE2_FINAL_RESULTS.md`, and
`benchmarks/results/2026-09-03/issue2-*`.

## Milestone 3 — adaptive coarse spaces

Tracked by issue #3. The diagnostic foundation is implemented; automatic repair
and oracle-gap closure remain active research.

- [x] Build a bounded sparse candidate graph from shared pair-marginal
      neighborhoods as a structural fallback.
- [x] Generate deterministic compatible test errors.
- [x] Define and validate the diagonal-energy compatible projection.
- [x] Measure compatible-relaxation contraction for proposed hard maps.
- [x] Record factor-block, energy, coarse-drift, and structural-defect histories.
- [x] Add explicit caller-supplied acceptance criteria with stable rejection
      reasons.
- [x] Compare oracle, current automatic, and deliberately misaligned maps on
      complete and parity-sparse refinements.
- [ ] Retain and expose the slowest compatible witnesses for repair.
- [ ] Score within-aggregate disagreement and candidate promotions.
- [ ] Split or promote bad aggregates under dimension and tuple budgets.
- [ ] Add bounded bootstrap slow-mode enrichment.
- [ ] Quantify the automatic-to-oracle gap in compatible contraction,
      two-grid spectra, Krylov work, setup, and hierarchy complexity.
- [ ] Add realistic sparse worker--firm--occupation and
      exporter--importer--product shaped holdouts.
- [ ] Evaluate energy correction before allowing richer interpolation.

## Milestone 4 — pair solver and production engineering

Pair-solver comparison is tracked by issue #4; reusable numerical state is
tracked by issue #5.

- [ ] Compare exact pair solves, the current `within` approximate-Cholesky
      local solver, and CMG on identical large pair subdomains.
- [ ] Determine whether pair-CMG is broadly useful, selectively useful, or only
      an oracle-quality reference after complete setup/application costs are
      charged.
- [ ] Use the issue #2 schedule findings to compare all-level MAP against
      finest-level-only pair-CMG on large systems.
- [ ] Add caller-owned allocation-free V-cycle and CMG workspaces.
- [ ] Add fused tuple kernels for multiple right-hand sides.
- [ ] Add deterministic parallel setup and application.
- [ ] Add exact production retained and peak memory reports.
- [ ] Add component-local hierarchy depths and terminals.
- [ ] Add prepared topology with changing numerical weights.
- [ ] Add failure injection and allocation-bound tests.

## Milestone 5 — fereg experiment

Tracked by issue #6.

- [ ] Add a private route for exactly three categorical intercept FEs in OLS.
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
