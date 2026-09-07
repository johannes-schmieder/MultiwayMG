# M5d preserved scalar baseline

Measured source `d11917f152704973c673728d85b3eae1190f76b5` was committed before
collection. These uninstrumented runs use the unchanged
[gated serial v2 policy](../../../policies/prepared-serial-gated-v2.json) and
canonical input generator. They preserve exact executables and complete costs
for the subsequent grouped hierarchy comparison. They execute scalar layouts;
no grouped-kernel timing or speedup is claimed.

| Artifact | Processes including warmups | Measured columns per route | PCG certified | Gated LSMR certified | Native LSMR certified/rejected |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mac smoke | 756 | 2,400 | 2,400 | 2,400 | 1,840 / 560 |
| Mac development | 756 | 2,400 | 2,400 | 2,400 | 1,465 / 935 |
| Linux smoke | 756 | 2,400 | 2,400 | 2,400 | 1,840 / 560 |

All three complete artifacts pass the evidence and eligible-route certification
gates. There are no process errors, timeouts or RSS-budget failures. Numerical,
work, payload and coefficient fingerprint records repeat exactly at fixed
configuration. All **2,268 corresponding processes** also match the preserved
M5c scalar baseline exactly in input hashes, complete numerical/work/fingerprint and
solver-payload records, including native negative controls and warmups.

The [receipts](receipts.json) identify both compared manifest hashes, exact
source/tree/policy, binaries and deterministic archive hashes. Canonical summaries
retain every cell's complete cold-process/setup/solve/certificate/output timing,
RHS-prefix costs, RSS and memory scopes. These separately collected runs are not
a paired speed comparison against M5c. Competitive qualification remains false.

Full manifests, build logs, exact binaries and every TSV/resource output are
preserved outside Git under
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/` as
`{mac-smoke,mac-development,linux-smoke}-m5d-scalar-d11917f`, with individual
checksum inventories and deterministic archives. Copied artifacts are independently
revalidated and all archived members are rehashed. This local preservation is
not an independent off-machine backup.

Source workflows `34071799728`/`34071799760` and PR workflows
`34071824688`/`34071824642` passed. The Linux artifact is from exact-source run
`34071799728`. All local Rust 1.85 required checks, 73 Python evidence tests,
release complete grouped-hierarchy/integration checks and complete allocator
contracts passed before collection. Final evidence-head CI/merge is
pending at this checkpoint. See the [M5d complete integration contract](../../../../docs/ISSUE5_GROUPED_HIERARCHY.md)
for the explicit grouped layouts and their allocation/ownership gates.

The first local collection attempt stopped during sandbox-blocked hardware
inventory before any case or output artifact was created. Its preflight log is
retained; the successful collection used actual hardware/RSS access. No measured
run was retried or removed. The dependency norm fix is separately qualified fork
PR #3, pinned at `fad1d462d44e7d5b5226370021d69dab4e854669`.
