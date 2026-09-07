# Fused transfer complete-layout regression (M5f)

Source `a95c84ec618429194da5184ac8abff124c680323` was committed before
collection. The unchanged [five-layout policy](../../../policies/prepared-layout-v1.json)
runs scalar, fine-row, all-row, fine-image and all-image with PCG/gated LSMR.
This qualifies the [fused prolong-add change](../../../../docs/ISSUE5_FUSED_TRANSFER.md)
against frozen M5e records; it is not an old/new paired timing experiment.

| Artifact | Processes including warmups | Measured columns across layouts/routes | Prior M5e records matched exactly |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 2,520 |
| Mac development | 2,520 | 24,000 | 2,520 |
| Mac expanded, one RHS | 360 | 300 | 360 |
| Linux smoke | 2,520 | 24,000 | 2,520 |

All 7,920 complete processes pass accounting, certification, fixed-configuration
repeatability and RSS gates. All 72,300 measured columns certify. There are no
process errors, timeouts, rejected columns or measured retries. The 6,336
within-source scalar/layout comparisons remain exact. Every corresponding
process also matches frozen M5e input hashes, dimensions, numerical/work/
coefficient fingerprints, retained payload, grouping admission and layout
inventory exactly, including warmups. No scratch, tolerance or iteration change
is hidden in the fused pass.

The implementation removes the logical temporary store/read of one fine vector
per nonterminal transition. All retained solver capacities and allocation counts
remain unchanged. Complete raw timing/setup/solve/certificate/output costs, RHS
prefixes, RSS and all scoped memory categories remain in the canonical summaries.
Their layout ratios compare layouts within this committed binary, not M5f to
M5e. No measured transfer speedup or DRAM-bandwidth reduction is claimed.
Small-case layout regressions remain visible and scalar remains default.

The [receipts](receipts.json) identify source/tree/policy, exact executable,
manifest, prior M5e manifest and deterministic archive hashes. Full artifacts
are preserved under
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-fused-a95c84e`.
Copied artifacts independently revalidate, saved summaries reproduce with the
qualified interpreter, and every archived member is rehashed. This local copy
is not an independent off-machine backup.

All required Rust1.85 checks and 88 Python evidence tests passed before source
freeze. Release transfer arithmetic, complete prepared/grouped cycle, PCG,
native/gated LSMR and full allocator gates pass in all/minimal configurations.
Source workflows `34077448539`/`34077448504` and PR workflows
`34077479671`/`34077479681` passed; Linux evidence is from source run
`34077448539`. Self-review checks shape-before-write, factor offsets, exact
addition order, numerical failure/output transaction boundaries and unchanged
payload/work. It is not independent external review. Final evidence-head
qualification and merge remain at this checkpoint.

These are bounded supplied-map development inputs. Expanded covers one RHS,
V=12,288 and at most 73,726 tuples. Out-of-cache, large multi-RHS, automatic
hierarchy, changing-weight and CPU scaling gates remain open. Campaign calibration
and holdout remain untouched; competitive qualification remains false.
