# Issue #3 complete-cycle holdout results

## Verdict

**The frozen v2 automatic-coarsening gate does not pass.**

The matrix uses the predeclared four-sheet hypergraph-cover fixtures and
policy in `benchmarks/policies/issue3-cycle-portfolio-v2.tsv`. The primary
bootstrap process still uses conservative Jacobi-compatible witnesses, while
final numerical acceptance measures the actual symmetric-MAP two-grid error
operator with a deterministic matrix-free energy power probe.

## Case matrix

| Case | Family | One-shot accepted | One-shot recovery | Primary accepted | Primary recovery | Portfolio accepted | Portfolio source | Portfolio recovery | Oracle κ | Portfolio κ | Probe factor | Exact error radius | Coarse tuples |
|---|---|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|
| cover-communities-seed-708 | cover-communities | False | 1.023 | False | 1.023 | False | `cycle-screened-rejected` | 1.023 | 7.686 | 4.017 | 0.751 | 0.751 | 238 |
| cover-communities-seed-709 | cover-communities | False | 1.009 | False | 1.009 | False | `cycle-screened-rejected` | 1.009 | 4.854 | 3.337 | 0.700 | 0.700 | 236 |
| cover-dominant-pair-seed-706 | cover-dominant-pair | True | 1.268 | True | 1.268 | True | `cycle-screened-bootstrap-final` | 1.268 | 1.634 | 1.361 | 0.265 | 0.265 | 478 |
| cover-dominant-pair-seed-707 | cover-dominant-pair | True | 1.306 | True | 1.128 | True | `cycle-screened-structural-baseline` | 1.306 | 1.737 | 1.389 | 0.280 | 0.280 | 476 |
| cover-latin-seed-700 | cover-latin | True | 1.437 | True | 1.437 | True | `cycle-screened-bootstrap-final` | 1.437 | 1.879 | 1.316 | 0.240 | 0.240 | 239 |
| cover-latin-seed-701 | cover-latin | True | 1.366 | True | 1.366 | True | `cycle-screened-bootstrap-final` | 1.366 | 1.695 | 1.297 | 0.229 | 0.229 | 236 |
| cover-nearly-nested-seed-704 | cover-nearly-nested | False | 0.943 | False | 0.943 | False | `cycle-screened-rejected` | 0.943 | 1.607 | 14.59 | 0.931 | 0.931 | 430 |
| cover-nearly-nested-seed-705 | cover-nearly-nested | False | 0.875 | False | 0.875 | False | `cycle-screened-rejected` | 0.875 | 1.839 | 30.28 | 0.967 | 0.967 | 418 |
| cover-weak-chain-seed-702 | cover-weak-chain | False | 1.543 | False | — | False | `cycle-screened-rejected` | — | 737.3 | — | — | — | 145 |
| cover-weak-chain-seed-703 | cover-weak-chain | False | 1.836 | False | — | False | `cycle-screened-rejected` | — | 1,067 | — | — | — | 148 |

## Aggregate diagnostics

- Accepted portfolio cases: **4 of 10**.
- Median accepted portfolio oracle-recovery fraction: **1.336**.
- Cases improving accepted one-shot recovery by at least 0.10: **0**.
- Exact oracle partitions selected by the portfolio: **0**. Exact partition recovery is diagnostic, not required when a different compact partition performs equally well.
- Maximum accepted true residual: `9.521e-11`.
- Maximum accepted two-level tuple complexity: `1.949`.
- Maximum positive exact-radius minus matrix-free-estimate gap: `2.870e-02`.
- Accepted portfolio source counts: `cycle-screened-bootstrap-final` 3, `cycle-screened-structural-baseline` 1.

## Scientific gates

- [FAIL] All ten supplied oracle maps pass structural, cycle, convergence, and residual checks.
- [FAIL] At least 8 of 10 portfolio cases are accepted.
- [PASS] Median accepted portfolio oracle-recovery fraction is at least 0.60.
- [FAIL] Portfolio improves one-shot recovery by at least 0.10 in at least 2 cases.
- [PASS] No accepted portfolio regresses more than 0.10 below an accepted one-shot map.
- [PASS] Every accepted PCG solve converges.
- [PASS] Every accepted true residual is at most 1.0e-08.
- [PASS] Every accepted map respects structural and tuple-complexity gates.
- [PASS] Maximum matrix-free probe underestimate is at most 0.030.
- [PASS] No candidate complete-cycle construction fails.
- [PASS] All selected-source labels are recognized.

## Interpretation

The matrix-free probe is used only after hard structural admission. It cannot
rescue an identity map, an over-large coarse space, or a map without sufficient
unique-tuple contraction. Conservative compatible relaxation remains valuable
for constructing signatures and repair witnesses, but the full fixed cycle is
the final numerical authority.

A passing result establishes feasibility on the frozen synthetic holdout, not
production runtime superiority. Large pair-solver comparisons, reusable
allocation-free state, and eventual fereg certification remain separate issues.

## Root-cause diagnosis

The negative result is not caused by invalid oracle maps, nondeterministic output,
failed solves, or a broken matrix-free probe. All ten oracle maps satisfy the hard
structural gates and reconstruct their declared coarse problems exactly. The two
independent executions are byte-identical.

The binding failure is the **predeclared one-sweep symmetric-MAP two-grid cycle**.
Four structurally valid oracle maps fail its frozen `0.50` estimated energy-factor
threshold:

| Case | Baseline MAP κ | Oracle two-grid κ | Exact cycle error radius | PCG residual |
|---|---:|---:|---:|---:|
| cover-weak-chain-seed-702 | 1514.343 | 737.267 | 0.998644 | 7.477e-11 |
| cover-weak-chain-seed-703 | 2139.640 | 1067.359 | 0.999063 | 3.325e-11 |
| cover-communities-seed-708 | 169.990 | 7.686 | 0.869890 | 9.161e-11 |
| cover-communities-seed-709 | 169.797 | 4.854 | 0.793987 | 9.970e-11 |

The weak-chain cycles remain extremely slow even with the correct coarse map; the
community cycles improve the baseline substantially but still miss the frozen
factor threshold. Every oracle PCG solve nevertheless converges to a recomputed
true residual below `1e-10`.

This is a second useful falsification after the preserved v1 result. V1 showed that
smoother-only compatible relaxation cannot be the final authority. V2 shows that a
single universal MAP complete cycle is also not robust enough for sparse graph-cover
fiber modes. The next policy must keep the same hard map gates and complete-cycle
authority while predeclaring a stronger smoother fallback before evaluating new
unseen seeds.

The v2 seeds `700`–`709` are now development evidence and must not be reused as an
unseen holdout.
