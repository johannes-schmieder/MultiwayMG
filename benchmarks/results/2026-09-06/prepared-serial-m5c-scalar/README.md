# M5c preserved scalar baseline

Measured source `6cdd0f70bc57a9d4e241d953b700b4890f640398` was committed before
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
M5a candidate exactly in input hashes, complete numerical/work/fingerprint and
solver-payload records, including native negative controls and warmups.

The [receipts](receipts.json) identify both compared manifest hashes, exact
source/tree/policy, binaries and deterministic archive hashes. Canonical summaries
retain every cell's complete cold-process/setup/solve/certificate/output timing,
RHS-prefix costs, RSS and memory scopes. These separately collected runs are not
a paired speed comparison against M5a. Competitive qualification remains false.

Full manifests, build logs, exact binaries and every TSV/resource output are
preserved outside Git under
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/` as
`{mac-smoke,mac-development,linux-smoke}-m5c-scalar-6cdd0f7`, with individual
checksum inventories and deterministic archives. Copied artifacts are independently
revalidated and all archived members are rehashed. This local preservation is
not an independent off-machine backup.

Source workflows `34068763134`/`34068763121` and PR workflows
`34068919030`/`34068919027` passed. The Linux artifact is from exact-source run
`34068763134`. All local Rust 1.85 required checks, 73 Python evidence tests,
release grouped-operator/Galerkin/width/MAP/frozen-loop checks and complete
allocator contracts passed before collection. Final evidence-head CI/merge is
pending at this checkpoint. See the [M5c primitive contract](../../../../docs/ISSUE5_GROUPED_KERNELS.md)
for the grouped implementation and its remaining complete-solve boundary.
