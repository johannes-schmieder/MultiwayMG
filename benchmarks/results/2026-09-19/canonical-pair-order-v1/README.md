# M6n canonical pair-order complete-cost evidence

Frozen source `f4a611d61287e2375c1812232f1df1ac451605a1`, tree `4f7376c270d919a4108259e7ae6576b936357e48`. One redundant comparison sort becomes an identity reset in existing storage, with unchanged six-pair visitation, floating-point reductions, capacities and policy. All timing builds use Rust1.85, release, lsmr only, one worker and zero profiler TLS capacities. See [the fixed qualification](../../../../docs/ISSUE5_CANONICAL_PAIR_ORDER.md), using the unchanged M6l layout policy.

Ratios below are complete cold-process control/candidate geometric means: above one favors the candidate. Every declared comparison is reported. Setup, failures, screening, fallback and original certificates remain charged. Aggregates balance families and RHS widths; single and repeated RHS are separate. No layout default, selector, competitive qualification, calibration or holdout decision is made.

## mac-smoke

15,120 isolated processes; 144,000/144,000 measured columns certified; 0 failed measured and 0 failed warmup attempts. Measured process sum 77.972649s; warmup sum 15.799006s; failed cost 0.000000s.

| Candidate | Control | All overall | All K1 | All repeated | Hard overall | Hard K1 | Hard repeated |
| --- | --- | --- | --- | --- | --- | --- | --- |
| automatic/scalar | global-map/scalar | 1.0879 | 0.9935 | 1.1045 | 1.0972 | 1.0016 | 1.1140 |
| automatic/scalar | global-map/all-row | 1.0625 | 0.9923 | 1.0747 | 1.0713 | 0.9995 | 1.0837 |
| automatic/fine-row | automatic/scalar | 1.0008 | 1.0001 | 1.0009 | 1.0006 | 0.9924 | 1.0020 |
| automatic/fine-row | global-map/scalar | 1.0888 | 0.9935 | 1.1055 | 1.0979 | 0.9940 | 1.1163 |
| automatic/fine-row | global-map/all-row | 1.0634 | 0.9924 | 1.0757 | 1.0719 | 0.9918 | 1.0859 |
| automatic/all-row | automatic/scalar | 1.0037 | 1.0066 | 1.0033 | 1.0050 | 0.9976 | 1.0063 |
| automatic/all-row | global-map/scalar | 1.0920 | 1.0000 | 1.1081 | 1.1028 | 0.9992 | 1.1210 |
| automatic/all-row | global-map/all-row | 1.0665 | 0.9989 | 1.0782 | 1.0766 | 0.9971 | 1.0905 |
| automatic/fine-image | automatic/scalar | 1.0060 | 1.0079 | 1.0057 | 1.0053 | 1.0047 | 1.0055 |
| automatic/fine-image | global-map/scalar | 1.0944 | 1.0013 | 1.1108 | 1.1031 | 1.0063 | 1.1201 |
| automatic/fine-image | global-map/all-row | 1.0689 | 1.0002 | 1.0808 | 1.0770 | 1.0042 | 1.0896 |
| automatic/all-image | automatic/scalar | 1.0062 | 1.0031 | 1.0068 | 1.0062 | 0.9993 | 1.0073 |
| automatic/all-image | global-map/scalar | 1.0947 | 0.9966 | 1.1119 | 1.1040 | 1.0009 | 1.1222 |
| automatic/all-image | global-map/all-row | 1.0691 | 0.9955 | 1.0819 | 1.0779 | 0.9987 | 1.0916 |
| components-map/scalar | global-map/scalar | 1.0932 | 0.9922 | 1.1110 | 1.1032 | 0.9920 | 1.1229 |
| components-map/all-row | global-map/all-row | 1.0691 | 0.9930 | 1.0824 | 1.0763 | 0.9969 | 1.0902 |
| global-map/all-row | global-map/scalar | 1.0239 | 1.0011 | 1.0277 | 1.0243 | 1.0022 | 1.0280 |

Each memory column is its own maximum over cells/repetitions; these maxima must not be added as if simultaneous. Requested bounds and actual retained capacities are scoped array accounting, while RSS includes runtime, allocator, stacks and decoder. Printed CPU zeros are below resolution; no CPU speedup is inferred.

