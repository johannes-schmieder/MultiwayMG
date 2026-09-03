# First feasibility results

## Evidence snapshot

The first broad deterministic experiment was run in GitHub Actions on
September 3, 2026, using Rust 1.85.0 and release builds. The authoritative
commands are:

```bash
cargo run --locked --release -p multiway-mg \
  --example feasibility_matrix --all-features

cargo run --locked --release -p multiway-mg \
  --example scaling_probe --all-features
```

The raw outputs are committed at:

- `benchmarks/results/2026-09-03/feasibility-matrix.tsv`;
- `benchmarks/results/2026-09-03/scaling-probe.tsv`; and
- `benchmarks/results/2026-09-03/feasibility.txt`.

Every target vector in these experiments was generated exactly from a known
three-way coefficient vector. Projected PCG reports a residual recomputed with
the submitted Gramian. Modified LSMR reports an independently recomputed
normal-equation residual

```text
||B' W (y - Bx)|| / ||B' W y||.
```

## Six-family iteration summary

| Synthetic family | Aggregation used | Diagonal PCG | Pair-CMG PCG | Three-way V-cycle PCG | Hybrid PCG | Hybrid LSMR |
|---|---|---:|---:|---:|---:|---:|
| Planted clones | Exact context | 8 | 6 | 5 | **3** | **3** |
| Noisy clones | Exact context | 10 | 8 | 7 | **4** | **4** |
| Latin square | Pair neighborhood | 9 | 7 | 6 | **3** | **3** |
| Weak chain | Exact context | 85 | 9 | 6 | **3** | **3** |
| Nested third factor | Exact context | 8 | 2 | 3 | **1** | **1** |
| Disconnected Latin squares | Pair neighborhood | 8 | 7 | 6 | **3** | **3** |

All 30 solver/case combinations converged. The largest recorded certified
residual was below `9e-10`; the hybrid residuals ranged from approximately
`3e-16` to `2e-10`.

## Recursive scaling result

The recursive probe expands the planted family from 96 to 768 coefficient
coordinates and from 2,048 to 131,072 unique tuples. It uses the hierarchy's
first-class adaptive policy rather than supplying an oracle map.

| Coefficient coordinates | Unique tuples | Hierarchy depth | Tuple complexity | Diagonal PCG | Hybrid PCG | Hybrid LSMR |
|---:|---:|---:|---:|---:|---:|---:|
| 96 | 2,048 | 1 | 1.125 | 10 | **4** | **4** |
| 192 | 8,192 | 2 | 1.210 | 9 | **3** | **3** |
| 384 | 32,768 | 3 | 1.225 | 9 | **3** | **3** |
| 768 | 131,072 | 4 | 1.235 | 9 | **3** | **3** |

The adaptive hierarchy selected a mixture of exact-context and
pair-neighborhood levels. Iteration counts did not grow with hierarchy depth,
and cumulative tuple work remained tightly bounded: the sum of tuple counts
across all levels was at most 1.235 times the finest tuple count.

This is strong evidence that the recursive representation and adaptive fallback
remain numerically effective beyond one coarse level. It is not yet evidence of
production speed superiority.

### Current cost gap

On this easy planted family, the unoptimized hybrid was slower in wall-clock
solve time despite its lower iteration count. At the largest point:

```text
diagonal PCG setup       approximately   0.001 ms
diagonal PCG solve       approximately   6.425 ms
hybrid setup             approximately 224.755 ms
hybrid PCG solve         approximately  13.395 ms
hybrid modified LSMR     approximately  27.035 ms
```

The gap is expected from the current research implementation:

- three pair graph hierarchies are constructed;
- the three-way hierarchy uses allocation-heavy maps and dense terminal setup;
- `ThreeWayProblem` clones still duplicate substantial immutable state;
- V-cycle recursion allocates temporary vectors;
- pair-CMG allocates a workspace on every application;
- kernels are serial and solve one right-hand side at a time.

The scaling result therefore separates two conclusions that should not be
conflated:

1. **Numerically**, the hybrid has excellent recursive iteration behavior on
   this family.
2. **As currently engineered**, it is not competitive with diagonal PCG on an
   already easy problem.

The next performance experiments should include difficult scaling families,
such as weakly coupled chains, where the baseline iteration count grows and the
more expensive hybrid cycle has a plausible solve-time crossover.

## What the experiments establish

### The operator and hierarchy are viable

The exact mapped-tuple Galerkin construction works in executable code, not only
on paper. Both the three-way V-cycle and the pair-CMG operator act as usable
fixed preconditioners for the singular incidence Gramian. The same hybrid also
works in the rank-robust rectangular modified-LSMR path.

### Pair and coarse corrections contribute separately

Pair-CMG alone materially reduced iteration counts in every family, especially
the weak chain: 85 diagonal iterations fell to 9. The three-way V-cycle alone
required 6 iterations there. Their symmetric composition required 3. Similar,
though smaller, complementarity appears in the other families.

This supports the intended decomposition:

- pair graph solves efficiently remove errors visible in two-factor marginals;
- the structure-preserving coarse correction captures remaining global
  three-factor modes.

### A narrow exact-context matcher is insufficient

The exact shared-context matcher correctly recovered clone structures but made
no progress on Latin-square patterns. Bounded pair-neighborhood matching
coarsened both the connected and disconnected Latin cases from 72 to 36
coefficient coordinates and substantially reduced tuple counts. The hierarchy
now exposes a deterministic adaptive policy that tries exact contexts first and
uses this fallback only when the first candidate fails declared progress gates.

### Rank deficiency and disconnectedness are manageable

The nested-third-factor case has rank deficiency beyond the two generic factor
shift directions. Modified LSMR nevertheless converged and independently
certified. Separate tests also verify two structural shift directions per
incidence component and scale-invariant rank decisions in the dense spectral
terminal.

## What the experiments do not establish

The first matrix's sub-millisecond timings are not performance evidence. The
recursive probe is large enough to expose real setup and cycle costs, but it is
still one manufactured, highly coarsenable family on a hosted runner.

The experiments do not yet show that MultiwayMG is faster than:

- the current approximate-Cholesky Schwarz implementation in `within`;
- optimized MAP or CG absorption;
- two-way CMG plus a small dense nuisance Schur complement; or
- a future production implementation on real three-large-FE designs.

They also do not prove mesh-independent or problem-size-independent convergence
outside the tested families.

## Feasibility verdict

The direction has passed the first important go/no-go test. A true
factor-preserving three-way hierarchy can be constructed, its cycle can be made
symmetric, CMG can serve as a pairwise graph smoother, and the combined
preconditioner can reduce difficult manufactured systems to a handful of
Krylov iterations while preserving original-operator residual accuracy.

Recursive scaling strengthens that conclusion: automatic coarsening produced
four useful levels, bounded tuple complexity, and stable three-iteration hybrid
convergence at 131,072 unique tuples.

The remaining uncertainty is primarily **automatic coarse-space quality and
production cost**, not whether the mathematical composition is implementable.
The next decisive evidence should come from:

1. difficult large sparse families where the hybrid's iteration reduction can
   offset its more expensive cycle;
2. worker--firm--occupation and exporter--importer--product shaped generators;
3. direct comparison with `within` approximate Cholesky on identical pair
   subdomains;
4. shared immutable topology, caller-owned reusable workspaces, and fused
   multiple-RHS tuple kernels;
5. relaxed test vectors and compatible relaxation for cases where structural
   matching gives a poor coarse space; and
6. an eventual private `fereg` OLS integration retaining fereg's independent
   observation-space certificate.
