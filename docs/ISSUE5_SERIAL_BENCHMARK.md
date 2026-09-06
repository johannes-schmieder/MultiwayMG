# Prepared serial development benchmark v1

The final M4 increment measures the complete supplied-map PCG and LSMR vertical
slice. This is an engineering baseline for M5, not competitive qualification.
The original upstream comparators, automatic hierarchy, ten-family balanced
matrix, observation collapse experiments, changing weights and Mac/SCC holdout
remain required by [the performance protocol](PERFORMANCE_PROTOCOL.md).

## Frozen input and execution contract

[The policy](../benchmarks/policies/prepared-serial-v1.json) and
[integer generator](../scripts/prepared_serial.py) must be committed before a
run. The runner requires a clean tree, builds with Rust 1.85.0 and locked release
LSMR-only features, records source/tree/lock/generator/policy/executable hashes,
and refuses undeclared compiler overrides or Cargo build/profile settings.
The same recipe generates canonical sorted unique tuples for both routes.
Decode and validate the actual binary input inside the measured executable;
raw-observation generation/collapse is outside this declared entry boundary.
No duplicate weight aggregation or observation throughput is claimed here.

The bounded smoke uses 16 levels per factor and 512 integer-generated draws;
the development profile uses 128 levels and 16,384 draws. Both add diagonal and
neighbor bridge tuples to connect every factor level, deduplicate and sort, then
assign weights. Families are uniform, four contiguous communities, and local
chain/ring coupling. Record realized E and V. Unit weights are exactly one per
unique tuple; heterogeneous weights are powers of two with exponents -6 through
6. Community bridges receive another factor of 2^-10. No positive edge is removed.
Maps halve each factor contiguously: depth 1 for smoke, 3 for development.
These supplied maps are diagnostic, not an automatically selected solver.

Seed 10001, widths 1,2,4,8,16,17,32, both routes and both weight regimes give 84
cells. Each has one separate-process OS-cache warmup and five measured fresh
processes: 504 processes total, including 420 measured runs. Rotate route order
within each paired repetition. An isolated process does not reuse the warmup's
allocator, numerical state or workspace. Every measured process pays fresh setup;
only scalar columns within that process reuse prepared state. Columns 16 and 31
(zero based) are zero. Every fourth column with index 1 modulo 4 is a manufactured
fitted value; remaining columns use dyadic pseudo-random targets. The single RHS
is nonzero. Target prefixes are identical across widths apart from the header.

Both native solvers use their fixed 1e-8 tolerance, 1,000 iteration cap and an
independent original-operator certificate tolerance of 1e-8. LSMR retains its
window of eight; PCG recomputes every 25 iterations. The terminal relative rank
tolerance is 1e-12, with the prepared whole-terminal cap of 256. Actual solver
configuration is emitted and checked against policy. Rejected certificates are
retained; no tighter rerun or fallback silently replaces them.

## Charged time and memory

The Rust probe separately measures binary decoding/validation, fine topology,
fine frame, supplied maps/coarse topology, complete coarse numerical frames,
bounded terminal, and workspace plus caller output allocation. Each RHS timing
includes its complete solver, independent certificate, coefficient copy and
bit fingerprint. Inner total includes all setup, inter-phase bookkeeping,
failed action time and destruction. Prefixes begin at input decoding. Failed
setup phases and failed RHS actions retain their elapsed time and work counts.

The outer process wall time additionally charges executable startup, TSV
serialization, resource-wrapper overhead and process exit. This is the true
cold process boundary, distinct from the inner API total and prefixes. Tiny
smoke process timings are dominated by fixed overhead and are not speedup
claims. Per-kernel, reduction, transfer and certificate timings are currently
combined in each RHS phase; none is fabricated as zero. Later profiling must
resolve those components before making the full M10 performance claim.

One child runs at a time. All common nested math/Rayon thread limits are one.
`/usr/bin/time -l` on macOS records peak RSS in bytes; GNU `time -v` on Linux
records KiB, explicitly converted to bytes. These are per-process peaks, never
cumulative child high-water marks. CPU user/system seconds include startup and
teardown and retain the resource tool's granularity. Record allowed CPU affinity
on Linux; macOS placement is explicitly unavailable and unpinned. Allowed CPUs
are not proof that the process used one particular physical core.

The complete reserved array-capacity inventory counts fine/coarse structure,
fine/coarse frames, retained terminal, complete hierarchy scratch, outer Krylov
scratch and caller input/output once at their direct owners. Stack roots and
allocator headers are excluded. Logical lengths and construction peak live
allocations are explicitly unavailable in this uninstrumented timing run;
transient dense factorization memory is observed only through RSS. There is one
workspace and one numerical generation, so there is no old/new overlap or pool
replication. This inventory is not an RSS guarantee. A 1 GiB payload admission
and observed RSS ceiling, plus a 60-second process deadline, bound this small
campaign. A timeout kills the entire child process group and retains partial raw
output, wall cost, missing resource scope and failure status. It is never rerun
automatically or treated as a fast solve.

## Evidence gate and use

```sh
# Use an output directory outside the source tree. Do not overwrite evidence.
python3 scripts/prepared_serial.py /path/to/new-evidence --profile smoke
python3 scripts/validate_prepared_serial.py /path/to/new-evidence
# Separate numerical-coverage gate (native convergence alone cannot pass it):
python3 scripts/validate_prepared_serial.py /path/to/new-evidence --require-certification
```

The runner preserves raw TSV/resource reports, a manifest after every attempt,
build log, exact executable, complete summary and source provenance. The
validator regenerates input hashes from the measured generator, verifies raw
hashes and source Git objects, checks emitted configuration, exact cells and
rotation including warmups, finite diagnostics, acceptance, work scopes,
complete costs/prefixes and memory. Fixed configurations must repeat numerical
fingerprints, work and capacity reports. Missing/duplicate evidence is an error.
Valid scientific negatives are retained and make the coverage gate fail; they
do not make an otherwise honest evidence artifact structurally invalid. CI
requires evidence validity and displays numerical coverage separately. It never
asserts competitive qualification, even when every column certifies.

Adversarial Python tests run in permanent CI. The binary decoder also rejects
truncation, trailing bytes, invalid scales and oversized headers. Existing
solver correctness, dense/Galerkin, frozen-recurrence and zero-allocation gates
remain authoritative; this timing harness does not replace them.

Raw local runs and exact executables live under
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`.
The [canonical v1 evidence](../benchmarks/results/2026-09-06/prepared-serial-v1/README.md)
records the Mac smoke/development and Linux smoke, including reproducible native
LSMR stops rejected by the original certificate. Their hashes and complete raw
archives are preserved; certificate-aware continuation is the next M4 increment. Hosted-runner artifacts provide
an additional temporary copy. No calibration or campaign holdout seeds are used.

The [separate gated comparison v2](ISSUE5_GATED_SERIAL_COMPARISON.md) adds a third
route on the same scientific inputs. Current probe schema 2 also reports native
LSMR final projection counts, absent from the older schema-1 work counter but
already charged in its elapsed solve phase. Use the measured source tools for
historical artifacts; do not reinterpret or overwrite their recorded schema.
