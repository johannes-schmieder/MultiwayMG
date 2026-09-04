# Development instructions

MultiwayMG is a numerical research package. Correctness, reproducibility, and
honest negative results take priority over benchmark wins.

## Required checks

Before merging a source change, the authoritative Rust 1.85 checks are:

```text
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked --workspace --no-default-features
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --all-features --no-deps
```

Run the milestone-specific scientific gates relevant to the changed code in
addition to the ordinary suite. Do not weaken a frozen policy after examining
its holdout.

## Numerical rules

- Never delete or threshold a positive tuple weight merely to improve
  performance.
- Preserve factor boundaries, exact incidence components, and structural
  factor-shift modes at every accepted level.
- Treat iterative convergence flags as candidates; certify against the
  submitted operator.
- Keep topology and numerical weights separable so changing-weight reuse can be
  audited.
- Automatic routing and hierarchy decisions must be deterministic and based on
  declared structural/numerical quantities, not elapsed time.
- Benchmarks must charge setup, workspace, retained memory, failed routes, and
  certification costs.
- Preserve predeclared negative results rather than tuning them away.

## Repository discipline

- Pin Git dependencies to exact commits.
- `multiway-incidence` must remain independent of CMG and `within`.
- `multiway-mg` may use CMG for pair solves and `schwarz-precond` for the
  rectangular LSMR/Schwarz infrastructure.
- Canonical generated evidence belongs under `benchmarks/results/<date>/` with
  the corresponding policy/checksum when applicable.
- Do not commit temporary duplicate-run directories, local diagnostics, or
  scratch matrices at repository root.
- One-time orchestration workflows may be used on development branches, but
  remove them before merge unless they are deliberately promoted into the
  permanent `.github/workflows/ci.yml` contract.
- Keep `README.md`, `docs/ROADMAP.md`, and the relevant ADR/result document in
  sync when a research milestone closes.
