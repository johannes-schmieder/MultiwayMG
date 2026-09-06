# Prepared serial v1: preserved development baseline

Actual measured source: `9b4f1740ce5e9bbb399059fe1615adbd10820ab7`, committed before collection.
Policy and generator were unchanged across all three runs. All evidence validators passed;
fixed-configuration numerical fingerprints, action counts and capacities repeated exactly.
No process errors, timeouts or observed RSS-budget failures occurred. The independent
numerical-coverage gate failed, as recorded below. No competitive claim is eligible.

| Run | Measured columns | Certified PCG | Certified LSMR | Rejected columns |
| --- | ---: | ---: | ---: | ---: |
| Mac smoke | 4,800 | 2,400/2,400 | 1,840/2,400 | 560 |
| Linux smoke | 4,800 | 2,400/2,400 | 1,840/2,400 | 560 |
| Mac development | 4,800 | 2,400/2,400 | 1,465/2,400 | 935 |

Each run has 504 fresh processes: 84 warmups and 420 measured, with five paired
repetitions. Rejected columns are included in the counts and all timing tables.
Every rejection here is LSMR native `NormalEquationTolerance` with an original
certificate above 1e-8; the maximum development certificate was about 9.29e-8.
Native and original-operator stopping metrics differ. Final certification correctly
refuses acceptance. PCG certificates all pass on this bounded supplied-map matrix.

The Mac is an Apple M2 Ultra, 192 GiB, 24 logical CPUs; execution is serial with
unpinned placement. Linux is a hosted CI smoke, not the planned SCC qualification.
These tiny diagnostics neither establish production coverage nor identify a winning
automatic hierarchy. Existing frozen upstream baselines are untouched.

## Mac development observations

Numbers are median inner API milliseconds over five repetitions at widths 1 and 32.
Inner time includes decode, fresh preparation, all solves/certificates, output and
destruction. External cold process time, dispersion, per-phase costs and RSS remain
in the full summaries/raw evidence. Retained MiB includes caller inputs/output.
Certified column counts below describe one repetition; repeats agree.

| Family | Weights | Route | RHS1 ms | RHS32 ms | Certified at RHS32 | Retained MiB at RHS32 | Peak RSS MiB at RHS32 |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| uniform | unit | pcg | 12.843 | 172.568 | 32/32 | 6.966 | 9.219 |
| uniform | unit | lsmr | 14.402 | 242.661 | 18/32 | 7.518 | 9.828 |
| uniform | heterogeneous | pcg | 14.246 | 218.470 | 32/32 | 6.966 | 9.219 |
| uniform | heterogeneous | lsmr | 17.428 | 312.846 | 32/32 | 7.518 | 9.828 |
| communities | unit | pcg | 10.251 | 138.316 | 32/32 | 5.741 | 7.984 |
| communities | unit | lsmr | 12.662 | 188.671 | 19/32 | 6.266 | 8.531 |
| communities | heterogeneous | pcg | 10.202 | 156.872 | 32/32 | 5.741 | 7.969 |
| communities | heterogeneous | lsmr | 12.317 | 220.604 | 32/32 | 6.266 | 8.547 |
| chain | unit | pcg | 7.818 | 159.860 | 32/32 | 2.894 | 5.125 |
| chain | unit | lsmr | 7.951 | 179.701 | 10/32 | 3.194 | 5.484 |
| chain | heterogeneous | pcg | 8.146 | 181.982 | 32/32 | 2.894 | 5.125 |
| chain | heterogeneous | lsmr | 8.758 | 203.910 | 12/32 | 3.194 | 5.484 |

The development matrix realizes 8,271–16,571 unique tuples and 384 coefficient
coordinates. At width 1, fresh preparation is a substantial part of inner cost;
at width 32, complete solve/certificate work dominates. Binary decoding dominates
much of the measured preparation at large RHS widths, so API and process/serialization
costs must continue to be reported separately. This harness does not yet separate
MAP, transfers and Krylov reductions inside the solve phase. No layout is declared
best based on these coarse timings.

## Required next increment

Before M5 optimization, add a separate certificate-aware LSMR path that can veto
a native tolerance stop without destroying the current recurrence. Keep the legacy
and frozen prepared v1 results unchanged. Charge each vetoed candidate certificate
and preserve final independent acceptance, native diagnostics, iteration limits,
breakdown/failure recovery and zero-allocation storage. Do not tune v1 tolerances
or silently replace its rejected runs. Qualify the new behavior on dense/algebraic
and allocation tests, then record a distinct declared development comparison.

## Retrieval and checks

[`receipts.json`](receipts.json) gives source/tree, policy, executable, manifest
and complete archive SHA-256 hashes. Raw artifacts and exact executables are
preserved outside Git at
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
(portable alias); each receipt also gives the known absolute path.
The Linux exact-source [CI run](https://github.com/johannes-schmieder/MultiwayMG/actions/runs/34059425511)
has the additional `prepared-serial-smoke` artifact, subject to GitHub retention.
Local archives are a preservation copy, not a guaranteed independent backup.
Use the measured source generator to revalidate an extracted artifact; the Git
object for its measured source must be available.

Pinned local Rust 1.85 checks, decoder regression and all 46 Python validator
checks passed. Exact-source CI `34059425511`/`34059425501` and PR CI
`34059472224`/`34059472243` passed before this evidence commit.
