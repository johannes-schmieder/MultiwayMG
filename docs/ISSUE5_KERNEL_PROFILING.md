# Prepared serial diagnostic kernel profiling

M5b, declared 2026-09-06. This increment supplies attribution for the next memory
layout experiment. It does not select a layout or measure a competitive speedup.
The separate `profiling` Cargo feature is disabled by default. Ordinary benchmark
builds omit every instrumentation hook, timer and profiler thread-local state.

## Collection and cost boundaries

`multiway_incidence::profiling::collect` scopes a callback on the current thread.
A fixed 128-frame span stack and sixteen aggregate counters retain call counts,
inclusive nanoseconds and exclusive nanoseconds. There is no event-history vector
or profiler array allocation. Collection does not propagate to other threads.
Span guards cannot be sent across threads. Nested collection rejects before its
callback; panic unwinding restores availability. Open/out-of-order span guards,
stack/counter overflow and impossible nested time accounting invalidate a report.
Stale guards cannot alter a subsequent collection generation.

The callback timer covers the complete solve, all candidate and final original
certificates, output copy and coefficient fingerprint. Per-column and complete
probe costs additionally include profiler reset, extraction and retained records.
Inclusive categories overlap, particularly recursive cycles and weighted incidence;
sum only exclusive categories. Exclusive costs include observer overhead. The
callback remainder covers output/fingerprint and any uninstrumented outer work.
All phase timings aggregate hierarchy levels; the actual tuple and coefficient
counts of every level are recorded separately, not inferred from fine dimensions.

| Categories | Meaning and limits |
| --- | --- |
| Incidence, adjoint, weighted incidence/adjoint | Shared tuple kernels. Weighted incidence currently nests the unweighted pass. |
| Gramian, RHS | Shared weighted normal action and original RHS/gradient construction, including nested certificate/cycle calls. |
| Projection, MAP | Shared structural projection and ordered forward/middle/reverse arithmetic. |
| Restriction, prolongation, dense terminal | Shared transfer kernels and complete terminal application. |
| Cycle | Recursive frame duration. Exclusive remainder includes residual/copy/add work and validation. |
| PCG recurrence, prepared PCG, prepared LSMR | Nested solver scopes. Remainders include vector operations, reductions/history, validation and copies; they are not pure Krylov timings. |
| Certificate | Original-operator acceptance including its nested incidence and two RHS/gradient actions. |

Fixed thread-local state bytes, whole probe `Record` inline bytes and per-report
inline bytes are emitted separately from solver retained-array payload. These are
layout sizes, not a complete stack peak: compiler copies, TLS runtime backing and
metadata are not separately observed. Process RSS includes them. Initial TLS
runtime initialization is not claimed free merely because Rust allocator tests
observe no events. The evidence validator checks reported payload plus fixed
instrumentation sizes against measured RSS and leaves unobserved scopes unknown.

## Frozen development protocol

[The dedicated v1 policy](../benchmarks/policies/prepared-kernel-profile-v1.json)
builds with `--no-default-features --features lsmr,profiling`. It preserves v2 input
generation, seed 10001, native/certificate tolerances, iteration/window limits,
serial thread caps, complete costs, process deadline and 1 GiB process admission.
Routes are PCG and gated LSMR. Each cell has one warmup and five rotated measured
repetitions; all failures and certificates remain in the artifact.

- Smoke: 16 levels per factor, 512 draws, one transition; RHS 1,2,4,8,16,17,32.
- Development: 128 levels per factor, 16,384 draws, three transitions; same widths.
- Expanded: 4,096 levels per factor, 65,536 draws, eight transitions; **RHS 1 only**.
  This separately declared larger diagnostic remains below 100,000 unique tuples
  and uses supplied contiguous-halving maps. It is not balanced RHS qualification
  or a substitute for larger automatic-hierarchy/SCC tests.

Commit source and recipe before collecting, then run sequentially:

```text
python3 scripts/prepared_kernel_profile.py <fresh-directory> --profile smoke
python3 scripts/prepared_kernel_profile.py <fresh-directory> --profile development
python3 scripts/prepared_kernel_profile.py <fresh-directory> --profile expanded
python3 scripts/validate_prepared_kernel_profile.py <directory> --require-diagnostics
```

The normal v1/v2 parser and generator remain unchanged. The diagnostic runner
separates extra TSV profile records before the common base parser. Its validator
explicitly admits only the committed diagnostic policy/scope/build, verifies
Git/tree/file/input/binary/raw hashes and actual per-level inventory, checks call
identities against complete solver/certificate work, and checks exclusive totals
against callback and column time. Fixed-configuration profile depth, call counts
and memory must repeat; clock values need not. Invalid reports remain negative
results. The normal validator continues to reject this diagnostic scope.

Permanent Linux CI collects and validates a diagnostic smoke alongside existing
uninstrumented serial smokes. Windows/all-feature allocator tests exercise active
profiling. Local required Rust 1.85 checks and 73 Python evidence tests pass before
collection. Allocator regressions cover first/repeated PCG/gated LSMR collection,
exact coefficient/report equivalence, recursive/certificate work identities and
failure recovery. Profiler tests cover malformed lifetimes, overflow, panic and
cross-thread guard rejection. No benchmark collection exists at this checkpoint.

## Next experiment

Use the attribution and measured level inventory to prioritize the
[grouped-layout alternatives](ISSUE5_GROUPED_LAYOUT_DESIGN.md). Preserve scalar
scatter as a candidate. Compare exact stable row gather and two-pass tuple-image
kernels with charged grouping/workspace setup and complete **uninstrumented**
paired solves before selecting any default. Keep scratch arenas, fused transfer
and certificate-reference reuse as individually reviewable hypotheses.
