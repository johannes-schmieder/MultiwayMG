# Certificate vector liveness (M5h)

The original-operator certificate needs the residual vector E, the residual
adjoint V, and the target adjoint V. The two coefficient vectors need not be live
at the same time: retain only the scaled residual-adjoint norm, then overwrite
its vector with the target adjoint. The certificate workspace now owns E+V
floating-point values in two arrays, saving 8V heap bytes and one allocation.
There is no target cache and no change to the mathematical certificate.

Both attempted adjoints and the original failure precedence are preserved.
Gradient finiteness and norm form an inert Result; the target adjoint is still
attempted before resolving that result. Reference finiteness, both norms,
zero/zero, ratio and ratio-underflow checks then run in their prior order.
Every admitted certificate starts with fresh residual/adjoint actions. Exact
frame/dimension/input checks precede scratch and work-counter mutation.

| Two-transition complete workspace | M5g arrays | M5h arrays |
| --- | ---: | ---: |
| Certificate alone | 3 | 2 |
| Cycle | 14 | 14 |
| PCG plus certificate | 25 | 24 |
| Native/gated LSMR plus certificate | 31 | 30 |

These counts cover scalar, row and image layouts. Hierarchy scratch is unchanged;
only certificate-containing outer workspaces lose 8V heap bytes. Inline object
layout also changes, but stack high-water and copies are unmeasured; no RSS gain
is inferred. Local LSMR history and every numerical element needed by the
recurrence remain intact. No solver tolerance, iteration limit or work count
changes. Scalar remains default.

Fault-injection tests distinguish reference action errors, gradient/reference
nonfiniteness, overflowing norms, zero denominator and underflow. They check
both attempted adjoints and successful recovery. Existing independent numerical,
owner, static-input, zero/zero, extreme-weight, complete-solve and zero-allocation
checks remain required. These injected actions test errors, not performance.

Run all Rust1.85 checks, Python evidence tests, release complete-solver/allocator
gates and actual layout protocol comparisons. Commit source before the unchanged
v2 layout regression on Mac smoke/development/expanded and exact-source Linux
smoke. Compare every corresponding M5g input/numerical/work/fingerprint record,
allowing only 8V reduction in outer_workspace/total payload. Layout, grouping,
hierarchy payload and failure coverage must remain exact. Preserve complete
costs and all attempts. No measured M5h timing or qualification exists at this
source checkpoint; calibration/holdout remain untouched.

Reference-norm caching across repeated certificates is a separate hypothesis.
M5b instrumented scalar profiles place the full certificate inclusive share at
median 3.43% PCG/5.10% gated LSMR of development callbacks, and 0.51%/1.33% on
expanded one-RHS. These are diagnostic prior-source shares, not current layout
speed estimates. They bound only the older full certificate cost; caching could
save a subset. Prioritize scalable construction after the exact liveness gate,
then reconsider a scope-bound reference token using complete current costs.
Never reuse a passing candidate certificate without a separate immutability and
accounting contract. M5/M6 closure must distinguish this evaluated hypothesis
from an implemented cache.

Required Rust1.85 format, strict Clippy, all/minimal workspace tests and warning-free
docs pass, together with 90 Python evidence tests. Release all/minimal complete
solver and allocator tests pass; private fault precedence also passes in release.
The release executable passes 120 layout protocol comparisons with exact legacy
scalar numerical/work/payload records and all explicit layouts. Freeze this
source before full v2 regression collection.
