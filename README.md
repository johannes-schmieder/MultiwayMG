# MultiwayMG

MultiwayMG is an experimental deterministic Rust workspace for solving weighted
multiway incidence problems. The first target is a three-factor categorical
design with one active level from each factor per tuple.

For a tuple incidence matrix `B` and positive diagonal weights `W`, the package
works with

```text
A = sqrt(W) B
G = B' W B.
```

The project explores whether graph multigrid ideas can be extended beyond the
special two-way case. It combines two complementary ingredients:

1. **pair-CMG corrections**: each factor pair is a bipartite graph Laplacian
   after a sign switch, so the existing [CMG](https://github.com/johannes-schmieder/CMG)
   package can supply fixed linear pair solves;
2. **a true three-way hierarchy**: hard aggregation occurs separately within
   each factor, maps fine triples to coarse triples, and preserves the weighted
   incidence-Gramian class exactly under Galerkin coarsening.

## Current status

The first research implementation includes:

- deterministic validation and collapse of repeated three-way tuples;
- matrix-free `B`, `B'`, `sqrt(W)B`, and `B'WB` kernels;
- incidence-component discovery and projection of the two structural shift
  directions per connected component;
- exact hard factor-respecting Galerkin coarsening;
- an automatic shared-context affinity matcher;
- stable weighted-Jacobi smoothing with the three-way `G <= 3D` bound;
- recursive symmetric V-cycles with a rank-revealing spectral terminal;
- pairwise CMG corrections for all three factor pairs;
- a symmetric hybrid of pair-CMG smoothing and three-way coarse correction;
- projected PCG for controlled Gramian experiments;
- modified LSMR on the original rectangular weighted incidence operator;
- independent normal-equation residual certification;
- manufactured oracle/automatic hierarchy tests and a feasibility executable.

This is a **research prototype**, not a production solver. In particular, the
current hierarchy allocates temporary vectors during each application, the
automatic aggregation rule is deliberately simple, and extra rank deficiency
beyond the known factor shifts is handled only at dense terminals and by the
rectangular LSMR path.

## Workspace

```text
crates/multiway-incidence
    Matrix class, tuple topology, components, kernels, and exact hard coarsening.

crates/multiway-mg
    Affinity aggregation, dense terminal, V-cycle, pair-CMG, PCG, and LSMR.
```

`multiway-incidence` intentionally has no dependency on CMG or `within`.
`multiway-mg` uses exact pinned revisions of CMG and `schwarz-precond` behind
features.

## Feasibility probe

GitHub Actions runs:

```bash
cargo run --release -p multiway-mg --example feasibility --all-features
```

The executable constructs a planted two-level three-way problem and compares
diagonal PCG, the three-way V-cycle, pair-CMG, the hybrid, and modified LSMR.
Its iteration counts are diagnostics, not claims about real fixed-effect data.

See `docs/MATHEMATICS.md`, `docs/ARCHITECTURE.md`, `docs/FEASIBILITY.md`, and
`docs/ROADMAP.md` for the mathematical contract, package boundary, evidence
plan, and next milestones.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo doc --workspace --all-features --no-deps
cargo run --release -p multiway-mg --example feasibility --all-features
```

The minimum supported Rust version is 1.85. The repository is licensed under
GNU GPL version 3 only.
