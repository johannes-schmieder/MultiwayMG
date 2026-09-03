# Oracle spectral feasibility results

## Purpose

The first MultiwayMG MVP showed that a factor-preserving hierarchy could be
constructed and used successfully. Issue #2 asks the more precise scientific
question:

> If the factor aggregation maps are known to be good, is the resulting
> three-way coarse correction fundamentally strong enough to justify research
> on automatic aggregate discovery?

The answer from the first quotient-space matrix is **yes on the manufactured
oracle families tested here**.

This is an oracle result, not an automatic-coarsening or production-performance
claim. The refinements deliberately create known parent maps, and their
within-parent child structure is favorable to the current pair smoother.

## Reproducible evidence

The authoritative run used Rust 1.85.0 in GitHub Actions at commit
`d3f28d64dd43601ff291869b5dcf6784cc427c3e`:

```bash
cargo run --locked --release -p multiway-mg \
  --example oracle_spectral_matrix --all-features
```

The complete matrix is retained as the `oracle-spectral-output` artifact of
workflow run `33809711701`. A compact machine-readable subset is committed at
`benchmarks/results/2026-09-03/oracle-spectral-summary.tsv`.

The matrix contains six weighted families:

- planted weakly coupled communities;
- a Latin-square incidence pattern;
- a weak chain;
- a nearly nested third factor;
- two disconnected Latin-square components; and
- a complete four-level hierarchy.

Each family begins with a small base problem. A refinement replaces every
factor level with children and every parent tuple with weighted child tuples
whose conductances sum exactly to the parent conductance. The exact parent maps
are retained and verified by recoarsening.

Methods compared:

- inverse diagonal at three safe damping values;
- symmetric MAP/block symmetric Gauss--Seidel;
- exact dense pairwise Schwarz;
- fixed pair-CMG Schwarz;
- an oracle Jacobi V-cycle;
- an oracle pair-CMG/coarse hybrid; and
- the exact global pseudoinverse as a calibration reference.

Every method was analyzed on the complete numerical range of the Gramian,
including null directions beyond the two generic factor shifts where present.
Every row also solved a deterministic compatible system with projected PCG and
recomputed the true relative residual.

## Main results

### Oracle pair-CMG V-cycles nearly diagonalize the test problems

Across all six families, the oracle pair-CMG/coarse hybrid had:

- geometric-mean preconditioned condition number: approximately `1.0029`;
- maximum preconditioned condition number: approximately `1.0056`;
- maximum optimally damped energy radius: approximately `0.0028`;
- projected-PCG iterations: `3` or `4`;
- no negative or numerically zero preconditioner-energy directions; and
- true relative residuals below `1.4e-11`.

The most difficult raw Gramian was the weak chain, with condition number about
`860`. Its diagnostics were:

| Method | Preconditioned condition number | Optimal energy radius | PCG iterations |
|---|---:|---:|---:|
| Inverse diagonal | `675.0` | `0.9970` | `45` |
| Symmetric MAP | `222.9` | `0.9911` | `22` |
| Pair-CMG | `2.064` | `0.3472` | `13` |
| Oracle Jacobi V-cycle | `1.410` | `0.1701` | `9` |
| Oracle pair-CMG V-cycle | **`1.0030`** | **`0.0015`** | **`4`** |

This cleanly separates the roles envisioned by the package design. Pair graph
solves remove most difficult pair-visible error, while the known global coarse
space removes the remaining slowly varying three-factor mode.

### A coarse correction helps even with a cheap smoother

The oracle Jacobi V-cycle had a preconditioned condition number between about
`1.34` and `1.46` on all six families, with `8` or `9` PCG iterations. Its
maximum optimal energy radius was about `0.187`.

This is important because it shows that the result is not solely an artifact of
using a very strong pair solver. A correct factor-preserving coarse space is
valuable even when smoothing is only one safe weighted-Jacobi correction before
and after recursion.

### Pair corrections alone are strong but incomplete

Exact pairwise Schwarz and pair-CMG produced identical reported spectra in this
small matrix. Their preconditioned condition numbers ranged from approximately
`1.66` to `2.98`, with `8` to `13` PCG iterations.

The equality is expected for these small pair systems: the pinned CMG
configuration reaches an exact direct terminal, so this experiment does **not**
measure the quality of an approximate large-graph CMG cycle against exact pair
solves. That comparison remains issue #4.

What this matrix does establish is that even exact pair solves leave a material
mode that the oracle three-way coarse correction can remove.

### Symmetric MAP is topology dependent

Symmetric MAP was excellent on the complete, Latin-square, and disconnected
Latin cases, with condition numbers near `1.05` to `1.09`. It degraded sharply
on weak-global-coupling structures:

- planted communities: condition number about `12.4`;
- nearly nested: about `68.4`;
- weak chain: about `222.9`.

This reinforces the motivation for graph-pair and multilevel corrections rather
than relying on a factor sweep alone.

### The hierarchy remains compact

Oracle hierarchy depths ranged from two to four. Tuple complexity remained
between approximately `1.1406` and `1.1428`, well below the provisional budget
of three. The four-level hierarchy therefore processed only about fourteen
percent more tuples cumulatively than a single finest-level pass, excluding the
additional smoother and terminal operations.

### Structural correctness held throughout

For every reported preconditioner:

- quotient symmetry defects were at roundoff scale;
- no materially negative preconditioner-energy direction was detected;
- no numerical range direction was left unpreconditioned;
- disconnected systems had the expected additional structural nullity; and
- every projected-PCG result passed a recomputed original-Gramian residual
  check.

The inverse diagonal does not map the raw singular range exactly into itself;
its reported leakage was about `0.7%` to `2.9%`. Projected PCG explicitly
projects the preconditioned vector, so the quotient spectrum remains the
relevant diagnostic. MAP, pair Schwarz, CMG, and both oracle cycles preserved
the range to numerical roundoff.

## Interpretation

The oracle gate is passed:

1. Hard factor-respecting interpolation is not inherently too weak on the
   tested structured systems.
2. The three-way coarse correction adds information that exact pairwise solves
   do not contain.
3. A symmetric V-cycle can be positive and extremely well conditioned on the
   complete numerical range.
4. Recursive depth and exact tuple closure can coexist with low operator
   complexity.
5. The numerical obstacle is no longer whether a useful three-way hierarchy can
   exist. It is whether MultiwayMG can discover sufficiently good maps cheaply
   on realistic unstructured systems.

## Important limitations

The near-unit hybrid spectra should not be generalized directly.

The current oracle refinement expands each parent tuple into a complete tensor
of child tuples. This creates clean within-aggregate high-frequency modes that
pair solves are unusually well positioned to remove. Real datasets can have:

- sparse and irregular child support;
- unequal aggregate sizes;
- factor-specific unresolved modes;
- weak links not aligned with the intended hierarchy;
- coarse maps that are only approximately correct; and
- weight patterns that change the best hierarchy.

The dense matrix also does not measure production cost. Materializing a
preconditioner is quadratic, dense eigendecomposition is cubic, and the oracle
maps are supplied rather than learned.

## Decision

Issue #2 has achieved its primary go/no-go objective. Research should proceed to
issue #3: compatible-relaxation and bootstrap aggregation.

That next phase should measure the gap between automatic and oracle spaces. It
should add sparse/adversarial refinements, deliberately imperfect maps, and
compatible-relaxation diagnostics rather than continuing to tune the idealized
oracle cases. Issue #4 remains responsible for determining whether CMG is the
right production local pair solver after large-system setup and application
costs are charged.
