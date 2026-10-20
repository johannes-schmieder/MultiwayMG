# Sparse-terminal policy: complete cold-process costs (M6s)

Development-only evidence for the [sparse-terminal policy](../../../../docs/ISSUE5_SPARSE_TERMINAL_POLICY.md).
Collected by `scripts/automatic_sparse_terminal.py` from committed source `145227e`
(clean tree). Build: Rust 1.85.0, locked release, one worker per thread variable,
on a Linux x86-64 cloud container. This is neither the Mac nor the SCC
qualification hardware. No number here is a gate, a routing input or a default.

Inputs are the frozen `prepared-automatic-v1` recipes: smoke and development
profiles, all ten families, both shapes, unit and heterogeneous weights, and seed
10001 only. Each input runs at widths K=1 and K=32. Seeds 10002/10003, calibration
and holdout inputs remain unused.

Every (input, width) cell runs five routes through the whole one-shot driver, all
with the frozen construction, screen and LSMR options:

- `automatic`: the frozen policy, 8 transitions;
- `automatic-sparse-t0`: the sparse-terminal policy with zero transitions, so the
  sparse terminal sits on each large component;
- `automatic-sparse-t1`: the sparse-terminal policy with one transition;
- `components-map`: per-component symmetric MAP baselines;
- `global-map`: one global MAP-preconditioned LSMR.

Each cell runs one unmeasured warmup per route, then three measured repetitions
in rotated route order: 2,400 measured cold processes. Each process records:

- driver and process wall time;
- peak RSS under GNU `time -v`, as the frozen protocol measures it;
- every column's original-problem certificate;
- engagement counts;
- an output fingerprint.

Ratios below are geometric means of per-cell median times. Each ratio is the
reference time divided by the candidate time, so a value above 1 means the
candidate is faster.

## Results

| quantity | value |
| --- | --- |
| measured processes / cells | 2,400 / 800 |
| cells with every column accepted (each route) | 160 / 160 |
| cells with bit-identical repeated outputs (each route) | 160 / 160 |
| `automatic-sparse-t0` cells with an admitted sparse terminal | 76 (all development; 6 then quality-rejected) |
| `automatic-sparse-t1` cells with an admitted sparse terminal | 36 (development) |
| peak RSS, development, max (t0 / automatic / global MAP) | 11.1 / 10.7 / 9.0 MB |

Driver-time speedups, development:

| candidate / reference | overall | K=1 | K=32 |
| --- | --- | --- | --- |
| frozen automatic / global MAP | 0.324 | | |
| sparse t0 / global MAP | 1.008 | 0.448 | 2.265 |
| sparse t0 / frozen automatic | 3.112 | | |
| sparse t1 / global MAP | 0.397 | | |

On total process wall time, t0 versus global MAP is 1.134 and frozen automatic
versus global MAP is 0.415.

Smoke components are all at or below 256 coordinates, so no sparse terminal
engages there. Automatic routes run 2.1× faster than global MAP on smoke, through
the existing dense direct terminals.

## Reading

- **The frozen automatic route remains about 3.1× slower than global MAP** on
  development driver time in this container. That preserves the M6i–M6n
  negative; it is larger than the Mac's 1.8× because this collection compares
  scalar global MAP at K=1 and K=32 only, on different hardware.
- **The sparse terminal on the component (t0) reaches parity with global MAP
  overall.** It is 3.1× faster than the frozen automatic route. The parity hides
  a width split: t0 loses 2.2× at K=1, where its setup is not amortized, and wins
  2.3× at K=32.
- **By family**, t0 wins on the harder ones and loses on the easy ones:

  | family | t0 / global MAP |
  | --- | --- |
  | nested | 2.46 |
  | pair-dominant | 2.20 |
  | chain | 2.03 |
  | communities | 1.55 |
  | ragged | 1.40 |
  | worker-firm-occupation | 0.84 |
  | exporter-importer-product | 0.72 |
  | hubs | 0.58 |
  | uniform | 0.44 |
  | tensor | 0.29 |

  The frozen automatic route quality-rejects communities and nested, where t0's
  screened cycle is accepted.
- **One shallow transition with a sparse terminal (t1) does not help.** It
  engages on 36 cells but stays as slow as the frozen route. Coarsening and
  screening cost remains the bottleneck.

These are the costs of a fixed diagnostic policy, not a calibrated selector. No
default, routing or competitive claim follows.

## Files

- `manifest.json`: provenance (commit, tree, rustc, binary hash, source hashes, hardware).
- `runs.jsonl`: one strict parsed record per measured process, with input hashes,
  wall time and peak RSS.
- `summary.json`: the aggregation in `scripts/automatic_sparse_terminal.py::summarize`.
- `build.log`: the locked release build output.

A first collection from source `872094e` measured RSS through `wait4` on a directly
spawned child. On Linux that reading includes the collector's own forked pre-exec
image, so every route reported the collector's growing footprint. That run was
discarded before commit (its timings agreed with this one), and the collector was
changed to GNU `time -v`.

SHA-256: `runs.jsonl` 0eccf2d22d765e8b664557f624ca824e7e3edb997ac875fb97a0c646a0ca0a45,
`summary.json` e06f344fb54acdb1c01a266736094d4c59035f7f84958ee7400f3439affb767d,
binary 3aae3d33cd418f7e8b77ac25ccefa3b9ecbbfbbfee4c0c93610fa651cd501853.
