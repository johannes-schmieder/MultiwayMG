# Issue #3 smoother fallback development rule

This document freezes the rule evaluated on the already observed issue #3 v2
seeds `700`–`709`. The experiment is calibration evidence only and cannot
reverse or replace the frozen v2 negative verdict.

For every structurally admissible hard map candidate:

1. build a one-sweep symmetric-MAP two-grid cycle and evaluate the unchanged
   complete-cycle quality criteria;
2. prefer an accepted MAP cycle whenever one exists;
3. only when no MAP candidate is accepted, build an all-pair fixed-CMG
   two-grid cycle and apply the same complete-cycle criteria;
4. within a smoother tier, choose the smallest estimated energy factor, then
   the smallest coarse tuple count, then the smallest coarse dimension, then
   stable candidate-source order;
5. return a declared rejection when neither smoother tier yields an accepted
   candidate.

The hard structural gates remain unchanged:

- coarse dimension ratio at most `0.80`;
- unique tuple reduction at least `0.05`;
- two-level tuple complexity at most `1.95`;
- exact component preservation and valid positive coarse weights.

The complete-cycle gates remain unchanged:

- estimated energy factor at most `0.50`;
- observed one-step energy factor at most `1.05`;
- structural defect at most `1e-10`;
- true PCG residual reported independently.

A favorable calibration result will justify implementing this as a first-class
cycle-smoother portfolio and freezing a new policy on new, unseen seeds. It is
not itself a scientific holdout pass.
