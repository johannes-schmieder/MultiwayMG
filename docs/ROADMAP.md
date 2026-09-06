# Roadmap

## Milestone 0 — mathematical core

- [x] Deterministic tuple validation and duplicate collapse.
- [x] Matrix-free weighted incidence and Gramian kernels.
- [x] Incidence components and structural-kernel projection.
- [x] Exact factor-respecting Galerkin coarsening.
- [x] Scale-invariant rank-revealing reference terminal.

## Milestone 1 — first research solver

- [x] Stable weighted-Jacobi smoother.
- [x] Recursive symmetric three-way V-cycle.
- [x] Exact-context and pair-neighborhood structural aggregation.
- [x] Pairwise CMG corrections and symmetric hybrid cycle.
- [x] Projected PCG and rectangular modified LSMR.
- [x] Original-operator residual certification.
- [x] Locked Rust 1.85 CI and manufactured feasibility probes.

## Milestone 2 — oracle hierarchy and spectral feasibility

Tracked by completed issue #2.

- [x] Complete numerical-range decomposition.
- [x] Additional-nullity detection.
- [x] Full and quotient symmetry, range, positivity, and spectral diagnostics.
- [x] Exact stationary-error and two-grid operators.
- [x] Jacobi, MAP, exact pair-Schwarz, pair-CMG, and selected-pair comparisons.
- [x] Two- through five-level oracle resolution sequences.
- [x] Setup, apply, retained-memory, and scratch diagnostics.
- [x] True PCG residual histories and deterministic evidence checksums.

**Result:** the true three-way coarse space is valuable and can add decisive
global information beyond exact pair solves.

## Milestone 3 — automatic coarse spaces

Tracked by completed issue #3.

- [x] Exact diagonal-energy compatible projector.
- [x] Deterministic compatible-relaxation histories and quality criteria.
- [x] Relaxed-signature bootstrap matching.
- [x] Protected structural-baseline arbitration.
- [x] Compatible-witness split/promotion repair.
- [x] Matrix-free complete-cycle quality probing.
- [x] MAP-first, all-pair-CMG-fallback cycle portfolio.
- [x] Complete-cycle witness split repair.
- [x] Recursive hierarchy planning and supplied-map cycle construction.
- [x] Permutation, component, determinism, residual, and minimal-feature tests.
- [x] Frozen v1/v2/v3 and recursive evidence with preserved negative results.
- [x] Automatic-to-reference recovery and structural-baseline comparisons.

**Result:** bounded pair-neighborhood maps plus complete-cycle fail-closed
screening are the useful automatic method found so far. Bootstrap and repair did
not materially outperform that baseline and remain experimental. Recursive
structural maps are numerically promising but need production cost and memory
admission.

## Milestone 4 — pair-solver economics

Tracked by completed issue #4.

- [x] Component-local fixed-CMG adapter hosted by the generic Schwarz executor.
- [x] Public pinned `within` comparator without copied elimination/Cholesky code.
- [x] Exact rectangular operator, preconditioner, and certificate work counters.
- [x] Identical connected pair-domain harness for Jacobi, exact, CMG, and
      `within`.
- [x] Fail-closed residual, algebra, coverage, failure-cost, and accounting
      validators.
- [x] Explicit CMG terminal reasons, hierarchy depths, direct-factor state,
      warnings, and retained-memory boundaries.
- [x] Complete three-way Schwarz comparisons under modified LSMR and projected
      PCG on identical problems.
- [x] Larger mixed, disconnected, weakly coupled, unbalanced, and balanced
      calibration families.
- [x] Frozen balanced size ladder through 72 levels per factor and 32 RHS.
- [x] Controlled integration with the issue-3 recursive hierarchy, keeping fine
      `within` and three-way maps fixed while changing only non-finest solvers.
- [x] Automatic-map and revealed oracle-map coarse-only calibrations.
- [x] Permanent adversarial validator regression tests and archived provenance.
- [x] Determine the current CMG role: explicit research/control route, not an
      advanced production-shaped pair-local solver.
- [x] Decide not to spend a fresh holdout after no calibrated candidate met the
      joint work-plus-economics gate.

**Result:** current fixed CMG can reduce Krylov work on some larger balanced
pair domains, reaching about 21.5 percent under LSMR and 24.2 percent under PCG
at the strongest size-ladder point. The effect is nonmonotone, does not yield a
stable structural selector, and never produces a fully charged finest-level
win through 32 RHS. Replacing only non-finest `within` solvers with CMG produces
no material work reduction on the controlled recursive calibration. The pinned
`within` route remains the pair-local baseline; MAP remains the preferred cheap
smoother where the complete-cycle gate admits it; CMG remains an explicit
research comparator.

Complete lifetime memory, thread scaling, caller-owned workspaces, fused
repeated-RHS kernels, and changing-weight replay are production-engineering work
owned by milestone 5 rather than unfinished measurements of the current issue-4
candidate. They may materially change future economics. Any such CMG improvement
must be treated as a new candidate and re-enter calibration before routing or
holdout qualification.

