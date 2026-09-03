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
- a deterministic adaptive aggregation policy that tries exact shared contexts
  before a bounded pair-neighborhood fallback;
- per-level diagnostics recording the selected aggregation method;
- stable weighted-Jacobi smoothing with the three-way `G <= 3D` bound;
- recursive symmetric V-cycles with a scale-invariant rank-revealing terminal;
- pairwise CMG corrections for all three factor pairs;
- a symmetric hybrid of pair-CMG smoothing and three-way coarse correction;
- projected PCG for controlled Gramian experiments;
- modified LSMR on the original rectangular weighted incidence operator;
- independent normal-equation residual certification;
- tests for disconnected components, additional nesting-induced rank
  deficiency, symmetry, weight-scale invariance, and exact Galerkin identities;
- planted and six-family release-mode feasibility probes in GitHub Actions.

This is a **research prototype**, not a production solver. In particular, the
current hierarchy allocates temporary vectors during each application, the
automatic structural aggregation rules are deliberately simple, and extra rank
deficiency beyond the known factor shifts is handled only at dense terminals
and by the rectangular LSMR path.

## First evidence

All 30 method/case combinations in the first six-family matrix converged and
passed their original-operator residual checks. The symmetric pair-CMG plus
three-way coarse hybrid required 1–4 iterations across planted clones, noisy
clones, a Latin-square pattern, a weak chain, a nested third factor, and two
disconnected Latin components. On the weak-chain case, diagonal PCG required
85 iterations, pair-CMG required 9, the three-way V-cycle required 6, and the
hybrid required 3.

These are small manufactured problems. The iteration results support the
mathematical direction, but the sub-millisecond timings do not establish a
production speed advantage. See [`docs/RESULTS.md`](docs/RESULTS.md) and the raw
files under `benchmarks/results/2026-09-03/`.

## Workspace

```text
crates/multiway-incidence
    Matrix class, tuple topology, components, kernels, and exact hard coarsening.

crates/multiway-mg
    Adaptive aggregation, dense terminal, V-cycle, pair-CMG, PCG, and LSMR.
```

`multiway-incidence` intentionally has no dependency on CMG or `within`.
`multiway-mg` uses exact pinned revisions of CMG and `schwarz-precond` behind
features.

## Feasibility probes

GitHub Actions runs both probes with the committed lockfile:

```bash
cargo run --locked --release -p multiway-mg \
  --example feasibility --all-features

cargo run --locked --release -p multiway-mg \
  --example feasibility_matrix --all-features
```

The first constructs a planted two-level problem. The second compares diagonal
PCG, the true V-cycle, pair-CMG, the hybrid, and modified LSMR across six
structural families. Iteration counts and certified residuals are evidence;
tiny-problem wall times are diagnostics only.

See `docs/MATHEMATICS.md`, `docs/ARCHITECTURE.md`, `docs/FEASIBILITY.md`,
`docs/RESULTS.md`, and `docs/ROADMAP.md` for the mathematical contract, package
boundary, evidence, limitations, and next milestones.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked --workspace --no-default-features
cargo doc --locked --workspace --all-features --no-deps
cargo run --locked --release -p multiway-mg --example feasibility --all-features
cargo run --locked --release -p multiway-mg --example feasibility_matrix --all-features
```

The minimum supported Rust version is 1.85. The repository is licensed under
GNU GPL version 3 only.