| Arm | Max requested bytes | Max admitted bytes | Max grouping bytes | Max process RSS bytes |
| --- | --- | --- | --- | --- |
| automatic/all-image | 205960 | 205960 | 2216 | 2457600 |
| automatic/all-row | 205960 | 205960 | 2216 | 2457600 |
| automatic/fine-image | 205960 | 205960 | 2216 | 2441216 |
| automatic/fine-row | 205960 | 205960 | 2216 | 2441216 |
| automatic/scalar | 205960 | 205960 | 0 | 2473984 |
| components-map/all-row | 205960 | 205960 | 2216 | 2441216 |
| components-map/scalar | 205960 | 205960 | 0 | 2424832 |
| global-map/all-row | 205192 | 205192 | 4456 | 2424832 |
| global-map/scalar | 205192 | 205192 | 0 | 2408448 |

Automatic work/routing counts summed once per family/shape/weight input at K1 (not multiplied by repetitions or K):

`{"accepted_hierarchies": 0, "baseline_components": 0, "components": 50, "dense_components": 50, "global_fallback_columns": 1, "hierarchy_attempts": 0, "hierarchy_rejections": 0, "large_components": 0, "quality_rejections": 0, "singleton_components": 0}`

## mac-development

15,120 isolated processes; 144,000/144,000 measured columns certified; 0 failed measured and 0 failed warmup attempts. Measured process sum 997.779475s; warmup sum 200.240227s; failed cost 0.000000s.

| Candidate | Control | All overall | All K1 | All repeated | Hard overall | Hard K1 | Hard repeated |
| --- | --- | --- | --- | --- | --- | --- | --- |
| automatic/scalar | global-map/scalar | 0.5599 | 0.3926 | 0.5941 | 0.5992 | 0.4298 | 0.6333 |
| automatic/scalar | global-map/all-row | 0.4463 | 0.3513 | 0.4645 | 0.4589 | 0.3710 | 0.4755 |
| automatic/fine-row | automatic/scalar | 1.1560 | 1.0829 | 1.1686 | 1.1981 | 1.1024 | 1.2148 |
| automatic/fine-row | global-map/scalar | 0.6473 | 0.4251 | 0.6943 | 0.7178 | 0.4738 | 0.7693 |
| automatic/fine-row | global-map/all-row | 0.5159 | 0.3805 | 0.5428 | 0.5498 | 0.4089 | 0.5776 |
| automatic/all-row | automatic/scalar | 1.2006 | 1.1175 | 1.2151 | 1.2437 | 1.1385 | 1.2622 |
| automatic/all-row | global-map/scalar | 0.6723 | 0.4388 | 0.7218 | 0.7452 | 0.4894 | 0.7993 |
| automatic/all-row | global-map/all-row | 0.5358 | 0.3926 | 0.5643 | 0.5708 | 0.4223 | 0.6001 |
| automatic/fine-image | automatic/scalar | 1.1911 | 1.1108 | 1.2050 | 1.2276 | 1.1299 | 1.2447 |
| automatic/fine-image | global-map/scalar | 0.6669 | 0.4361 | 0.7159 | 0.7355 | 0.4857 | 0.7882 |
| automatic/fine-image | global-map/all-row | 0.5316 | 0.3903 | 0.5597 | 0.5634 | 0.4192 | 0.5918 |
| automatic/all-image | automatic/scalar | 1.2472 | 1.1565 | 1.2630 | 1.2826 | 1.1721 | 1.3020 |
| automatic/all-image | global-map/scalar | 0.6984 | 0.4540 | 0.7503 | 0.7685 | 0.5038 | 0.8245 |
| automatic/all-image | global-map/all-row | 0.5566 | 0.4063 | 0.5866 | 0.5886 | 0.4348 | 0.6191 |
| components-map/scalar | global-map/scalar | 0.9996 | 0.9620 | 1.0060 | 0.9785 | 0.9857 | 0.9773 |
| components-map/all-row | global-map/all-row | 0.9853 | 0.9706 | 0.9878 | 0.9776 | 0.9929 | 0.9751 |
| global-map/all-row | global-map/scalar | 1.2546 | 1.1175 | 1.2791 | 1.3056 | 1.1587 | 1.3318 |

