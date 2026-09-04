# Issue #3 v3 execution contract

This execution contract was committed before numerical evaluation of the frozen
seeds `900`–`909`.

The authoritative runner must:

1. verify the machine-readable v3 policy and smoother order;
2. pass Rust 1.85 formatting, strict Clippy, targeted bootstrap and smoother
   portfolio tests, and the minimal-feature workspace tests;
3. execute `issue3_cycle_holdout_v3` twice from clean output directories;
4. require byte-identical decision matrices and true-residual traces;
5. evaluate the predeclared gates with
   `scripts/summarize_issue3_cycle_holdout_v3.py` without changing thresholds;
6. preserve the matrix, traces, descriptive timing, gate status, generated
   report, and SHA-256 checksums whether the outcome is positive or negative;
7. treat timing as descriptive only and exclude it from deterministic or
   scientific acceptance decisions.

A negative result is evidence and must not be tuned away on the same seeds.
