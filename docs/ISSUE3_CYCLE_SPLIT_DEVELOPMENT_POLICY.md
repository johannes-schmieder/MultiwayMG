# Issue #3 complete-cycle split-repair development policy

This observed-seed experiment was declared before running the new repair on the
v3 fixtures. Seeds `900`–`909` are already development data because the frozen
v3 holdout has been evaluated; this experiment cannot alter that verdict.

Starting from the protected pair-neighborhood map, the repair:

1. evaluates the exact intended two-grid cycle with either one symmetric-MAP
   sweep or one all-pair fixed-CMG correction before and after the coarse solve;
2. retains the slowest matrix-free complete-cycle witness;
3. selects the aggregate carrying the largest share of its diagonal energy;
4. splits that aggregate at the largest deterministic witness-value gap;
5. admits the split only if the worst estimated cycle factor falls by at least
   two percent;
6. permits at most eight splits, a coarse-dimension ratio at most `0.65`, tuple
   reduction at least `0.05`, and two-level tuple complexity at most `1.95`;
7. stops fail-closed at the first insufficient improvement or structural-budget
   violation.

The development mechanism is considered promising only if it produces at least
ten percent dense condition-number improvement on multiple structurally
distinct cases while retaining true residual accuracy and structural budgets.
Otherwise issue #3 should conclude that the protected structural matcher is the
useful automatic method found so far and that witness/bootstrap refinements did
not justify their additional setup complexity.
