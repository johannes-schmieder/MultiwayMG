# Exact kernel certification and full-range action diagnostics (M6p)

Development-only evidence for the [range-admission increment](../../../../docs/ISSUE5_RANGE_ADMISSION.md).
Collected by `scripts/range_admission.py` from committed source `8ed28db`
(clean tree), Rust 1.85.0 locked release, one worker per thread variable, on a
Linux x86-64 cloud container. This is neither the Mac nor the SCC qualification
hardware, and no timing number here is a gate or a routing input.

Inputs are the frozen `prepared-automatic-v1` recipes: smoke and development
profiles, all ten families, both shapes, unit and heterogeneous weights, seed
10001 only, one RHS column. That gives 80 runs. Seeds 10002/10003, calibration
and holdout inputs remain unused.

Each run certifies the exact incidence kernel, builds the certified range
projector, and materializes three fixed `within` comparator actions against the
dense oracle range of the unit-weight Gramian on the same topology:

- `within-output`: the unchanged comparator, structural projection on output only;
- `structural-both`: the M6o audit's two-sided structural projection;
- `certified-full-both`: two-sided certified full-range projection.

`admitted` means symmetric, range-preserving and positive on the range, all at
the oracle's default 1e-11 rank / 1e-10 structure tolerances.

## Results

| quantity | value |
| --- | --- |
| runs | 80 |
| exact nullity equals oracle nullity (unit / submitted weights) | 80 / 80 |
| runs with additional exact nullity | 12 (6 on a component above 256 coordinates) |
| `within-output` admitted | 0 (never fully symmetric; 68 preserve range) |
| `structural-both` admitted | 68; fails exactly the 12 additional-nullity runs, leakage up to 2.01e-2 |
| `certified-full-both` admitted | 80; leakage at most 1.10e-12 on additional-nullity runs |
| largest certificate peak requested payload | 73,144 bytes (development) |
| largest stored modular echelon entries | 1,728 |
| largest integer kernel entry | 1 |
| projector orthogonality / incidence residual | at most 3.8e-15 / 1.1e-16 |

The additional-nullity runs are communities-balanced and nested-balanced/unbalanced
in both profiles. On structural-kernel runs the full and structural wrappers
give identical minimum quotient energies, as expected when the two projectors coincide.

## Oracle resolution limit

The largest `certified-full-both` leakage, 8.08e-11 (smoke uniform balanced), is
identical for all three actions on that structural-kernel topology. That points
to the oracle, not to the actions. The separately preserved [oracle-basis probe](oracle-basis-probe/)
confirms this. It measures, against the certified exact kernel, the kernel component
of the oracle's own range basis (nalgebra `SymmetricEigen`, unit Gramian). On that
topology the component is 7.74e-10, while `within`'s raw output there has relative
kernel component 7.4e-17. The oracle's 1e-10 range classification therefore sits near
its own eigenvector accuracy on some well-conditioned inputs. The certified projector,
not the dense oracle, is the authoritative range statement. The probe was built
as a temporary example at the same commit and is not part of the crate.

The probe also shows how far raw `within` outputs leave the exact range on the
additional-nullity topologies: up to 0.207 relative (smoke nested unbalanced).

## Files

- `manifest.json`: provenance (commit, tree, rustc, binary hash, source hashes, hardware).
- `runs.jsonl`: one strict parsed record per run, with input and stdout hashes.
- `NNN.stdout` / `NNN.stderr`: raw example output per run.
- `summary.json`: the aggregation in `scripts/range_admission.py::summarize`.
- `build.log`: the locked release build output.
- `oracle-basis-probe/`: probe source, runner and output.

SHA-256: `runs.jsonl` 09a91934fdd624c037c37fe0236705ce454dbd20fa2d7261c9d21df052d8e077,
`summary.json` a2ee6b499169e3783c97fe7d225689553fd07894a477b4d33b19aba555b631fc,
binary f41cebd7d820a37156f761ea748ec4b3e6021bdbfc52ace5c520285e2e4a6e43.
