# M5a serial pass removal and scratch liveness

Engineering baseline: M4e measured source 3c1273eb00698c59b27eb79d1539c137fe3632a9, preserved v2 executable. Change neither fixed operator, factor order, tuple order, projection order, native tolerances, gate/final certificate nor Krylov recurrence.

Increment A removes two empty MAP coupling traversals. Forward factor 0 and reverse factor 2 subtract exact positive zero; all finite inputs including signed zero keep their bit patterns. Remove forward/solution whole-vector fills because each factor is fully copied before its first read. Replace the separate middle vector with in-place forward *= diagonal after all forward factors are complete. Every multiplication remains rounded and stored before the reverse sweep; no multiply/divide cancellation.

Cycle liveness before recursion: compatible RHS and result remain live; residual dies after restriction. During recursion coarse RHS/solution remain distinct and all parent frames remain borrowed. After recursion, reuse the dead residual vector for prolongation then for the post residual. Once the post residual is formed, the original compatible RHS is dead and may hold the post smoother correction. Thus each transition needs two fine and two coarse vectors, instead of five fine and two coarse. Remove fills only where complete overwrite is guaranteed by project/smooth/residual/restrict/prolong/terminal contracts; retain required reduction initialization and transactional root output.

Requested reduction: 8V bytes per nonterminal MAP plus 24V bytes per cycle transition, three fewer heap Vec descriptors per transition, and a smaller per-level optional MAP workspace descriptor (including the terminal level). Arrays removed: four per transition; no new persistent array. Runtime may improve but no win is assumed before a rotated baseline/candidate measurement. Arena flattening, fused prolong-add, grouped indices and certificate-reference caching stay separate follow-on increments.

Tests: compare the optimized scalar MAP to the exact pre-change loop on finite/heterogeneous/component/rank-deficient and signed-zero fixtures with poisoned scratch; repeat zero and nonzero applications after errors. Full ordinary/prepared cycle and PCG/LSMR equivalence, dense/Galerkin tests, all reservation failures, exact allocation/release, old/new hierarchy sizes and poisoned scratch must pass. Compare preserved executable fingerprints and numerical/work diagnostics on every frozen v2 case; capacities change only as derived. Run full Rust 1.85/all/minimal/rustdoc and platform allocation gates.

Measurement: commit source and an explicit baseline/candidate paired protocol before collecting. Reuse fixed v2 dimensions/input recipes/routes and 1 warmup/5 measured reps; rotate route order and baseline/candidate order. Record both executable build identities/logs; the frozen baseline binary is reused and compilation stays outside solve timings. Charge every isolated process, setup, solve/cert/output, teardown, failure, RSS and capacity. Preserve all results, including regressions. This is development evidence, never a competitive/holdout claim. Kernel-specific profiling still precedes layout selection.

The benchmark compares the actual reserved payload of all owners. On 64-bit targets with exact fresh capacities the expected hierarchy-workspace reduction is `32*sum(V_nonterminal) + 72*depth + 24*(depth+1)` bytes; descriptor sizes and actual capacities remain authoritative.


## Frozen comparison

The [M5a paired policy](../benchmarks/policies/prepared-serial-m5a-paired-v1.json)
fixes the M4 source, policy and preserved Mac/Linux executable hashes. The input
generator and v2 policy are unchanged. Each profile runs 1,512 processes: 252
warmups and 1,260 measured processes, with 14,400 measured columns. Alternate old/new
order by `(case_index + repeat + route_position) % 2`, while retaining the existing
three-route rotation within each source. Raw outputs and child manifests persist
after each attempt; a top-level trace charges actual order and global elapsed time.

Each child artifact is independently checked by the existing v2 validator. The
paired validator additionally verifies committed harness/policy hashes, pinned
baseline binaries, identical execution environment, complete pairing order,
per-process elapsed boundaries, input identity and exact complete numerical/work
reports. It checks the derived capacity reduction without hiding unchanged owners.
The M5a gate is a correctness/memory gate; timing ratios are reported separately.
Failures prevent an aggregate timing comparison, and native negatives remain.
This development increment is retained for proven lower scratch payload only
if all scientific gates pass; timing regressions remain explicit limitations
and cannot be labeled performance wins. M10 performance gates are unchanged.

```sh
python3 scripts/prepared_serial_pair.py /path/to/preserved-m4-v2 /path/to/new-pair --profile smoke
python3 scripts/validate_prepared_serial_pair.py /path/to/new-pair --require-m5a
```

Repeat with `--profile development` on the Mac from the same committed source.
The regular Linux native/gated smoke and three-platform allocation CI remain
required. Hosted Linux timings are not a substitute for SCC qualification.
No paired M5 timing was collected before the recipe/source commit.
