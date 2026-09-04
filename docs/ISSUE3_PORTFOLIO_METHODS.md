# Issue #3 two-stage automatic-coarsening portfolio

## Purpose

Issue #2 established that a good hard factor-respecting coarse space can provide
the missing global three-way correction. Issue #3 asks whether MultiwayMG can
discover a compact approximation to that space automatically, deterministically,
and at bounded setup cost.

The first issue #3 development matrix exposed a policy conflict. Weighted
Jacobi is a useful conservative generator of compatible-relaxation witnesses,
but it can reject a map whose complete symmetric-MAP two-grid cycle is already
close to the oracle. Conversely, using only a strong smoother can conceal a
poor coarse space.

The package therefore evaluates a two-stage portfolio:

1. **Primary bootstrap screen.** Weighted Jacobi generates relaxed signatures,
   compatible witnesses, rematching decisions, and monotone split repairs.
2. **Secondary cycle-aligned screen.** Only after primary rejection, symmetric
   MAP evaluates structurally admissible candidates already produced by the
   primary process or by the protected pair-neighborhood baseline.

The secondary stage cannot create a new map, loosen dimension or tuple budgets,
or erase the conservative diagnostic. It can only rescue a candidate that is
compact and effective with the smoother intended for the evaluated two-grid
cycle.

## Candidate set

The secondary screen considers at most two distinct maps:

- the final map returned by primary bootstrap and optional split repair;
- the protected pair-neighborhood structural baseline.

If both maps pass, selection is deterministic and lexicographic:

1. fewer unique coarse tuples;
2. smaller coarse coefficient dimension;
3. lower secondary compatible-relaxation factor;
4. stable candidate-source order.

## Non-negotiable structural gates

Every accepted map must satisfy:

- coarse coefficient dimension strictly below the fine dimension;
- coarse-dimension ratio at most `0.80`;
- unique-tuple reduction at least `0.05`;
- two-level tuple complexity at most `1.95`;
- exact factor boundaries and exact incidence-component preservation.

Neither compatible-relaxation screen may bypass these gates.

## Frozen second holdout

The development matrix using seeds 512--521 remains committed as negative and
development evidence. It was not overwritten after motivating the two-stage
policy.

The second holdout was declared in
`benchmarks/policies/issue3-portfolio-holdout.tsv` before numerical evaluation.
It uses seeds 600--609 across:

- two dominant-pair/weak-third fixtures;
- two nearly nested fixtures;
- weak chain;
- planted communities;
- hub/power-law degree structure;
- twelve-order-of-magnitude positive weights;
- Latin-square incidence;
- rectangular tensor structure.

Both primary and secondary compatible-relaxation thresholds are fixed at `0.85`
per sweep. The initial scientific target is median recovery of at least `60%`
of the oracle condition-number improvement, with no material regression more
than `0.10` below the one-shot structural baseline, true PCG residuals below
`1e-8`, and all structural gates satisfied.

## Measured methods

For each holdout fixture, the frozen matrix records:

- symmetric-MAP baseline;
- supplied oracle MAP two-grid cycle;
- one-shot pair-neighborhood map;
- primary bootstrap final map;
- final two-stage portfolio map.

Every structurally admissible map is evaluated with the same symmetric-MAP
two-grid cycle and complete quotient-space spectral analysis. Projected PCG
recomputes and records the true original-Gramian residual after every iteration.
Partition agreement with the supplied oracle is reported only as a diagnostic;
spectral benefit and tuple complexity are the scientific authorities because a
non-oracle partition may be equally good or better.

## Fail-closed interpretation

A portfolio rejection is a valid outcome. It means no candidate passed the
frozen structural and compatible-relaxation rules. Such a case must route to a
declared baseline in any future production integration; it is not counted as a
successful automatic hierarchy.

The holdout is run twice and both the summary matrix and residual traces must be
byte-identical. Threshold changes, new candidate mechanisms, or fixture changes
require a new explicitly versioned policy and a new unseen holdout rather than
rewriting the prior evidence.
