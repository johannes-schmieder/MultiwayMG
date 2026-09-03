# First feasibility results

## Evidence snapshot

The first broad deterministic experiment was run in GitHub Actions on
September 3, 2026, using Rust 1.85.0 and release builds at commit
`c9d3ded1e888d72b78c7564b2c34cadad4073af5`. The authoritative command was:

```bash
cargo run --locked --release -p multiway-mg \
  --example feasibility_matrix --all-features
```

The raw output is committed at
`benchmarks/results/2026-09-03/feasibility-matrix.tsv`. The smaller planted
probe is preserved beside it as `feasibility.txt`.

Every target vector in this experiment was generated exactly from a known
three-way coefficient vector. Projected PCG reports a residual recomputed with
the submitted Gramian. Modified LSMR reports an independently recomputed
normal-equation residual

```text
||B' W (y - Bx)|| / ||B' W y||.
```

## Iteration summary

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

## What the experiment establishes

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

## What the experiment does not establish

The sub-millisecond timings in the raw file are **not performance evidence**.
The problems are intentionally tiny, terminal solves are dense, temporary
vectors are allocated during every V-cycle and pair application, and GitHub
host timing noise is large relative to the measured intervals. Iteration counts
and certified residuals are the meaningful outputs of this stage.

The experiment does not yet show that MultiwayMG is faster than:

- the current approximate-Cholesky Schwarz implementation in `within`;
- optimized MAP or CG absorption;
- two-way CMG plus a small dense nuisance Schur complement; or
- a future production implementation on real three-large-FE designs.

It also does not prove mesh-independent or problem-size-independent convergence.
The six families were chosen to expose several different structures, but they
remain manufactured.

## Feasibility verdict

The direction has passed the first important go/no-go test. A true
factor-preserving three-way hierarchy can be constructed, its cycle can be made
symmetric, CMG can serve as a pairwise graph smoother, and the combined
preconditioner can reduce difficult manufactured systems to a handful of
Krylov iterations while preserving original-operator residual accuracy.

The remaining uncertainty is primarily **automatic coarse-space quality and
production cost**, not whether the mathematical composition is implementable.
The next decisive evidence should come from:

1. large sparse families where dense terminals are a negligible fraction of
   total work;
2. worker--firm--occupation and exporter--importer--product shaped generators;
3. direct comparison with `within` approximate Cholesky on identical pair
   subdomains;
4. caller-owned reusable workspaces and fused multiple-RHS tuple kernels;
5. relaxed test vectors and compatible relaxation for cases where structural
   matching gives a poor coarse space; and
6. an eventual private `fereg` OLS integration retaining fereg's independent
   observation-space certificate.