Each memory column is its own maximum over cells/repetitions; these maxima must not be added as if simultaneous. Requested bounds and actual retained capacities are scoped array accounting, while RSS includes runtime, allocator, stacks and decoder. Printed CPU zeros are below resolution; no CPU speedup is inferred.

| Arm | Max requested bytes | Max admitted bytes | Max grouping bytes | Max process RSS bytes |
| --- | --- | --- | --- | --- |
| automatic/all-image | 7816764 | 7816764 | 249528 | 11075584 |
| automatic/all-row | 7682084 | 7682084 | 249528 | 11173888 |
| automatic/fine-image | 7706484 | 7706484 | 139248 | 10911744 |
| automatic/fine-row | 7571804 | 7571804 | 139248 | 10813440 |
| automatic/scalar | 7432556 | 7432556 | 0 | 10371072 |
| components-map/all-row | 6204928 | 6204928 | 139184 | 8830976 |
| components-map/scalar | 6065744 | 6065744 | 0 | 8667136 |
| global-map/all-row | 6204928 | 6204928 | 139184 | 8634368 |
| global-map/scalar | 6065744 | 6065744 | 0 | 8454144 |

Family breakdown of the declared automatic/all-image versus grouped global MAP comparison. This exposes heterogeneity; it does not select a different arm per family.

| Family | Overall | K1 | Repeated |
| --- | --- | --- | --- |
| uniform | 0.4130 | 0.2892 | 0.4382 |
| communities | 0.6634 | 0.4279 | 0.7138 |
| chain | 0.6185 | 0.3902 | 0.6679 |
| pair-dominant | 0.5409 | 0.4192 | 0.5644 |
| nested | 0.7770 | 0.5754 | 0.8169 |
| hubs | 0.4598 | 0.3559 | 0.4799 |
| ragged | 0.8965 | 0.5735 | 0.9657 |
| tensor | 0.4035 | 0.3075 | 0.4222 |
| worker-firm-occupation | 0.5358 | 0.4494 | 0.5517 |
| exporter-importer-product | 0.4500 | 0.3732 | 0.4642 |

Automatic work/routing counts summed once per family/shape/weight input at K1 (not multiplied by repetitions or K):

`{"accepted_hierarchies": 29, "baseline_components": 9, "components": 108, "dense_components": 14, "global_fallback_columns": 0, "hierarchy_attempts": 38, "hierarchy_rejections": 9, "large_components": 38, "quality_rejections": 9, "singleton_components": 56}`

## linux-smoke

15,120 isolated processes; 144,000/144,000 measured columns certified; 0 failed measured and 0 failed warmup attempts. Measured process sum 31.194802s; warmup sum 6.301358s; failed cost 0.000000s.

