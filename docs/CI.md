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

## Standalone campaign additions

M1 numerical boundary regressions run in the ordinary all/minimal-feature Rust
suite (LSMR cases require its feature). M2 frame-view dense/adjoint/Galerkin,
identity and mutation tests run in `multiway-incidence`. The permanent isolated
`workspace_allocations` executable additionally measures frame-view creation,
first/repeated actions and errors on Linux/macOS/Windows, debug/release and
minimal/all-feature configurations. See `NUMERICAL_BOUNDARIES.md` and
`ISSUE5_OPERATOR_VIEWS.md` for the qualified scope of each boundary.

M3 additionally composes immutable frame actions, caller-owned MAP scratch,
forked serial LSMR and an independent certificate for 32 RHS inside the same
isolated allocation executable (when `lsmr` is enabled). Both dependency pins
move together; immutable performance baselines remain separately identified in
`PERFORMANCE_PROTOCOL.md` and `ISSUE5_LSMR_DEPENDENCY.md`.

M4a adds complete supplied-map structural/replay ownership to the permanent
allocation executable: exact retained/released payload (including heap descriptor
arrays), zero-allocation budget rejection, first/repeated level actions, and
balanced changed-frame construction/destruction. Unit tests inject errors and
unwinds at all 21 structural and 11 numerical reservations of a two-transition
case. This does not yet qualify a complete prepared hierarchy solve.

M4b checks ordinary/prepared projection and MAP bitwise equivalence, symmetry,
exact generation rejection and transactional numerical failure/recovery. The
permanent allocation executable now also tests prepared MAP's five-array setup,
first/repeat32 calls, allocation-free errors and exact release; the LSMR composed
path uses this exact-current-frame MAP directly.

M4c's prepared fixed-cycle increment compares all eight recursive fixtures with
ordinary V-cycle output, including changed weights. The permanent allocation
executable measures complete first/repeat32 calls, failure/recovery and exact
terminal/workspace release; unit tests inject errors/unwinds at all 31 workspace
reservation boundaries of a two-transition case. The ordinary path continues to
run all existing scientific gates through the same shared recurrence.

M4c's complete prepared LSMR increment matches the ordinary driver on all eight
recursive fixtures, including bitwise coefficients/certificates, native diagnostics
and action counts. Permanent allocation tests cover complete first/RHS1,2,4,8,16,
17,32 solves, exact 48-array release and zero-allocation budget rejection. The
feature-independent original-operator certificate has separate allocation/failure
coverage and shares its implementation with the existing ordinary driver.

M4c prepared PCG shares the ordinary untraced recurrence and passes complete
recursive/acceptance/RHS tests. The permanent allocator gate measures first and
RHS1,2,4,8,16,17,32 solves, complete payload, zero-allocation budget rejection and
exact 42-array release. A separate local 144-case frozen-source comparison is
recorded in `ISSUE5_PREPARED_PCG.md`; it is not a cross-platform bitwise gate.
