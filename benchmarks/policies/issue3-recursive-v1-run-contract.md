# Issue #3 recursive v1 execution contract

This contract preserves the previously frozen recursive policy without changing
its seeds, thresholds, cycle smoother, or scientific gates.

The authoritative run must:

1. validate the Rust 1.85 implementation and minimal feature set;
2. execute all eight seeded recursive fixtures twice from clean directories;
3. require byte-identical decision matrices and true-residual traces;
4. evaluate `scripts/summarize_issue3_recursive_holdout.py` with its frozen
   defaults;
5. preserve the matrix, traces, generated report, gate status, and SHA-256
   checksums whether the outcome is positive or negative;
6. interpret a negative result as scientific evidence rather than retuning the
   same seeds.
