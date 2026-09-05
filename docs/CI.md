# Continuous integration

The main Rust repository checks run from `.github/workflows/ci.yml` on Rust
1.85. The workflow covers:

- formatting and strict Clippy across all targets and features;
- all-feature and minimal-feature tests;
- warning-free rustdoc;
- manufactured feasibility probes;
- the oracle quotient-space spectral matrix;
- compatible-relaxation diagnostics; and
- the deterministic issue #2 two-grid and resolution gate.

The path-scoped
`.github/workflows/issue4-coarse-cmg-validator.yml` is also permanent. It runs
the issue-4 coarse-level evidence regression suite whenever its validator,
adversarial tests, or workflow definition changes. That suite freezes the full
eight-fixture universe and fails closed on schema, coverage, mixed/duplicate
plan states, hierarchy shape, charged-time identities, cumulative-prefix
monotonicity, solver work units, certificate accounting, CMG terminal coverage,
fallback allocations, and asymmetric admission errors.

Frozen issue #3 policies, holdouts, traces, checksums, and negative results are
preserved under `benchmarks/` and `docs/`. Frozen issue #4 size-ladder and
coarse-level evidence is preserved under `evidence/`, with the earlier pair-
local and whole-system artifacts under `benchmarks/results/2026-09-04/`. These
are research evidence rather than one-time orchestration jobs, so the temporary
workflows used to generate and finalize the matrices are not retained on
`main`.

Milestone-specific development workflows may be used on temporary branches when
needed, but they must be removed before merge unless they are deliberately
promoted into the permanent workflow contract. A permanent specialized
workflow should be narrow, path-scoped, deterministic, and test a scientific or
evidence-integrity contract not already covered by the general Rust matrix.
Scratch benchmark directories must not be committed; canonical generated
evidence belongs under `benchmarks/results/<date>/` or a documented permanent
`evidence/<milestone>/` directory with checksums and provenance.
