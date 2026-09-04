# Continuous integration

The permanent repository checks run from `.github/workflows/ci.yml` on Rust
1.85. The workflow covers:

- formatting and strict Clippy across all targets and features;
- all-feature and minimal-feature tests;
- warning-free rustdoc;
- manufactured feasibility probes;
- the oracle quotient-space spectral matrix;
- compatible-relaxation diagnostics; and
- the deterministic issue #2 two-grid and resolution gate.

Frozen issue #3 policies, holdouts, traces, checksums, and negative results are
preserved under `benchmarks/` and `docs/`. They are research evidence rather
than one-time orchestration jobs, so the temporary workflows used to generate
and finalize those experiments are not retained on `main`.

Milestone-specific development workflows may be used on temporary branches when
needed, but they must be removed before merge unless they are deliberately
promoted into the permanent `ci.yml` contract. Scratch benchmark directories
must not be committed; canonical generated evidence belongs under
`benchmarks/results/<date>/`.
