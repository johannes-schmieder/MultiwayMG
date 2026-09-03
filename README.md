# MultiwayMG

Experimental deterministic multilevel solvers for weighted multiway incidence Gramians.

The project targets sparse categorical designs whose rows contain one active level from each of several factors. Its first research target is the three-way operator

\[
G = B^\top W B,
\]

where every row of `B` contains exactly three ones. The repository will develop two related methods:

1. pairwise graph-Laplacian corrections, optionally powered by [`CMG`](https://github.com/johannes-schmieder/CMG); and
2. a structure-preserving three-way multigrid hierarchy.

The code is experimental and not yet suitable for production use.
