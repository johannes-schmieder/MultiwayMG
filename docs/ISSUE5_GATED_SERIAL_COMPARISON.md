# Gated serial development comparison v2 (M4 closure)

This distinct [v2 policy](../benchmarks/policies/prepared-serial-gated-v2.json)
adds original-certificate-gated LSMR to the unchanged v1 PCG and native LSMR
controls. It preserves all input recipes, seed 10001, dimensions, positive
weights, supplied maps, RHS widths, native/certificate tolerances, iteration
limits and workspaces. Native v1 results and executables remain archived.
No calibration or campaign holdout is involved.

The input generator body is unchanged. Before timing, commit the policy,
probe and validator and record their hashes. There are 126 route/case cells,
each with one separate-process warmup and five measured repetitions: 756
processes, 630 measured runs and 7,200 measured RHS columns per profile.
Rotate all three routes by repetition index modulo three. Record every route,
including the known native-control rejections and all their cost.

PCG and gated LSMR are the predeclared candidate routes. Each must certify all
2,400 measured columns per profile without an error, timeout or RSS-budget
failure. Native LSMR remains in the overall denominator and is reported
separately as a control. `all_measured_columns_certified` therefore continues to
reflect every route, including that control. `eligible_routes_certified` is the
separate, predeclared two-route gate. Neither implies M10 competitiveness.

## Accounting update

Probe schema 2 records all LSMR outer projection attempts, including the final
projection. Schema 1's projection counter represented tracked PCG projections;
LSMR final projection was charged in time but not counted there. Historical
schema-1 evidence remains unchanged and is validated with its measured tools.

For both native and gated LSMR, a `gate` record names candidate checks, vetoes,
outer projections and additional candidate certificate incidence/adjoint work.
The ordinary `work` record now includes those extra actions and the fresh final
certificate. A successful gated solve must account for one complete original
certificate per candidate check plus the final certificate, and one outer
projection per candidate plus the final projection. Native LSMR has zero
candidate checks but records its final projection. Failure records retain
partial gate work and the failed action's elapsed time. No time or allocation
scope is reset when a candidate is vetoed.

Every native candidate that the original gate permits to stop has the same
immutable operator/coefficient state at final certification, so a passing gate
cannot be used to excuse a rejected final certificate. Non-resumable exits may
still reject and fail candidate-route coverage. Fixed-configuration checks
include all gate counters, work, output fingerprints and capacities.

The [complete-cost boundaries](ISSUE5_SERIAL_BENCHMARK.md) otherwise remain:
inner decode/setup/solve/certificate/output/destruction, outer cold process wall,
per-RHS prefixes, scoped reserved payload, isolated process RSS/CPU, explicit
unknown memory scopes and serial execution. Updated summaries additionally
show median inner and aggregate solve/certificate/output time. Individual
kernel/reduction/transfer profiling remains required for later optimization.

```sh
python3 scripts/prepared_serial.py /path/to/new-gated-evidence --profile smoke --policy gated
python3 scripts/validate_prepared_serial.py /path/to/new-gated-evidence --require-certification
```

Repeat with `--profile development` on the Mac after the recipe/source commit.
The permanent Linux CI matrix retains the native-policy smoke and adds the
three-route gated-policy smoke. Evidence validity is mandatory in both jobs;
the gated-policy job additionally requires complete predeclared candidate-route
coverage. Artifact names are `prepared-serial-native-smoke` and
`prepared-serial-gated-smoke`. A failed gate is preserved and investigated;
it is not replaced with a tighter-tolerance rerun.

New adversarial tests cover missing or omitted candidate work, fake native
continuation, altered eligible routes, three-route ordering and honest controls
that fail while the predeclared candidate routes pass. No performance winner
is selected from these development timings.

M4 closes only after the separately checked solver integration, complete
validated Mac smoke/development and Linux smoke, and the evidence PR have
qualified and merged. Then use this source/executable as the engineering
baseline for M5 memory/traffic changes; keep the original upstream baselines
separate for the eventual full competitive campaign.
