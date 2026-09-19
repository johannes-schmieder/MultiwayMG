# M6l complete automatic layout evidence

Frozen source `3a1edb6837c02959828811730abdaad6a3f96b78`, tree `a57d5e25d295efe96f2085f9d2a717d798309189`. No Rust/probe/dependency changes from qualified M6k. All timing builds use Rust1.85, release, lsmr only, one worker and zero profiler TLS capacities. See [the fixed experiment](../../../../docs/ISSUE5_AUTOMATIC_LAYOUT_ECONOMICS.md).

Ratios below are complete cold-process control/candidate geometric means: above one favors the candidate. Every declared comparison is reported. Setup, failures, screening, fallback and original certificates remain charged. Aggregates balance families and RHS widths; single and repeated RHS are separate. No layout default, selector, competitive qualification, calibration or holdout decision is made.

## Main result

All 51,840 processes and 505,800 measured columns pass; no failed warmups,
measured attempts, launch/protocol errors, timeouts or RSS violations occurred.
Successful fallback and rejected quality screens remain charged, so this does
not mean every attempted hierarchy was accepted.

On Mac development, automatic/all-image is 1.2321x automatic/scalar but only
0.5363x grouped global MAP (1.86x slower). On expanded inputs those ratios are
1.3128x and 0.3568x (2.80x slower). Against grouped MAP, single/repeated RHS ratios
are 0.3905/0.5654 in development and 0.1974/0.4797 in expanded. The profiles have
different sizes and RHS-width mixes; their difference is not an isolated causal
size effect. All five automatic layouts remain below grouped MAP in both complete
profile aggregates. Component scheduling with hierarchy disabled is close to its
matched grouped baseline: 0.9877x/0.9797x. Grouped global MAP itself improves its
scalar version by 1.2478x/1.2479x.

Every whole-family automatic/all-image aggregate remains below grouped MAP in
both Mac profiles. Expanded chain repeated RHS gives 1.2101x, but its K1 ratio is
0.4605 and complete family ratio is 0.8769. This limited favorable subset is not
a selector or campaign pass. The positive tiny-smoke automatic/MAP ratios mostly
measure direct dense terminals: smoke has no hierarchy attempts, and one input
at K1 needs successful original-certificate/global fallback.

At K1, counting each input once, development accepts 29 of 38 hierarchy attempts
and rejects nine quality screens; expanded accepts 32 of 44 and rejects twelve.
Both profiles have zero global fallback columns at K1. Expanded all-image maximum
requested/admitted array payload is 37,647,224 bytes and maximum process RSS is
42,156,032 bytes. These are different memory scopes; full per-cell paired RSS
ratios and other arms are included in the summaries and tables. The 1GiB process budget is not binding here.

Layout optimization is useful but insufficient. Keep scalar as the existing
explicit default and preserve all negative results. Next test dense terminal
traversal with unchanged ordered FMA arithmetic and no extra arrays, then reduce
structural preparation cost separately. Terminal/admission and screening policy
changes require distinct evidence. Bounded parallelism, independent RHS panels,
fresh-weight replay and the upstream-Schwarz competitive holdout remain open.

## mac-smoke

15,120 isolated processes; 144,000/144,000 measured columns certified; 0 failed measured and 0 failed warmup attempts. Measured process sum 68.381023s; warmup sum 14.009223s; failed cost 0.000000s.

