# Prepared serial gated v2: M4 coverage

Measured source `3c1273eb00698c59b27eb79d1539c137fe3632a9` and its v2 recipe were
committed before collection. Every complete artifact validates, including raw
hashes, executable/source identity, fixed-configuration numerical/work/capacity
repeatability and predeclared candidate-route coverage. No process error, timeout
or observed RSS-budget failure occurred. The native control remains negative.

| Profile | Measured columns | PCG certified | Native LSMR certified | Gated LSMR certified | Native rejects |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mac smoke | 7,200 | 2,400/2,400 | 1,840/2,400 | 2,400/2,400 | 560 |
| Linux smoke | 7,200 | 2,400/2,400 | 1,840/2,400 | 2,400/2,400 | 560 |
| Mac development | 7,200 | 2,400/2,400 | 1,465/2,400 | 2,400/2,400 | 935 |

Each profile has 756 fresh processes: 126 separate-process warmups and 630
measured runs, five repetitions with the declared three-route rotation. The
original 1e-8 certificate and native settings are unchanged. Native PCG/LSMR
input hashes, coefficient fingerprints and all numerical diagnostics match v1
exactly in all 504 corresponding Mac processes per profile. The schema-2 LSMR
projection counter additionally reports the already charged final projection.

Gated LSMR attempted 2,870 candidate checks with 560 vetoes per smoke, and 3,335
checks with 1,025 vetoes in development. At most one candidate was vetoed per
smoke solve, and two per development solve. Every final candidate received a
fresh original certificate. Maximum gated final certificate was 9.34e-9 on the
smokes and 9.71e-9 in development. Zero RHS and other non-resumable exits remain
subject to final certification. All extra work is included in the artifact.

## Complete costs and limits

The following are median inner milliseconds at RHS width 32, including input
decode, fresh preparation, solves/certificates, output and destruction. All five
repetitions, cold-process wall/CPU, dispersion, phase times, per-RHS prefixes,
work and memory scopes remain in the summaries and raw manifests. Native LSMR
has rejected columns, so its smaller time is not a time-to-certified-solution
comparison. These numbers do not establish competitiveness.

| Family | Weights | PCG ms | Native LSMR ms | Gated LSMR ms | Gated retained MiB | Gated peak RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| uniform | unit | 184.755 | 262.020 | 294.106 | 7.518 | 9.891 |
| uniform | heterogeneous | 230.784 | 329.676 | 335.909 | 7.518 | 9.906 |
| communities | unit | 148.309 | 202.588 | 224.031 | 6.266 | 8.547 |
| communities | heterogeneous | 165.015 | 233.639 | 241.175 | 6.266 | 8.547 |
| chain | unit | 170.327 | 190.592 | 208.032 | 3.194 | 5.484 |
| chain | heterogeneous | 196.732 | 220.030 | 232.199 | 3.194 | 5.531 |

Gated/native LSMR retain identical array capacities on every case. The Mac is an
Apple M2 Ultra with 192 GiB and 24 logical CPUs; this serial run is unpinned.
The development inputs have 8,271–16,571 tuples and 384 coefficients. Hosted
Linux smoke is correctness/coverage evidence, not SCC performance qualification.
Fine-grained kernel, transfer and reduction profiles remain an M5 prerequisite
for choosing a layout. No automatic hierarchy, large workload, changed-weight,
thread/panel, calibration or holdout qualification is claimed here.

## Retrieval and validation

[Receipts](receipts.json) identify exact source/tree, policy, executable, manifest,
summary and archive SHA-256 hashes. Full copies, binaries and deterministic
archives are outside Git at
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`;
receipts also record current absolute paths. The exact-source Linux artifact
`prepared-serial-gated-smoke` is from [CI run 34063265592](https://github.com/johannes-schmieder/MultiwayMG/actions/runs/34063265592).
These are local preservation copies, not independent off-machine backups.
Use the measured generator and available source Git objects to revalidate.

Required local Rust 1.85 checks and all 51 Python evidence tests passed before
collection. Exact-source workflows `34063265592`/`34063265572` and PR workflows
`34063297969`/`34063297964` passed. Final evidence-head CI and merge remain the
last M4 boundary. Preserve v1 negatives and immutable upstream baselines; use
this qualified serial executable as the M5 engineering baseline only.
