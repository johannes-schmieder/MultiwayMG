# Prepared serial kernel diagnostics v1

Measured source: `42d6f527bdf09a2b38769d5724b88829cce97877`, committed with the
[policy](../../../policies/prepared-kernel-profile-v1.json) before collection.
The [M5b protocol](../../../../docs/ISSUE5_KERNEL_PROFILING.md) defines exact
scope, memory and observer limitations. These are **instrumented diagnostics**;
no speedup, layout selection or competitive qualification follows from them.

## Complete evidence

| Artifact | Processes, including warmup | Measured RHS columns | Independently certified | Profile gate |
| --- | ---: | ---: | ---: | --- |
| Mac smoke | 504 | 4,800 | 4,800 | Pass |
| Mac development | 504 | 4,800 | 4,800 | Pass |
| Mac expanded, RHS 1 only | 72 | 60 | 60 | Pass |
| Linux smoke | 504 | 4,800 | 4,800 | Pass |

Both PCG and gated LSMR certify every submitted measured column. All profiles
have valid nested accounting, complete action identities, actual level inventory
and exact fixed-configuration numerical/work/memory repeats. There are no process
errors, timeouts, certificate rejects or RSS admission failures. The expanded
maximum certificate is 9.98631e-9 against 1e-8; its iterations span 5–174 and
maximum observed process RSS is 37,404,672 bytes. This is bounded supplied-map
execution on development seed 10001, not automatic-hierarchy or holdout evidence.

For each Mac smoke/development artifact, all 504 corresponding process inputs,
numerical reports, work, coefficient fingerprints and solver payloads exactly
match the preserved uninstrumented M5a candidate. Comparison includes warmups and
all RHS columns, while excluding clocks and explicit instrumentation records.
[Attribution and equivalence receipts](attribution.json) state the calculation.

Each tested platform reports 6,960 bytes of fixed thread-local state, 33,696 bytes
for the complete inline probe record and 800 bytes per report. The full record
already contains its column reports; do not add them a second time. These sizes
do not claim a complete stack/TLS-runtime peak. Retained solver payload excludes
instrumentation and remains identical to M5a; isolated process RSS includes it.

## Attribution and implications

Shares below are the median across cells of exclusive phase median divided by
callback median. They include observer overhead. Different phase medians need
not sum to 100%; inclusive categories must never be summed.

| Mac development route | MAP sweeps | Gramian actions | Implication |
| --- | ---: | ---: | --- |
| PCG | 64.27% | 26.44% | Prioritize exact grouped MAP/Gramian experiments. |
| Gated LSMR | 62.95% | 18.94% | Same priority; weighted adjoint is another 5.24%. |

Across expanded cells, MAP occupies 51.81–65.90% and Gramian 16.87–25.57% of the
instrumented callback. Transfer fractions are small in the current development
matrix. Scratch arenas, fused transfer and certificate-reference reuse remain
valid hypotheses but follow the dominant tuple-kernel experiment.

The expanded level inventory also changes memory expectations. The sum of
nonterminal tuples divided by fine tuples is 6.7092 for uniform, 5.5765 for
communities and 1.80835 for chain. Although coefficient dimensions halve each
level, tuple counts may remain high for many levels. Adding narrow grouped
indices and offsets at **every nonterminal level** would require about
4,153,168 / 3,485,104 / 1,150,464 requested bytes respectively, before descriptors
and construction cursors. These are layout estimates from actual counts, not
measured allocations of an implemented grouping.

Accordingly, compare scalar scatter, stable row gather and tuple-image gathering
with explicit per-level layout choices and full hierarchy memory admission.
Keep fine-only and all-nonterminal choices visible before any selector. M6 must
admit aggregate tuple/operator complexity before numerical candidate construction;
coefficient reduction alone is insufficient. The current V=12,288 expansion
still does not establish behavior beyond CPU caches or at the campaign's large
SCC scales. Preserve that distinction when evaluating grouping or threading.

## Provenance and preservation

[Receipts](receipts.json) identify source/tree/policy, exact Mac/Linux binaries,
manifest and deterministic archive hashes. Full TSV/resource outputs, manifests,
build logs and binaries are preserved outside Git under
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/` as
`{mac-smoke,mac-development,mac-expanded,linux-smoke}-profile-42d6f52`, each with
its own checksum inventory and deterministic archive. Preserved copies are
independently revalidated; every archived member is rehashed against its copy.
This local preservation is not an independent off-machine backup.

Canonical summaries retain every phase, level, complete-cost timing and declared
memory scope. The evidence validator marks `authoritative_performance_measurement`
and `competitive_qualification` false. Normal v1/v2 validators reject this
separate diagnostic scope. Authoritative complete-cost comparisons must use
uninstrumented executables and the frozen paired-performance rules.

Source workflows `34067385068`/`34067385070` and PR workflows
`34067398348`/`34067398347` passed, including all three allocator platforms and
unchanged uninstrumented native/gated serial smokes. Linux diagnostic artifact
comes from exact-source run `34067385068`. All required local Rust 1.85 checks,
73 Python evidence tests and the benchmark input test passed. Final evidence-head
CI and merge remain at this recorded checkpoint; M5 remains open.