| Candidate | Control | All overall | All K1 | All repeated | Hard overall | Hard K1 | Hard repeated |
| --- | --- | --- | --- | --- | --- | --- | --- |
| automatic/scalar | global-map/scalar | 1.0985 | 0.9963 | 1.1165 | 1.1073 | 0.9950 | 1.1272 |
| automatic/scalar | global-map/all-row | 1.0700 | 0.9947 | 1.0831 | 1.0795 | 0.9932 | 1.0946 |
| automatic/fine-row | automatic/scalar | 1.0015 | 0.9997 | 1.0018 | 1.0016 | 1.0022 | 1.0015 |
| automatic/fine-row | global-map/scalar | 1.1001 | 0.9959 | 1.1185 | 1.1091 | 0.9971 | 1.1289 |
| automatic/fine-row | global-map/all-row | 1.0716 | 0.9944 | 1.0850 | 1.0813 | 0.9954 | 1.0963 |
| automatic/all-row | automatic/scalar | 1.0015 | 1.0019 | 1.0014 | 1.0003 | 1.0047 | 0.9996 |
| automatic/all-row | global-map/scalar | 1.1001 | 0.9981 | 1.1181 | 1.1076 | 0.9997 | 1.1267 |
| automatic/all-row | global-map/all-row | 1.0716 | 0.9966 | 1.0846 | 1.0799 | 0.9979 | 1.0942 |
| automatic/fine-image | automatic/scalar | 1.0013 | 0.9999 | 1.0016 | 0.9992 | 1.0014 | 0.9989 |
| automatic/fine-image | global-map/scalar | 1.1000 | 0.9962 | 1.1183 | 1.1064 | 0.9964 | 1.1259 |
| automatic/fine-image | global-map/all-row | 1.0714 | 0.9947 | 1.0848 | 1.0787 | 0.9946 | 1.0934 |
| automatic/all-image | automatic/scalar | 1.0026 | 1.0075 | 1.0018 | 1.0017 | 1.0110 | 1.0002 |
| automatic/all-image | global-map/scalar | 1.1014 | 1.0037 | 1.1186 | 1.1092 | 1.0060 | 1.1274 |
| automatic/all-image | global-map/all-row | 1.0728 | 1.0021 | 1.0851 | 1.0814 | 1.0042 | 1.0948 |
| components-map/scalar | global-map/scalar | 1.0995 | 0.9954 | 1.1178 | 1.1088 | 0.9941 | 1.1292 |
| components-map/all-row | global-map/all-row | 1.0711 | 0.9928 | 1.0848 | 1.0799 | 0.9927 | 1.0951 |
| global-map/all-row | global-map/scalar | 1.0266 | 1.0016 | 1.0309 | 1.0257 | 1.0018 | 1.0297 |

Each memory column is its own maximum over cells/repetitions; these maxima must not be added as if simultaneous. Requested bounds and actual retained capacities are scoped array accounting, while RSS includes runtime, allocator, stacks and decoder. Printed CPU zeros are below resolution; no CPU speedup is inferred.

| Arm | Max requested bytes | Max admitted bytes | Max grouping bytes | Max process RSS bytes |
| --- | --- | --- | --- | --- |
| automatic/all-image | 205960 | 205960 | 2216 | 2408448 |
| automatic/all-row | 205960 | 205960 | 2216 | 2424832 |
| automatic/fine-image | 205960 | 205960 | 2216 | 2441216 |
| automatic/fine-row | 205960 | 205960 | 2216 | 2424832 |
| automatic/scalar | 205960 | 205960 | 0 | 2441216 |
| components-map/all-row | 205960 | 205960 | 2216 | 2424832 |
| components-map/scalar | 205960 | 205960 | 0 | 2441216 |
| global-map/all-row | 205192 | 205192 | 4456 | 2441216 |
| global-map/scalar | 205192 | 205192 | 0 | 2408448 |

Automatic work/routing counts summed once per family/shape/weight input at K1 (not multiplied by repetitions or K):

`{"accepted_hierarchies": 0, "baseline_components": 0, "components": 50, "dense_components": 50, "global_fallback_columns": 1, "hierarchy_attempts": 0, "hierarchy_rejections": 0, "large_components": 0, "quality_rejections": 0, "singleton_components": 0}`

## mac-development

15,120 isolated processes; 144,000/144,000 measured columns certified; 0 failed measured and 0 failed warmup attempts. Measured process sum 974.562852s; warmup sum 195.122601s; failed cost 0.000000s.