See `ISSUE4_FINAL_RESULTS.md` and
`ADR_0002_ISSUE4_PAIR_SOLVER_POLICY.md`.

## Milestone 5 — production engineering and changing weights

Tracked by issue #5. **Current primary milestone.**

The accepted standalone CPU campaign is tracked incrementally in
[`DEVELOPMENT_PLAN.md`](DEVELOPMENT_PLAN.md) (M0–M10), with predeclared complete-
cost criteria in [`PERFORMANCE_PROTOCOL.md`](PERFORMANCE_PROTOCOL.md).

Already implemented: prepared fine topology, one-transition symbolic coarse/pair
maps, immutable numerical frames, compensated one-transition replay, complete
MAP-cycle and traced-PCG workspace primitives, and scoped payload admission.
These do not yet form a complete prepared solver or total lifetime memory report.
The checkboxes below describe complete integrated outcomes.
Standalone M1 adds [numerical acceptance guards](NUMERICAL_BOUNDARIES.md) before
prepared-operator integration; these are correctness improvements, not speed claims.
Standalone M2 adds [frame-bound operator views](ISSUE5_OPERATOR_VIEWS.md) sharing
the scalar kernels. [M3 pins reusable LSMR and mutable actions](ISSUE5_LSMR_DEPENDENCY.md)
from a narrowly qualified owner-controlled within fork. Complete current-frame
solver integration remains M4 work. Its [owning structural hierarchy and complete
weight replay](ISSUE5_PREPARED_HIERARCHY.md) now preserve one exact fine generation
across all supplied levels. [Prepared projection and MAP](ISSUE5_PREPARED_MAP.md)
now share scalar kernels with ordinary owners. The [prepared serial fixed
cycle](ISSUE5_PREPARED_CYCLE.md) adds a bounded terminal and complete application
scratch. [Prepared serial LSMR](ISSUE5_PREPARED_LSMR.md) now integrates the
original-operator certificate and bounded 1–32 RHS scalar reuse.
[Prepared PCG](ISSUE5_PREPARED_PCG.md) adds the same complete ownership/certificate
boundary. The [complete-cost serial benchmark](ISSUE5_SERIAL_BENCHMARK.md) now preserves
validated Mac/Linux evidence. The [certificate-gated LSMR
route](ISSUE5_CERTIFICATE_GATED_LSMR.md) addresses native stopping rejections
without restarting. The [three-route development comparison](ISSUE5_GATED_SERIAL_COMPARISON.md)
has complete PCG/gated-LSMR certification on Mac smoke/development and Linux
smoke, with native-control negatives retained. See [the evidence](../benchmarks/results/2026-09-06/prepared-serial-gated-v2/README.md)
for complete costs and limits. M4 is complete through PR #45; M5 starts with
[serial pass removal and scratch reuse](ISSUE5_SERIAL_SCRATCH.md).
Its frozen paired Mac evidence preserves numerical/work identity, reduces exact
scratch capacity and measures about 1.06x inner improvement on the small development
matrix. [Opt-in kernel profiling](ISSUE5_KERNEL_PROFILING.md) now supplies
validated attribution and level inventory for larger development inputs. Its
[four diagnostic artifacts](../benchmarks/results/2026-09-06/prepared-kernel-profile-v1/README.md)
pass complete accounting/certification and prioritize grouped MAP/Gramian trials.
Instrumented times are diagnostic; M5 remains open for measured layout work.

- [ ] Prepared tuple, pair-edge, and hierarchy topology.
- [ ] Shared candidate construction across smoother tiers.
- [ ] Caller-owned allocation-free cycle and Krylov workspaces.
- [ ] Fused multiple-RHS incidence and restriction/prolongation kernels.
- [ ] Exact retained and peak memory reports.
- [ ] Single- and multi-thread setup/apply/repeated-RHS accounting.
- [ ] Component-local hierarchy depths and terminals.
- [ ] Exact numerical replay under changing positive weights.
- [ ] Hierarchy quality invalidation and deterministic rebuild policy.
- [ ] Generation-safe caches that cannot silently mix incompatible weights.
- [ ] Preserve `within` as the pair-local baseline and MAP as the preferred cheap
      smoother while retaining CMG as an explicit comparator.
- [ ] Return any materially improved CMG implementation to issue-4-style
      calibration before changing routing policy.

## Milestone 6 — certified fereg integration

Tracked by issue #6. Outside the current standalone M0–M10 campaign.

- [ ] Private OLS route for exactly three categorical intercept fixed effects.
- [ ] Exact tuple collapse and bounded RHS blocks.
- [ ] Preserve fereg's observation-space certificate and fallback.
- [ ] Compare with MAP, CG, Schwarz-LSMR, and two-way CMG plus nuisance.
- [ ] Calibrate on common saved inputs and qualify on a fresh holdout.

## Milestone 7 — PPML and broader multiway generalization

- [ ] Replay symbolic maps with fresh numerical state for IRLS weights.
- [ ] Preserve PPML score, objective, line-search, and final-Hessian contracts.
- [ ] Explore `Q > 3` only after the three-way production path is understood.
