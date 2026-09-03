# Development instructions

MultiwayMG is a numerical research package. Correctness and reproducibility take priority over benchmark wins.

## Required checks

Before merging a source change, run through GitHub Actions:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo doc --workspace --all-features --no-deps
cargo run --release -p multiway-mg --example feasibility
```

## Numerical rules

- Never delete or threshold a positive tuple weight merely to improve performance.
- Preserve the structural factor-shift kernel at every level.
- Treat iterative convergence flags as candidates; certify against the submitted operator.
- Keep topology and numerical weights separable so changing-weight reuse can be audited.
- Any automatic route must be deterministic and based on structural dimensions, not elapsed time.
- Benchmarks must charge setup, workspace, and certification costs.

## Dependency policy

Pin Git dependencies to exact commits. `multiway-incidence` must remain independent of CMG and `within`. `multiway-mg` may use CMG for pair solves and `schwarz-precond` for the rectangular LSMR driver.