| Candidate | Control | All overall | All K1 | All repeated | Hard overall | Hard K1 | Hard repeated |
| --- | --- | --- | --- | --- | --- | --- | --- |
| automatic/scalar | global-map/scalar | 0.5431 | 0.3815 | 0.5760 | 0.5807 | 0.4189 | 0.6131 |
| automatic/scalar | global-map/all-row | 0.4352 | 0.3420 | 0.4531 | 0.4473 | 0.3614 | 0.4635 |
| automatic/fine-row | automatic/scalar | 1.1496 | 1.0792 | 1.1617 | 1.1902 | 1.0976 | 1.2064 |
| automatic/fine-row | global-map/scalar | 0.6243 | 0.4117 | 0.6691 | 0.6911 | 0.4598 | 0.7397 |
| automatic/fine-row | global-map/all-row | 0.5003 | 0.3691 | 0.5263 | 0.5324 | 0.3966 | 0.5591 |
| automatic/all-row | automatic/scalar | 1.1908 | 1.1140 | 1.2041 | 1.2330 | 1.1332 | 1.2505 |
| automatic/all-row | global-map/scalar | 0.6467 | 0.4250 | 0.6936 | 0.7159 | 0.4747 | 0.7667 |
| automatic/all-row | global-map/all-row | 0.5183 | 0.3810 | 0.5455 | 0.5515 | 0.4095 | 0.5795 |
| automatic/fine-image | automatic/scalar | 1.1798 | 1.1023 | 1.1932 | 1.2161 | 1.1183 | 1.2332 |
| automatic/fine-image | global-map/scalar | 0.6407 | 0.4205 | 0.6873 | 0.7061 | 0.4684 | 0.7561 |
| automatic/fine-image | global-map/all-row | 0.5135 | 0.3770 | 0.5406 | 0.5439 | 0.4041 | 0.5715 |
| automatic/all-image | automatic/scalar | 1.2321 | 1.1416 | 1.2479 | 1.2670 | 1.1564 | 1.2865 |
| automatic/all-image | global-map/scalar | 0.6691 | 0.4355 | 0.7188 | 0.7357 | 0.4844 | 0.7888 |
| automatic/all-image | global-map/all-row | 0.5363 | 0.3905 | 0.5654 | 0.5667 | 0.4179 | 0.5962 |
| components-map/scalar | global-map/scalar | 1.0042 | 0.9675 | 1.0104 | 0.9827 | 0.9873 | 0.9820 |
| components-map/all-row | global-map/all-row | 0.9877 | 0.9723 | 0.9903 | 0.9815 | 0.9933 | 0.9795 |
| global-map/all-row | global-map/scalar | 1.2478 | 1.1154 | 1.2713 | 1.2982 | 1.1592 | 1.3229 |

Each memory column is its own maximum over cells/repetitions; these maxima must not be added as if simultaneous. Requested bounds and actual retained capacities are scoped array accounting, while RSS includes runtime, allocator, stacks and decoder. Printed CPU zeros are below resolution; no CPU speedup is inferred.

| Arm | Max requested bytes | Max admitted bytes | Max grouping bytes | Max process RSS bytes |
| --- | --- | --- | --- | --- |
| automatic/all-image | 7816764 | 7816764 | 249528 | 10698752 |
| automatic/all-row | 7682084 | 7682084 | 249528 | 10584064 |
| automatic/fine-image | 7706484 | 7706484 | 139248 | 10633216 |
| automatic/fine-row | 7571804 | 7571804 | 139248 | 10502144 |
| automatic/scalar | 7432556 | 7432556 | 0 | 10371072 |
| components-map/all-row | 6204928 | 6204928 | 139184 | 8814592 |
| components-map/scalar | 6065744 | 6065744 | 0 | 8503296 |
| global-map/all-row | 6204928 | 6204928 | 139184 | 8585216 |
| global-map/scalar | 6065744 | 6065744 | 0 | 8437760 |

Family breakdown of the declared automatic/all-image versus grouped global MAP comparison. This exposes heterogeneity; it does not select a different arm per family.

