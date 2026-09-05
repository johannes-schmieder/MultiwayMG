# Issue 4: whole-system pair-solver results

> **Historical checkpoint.** This document records the whole-system calibration
> state on September 4, 2026. Issue #4 was completed on September 5, 2026 after
> the size-ladder and coarse-hierarchy experiments. See
> [`ISSUE4_FINAL_RESULTS.md`](ISSUE4_FINAL_RESULTS.md) and
> [`ADR_0002_ISSUE4_PAIR_SOLVER_POLICY.md`](ADR_0002_ISSUE4_PAIR_SOLVER_POLICY.md)
> for the final evidence synthesis and current policy. The “next experiments”
> below are preserved as the contemporaneous plan for this checkpoint.

Date: 2026-09-04. Status: calibration evidence, **not a fresh holdout and not a production routing decision**.

## Question

Issue #4 asks whether fixed CMG cycles are valuable as local solvers for the
three factor-pair subproblems, separately from the genuinely three-way coarse
hierarchy. Pair-local quality is not enough: the relevant question is whether
that local action reduces work and wall time after all pair corrections are
assembled on the complete three-way operator.

PR #14 added a deterministic whole-system harness that compares:

- diagonal preconditioning;
- all-pair component-local fixed-CMG Schwarz; and
- the pinned public `within` Schwarz comparator.

The same `ThreeWayProblem` and target vectors are used for every route. Modified
LSMR supplies exact weighted-incidence/adjoint work counts and an independent
normal-equation certificate. Traced projected PCG supplies original-Gramian
application counts and true residuals as a diagnostic cross-check. The issue #3
coarse hierarchy is deliberately absent from these experiments.

## Frozen smoke checkpoint

Implementation PR #14 was merged as
`12688bdf3b7985dd04daa8d62ad775508f7c02c5`. The exact reviewed source head was
`d5ae83435fc95c80983d5f9fc573d4e01b696f39`; GitHub Actions run `33934077763`
passed all six jobs. Whole-system artifact `9959561558` has ZIP SHA256
`40751a00e7e840bb395119e9ac4af5fb3c283e4b873836c9782512143f11f7b5`.

The smoke matrix contains 288 certified rows: six topology families, two builds,
three preconditioners, two outer solvers, and four RHS vectors. Maximum true
relative residual was `9.776001e-11`, there were no warnings, and sequential
fallback workspace allocations were zero. Every CMG case exercised multilevel
pair components.

At this first scale, pair-CMG used **more outer work than within on every case
under both outer solvers**. Median pair-CMG/within LSMR work ratios were:

| Case | Pair-CMG / within LSMR work | Pair-CMG / within PCG work |
|---|---:|---:|
| planted clones | 1.353 | 1.429 |
| noisy clones | 1.190 | 1.222 |
| Latin square | 1.211 | 1.250 |
| weak chain | 1.741 | 1.833 |
| disconnected Latin | 1.158 | 1.125 |
| unbalanced cycle | 1.324 | 1.375 |

Four-RHS setup-plus-solve timings also favored within in every case. CMG was
nevertheless much stronger than diagonal on the difficult weak-chain and
unbalanced systems. This establishes that the CMG action is numerically useful;
it simply did not beat the mature within local solver at this scale.

## Larger calibration profile

To test whether the smoke result was merely a small-domain artifact, the same
frozen harness was rerun with the calibration profile, which doubles its scale
parameter. This is a calibration continuation, **not an independent holdout**.
The fixed source base is the merged PR #14 commit above; the one-time calibration
source commit is `3f3b56c47b1f3f50cc88d1d313c1cd0a0e159f72`.

GitHub Actions run `33934212856` passed its validator, release build, larger
matrix, evidence gate, and artifact upload. Artifact `9959613244` has ZIP SHA256
`9223762bd466191ce282e499bd6cc606b62ca2d75314f33e1a3716350216f149`.
The byte-preserved files and `SOURCE.json` are under
`benchmarks/results/2026-09-04/issue4-whole-system-calibration/`.

The larger instances are:

