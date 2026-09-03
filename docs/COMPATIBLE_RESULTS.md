# Compatible-relaxation map-quality results

## Purpose

The oracle spectral milestone showed that a good factor-preserving coarse space
can close almost all of the global spectral gap left by pairwise solves. The
first issue #3 experiment asks whether projected compatible relaxation can
distinguish useful hard maps from maps that leave important smooth error outside
the coarse space.

The answer is **yes for the deliberately difficult weak-chain and nearly nested
controls**, with two important qualifications:

1. a very strong smoother can conceal some coarse-map defects; and
2. contraction quality must be balanced against coarse dimension and unique
   tuple complexity.

## Reproducible evidence

The authoritative matrix was produced by GitHub Actions workflow run
`33812037549` at commit
`36cb2791218fdc8efdf553140a89ede319705d9a`:

```bash
cargo run --locked --release -p multiway-mg \
  --example compatible_relaxation_matrix --all-features
```

The complete matrix is committed at
`benchmarks/results/2026-09-03/compatible-relaxation-matrix.tsv` and retained as
the `compatible-relaxation-output` workflow artifact.

Every experiment uses:

- 16 deterministic test errors;
- 12 homogeneous relaxation sweeps;
- diagonal-energy projection after every sweep;
- total and per-sweep diagonal and Gramian-energy contractions;
- defects normalized against the initial compatible test-vector norm; and
- exact component-preserving hard aggregations.

The six cases include complete and parity-sparse child refinements, weak global
chains, Latin-square patterns, nearly nested structure, and disconnected
components. The parity-sparse refinement retains a valid oracle parent map but
gives sibling levels complementary child contexts, making exact-context
matching substantially harder.

Maps compared:

- the known oracle parent map;
- exact shared-context matching;
- bounded pair-neighborhood matching;
- a deliberately misaligned component-preserving control.

Smoothers compared:

- weighted Jacobi with effective damping `0.5`;
- symmetric MAP/block symmetric Gauss--Seidel;
- fixed pair-CMG Schwarz.

## Projection correctness

Across every nontrivial row, the largest final coarse defect normalized by the
initial compatible norm was approximately `3.5e-17`, and the largest structural
shift defect was approximately `2.3e-17`.

The direct tests also verify:

- idempotence of the diagonal-energy complement projector;
- the `D`-norm Pythagorean identity;
- `P' D e = 0` after projection;
- component-local structural-shift removal;
- deterministic repeated reports; and
- rejection of aggregates crossing exact incidence components.

The final defects were deliberately normalized against the initial compatible
norm rather than the final norm. Some pair-CMG experiments reduce the error by
more than one hundred orders of magnitude; dividing roundoff-sized moments by
that vanishing final norm would produce a misleading large relative defect.

## Map recovery

### Pair-neighborhood matching recovered the oracle maps

The bounded pair-neighborhood matcher recovered the exact oracle partition in
all parity-sparse cases and in the complete planted case. It also reproduced the
oracle coarse tuple counts:

| Case | Fine dimension | Fine tuples | Oracle coarse dimension | Oracle coarse tuples |
|---|---:|---:|---:|---:|
| Planted complete | 24 | 512 | 12 | 64 |
| Planted parity-sparse | 24 | 256 | 12 | 64 |
| Weak chain | 48 | 116 | 24 | 29 |
| Latin square | 48 | 256 | 24 | 64 |
| Nearly nested | 48 | 512 | 24 | 128 |
| Disconnected Latin | 48 | 128 | 24 | 32 |

This is useful evidence for the current structural fallback, although the
manufactured sibling structure remains favorable and does not replace relaxed
mode information.

### Exact-context matching frequently stagnated

On the parity-sparse weak chain, Latin square, and disconnected Latin cases,
exact-context matching returned the identity map: coarse dimension and tuple
count equaled the fine problem, leaving no compatible complement.

On the nearly nested case it built a less aggressive map with coarse dimension
40 rather than 24. That map left only eight compatible directions and therefore
relaxed extremely quickly. This illustrates a central tradeoff: compatible
relaxation alone naturally favors a larger coarse space. Hierarchy admission
must also charge coarse dimension, tuple count, terminal cost, and cumulative
operator complexity.

