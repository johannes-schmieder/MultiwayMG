# Complete prepared serial LSMR and certification (M4c)

`PreparedLsmrWorkspace` borrows one exact prepared numerical hierarchy and owns
all mutable LSMR, hierarchy, projection, candidate, weighted-target and certificate
storage. Setup fixes native tolerance, independent certificate tolerance,
iteration limit and local reorthogonalization capacity. Applications do not
rebind numerical generations, resize buffers or enter an implicit thread pool.
The fork's unchanged serial modified-LSMR recurrence remains the outer algorithm.

`solve_prepared_least_squares` accepts one target column in **canonical unique-
tuple order**, not original observation order. It validates exact hierarchy,
dimensions and finite input before mutating admitted solve state. The candidate
is structurally projected and independently certified against that same fine
frame's original weighted incidence operator. It borrows workspace storage, so
another solve cannot coexist with continued use of its coefficient borrow.

The original allocating driver and new prepared certificate share one fail-
closed implementation of `||B'W(y-Bx)|| / ||B'Wy||`. The prepared certificate owns
one tuple residual and two coefficient vectors, binds to the exact fine frame,
and rejects nonfinite arithmetic, weighted-product underflow and unrepresentable
norms/ratios. Zero/zero is zero. It runs independently of the preconditioner metric
and native stopping flags. This is neither a numerical-rank nor minimum-norm
certificate, and does not imply arbitrary extreme-scale input support.

The result preserves native convergence, stop reason, iteration count and native
residual diagnostics. **Acceptance uses only the finite original-operator
certificate and declared positive certificate tolerance.** A valid returned
candidate can have `accepted=false`; callers must inspect it. Native convergence
alone never overrides a failed independent certificate. A numerical error returns
no candidate. The same workspace can solve a subsequent valid RHS.

## Independent scalar RHS reuse

The batch wrapper handles 1–32 independent columns in column-major target and
coefficient arrays. It validates the entire layout and finite target panel before
mutation, then clears report slots to `None`. Each completed candidate is copied
to its caller-owned output column with `Some(report)`. That report's `accepted`
flag still determines acceptance. On numerical failure the completed prefix
remains intact, failed/unprocessed outputs remain unchanged, and the first `None`
names the failed column. `last_work()` retains that admitted column's work.

This wrapper is serial scalar scheduling. It neither fuses RHS kernels nor uses
a block Krylov recurrence. Fused panels, worker scheduling and explicit controlled
parallelism remain M7/M8; no parallel speed claim is made here.

## Complete payload and work accounting

Requested outer scratch is `8*(5E + 9V + 2kV) + 72C` bytes for E tuples,
V coefficients, C exact components and k=min(window,E,V). Add the complete
hierarchy application workspace. Both MGS windows and warm-start capacity in the
fork are charged even though this driver uses cold starts. Candidate normalization
uses an explicit V-vector copy of the borrowed fork result; later liveness/kernel
work must measure any proposed elimination.

The setup budget includes exact retained fine/coarse topology, maps, all numerical
frames and terminal factors plus requested complete new solve scratch and
caller-declared other live arrays. RHS/output panels, reports, unrelated workers
and old generations are not discovered implicitly. Retained reports count actual
array and heap descriptor capacities, with every direct owner counted once.
Inline roots, allocator metadata, prior construction temporaries and RSS remain
separate; these are payload limits, not hard allocator quotas.

Per-solve counts record actual attempted weighted incidence/adjoint and complete
hierarchy actions, including admitted failures. Independent certification counts
its own one incidence and two weighted-adjoint actions on success. Static rejected
calls preserve the previous admitted work record. Admitted failures retain their
attempted work, including zero actions if weighted-target construction fails.
The typed external error is preserved without adding a string conversion; native
error construction is not claimed universally allocation-free.

## Verification and remaining M4 scope

On all eight existing recursive fixtures, the complete prepared driver matches
ordinary coefficients, native diagnostics, certificate values and action counts,
including bitwise coefficients/certificates. Tests compare every declared RHS
count with separate scalar calls, zero RHS, failure-prefix preservation/recovery,
exact ownership and native convergence with a rejected stricter certificate.
Existing numerical-boundary regressions exercise the shared certificate.

The permanent allocation executable measures exact full scratch payload and
release of all 40 arrays (48 before M5a scratch reuse) in a two-transition/window-8 case, zero-allocation
one-byte-short budget rejection, first solve and RHS counts 1,2,4,8,16,17,32.
The feature-independent certificate test separately measures three-array setup,
first/repeated numerical failures and recovery. Prior component-level injection
covers all fork and hierarchy reservation boundaries.

Prepared PCG and the complete-cost serial v1 benchmark are now merged. Its
LSMR certificate rejections motivated the separate [certificate-gated
route](ISSUE5_CERTIFICATE_GATED_LSMR.md); M4 is now complete through PR #45.
Its declared complete-cost development coverage passes. Automatic routing,
changed-weight quality admission, CPU scaling and competitive qualification gates remain unchanged.
No competitive performance result is claimed from these correctness/allocation
checks, and the campaign holdout has not been used for tuning.