| Family | Overall | K1 | Repeated |
| --- | --- | --- | --- |
| uniform | 0.3946 | 0.2704 | 0.4203 |
| communities | 0.6469 | 0.4114 | 0.6976 |
| chain | 0.5860 | 0.3756 | 0.6311 |
| pair-dominant | 0.5315 | 0.4106 | 0.5549 |
| nested | 0.7712 | 0.5637 | 0.8125 |
| hubs | 0.4437 | 0.3406 | 0.4636 |
| ragged | 0.8719 | 0.5656 | 0.9372 |
| tensor | 0.3889 | 0.2970 | 0.4068 |
| worker-firm-occupation | 0.5015 | 0.4271 | 0.5151 |
| exporter-importer-product | 0.4251 | 0.3486 | 0.4394 |

Automatic work/routing counts summed once per family/shape/weight input at K1 (not multiplied by repetitions or K):

`{"accepted_hierarchies": 29, "baseline_components": 9, "components": 108, "dense_components": 14, "global_fallback_columns": 0, "hierarchy_attempts": 38, "hierarchy_rejections": 9, "large_components": 38, "quality_rejections": 9, "singleton_components": 56}`

## mac-expanded

6,480 isolated processes; 73,800/73,800 measured columns certified; 0 failed measured and 0 failed warmup attempts. Measured process sum 4248.171878s; warmup sum 849.257722s; failed cost 0.000000s.

| Candidate | Control | All overall | All K1 | All repeated | Hard overall | Hard K1 | Hard repeated |
| --- | --- | --- | --- | --- | --- | --- | --- |
| automatic/scalar | global-map/scalar | 0.3392 | 0.1865 | 0.4575 | 0.5056 | 0.3025 | 0.6538 |
| automatic/scalar | global-map/all-row | 0.2718 | 0.1593 | 0.3551 | 0.3884 | 0.2465 | 0.4875 |
| automatic/fine-row | automatic/scalar | 1.1084 | 1.0597 | 1.1336 | 1.1535 | 1.0841 | 1.1899 |
| automatic/fine-row | global-map/scalar | 0.3760 | 0.1976 | 0.5186 | 0.5833 | 0.3279 | 0.7779 |
| automatic/fine-row | global-map/all-row | 0.3013 | 0.1688 | 0.4026 | 0.4480 | 0.2672 | 0.5801 |
| automatic/all-row | automatic/scalar | 1.2324 | 1.1721 | 1.2637 | 1.2482 | 1.1710 | 1.2888 |
| automatic/all-row | global-map/scalar | 0.4181 | 0.2186 | 0.5781 | 0.6312 | 0.3542 | 0.8425 |
| automatic/all-row | global-map/all-row | 0.3350 | 0.1867 | 0.4488 | 0.4848 | 0.2887 | 0.6283 |
| automatic/fine-image | automatic/scalar | 1.1295 | 1.0744 | 1.1582 | 1.1703 | 1.0958 | 1.2095 |
| automatic/fine-image | global-map/scalar | 0.3832 | 0.2004 | 0.5298 | 0.5918 | 0.3315 | 0.7907 |
| automatic/fine-image | global-map/all-row | 0.3070 | 0.1711 | 0.4113 | 0.4546 | 0.2701 | 0.5897 |
| automatic/all-image | automatic/scalar | 1.3128 | 1.2398 | 1.3508 | 1.3037 | 1.2192 | 1.3482 |
| automatic/all-image | global-map/scalar | 0.4453 | 0.2313 | 0.6180 | 0.6592 | 0.3688 | 0.8814 |
| automatic/all-image | global-map/all-row | 0.3568 | 0.1974 | 0.4797 | 0.5064 | 0.3006 | 0.6573 |
| components-map/scalar | global-map/scalar | 0.9848 | 0.9773 | 0.9886 | 0.9806 | 0.9815 | 0.9801 |
| components-map/all-row | global-map/all-row | 0.9797 | 0.9815 | 0.9789 | 0.9796 | 0.9886 | 0.9751 |
| global-map/all-row | global-map/scalar | 1.2479 | 1.1712 | 1.2882 | 1.3018 | 1.2270 | 1.3409 |

Each memory column is its own maximum over cells/repetitions; these maxima must not be added as if simultaneous. Requested bounds and actual retained capacities are scoped array accounting, while RSS includes runtime, allocator, stacks and decoder. Printed CPU zeros are below resolution; no CPU speedup is inferred.