| Case | Factor sizes | Unique tuples | Components |
|---|---|---:|---:|
| planted clones | 48 / 48 / 48 | 4,608 | 1 |
| noisy clones | 48 / 48 / 48 | 5,184 | 1 |
| Latin square | 48 / 48 / 48 | 2,304 | 1 |
| weak chain | 64 / 64 / 64 | 380 | 1 |
| disconnected Latin | 48 / 48 / 48 | 1,152 | 2 |
| unbalanced cycle | 192 / 96 / 24 | 1,536 | 1 |

All 288 rows again passed the numerical/accounting gate. Maximum true relative
residual was `9.896386e-11`; warnings and sequential fallback allocations were
zero.

### Scale-dependent outer-work result

The larger matrix rejects a simple conclusion that pair-CMG is always inferior.
For the balanced, dense-coupling families, pair-CMG now reduces outer work:

| Case | Pair-CMG / within LSMR work | Reduction vs within | Pair-CMG / within PCG work |
|---|---:|---:|---:|
| planted clones | 0.809 | 19.1% | 0.784 |
| noisy clones | 0.852 | 14.8% | 0.871 |
| Latin square | 0.852 | 14.8% | 0.833 |
| weak chain | 2.276 | worse | 2.417 |
| disconnected Latin | 1.211 | worse | 1.250 |
| unbalanced cycle | 1.513 | worse | 1.618 |

The planted-clone LSMR result is very close to the issue's provisional 20%
outer-work target, while its PCG reduction exceeds 20%. The sign reversal from
the smaller smoke matrix is important evidence that component scale/topology can
matter materially.

It is equally important that this **does not yet make CMG economically
competitive**. Four-RHS setup-plus-solve timing still favored within on every
larger case. For LSMR, within/CMG charged timing ratios ranged from roughly
0.34 to 0.92; a value above one would be required for CMG to win. Even where
CMG saved Krylov work, its extra setup/application cost more than consumed the
saving at four RHS.

The controls also prevent over-interpreting those balanced cases. At this
scale, diagonal matches or beats CMG's outer work on planted clones and Latin
square, while within remains the fastest fully charged route. By contrast CMG
massively improves over diagonal on weak-chain and unbalanced designs, but
within improves still further. There is therefore no current evidence for a
simple scalar "difficulty" threshold that routes hard cases to CMG.

## Interpretation

The combined pair-local, whole-system smoke, and larger calibration evidence now
supports a narrower hypothesis:

1. **Universal finest-level pair-CMG is not supported.** It is slower than within
   in every fully charged whole-system experiment so far and can increase outer
   work dramatically on weak-chain or unbalanced structures.
2. **A selective size/topology effect is real enough to investigate.** Larger
   balanced dense-coupling cases cross from higher to lower outer work, reaching
   roughly a 15–21% reduction, although not yet an end-to-end win.
3. **CMG remains a strong research control against diagonal.** On weakly coupled
   or unbalanced cases it can remove most of diagonal's Krylov work even when
   within remains superior.
4. **The coarse-level role is increasingly plausible.** After three-way
   coarsening, pair systems are smaller and their topology changes. The right
   experiment is therefore not necessarily to put CMG on every finest-level
   pair, but to compare within on the fine level with fixed CMG on selected
   coarser levels while keeping the accepted issue #3 map/cycle policy frozen.

## Next experiments

Before any routing rule is proposed:

- extend pair-component reports with explicit CMG terminal reasons and direct
  terminal flags, rather than using level count alone;
- run a size ladder on the balanced planted/noisy/Latin families to locate
  whether the outer-work crossover is stable and whether setup amortizes at
  larger components or more RHS vectors;
- test selected-pair and selected-component portfolios using only pre-solve
  structural quantities;
- add the unchanged issue #3 coarse hierarchy back and separately attribute
  gains to fine local solves versus the true three-way coarse correction;
- measure multi-thread scaling and complete solver-lifetime memory;
- freeze any proposed selective policy before a fresh holdout.

The issue #4 advancement criterion remains unchanged: a broad, structurally
interpretable regime should deliver both a material outer-work reduction and
positive fully charged economics. Current evidence has reached the first
criterion only narrowly and only in calibration cases; it has not reached the
second.