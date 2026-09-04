# ADR 0001: automatic coarsening after issue #3

- **Status:** Accepted
- **Date:** 2026-09-04
- **Scope:** Research automatic hierarchy construction for weighted three-way
  incidence Gramians

## Context

Issue #2 established that a good hard factor-respecting coarse space can produce
an excellent symmetric multilevel preconditioner. Issue #3 investigated how to
discover such a space automatically.

Several increasingly adaptive methods were implemented and evaluated:

- exact-context and pair-neighborhood structural matching;
- relaxed-signature bootstrap matching;
- compatible-relaxation admission;
- monotone compatible-witness split repair;
- matrix-free complete-cycle screening;
- MAP-to-CMG smoother fallback;
- complete-cycle witness split repair;
- recursive automatic hierarchy planning.

Frozen holdouts showed that the structural pair-neighborhood map was reliable,
while bootstrap and repair did not materially improve it. A single universal
MAP cycle also failed on some valid graph-cover maps, making fail-closed cycle
screening necessary.

## Decision

### Candidate map

Use the bounded pair-neighborhood matcher as the default automatic hard-map
candidate.

The map must:

- aggregate only within one factor;
- preserve exact incidence components;
- retain positive finite coarse tuple weights;
- satisfy declared coarse-dimension, tuple-reduction, and tuple-complexity
  limits.

### Cycle authority

A structural map is not accepted merely because it coarsens or passes
compatible relaxation. Build and probe the intended complete cycle.

The research smoother order is:

1. one fixed symmetric-MAP two-grid cycle;
2. all-pair fixed-CMG two-grid cycle only after every MAP candidate is rejected.

Every accepted cycle must satisfy the declared matrix-free energy-factor,
observed-step, structural-defect, and true-residual requirements.

When no candidate passes, return an explicit no-hierarchy result. Do not return
an identity, diagonal, or partially screened preconditioner as success.

### Adaptive diagnostics

Retain compatible relaxation, relaxed-signature bootstrap, and witness-driven
repair as public research diagnostics. Do not include them in default automatic
routing until new evidence shows a material advantage over the structural
baseline after setup and complexity costs are charged.

### Recursion

Recursive pair-neighborhood construction is permitted as a research path because
it succeeded on every frozen recursive fixture and retained solve accuracy.
Each accepted level must report cumulative dimension and tuple complexity.
Production admission is deferred until prepared state, memory accounting, and
amortization are implemented.

## Consequences

### Positive

- The automatic route is simpler and more reproducible.
- It is fail-closed on systems whose reference cycle is itself inadequate.
- It avoids paying for adaptive machinery that did not improve the frozen
  matrices materially.
- It cleanly separates map construction from cycle quality.
- It permits CMG to remain a selective fallback rather than a universal cost.

### Negative

- The final issue #3 route is less adaptive than originally planned.
- Some systems may contain useful coarse spaces not visible to pair-neighborhood
  structure.
- Recursive hierarchies can exceed provisional cumulative complexity budgets.
- The current MAP-to-CMG fallback rebuilds deterministic candidate state; issue
  #5 must remove this duplication without changing selection semantics.

## Rejected alternatives

### Compatible relaxation as sole authority

Rejected because strong smoothers can conceal coarse-map defects and larger
coarse spaces can make the compatible complement artificially easy.

### Universal symmetric-MAP cycle

Rejected by the frozen v2 graph-cover result: valid weak-chain and community
maps failed the fixed MAP complete-cycle threshold.

### Universal pair-CMG cycle

Rejected as a policy assumption. It rescued some nearly nested maps but not the
weak-chain/community reference failures, and its production economics remain
unmeasured.

### Bootstrap or witness repair by default

Rejected by the frozen v3 and complete-cycle split evidence. The learned maps
did not beat the protected structural baseline by the declared margin, and no
cycle-split row improved dense condition number by ten percent.

### Retuning the frozen holdouts

Rejected. Negative results are preserved with policies, matrices, traces,
status files, and checksums.

## Follow-up

- Issue #4 determines whether pair-CMG pays relative to the existing
  approximate-Cholesky local pair solver.
- Issue #5 adds prepared candidates, reusable workspaces, exact memory reports,
  repeated-RHS execution, and changing-weight replay.
- Issue #6 evaluates a private certified fereg route after the numerical and
  engineering gates above.
