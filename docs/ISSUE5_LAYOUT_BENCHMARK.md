# Paired explicit layout experiment (M5e)

M5d merged complete grouped execution with unchanged scalar numerical payloads.
The new [policy](../benchmarks/policies/prepared-layout-v1.json) compares five
explicit layouts, with no automatic selection or performance claim at this
pre-collection checkpoint. Commit the policy, harness and source before timing.

| Layout | Nonterminal prefix | Gramian | Extra solver scratch |
| --- | --- | --- | --- |
| scalar | none | canonical tuple scatter | none |
| fine-row | one | stable coefficient-row gather | none |
| all-row | full depth | stable coefficient-row gather | none |
| fine-image | one | tuple image then stable row gather | one E-vector + descriptor |
| all-image | full depth | tuple image then stable row gather | one maximum-E vector + descriptor |

All selected levels also use grouped MAP. The scalar original-operator
certificates, exact current weights, fixed supplied maps, native options and
independent scalar RHS reuse remain identical. Scalar remains the library default.
Depth-one smoke intentionally makes fine/all storage equivalent; depth-three
and depth-eight inputs distinguish their full hierarchy costs.

## Declared scope and pairing

Reuse the unchanged canonical v2 input generator, families uniform/communities/
chain, unit/heterogeneous positive weights and development seed10001. Smoke
(n16,512 draws,depth1) and development (n128,16384 draws,depth3) use all widths
1,2,4,8,16,17,32. Expanded (n4096,65536 draws,depth8) uses one RHS only, matching
M5b's larger diagnostic inventory. It is still a supplied-map, cache-bounded
engineering comparison. Larger multi-RHS, out-of-cache, automatic hierarchy and
SCC competitive qualification remain later gates. Campaign holdout stays untouched.

Compare PCG and certificate-gated LSMR. Native-control negative evidence remains
in the M5d scalar regression. Each case/route has one separate-process warmup
and five measured repetitions. Rotate route order by repetition; rotate layout
order using case index, repetition and the stable route index. Each layout then
occupies every position once for that route's five measured repetitions. Using
the *rotating route position* instead was caught by a schedule test before this
policy was committed or any performance collection occurred.

Every run is an isolated cold process, with six thread environment caps set to
one, a 60-second process-group timeout and 1GiB admission/RSS budget. Record
actual hardware, target/compiler, available placement, exact tree/binary/input/
policy hashes, raw outputs, RSS and CPU usage. No retries, dropped failed routes,
tolerance adjustment or online timing-based selection. Compilation and canonical
input generation lie outside the submitted-tuple solve boundary and are recorded
separately; decode, setup, solve, certificates, output and destruction are charged.

## Construction, memory and output protocol

Build grouping after structural maps and before coarse numerical frames. Admit
fine/coarse structure, input arrays, the live fine frame, all selected grouping
arrays/descriptors and only the currently live construction cursor. Carry retained
group bytes into subsequent coarse-frame and solver-workspace admission. Retained
payload is distinct from requested construction peak, allocator excess, stack
copies and process RSS. The full fixed benchmark record's inline byte size is
reported, including layout metadata, and must repeat for the same binary.

Explicit layout arguments produce schema3, while the ordinary scalar invocation
retains schema2. Schema3 adds grouping phase/payload, selected prefix/mode, ABI
sizes, actual per-level tuple/coefficient/index-width inventory, requested setup
peak and the shared image length. For these bounded 64-bit recipes all IDs are
narrow. Group arrays cost `64*prefix + sum(8*(V_level+3)+8*E_level)` at the
qualified Rust1.85 descriptor layout. Image mode adds `8*max(E_selected)+24`
bytes to the actual scalar hierarchy workspace once. Other solver payload
categories must match scalar exactly. No alternative tuple or weight copies.
Profiling plus explicit layout is rejected, so diagnostic hooks cannot enter the
new authoritative timing route. The original profiler and v1/v2 workflows remain.

The validator reconstructs the entire coarsened tuple inventory from frozen
inputs, proves grouping/cursor/owner sums and one-image deltas, checks every
phase/prefix/failed-action boundary and RSS scope, verifies raw records/hashes
and deterministic nonoverlapping paired order, and compares complete numerical/
work/fingerprint results across all layouts and fixed-configuration repeats.
Black-box debug tests independently compare schema3 scalar records to the
ordinary schema2 path and exercise malformed-input and unknown-layout rejection.
These synthetic/debug fixtures are tests, not performance evidence.

## Reporting and acceptance

Preserve every attempted process cost, timeout and rejection. Report paired
per-repetition scalar/layout process and inner-time ratios, their median within
each cell, and balanced geometric means across family/weight/width cells for
each route. A cell with incomplete or uncertified measured pairs has no qualified
speedup; an incomplete route has no balanced speedup. Keep raw negative costs.
The full comparison gate requires all pairs and certificates, exact numerical/
work identity and complete physical payload accounting. Passing it demonstrates
a valid layout comparison, not competitive performance or a default selector.

No M5e performance collection has occurred at this checkpoint. Required local,
release/protocol and adversarial checks precede the measured source commit;
Linux CI runs the frozen smoke, and Mac runs smoke/development/expanded. Preserve
exact executables, raw evidence, archives and compact canonical receipts before
closing this increment. Update the development ledger and measured limits.


Pre-collection local qualification passes: every required Rust1.85 check,
88 Python adversarial tests, and 120 release black-box comparisons over all
layouts, two families/weight regimes, PCG/gated LSMR and widths1/17/32. The
release fixture record is 7,440 inline bytes on this Mac. Timeout fixtures retain
full attempted costs and suppress incomplete-route speedups. The measured source
commit and exact-source/PR CI precede any performance conclusions.
