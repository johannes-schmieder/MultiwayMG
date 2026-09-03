# Feasibility assessment

## Questions the first version can answer

1. Does exact factor-respecting coarsening preserve the matrix class in code?
2. Can a planted hierarchy be recovered from tuple contexts?
3. Is the resulting V-cycle numerically symmetric?
4. Does it serve as a usable preconditioner for projected PCG and rectangular
   modified LSMR on manufactured three-way systems?
5. Does pair-CMG add value beyond diagonal smoothing?
6. Does one three-way coarse correction add value beyond pair-CMG alone?

The GitHub Actions feasibility example prints iteration counts and independently
certified residuals for these alternatives.

## What positive manufactured results would establish

A successful planted experiment would establish that:

- the algebraic representation and Galerkin recursion are correct;
- the pair-CMG adapter is operational;
- an appropriate hard coarse space can remove slow global error;
- the hybrid can be used by a rank-robust rectangular Krylov method.

It would not establish that the current automatic aggregator is robust on real
worker--firm--occupation or exporter--importer--product data.

## Central unresolved risk

Automatic coarse-space construction remains the core research problem. The
first matcher only pairs same-factor levels sharing exact contexts in the other
two factors. That is effective for planted clone models but can be too strict
for sparse real data, where two levels may represent the same slow mode without
sharing an exact pair context.

The next setup methods should use relaxed test vectors, sparse candidate
neighborhoods, compatible relaxation, and bounded bootstrap repair. The
hierarchy must also reject levels that reduce coefficient dimension without
reducing unique tuple count.

## Other risks

- A hard piecewise-constant interpolation may have unacceptable energy
  inflation even with good aggregates.
- Three pair CMG hierarchies can cost more to build and apply than the current
  approximate-Cholesky Schwarz subdomains.
- Pairwise corrections may capture nearly all useful structure, leaving little
  benefit for a full hierarchy.
- Extra exact rank deficiency can make projected PCG unsuitable even when LSMR
  remains reliable.
- Weight changes in PPML may invalidate an aggregation learned in an earlier
  frame.
- Temporary allocation and serial tuple kernels currently obscure realistic
  performance.

## Go/no-go gates for the next research phase

Proceed to adaptive coarsening when CI demonstrates:

- exact dense Galerkin identity tests;
- structural-kernel preservation;
- numerical symmetry of V-cycle and hybrid;
- certified convergence on the manufactured hierarchy;
- an oracle or recovered coarse correction materially reducing Krylov work on
  at least one difficult planted family;
- bounded tuple and dimension complexity.

Do not integrate into fereg automatic routing until real-data-shaped holdouts
show an end-to-end advantage after setup, workspace, scatter/gather, and final
certification are charged.
