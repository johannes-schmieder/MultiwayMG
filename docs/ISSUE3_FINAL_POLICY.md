# Issue #3 final automatic-coarsening policy

The current research automatic route is deliberately simple and fail-closed.

1. Build a bounded pair-neighborhood hard aggregation.
2. Reject maps that cross exact incidence components or violate declared
   coefficient-dimension, tuple-reduction, or tuple-complexity budgets.
3. Build and probe the intended complete two-grid cycle.
4. Prefer one symmetric-MAP cycle.
5. Evaluate an all-pair fixed-CMG cycle only after MAP rejection.
6. Accept only a candidate satisfying the complete-cycle and true-residual
   criteria.
7. Return an explicit no-hierarchy result when no candidate passes.

Compatible relaxation, relaxed-signature bootstrap matching, compatible-witness
repair, and complete-cycle witness repair remain diagnostic or experimental.
Frozen issue #3 evidence did not show a material advantage over the protected
pair-neighborhood baseline.

Recursive pair-neighborhood hierarchies are a promising research path but are
not production-admitted until cumulative setup, tuple work, memory, and
repeated-RHS amortization are handled under issue #5.
