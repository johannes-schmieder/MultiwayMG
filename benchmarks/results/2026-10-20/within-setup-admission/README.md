# Checked within setup admission (M6r)

Development-only evidence for the [checked within setup increment](../../../../docs/ISSUE5_FACTOR_MEMORY.md#checked-within-setup-m6r).
Collected by `scripts/within_setup_admission.py` from committed source `186b880`
(clean tree). Build: Rust 1.85.0, locked release, one worker per thread variable, on
a Linux x86-64 cloud container. This is neither the Mac nor the SCC qualification
hardware, and no timing number here is a gate or a routing input.

Inputs are the frozen `prepared-automatic-v1` recipes, as for the
[range-admission evidence](../range-admission/README.md): 80 runs over the
smoke and development profiles, all ten families, both shapes, unit and
heterogeneous weights, seed 10001 only. Seeds 10002/10003, calibration and
holdout inputs remain unused. Each run does the following with the default
comparator options:

- builds the frozen `within` comparator unchecked (after one untimed warm-up
  build) and through `within::Preconditioner::build_checked`;
- reports the checked setup peak, the retained payload, and both factor capacity
  inventories;
- rebuilds at a limit of exactly the peak and one byte below it;
- applies both `within` preconditioners through their deterministic serial
  workspaces to three fixed vectors, checking bit identity, and compares the two
  comparator actions.

## Results

| quantity | value |
| --- | --- |
| runs | 80 |
| admitted at a limit equal to the peak | 80 |
| refused one byte below the peak (`limit < required <= peak`) | 80 |
| serial actions bit-identical to the `Solver::new` route | 80 |
| largest comparator action difference | 0 |
| largest setup peak / retained payload | 2,584,356 / 1,628,368 bytes (development) |
| peak / retained payload | 1.008 to 2.18 |
| retained payload / factor capacity | 1.89 to 75.3 (median 7.33) |
| checked factor capacity at most unchecked | 80 (median ratio 0.79, minimum 0.26) |
| checked / unchecked setup time, median (informational) | 0.51 smoke, 0.91 development |

The largest setups are the development uniform-balanced and tensor-balanced
inputs: dimension 384 and 10,534–16,572 tuples.

**Factor capacity understates retained state.** The factor inventory reported
since M6q is a weak lower bound: the complete retained payload is 1.9 to 75 times
larger. The cross-tabulations, which scale with tuple count, are the likely
bulk of the difference. No per-category breakdown was measured.

**Checked factors retain less spare capacity.** The checked builder grows arrays
without the unaccounted builder's amortized slack, so its factors retain no more
capacity, and often less. Capacity is not part of a factor's identity: the
serialized factors are equal.

The timing ratio compares one-thread checked setup with one-thread unchecked
setup. Both include the wrapper's level copies; the unchecked side also builds the
design and solver. It is not a gate.

## Files

- `manifest.json`: provenance (commit, tree, rustc, binary hash, source hashes, hardware).
- `runs.jsonl`: one strict parsed record per run, with input and stdout hashes.
- `NNN.stdout` / `NNN.stderr`: raw example output per run.
- `summary.json`: the aggregation in `scripts/within_setup_admission.py::summarize`.
- `build.log`: the locked release build output.

SHA-256: `runs.jsonl` 33a34475f45e7d0930de67d8c34d573ef179f390901570f0a872db18b47534eb,
`summary.json` c9ad93101638ac83cef9c666f780696dd21d9e07660146666f1dc703160cf647,
binary 577f6fbf428f2a176d0ded6a42dfa25d60c781c6e217a0bc5304ed07537baf10.
