# Complete explicit layout comparison (M5e)

Measured source `7c2794c3b279d7fa59575ef8b36d29366d33ee3a` and the
[five-layout policy](../../../policies/prepared-layout-v1.json) were committed
before collection. All layouts run the same fixed-weight supplied-map solver,
current numerical frames, native options and scalar original-operator certificates.
No default layout or competitive qualification is selected by this experiment.

| Artifact | Processes including warmups | Measured columns, all layouts/routes | Exact scalar/layout comparisons |
| --- | ---: | ---: | ---: |
| Mac smoke | 2,520 | 24,000 | 2,016 |
| Mac development | 2,520 | 24,000 | 2,016 |
| Mac expanded, one RHS | 360 | 300 | 288 |
| Linux smoke | 2,520 | 24,000 | 2,016 |

All **7,920 processes and 72,300 measured columns** pass complete evidence,
certificate, repeatability and RSS gates. There are no process errors, timeouts,
rejected columns or measured retries. Every one of 6,336 corresponding
scalar/layout comparisons preserves numerical results, coefficient fingerprints
and work counts exactly. Payload changes are precisely the admitted group owner
and one shared image vector/descriptor; other solver payloads remain identical.
The 1,584 scalar processes also match prior M5d scalar evidence (Mac smoke/
development and Linux smoke) and M5b diagnostic expanded evidence in input hashes,
numerical/work/fingerprint and solver-payload fields. This cross-revision check
does not compare timings to instrumented or separately collected runs.

## Paired complete costs

Each cell uses five measured repetitions following a separate warmup. Routes
and layouts rotate prospectively; each layout occupies every position once per
case/route. Values below are balanced geometric means of within-cell median
paired scalar/layout ratios. Larger than one means faster. Full process includes
decode, setup, solve, certificates, printing and teardown. Inner includes decode,
setup, solve, certificates and charged destruction before printing.

| Mac profile/layout | PCG process | PCG inner | Gated LSMR process | Gated LSMR inner |
| --- | ---: | ---: | ---: | ---: |
| Development / fine-row | 1.1572 | 1.1866 | 1.1793 | 1.2131 |
| Development / all-row | 1.3070 | 1.3749 | 1.3540 | 1.4244 |
| Development / fine-image | 1.2112 | 1.2533 | 1.2132 | 1.2530 |
| Development / all-image | 1.3984 | 1.4904 | 1.4439 | 1.5291 |
| Expanded / fine-row | 1.0106 | 1.0113 | 1.0136 | 1.0141 |
| Expanded / all-row | 1.1270 | 1.1295 | 1.1423 | 1.1463 |
| Expanded / fine-image | 1.0433 | 1.0426 | 1.0370 | 1.0394 |
| Expanded / all-image | 1.2318 | 1.2390 | 1.2381 | 1.2446 |

For all-image every development and expanded cell improves both inner and full
process medians. Development PCG full-process geomeans by family are 1.3427
uniform, 1.3759 communities and 1.4803 chain; gated LSMR is 1.3976, 1.4454 and
1.4902. Width-one development geomeans are 1.1504/1.1844 (PCG/gated), rising to
1.5729/1.6128 at width32; these runs reuse scalar workspaces serially, without
fused RHS panels or parallelism. Expanded full-process gains by family are
1.2005/1.2124 uniform, 1.2215/1.2389 communities and 1.2744/1.2636 chain.

Small-case negative results matter. Mac smoke all-image full-process geomeans
are only 1.0399/1.0637, with 12/42 PCG and 9/42 gated cells below one; worst
ratios are 0.9437/0.9421. Linux smoke gives 1.0784/1.0904, with 4/42 cells below
one for each route. All smoke inner medians improve, but the full submitted
process remains the relevant boundary. Fine/all layouts are structurally
equivalent at smoke depth1; their timing differences are measurement variation.
Canonical summaries retain every layout/cell, including these negatives.

## Memory and scope

For expanded uniform/communities/chain respectively, all-level grouping retains
4,153,680 / 3,485,616 / 1,150,976 bytes including descriptors. One shared image
adds 589,832 / 589,832 / 527,816 bytes. Total additional capacities are therefore
4,743,512 / 4,075,448 / 1,678,792 bytes, with no duplicate numerical frames or
per-level image buffers. Expanded complete retained payload increases by
11.57–15.54% across routes/families; development increases by 4.60–17.68%
depending on family, route and RHS width. These are scoped retained capacities,
not allocator high-water estimates. Expanded all-image maximum RSS is
42,319,872 bytes, within the declared 1GiB cap. Every raw record reports actual
grouping setup admission, per-level E/V/index widths, record size and RSS.

The Mac is an unpinned single-worker M2 Ultra; six thread-library caps are one.
Linux smoke is an exact-source GitHub runner qualification, not SCC scaling.
The largest cases have V=12,288 and at most 73,726 tuples, supplied depth8 maps
and one RHS. Many coarse levels retain substantial tuple work. This does not
test out-of-cache large multi-RHS, automatic maps, upstream competitiveness,
changing weights or thread scaling. Scalar remains default; all-image is a
promising explicit candidate for later admitted, frozen selection. Campaign
calibration and holdout remain untouched.

## Reproducibility and review

The [receipts](receipts.json) identify exact source/tree/policy, binaries,
manifests, legacy comparisons and deterministic archive hashes. Raw manifests,
TSV/resources, build logs and executables are preserved under
`$GIT_HOME/MultiwayMG-assessments/2026-09-06-implementation/serial-benchmark/`
as `{mac-smoke,mac-development,mac-expanded,linux-smoke}-layout-7c2794c`.
Copied artifacts independently revalidate, saved summaries reproduce exactly
with the qualified Python interpreter, and every archived member is rehashed.
This local preservation is not an independent off-machine backup.

All required local Rust1.85 checks, 88 Python adversarial tests and 120 release
black-box layout protocol comparisons passed before collection. Source workflows
`34074747242`/`34074747247` and PR workflows `34074829795`/`34074829807` passed.
Linux evidence comes from source run `34074747242`. Self-review covers cold
process boundaries, failures, stable rotation, exact source/input ownership,
physical memory deltas and unchanged legacy parsing. It is not independent
external review. Final evidence-head checks and merge remain at this checkpoint.