| Arm | Max requested bytes | Max admitted bytes | Max grouping bytes | Max process RSS bytes |
| --- | --- | --- | --- | --- |
| automatic/all-image | 37647224 | 37647224 | 1887584 | 42156032 |
| automatic/all-row | 37091328 | 37091328 | 1887584 | 41287680 |
| automatic/fine-image | 36333544 | 36333544 | 573904 | 40976384 |
| automatic/fine-row | 35777648 | 35777648 | 573904 | 40878080 |
| automatic/scalar | 35203744 | 35203744 | 0 | 40288256 |
| components-map/all-row | 25571712 | 25571712 | 573840 | 28508160 |
| components-map/scalar | 24997872 | 24997872 | 0 | 27967488 |
| global-map/all-row | 25571712 | 25571712 | 573840 | 27869184 |
| global-map/scalar | 24997872 | 24997872 | 0 | 27312128 |

Family breakdown of the declared automatic/all-image versus grouped global MAP comparison. This exposes heterogeneity; it does not select a different arm per family.

| Family | Overall | K1 | Repeated |
| --- | --- | --- | --- |
| uniform | 0.1657 | 0.0791 | 0.2400 |
| communities | 0.4675 | 0.2223 | 0.6781 |
| chain | 0.8769 | 0.4605 | 1.2101 |
| pair-dominant | 0.5173 | 0.3401 | 0.6380 |
| nested | 0.7343 | 0.5082 | 0.8826 |
| hubs | 0.2108 | 0.1104 | 0.2912 |
| ragged | 0.3116 | 0.1570 | 0.4390 |
| tensor | 0.1824 | 0.0891 | 0.2610 |
| worker-firm-occupation | 0.4508 | 0.3436 | 0.5164 |
| exporter-importer-product | 0.2401 | 0.1213 | 0.3380 |

Automatic work/routing counts summed once per family/shape/weight input at K1 (not multiplied by repetitions or K):

`{"accepted_hierarchies": 32, "baseline_components": 12, "components": 324, "dense_components": 32, "global_fallback_columns": 0, "hierarchy_attempts": 44, "hierarchy_rejections": 12, "large_components": 44, "quality_rejections": 12, "singleton_components": 248}`

## linux-smoke

15,120 isolated processes; 144,000/144,000 measured columns certified; 0 failed measured and 0 failed warmup attempts. Measured process sum 31.157789s; warmup sum 6.332042s; failed cost 0.000000s.

| Candidate | Control | All overall | All K1 | All repeated | Hard overall | Hard K1 | Hard repeated |
| --- | --- | --- | --- | --- | --- | --- | --- |
| automatic/scalar | global-map/scalar | 1.3542 | 0.9930 | 1.4261 | 1.3855 | 0.9911 | 1.4650 |
| automatic/scalar | global-map/all-row | 1.2571 | 0.9769 | 1.3111 | 1.2857 | 0.9759 | 1.3462 |
| automatic/fine-row | automatic/scalar | 1.0031 | 1.0018 | 1.0033 | 1.0028 | 1.0020 | 1.0030 |
| automatic/fine-row | global-map/scalar | 1.3584 | 0.9947 | 1.4308 | 1.3894 | 0.9931 | 1.4694 |
| automatic/fine-row | global-map/all-row | 1.2610 | 0.9786 | 1.3154 | 1.2894 | 0.9778 | 1.3502 |
| automatic/all-row | automatic/scalar | 1.0025 | 1.0031 | 1.0024 | 1.0018 | 1.0037 | 1.0015 |
| automatic/all-row | global-map/scalar | 1.3576 | 0.9961 | 1.4295 | 1.3879 | 0.9948 | 1.4671 |
| automatic/all-row | global-map/all-row | 1.2602 | 0.9799 | 1.3142 | 1.2880 | 0.9795 | 1.3481 |
| automatic/fine-image | automatic/scalar | 1.0043 | 1.0031 | 1.0045 | 1.0032 | 1.0032 | 1.0032 |
| automatic/fine-image | global-map/scalar | 1.3600 | 0.9961 | 1.4325 | 1.3899 | 0.9943 | 1.4697 |
| automatic/fine-image | global-map/all-row | 1.2625 | 0.9800 | 1.3170 | 1.2899 | 0.9790 | 1.3505 |
| automatic/all-image | automatic/scalar | 1.0033 | 1.0003 | 1.0038 | 1.0016 | 1.0036 | 1.0013 |
| automatic/all-image | global-map/scalar | 1.3587 | 0.9933 | 1.4316 | 1.3877 | 0.9947 | 1.4668 |
| automatic/all-image | global-map/all-row | 1.2613 | 0.9772 | 1.3162 | 1.2878 | 0.9794 | 1.3479 |
| components-map/scalar | global-map/scalar | 1.3539 | 0.9919 | 1.4259 | 1.3850 | 0.9920 | 1.4643 |
| components-map/all-row | global-map/all-row | 1.2588 | 0.9760 | 1.3133 | 1.2846 | 0.9810 | 1.3436 |
| global-map/all-row | global-map/scalar | 1.0772 | 1.0165 | 1.0877 | 1.0776 | 1.0157 | 1.0883 |

