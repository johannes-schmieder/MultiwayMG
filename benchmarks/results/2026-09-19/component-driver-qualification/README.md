# Component-driver qualification (M6h)

The [records](records.json) preserve eleven serial setup/execute/drop peak-memory
controls, identical between Rust 1.85 debug and release. Each repeats twice at
K=1,2,4,8,16,17,32: 1,760 accepted columns per configuration. All net allocation
counts/bytes are zero; caller/input owners are separately charged to admission.
Positive controls validate simultaneous, disjoint and realloc lifetimes. This
is requested heap accounting, not RSS or parallel allocation measurement.

Dense peaks at n=6,130,255,256 exactly match `8*(2*n*n+3*n-2)`. Forty small
components need only 7,360 peak newly allocated bytes; two large local roots use
531,120 peak bytes and release each local hierarchy before building the next.
Those numbers exclude original/caller arrays, which are included in admission.

Seven complete-driver scientific tests independently certify original-problem
outputs and preserve failed construction, screens and final nonconvergence.
Private root/frame tests inject every new reservation failure and unwind;
borrow lifetimes, current weights, owner errors and exact budgets are covered.
All six required check groups (including 90 Python checks) and seven release
check groups (including 120 protocol comparisons) pass. Source-file hashes and
raw log hashes are preserved; full logs live outside the repository under
`$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6h`.

See [the API and limitations](../../../../docs/ISSUE5_COMPONENT_DRIVER.md).
No automatic timing, competitive/default promotion or holdout result is claimed.

Count correction (2026-09-19, M6j review): the earlier derived total of 1,980
was an arithmetic error. Eleven controls, two repetitions and widths summing
to 80 give 1,760 certified columns in each LSMR-enabled debug/release build.
The minimal-feature executable exercises component-root controls only.
`records.json` records the correction and original file hash; every raw peak,
allocation record, source hash and numerical outcome is unchanged.