| Candidate | Control | All overall | All K1 | All repeated | Hard overall | Hard K1 | Hard repeated |
| --- | --- | --- | --- | --- | --- | --- | --- |
| automatic/scalar | global-map/scalar | 1.3490 | 0.9951 | 1.4192 | 1.3819 | 0.9981 | 1.4589 |
| automatic/scalar | global-map/all-row | 1.2526 | 0.9813 | 1.3046 | 1.2816 | 0.9819 | 1.3397 |
| automatic/fine-row | automatic/scalar | 1.0035 | 0.9973 | 1.0045 | 1.0023 | 0.9946 | 1.0036 |
| automatic/fine-row | global-map/scalar | 1.3537 | 0.9924 | 1.4256 | 1.3851 | 0.9927 | 1.4642 |
| automatic/fine-row | global-map/all-row | 1.2569 | 0.9786 | 1.3105 | 1.2845 | 0.9765 | 1.3446 |
| automatic/all-row | automatic/scalar | 1.0032 | 1.0018 | 1.0034 | 1.0005 | 0.9992 | 1.0007 |
| automatic/all-row | global-map/scalar | 1.3533 | 0.9969 | 1.4241 | 1.3826 | 0.9973 | 1.4600 |
| automatic/all-row | global-map/all-row | 1.2566 | 0.9830 | 1.3091 | 1.2822 | 0.9811 | 1.3407 |
| automatic/fine-image | automatic/scalar | 1.0035 | 1.0008 | 1.0040 | 1.0022 | 0.9994 | 1.0027 |
| automatic/fine-image | global-map/scalar | 1.3538 | 0.9959 | 1.4249 | 1.3849 | 0.9975 | 1.4628 |
| automatic/fine-image | global-map/all-row | 1.2570 | 0.9820 | 1.3098 | 1.2844 | 0.9813 | 1.3433 |
| automatic/all-image | automatic/scalar | 1.0043 | 1.0005 | 1.0049 | 1.0032 | 0.9977 | 1.0042 |
| automatic/all-image | global-map/scalar | 1.3547 | 0.9956 | 1.4261 | 1.3864 | 0.9958 | 1.4650 |
| automatic/all-image | global-map/all-row | 1.2579 | 0.9818 | 1.3110 | 1.2857 | 0.9796 | 1.3453 |
| components-map/scalar | global-map/scalar | 1.3506 | 0.9939 | 1.4214 | 1.3826 | 0.9931 | 1.4609 |
| components-map/all-row | global-map/all-row | 1.2558 | 0.9809 | 1.3086 | 1.2820 | 0.9790 | 1.3410 |
| global-map/all-row | global-map/scalar | 1.0770 | 1.0141 | 1.0878 | 1.0783 | 1.0165 | 1.0889 |

Each memory column is its own maximum over cells/repetitions; these maxima must not be added as if simultaneous. Requested bounds and actual retained capacities are scoped array accounting, while RSS includes runtime, allocator, stacks and decoder. Printed CPU zeros are below resolution; no CPU speedup is inferred.

| Arm | Max requested bytes | Max admitted bytes | Max grouping bytes | Max process RSS bytes |
| --- | --- | --- | --- | --- |
| automatic/all-image | 205960 | 205960 | 2216 | 3510272 |
| automatic/all-row | 205960 | 205960 | 2216 | 3543040 |
| automatic/fine-image | 205960 | 205960 | 2216 | 3551232 |
| automatic/fine-row | 205960 | 205960 | 2216 | 3534848 |
| automatic/scalar | 205960 | 205960 | 0 | 3485696 |
| components-map/all-row | 205960 | 205960 | 2216 | 3522560 |
| components-map/scalar | 205960 | 205960 | 0 | 3547136 |
| global-map/all-row | 205192 | 205192 | 4456 | 3567616 |
| global-map/scalar | 205192 | 205192 | 0 | 3579904 |

Automatic work/routing counts summed once per family/shape/weight input at K1 (not multiplied by repetitions or K):

`{"accepted_hierarchies": 0, "baseline_components": 0, "components": 50, "dense_components": 50, "global_fallback_columns": 1, "hierarchy_attempts": 0, "hierarchy_rejections": 0, "large_components": 0, "quality_rejections": 0, "singleton_components": 0}`

## Completeness and interpretation

Total: 45,360 processes and 432,000/432,000 measured columns certified, with 0 failed measured and 0 failed warmup attempts. Warmup work remains separately charged. Full per-cell timing, native/certificate work, routing and capacity records remain in the linked summaries; all raw stdout/stderr, input checksums, journals, executables and build logs are preserved in the archives listed by `preservation.json`.

All45,360 Mac smoke/development and Linux smoke raw records exactly match retained M6m schedule/input/math/work/payload/layout/routing signatures. `comparison.json` records this comparison. Cross-source times are unpaired; no paired sort-removal speedup follows. `preservation.json` records independent original/copy validation, every archive member hash and predeclared Linux derived-geomean archival roundoff. Original summaries remain preserved and raw/scientific fields are exact.

This is a bounded development experiment: one seed, canonical unique tuples, no raw observation-collapse/multiplicity timing, the unchanged smoke/development sizes, unpinned Apple Silicon execution, and Linux smoke only. It is not an out-of-cache study, SCC scaling result, Windows timing comparison or competitive qualification against immutable upstream within/Schwarz. M6 terminal/admission work and M7–M10 remain open.