Each memory column is its own maximum over cells/repetitions; these maxima must not be added as if simultaneous. Requested bounds and actual retained capacities are scoped array accounting, while RSS includes runtime, allocator, stacks and decoder. Printed CPU zeros are below resolution; no CPU speedup is inferred.

| Arm | Max requested bytes | Max admitted bytes | Max grouping bytes | Max process RSS bytes |
| --- | --- | --- | --- | --- |
| automatic/all-image | 205960 | 205960 | 2216 | 3469312 |
| automatic/all-row | 205960 | 205960 | 2216 | 3514368 |
| automatic/fine-image | 205960 | 205960 | 2216 | 3534848 |
| automatic/fine-row | 205960 | 205960 | 2216 | 3559424 |
| automatic/scalar | 205960 | 205960 | 0 | 3485696 |
| components-map/all-row | 205960 | 205960 | 2216 | 3514368 |
| components-map/scalar | 205960 | 205960 | 0 | 3510272 |
| global-map/all-row | 205192 | 205192 | 4456 | 3559424 |
| global-map/scalar | 205192 | 205192 | 0 | 3559424 |

Automatic work/routing counts summed once per family/shape/weight input at K1 (not multiplied by repetitions or K):

`{"accepted_hierarchies": 0, "baseline_components": 0, "components": 50, "dense_components": 50, "global_fallback_columns": 1, "hierarchy_attempts": 0, "hierarchy_rejections": 0, "large_components": 0, "quality_rejections": 0, "singleton_components": 0}`

## Completeness and interpretation

Total: 51,840 processes and 505,800/505,800 measured columns certified, with 0 failed measured and 0 failed warmup attempts. Warmup work remains separately charged. Full per-cell timing, native/certificate work, routing and capacity records remain in the linked summaries; all raw stdout/stderr, input checksums, journals, executables and build logs are preserved in the archives listed by `receipts.json`.

Mac smoke/development and Linux smoke overlap the earlier experiments. `comparison.json` records exact comparisons to M6j scalar and M6k uninstrumented reference math/work/payload records; expanded inputs have no previous same-input baseline. `receipts.json` records independent copy validation, every archive member checksum and any explicitly scoped cross-host rounding in derived geometric means. Scientific records and hashes are exact.

This is a bounded development experiment: one seed, canonical unique tuples, no raw observation-collapse/multiplicity timing, at most 69,487 unique tuples in expanded inputs, unpinned Apple Silicon execution, and Linux smoke only. It is not an out-of-cache study, SCC scaling result, Windows timing comparison or competitive qualification against immutable upstream within/Schwarz. M6 terminal/admission work and M7–M10 remain open.

All three Mac summaries recompute exactly. Linux has eight differences only in
derived geometric means, within the preregistered archival-only 1e-14 relative
scope; every path and both values are preserved in `receipts.json`. No raw or
scientific comparison uses that tolerance. Provider ZIPs were independently
SHA256-checked against GitHub artifact digests; see the qualification directory's
`github-artifact-receipts.json`.
