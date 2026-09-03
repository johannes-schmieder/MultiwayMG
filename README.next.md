# MultiwayMG

MultiwayMG is an experimental Rust package for solving large weighted **multiway categorical-incidence problems**, with an initial focus on regressions containing **three genuinely high-dimensional fixed-effect dimensions**.

The project exists because the exceptional graph structure available with two fixed effects does not carry over directly to three.

## The problem

Suppose every retained observation belongs to one level in each of three categorical factors. Examples include:

- worker × firm × occupation;
- exporter × importer × product;
- student × teacher × school;
- origin × destination × time when all three dimensions are large.

After collapsing observations with the same factor tuple, let `B` be the tuple-by-level incidence matrix. Every row of `B` contains exactly three ones—one in each factor block—and let `W` contain positive tuple weights. Absorbing the fixed effects requires repeated applications or solves involving

```text
A = sqrt(W) B
G = B' W B.
```

For **two** fixed-effect dimensions, a sign change converts the normal equations into a weighted bipartite graph Laplacian. That is why graph-Laplacian solvers such as [CMG](https://github.com/johannes-schmieder/CMG) can be extraordinarily effective.

For **three** dimensions, no choice of signs makes all three pairwise cross-blocks simultaneously Laplacian. The full Gramian is generally neither a graph Laplacian nor an SDDM matrix. Applying ordinary CMG directly is therefore mathematically incorrect. Generic alternating-projection or one-level Schwarz methods remain available, but difficult weakly coupled designs can require substantial iterative work.

MultiwayMG asks whether the special incidence structure can support a new solver that is both more general than graph CMG and more structure-aware than a generic sparse linear solver.

## Goal

The intended outcome is a reusable, deterministic numerical package that can solve or strongly precondition weighted three-way incidence systems with:

- work close to linear in the number of unique tuples per operator or multigrid cycle;
- a small, preferably problem-size-stable number of outer Krylov iterations on difficult structured problems;
- exact preservation of the multiway incidence class under coarsening;
- explicit handling of connected components, structural null spaces, and additional rank deficiency;
- reusable state for many right-hand sides;
- a prepared-topology path for changing positive weights, such as PPML IRLS;
- bounded, reported memory use and deterministic execution;
- true residual checks against the submitted operator.

This is a research objective, not yet a performance claim. A successful package should ultimately demonstrate an end-to-end advantage over mature alternatives on a predeclared collection of realistic three-large-FE problems, after charging setup, memory, workspaces, and certification.

MultiwayMG is **not** a regression estimator. It does not define samples, fit finite regressors, compute covariance matrices, or decide whether an econometric result is publishable. It supplies numerical operators and candidate solves to downstream software.

## Core idea

The first design combines two complementary levels of structure.

### 1. CMG solves the pairwise graph subproblems

Each of the three factor pairs—`(1,2)`, `(1,3)`, and `(2,3)`—does retain a bipartite graph-Laplacian representation after a sign switch. MultiwayMG forms those weighted pair marginals and can apply fixed CMG cycles as symmetric pairwise corrections.

These pair solves target error modes visible within two-factor interactions. They reuse CMG for exactly the matrix class CMG is designed to solve; MultiwayMG does not claim that the complete three-way system is itself a graph Laplacian.

### 2. A true three-way hierarchy captures global coupling

Let

```text
P = diag(P1, P2, P3),
```

where each `Pq` aggregates levels only within factor `q`, and every fine level has exactly one coarse parent. Then

```text
Gc = P' G P = (B P)' W (B P).
```

Every row of `B P` still contains one active coarse level in each factor. Fine tuples that map to the same coarse tuple can be merged by summing their weights. The coarse problem is therefore another weighted three-way incidence problem—not a general sparse matrix.

This exact closure permits a recursive, factor-preserving multigrid hierarchy. Pair-CMG or weighted Jacobi can smooth local/pairwise error, while coarse levels represent modes involving all three factors jointly.

## Relationship to CMG

[CMG](https://github.com/johannes-schmieder/CMG) is the lower-level solver for weighted graph Laplacians and SDDM systems. It remains an independent package with a precise graph-matrix contract.

MultiwayMG depends on CMG rather than extending CMG's public matrix class. In the current architecture:

```text
CMG
  ↑
MultiwayMG
```

CMG contributes:

- pairwise Laplacian hierarchy construction;
- fixed linear CMG preconditioner cycles;
- graph component and null-space handling;
- reusable graph-solver infrastructure.

MultiwayMG contributes:

- the weighted three-way incidence operator;
- factor-respecting coarsening and coarse-tuple construction;
- the genuinely three-way hierarchy;
- composition of the three pair corrections;
- multiway rank, component, terminal, and hierarchy logic.

Improvements that are generally useful for graph Laplacians should remain in CMG. Algorithms specific to weighted multiway incidence Gramians belong here.

## Relationship to fereg

[fereg](https://github.com/johannes-schmieder/fereg) is the intended first downstream application. It is a high-performance regression package whose Rust backend absorbs high-dimensional fixed effects for Stata and, potentially, other front ends.

The intended dependency direction is:

```text
CMG
  ↑
MultiwayMG
  ↑
fereg
```

A future fereg integration would use an exact pinned MultiwayMG revision. MultiwayMG would perform the numerical three-way solve; fereg would continue to own:

- observation ingestion and retained-sample construction;
- fixed-effect term selection and coding;
- outcome and regressor right-hand sides;
- solver routing, continuation, and fallback;
- finite-regressor estimation and covariance calculations;
- saved fixed-effect normalization and reconstruction;
- command-level memory admission and reporting;
- independent certification in the original observation-space fixed-effect operator.

That boundary is important. MultiwayMG convergence would create a candidate solution, but fereg's existing scientific certificate would remain the final authority.

The current two-way-CMG-plus-small-nuisance route in fereg should also remain preferred when the third factor is small. MultiwayMG targets the harder regime in which all three dimensions are genuinely high-dimensional and a dense nuisance Schur complement is no longer appropriate.

## Current stage

The project has completed its **first research MVP**. The repository currently contains:

- deterministic validation and collapse of repeated weighted triples;
- matrix-free `B`, `B'`, `sqrt(W)B`, and `B'WB` kernels;
- incidence-component discovery and structural-null-space projection;
- exact factor-respecting Galerkin coarsening;
- rank-revealing dense reference/terminal solves;
- weighted-Jacobi smoothing;
- pair-CMG corrections for all three factor pairs;
- a symmetric pair-CMG plus three-way coarse hybrid;
- projected PCG and rectangular modified-LSMR research drivers;
- deterministic structural aggregation prototypes;
- correctness, symmetry, rank, disconnected-component, and feasibility tests;
- release-mode manufactured experiments run in GitHub Actions.

The first experiments establish **mathematical and software feasibility**: the hierarchy can be built, its coarse operators remain in the intended class, CMG can be used as a pairwise smoother, and certified manufactured solves converge.

They do **not** yet establish production superiority. The current implementation still has research-grade setup costs and allocation patterns, and automatic coarse-space quality has not been validated on a sufficiently broad set of realistic three-way systems.

### Next research step

The next milestone is the oracle spectral-feasibility study tracked in GitHub issue **#2**. It will hold the coarse maps fixed at known-good values and measure:

- two-grid and V-cycle behavior on the quotient space;
- preconditioned eigenvalue ranges and convergence factors;
- the separate contributions of Jacobi, pair-CMG, and the three-way coarse correction;
- sensitivity to hierarchy depth, weak global coupling, nesting, disconnectedness, and weight heterogeneity;
- whether a good factor-preserving coarse space is intrinsically strong before further investment in automatic coarsening.

If oracle hierarchies work broadly, the following milestone is adaptive coarse-space construction using relaxed test vectors, compatible relaxation, and bounded bootstrap repair. If they do not, the project should revise the smoother or interpolation model before optimizing implementation details.

See [`docs/RESULTS.md`](docs/RESULTS.md), [`docs/FEASIBILITY.md`](docs/FEASIBILITY.md), and [`docs/ROADMAP.md`](docs/ROADMAP.md) for current evidence, limitations, and the staged research plan.

## Workspace

```text
crates/multiway-incidence
    Three-way tuple topology, components, matrix-free operators,
    structural kernel projection, and exact hard coarsening.

crates/multiway-mg
    Aggregation, terminals, three-way hierarchy, pair-CMG,
    projected PCG, modified LSMR, diagnostics, and experiments.
```

`multiway-incidence` intentionally has no dependency on CMG, fereg, or `within`. The numerical `multiway-mg` crate pins its external numerical dependencies through the workspace lockfile.

## Reproduce the current checks

The authoritative GitHub Actions workflow uses Rust 1.85 and runs the locked dependency graph:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked --workspace --no-default-features
cargo doc --locked --workspace --all-features --no-deps
```

The release-mode research probes are available under `crates/multiway-mg/examples/` and the recorded first-stage outputs under `benchmarks/results/`.

## Status

MultiwayMG is an experimental numerical research package. Its APIs, hierarchy policy, and benchmark conclusions may change substantially. It should not yet be used as a production regression solver or cited as demonstrating a general performance advantage.

The repository is licensed under GNU GPL version 3 only.