## Conservative weighted-Jacobi screening

The worst diagonal contraction factor per sweep for the oracle maps was:

| Case | Oracle factor per sweep |
|---|---:|
| Planted complete | `0.527` |
| Planted parity-sparse | `0.550` |
| Weak chain | `0.606` |
| Latin square | `0.536` |
| Nearly nested | `0.526` |
| Disconnected Latin | `0.565` |

The deliberately misaligned weak-chain and nearly nested maps retained much
slower compatible error:

| Case | Oracle | Misaligned control |
|---|---:|---:|
| Weak chain | `0.606` | **`0.963`** |
| Nearly nested | `0.526` | **`0.948`** |

The corresponding worst energy factors were approximately `0.602` versus
`0.851` for the weak chain and `0.525` versus `0.799` for the nearly nested
case.

An explicit research criterion requiring a worst diagonal and energy factor no
larger than `0.75` therefore accepts the weak-chain oracle map and rejects the
misaligned map. This rule is an executable test of the decision machinery, not
a production threshold.

On the complete planted, Latin, and disconnected Latin cases, the misaligned
controls still relaxed reasonably quickly under Jacobi. This is not a failure
of the diagnostic: those particular maps do not leave a severely slow
compatible mode under that smoother. They are often much worse in tuple
contraction—for example, 192 versus 64 coarse tuples in the Latin case—and
should be disfavored by structural complexity rather than falsely rejected as
spectrally invalid.

## Smoother dependence

Compatible relaxation evaluates a **map–smoother pair**, not a map in isolation.

The pair-CMG smoother made oracle compatible error extremely small, with worst
per-sweep factors ranging from roughly `0.040` to `0.141`. It also damped some
misaligned maps strongly:

| Case | Oracle pair-CMG factor | Misaligned pair-CMG factor |
|---|---:|---:|
| Weak chain | `0.141` | `0.472` |
| Nearly nested | `0.054` | `0.484` |
| Latin square | `0.067` | `0.117` |
| Disconnected Latin | `0.100` | `0.105` |

Thus pair-CMG still exposes severe weak-chain and nesting errors, but almost
masks the disconnected Latin misalignment. Symmetric MAP displays the same
basic phenomenon with different strengths.

This supports a two-stage hierarchy policy:

1. use weighted Jacobi or another deliberately conservative fixed smoother to
   screen the intrinsic coarse-space quality;
2. separately measure the intended production smoother and complete cycle.

Using only the strongest available smoother as the acceptance test risks
admitting an unnecessarily poor or expensive coarse map because the smoother
already solves most of its complement.

## Deterministic decision API

The package now exposes explicit, caller-supplied
`CompatibleRelaxationCriteria`. It has no implicit production default.
Evaluation reports all failed criteria in stable order:

- worst diagonal factor per sweep;
- optional worst energy factor per sweep;
- final coarse defect;
- final structural-shift defect.

This separates measured diagnostics from routing policy. Thresholds must be
calibrated for the chosen smoother, sweep count, hierarchy level, and target
problem matrix before being used automatically.

## Current conclusion

Projected compatible relaxation is a viable quality diagnostic for hard
factor-preserving aggregation. It correctly identifies the deliberately missed
slow modes in the weak-chain and nearly nested controls, remains deterministic,
and preserves the required component and structural-kernel invariants.

The result also clarifies what the next repair algorithm must optimize:

```text
compatible contraction
    + coarse coefficient dimension
    + coarse unique tuple count
    + cumulative hierarchy complexity
    + setup and smoother cost.
```

A coarse-space algorithm should not simply minimize compatible contraction by
promoting many fine levels. It should seek the smallest hard factor-respecting
space that brings the conservative compatible-relaxation factor below an
explicit target while maintaining useful tuple contraction.

The next issue #3 slice should retain the slowest compatible error vectors,
attribute disagreement to individual aggregates and factor blocks, and perform
bounded deterministic split/promotion repairs. The oracle-to-automatic gap
should then be measured in both compatible contraction and complete V-cycle
spectra.
