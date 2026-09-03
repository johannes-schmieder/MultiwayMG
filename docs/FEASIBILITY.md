# Feasibility assessment

## Questions answered by the first version

The first implementation and its locked GitHub Actions matrix establish that:

1. exact factor-respecting coarsening preserves the matrix class in executable
   code;
2. the hierarchy preserves and projects the two structural factor-shift modes
   per incidence component;
3. the V-cycle and pair-CMG hybrid are numerically symmetric under their
   documented configuration;
4. the hierarchy is usable by projected PCG and by modified LSMR on the
   original rectangular weighted incidence operator;
5. pair-CMG removes difficult pairwise modes that diagonal smoothing leaves;
6. a three-way coarse correction supplies additional value beyond pair-CMG;
7. a deterministic pair-neighborhood fallback coarsens Latin-square structures
   on which exact shared-context aggregation stagnates; and
8. disconnected components and nesting-induced additional rank deficiency can
   be handled without weakening original-operator residual checks.

The exact results and raw evidence are in `docs/RESULTS.md` and
`benchmarks/results/2026-09-03/`.

## Numerical hardening completed

The first version includes tests proving:

- dense terminal rank decisions are invariant to a global rescaling of every
  positive tuple weight;
- a V-cycle advertised as symmetric rejects unequal pre- and post-smoothing
  sweep counts;
- pair-CMG projects both its input and output onto the known three-way
  structural range, making the exposed operator symmetric even for arbitrary
  submitted coefficient vectors;
- disconnected problems retain two structural shift directions per incidence
  component;
- mapped coarse tuples equal the dense Galerkin product;
- matrix-free Gramian applications equal dense references; and
- rectangular modified LSMR remains reliable when nesting creates rank
  deficiency beyond the two generic shifts.

These are correctness properties, not performance heuristics.

## First-stage verdict

The direction is **mathematically and algorithmically feasible**. The
structure-preserving hierarchy is not merely a conceptual analogy to graph
multigrid: it constructs valid recursive weighted three-way incidence problems,
combines with fixed CMG pair corrections, and produces certified solutions on a
varied manufactured matrix.

The strongest diagnostic is the weak-chain family. Diagonal PCG required 85
iterations, pair-CMG required 9, the three-way V-cycle required 6, and their
symmetric hybrid required 3. Across all six families, hybrid PCG and hybrid
modified LSMR required 1–4 iterations.

This is enough to justify the next research phase. It is not enough to claim a
production speed advantage.

## Central unresolved risk

Automatic coarse-space construction remains the core research problem. The
first adaptive policy combines two deterministic structural rules:

1. exact shared contexts in the other two factors; and
2. bounded shared neighbors in the two pair marginals when the first rule fails
   declared dimension/tuple progress gates.

This is substantially broader than the original clone matcher, but it still
does not inspect the slow error modes of the actual weighted operator. Two
levels may represent the same smooth mode even when their local neighborhoods
do not look similar enough to either structural rule.

The next setup method should use relaxed test vectors, sparse candidate
neighborhoods, compatible relaxation, and bounded bootstrap repair. It should
retain hard factor-respecting interpolation initially so exact tuple closure is
preserved.

## Remaining performance uncertainties

- The current problems are intentionally small; their wall times are dominated
  by setup, dense terminals, allocation, and runner noise.
- V-cycle and pair applications allocate temporary vectors and CMG workspaces.
- Tuple kernels are serial and solve one RHS at a time.
- Three pair CMG hierarchies may cost more to build and apply than the current
  approximate-Cholesky Schwarz subdomains in `within`.
- Pairwise corrections may capture most useful structure in some regimes,
  leaving too little gain to justify a full hierarchy.
- Hard piecewise-constant interpolation may have unacceptable energy inflation
  on real sparse designs even when it reduces tuple count.
- A symbolic aggregation learned under one PPML weight frame may become poor
  after weights change substantially.

## Gates before a private fereg integration

A private OLS experiment is justified after the package adds:

- larger sparse cases where dense-terminal and timer noise are negligible;
- explicit operator and preconditioner work counters;
- reusable caller-owned workspaces;
- worker--firm--occupation and exporter--importer--product shaped generators;
- direct comparison with the pinned `within` Schwarz/approximate-Cholesky route;
- memory accounting for retained pair hierarchies and V-cycle workspaces; and
- a fail-closed route that rejects poor hierarchy construction before solving.

Any fereg experiment must retain fereg's independent observation-space FE
certificate, normalization, memory admission, fallback behavior, and public
solver semantics.
