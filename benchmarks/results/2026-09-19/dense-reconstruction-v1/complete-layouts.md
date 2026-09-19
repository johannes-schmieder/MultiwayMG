# M6m complete automatic layout regression

Frozen source `55388b14028386effa35e7faaa3a6aa7a9e02b5e`, tree `50c9adc72b7452b2cd906dd5d914801670846dc6`. Only dense reconstruction traversal changes numerical execution; each output retains exactly the same ordered FMAs, payload and work. All timing builds use Rust1.85, release, lsmr only, one worker and zero profiler TLS capacities. See [the fixed experiment](../../../../docs/ISSUE5_AUTOMATIC_LAYOUT_ECONOMICS.md).

Ratios below are complete cold-process control/candidate geometric means: above one favors the candidate. Every declared comparison is reported. Setup, failures, screening, fallback and original certificates remain charged. Aggregates balance families and RHS widths; single and repeated RHS are separate. No layout default, selector, competitive qualification, calibration or holdout decision is made.

## mac-smoke

15,120 isolated processes; 144,000/144,000 measured columns certified; 0 failed measured and 0 failed warmup attempts. Measured process sum 68.267261s; warmup sum 14.029466s; failed cost 0.000000s.

| Candidate | Control | All overall | All K1 | All repeated | Hard overall | Hard K1 | Hard repeated |
| --- | --- | --- | --- | --- | --- | --- | --- |
| automatic/scalar | global-map/scalar | 1.0991 | 0.9934 | 1.1178 | 1.1097 | 0.9968 | 1.1298 |
| automatic/scalar | global-map/all-row | 1.0667 | 0.9842 | 1.0811 | 1.0752 | 0.9808 | 1.0918 |
| automatic/fine-row | automatic/scalar | 1.0031 | 1.0052 | 1.0027 | 1.0033 | 1.0096 | 1.0023 |
| automatic/fine-row | global-map/scalar | 1.1025 | 0.9986 | 1.1208 | 1.1134 | 1.0063 | 1.1323 |
| automatic/fine-row | global-map/all-row | 1.0699 | 0.9893 | 1.0840 | 1.0787 | 0.9902 | 1.0942 |
| automatic/all-row | automatic/scalar | 1.0036 | 1.0042 | 1.0035 | 1.0043 | 1.0088 | 1.0035 |
| automatic/all-row | global-map/scalar | 1.1030 | 0.9976 | 1.1216 | 1.1145 | 1.0056 | 1.1338 |
| automatic/all-row | global-map/all-row | 1.0705 | 0.9883 | 1.0848 | 1.0798 | 0.9895 | 1.0956 |
| automatic/fine-image | automatic/scalar | 1.0028 | 1.0117 | 1.0013 | 1.0025 | 1.0090 | 1.0014 |
| automatic/fine-image | global-map/scalar | 1.1022 | 1.0050 | 1.1193 | 1.1125 | 1.0058 | 1.1314 |
| automatic/fine-image | global-map/all-row | 1.0697 | 0.9957 | 1.0826 | 1.0779 | 0.9897 | 1.0934 |
| automatic/all-image | automatic/scalar | 1.0052 | 1.0095 | 1.0045 | 1.0038 | 1.0065 | 1.0033 |
| automatic/all-image | global-map/scalar | 1.1048 | 1.0029 | 1.1228 | 1.1139 | 1.0033 | 1.1335 |
| automatic/all-image | global-map/all-row | 1.0722 | 0.9935 | 1.0859 | 1.0792 | 0.9872 | 1.0954 |
| components-map/scalar | global-map/scalar | 1.1024 | 0.9940 | 1.1216 | 1.1126 | 0.9997 | 1.1326 |
| components-map/all-row | global-map/all-row | 1.0723 | 0.9895 | 1.0867 | 1.0803 | 0.9829 | 1.0975 |
| global-map/all-row | global-map/scalar | 1.0304 | 1.0094 | 1.0339 | 1.0321 | 1.0163 | 1.0348 |

Each memory column is its own maximum over cells/repetitions; these maxima must not be added as if simultaneous. Requested bounds and actual retained capacities are scoped array accounting, while RSS includes runtime, allocator, stacks and decoder. Printed CPU zeros are below resolution; no CPU speedup is inferred.

| Arm | Max requested bytes | Max admitted bytes | Max grouping bytes | Max process RSS bytes |
| --- | --- | --- | --- | --- |
| automatic/all-image | 205960 | 205960 | 2216 | 2424832 |
| automatic/all-row | 205960 | 205960 | 2216 | 2473984 |
| automatic/fine-image | 205960 | 205960 | 2216 | 2473984 |
| automatic/fine-row | 205960 | 205960 | 2216 | 2473984 |
| automatic/scalar | 205960 | 205960 | 0 | 2441216 |
| components-map/all-row | 205960 | 205960 | 2216 | 2441216 |
| components-map/scalar | 205960 | 205960 | 0 | 2441216 |
| global-map/all-row | 205192 | 205192 | 4456 | 2392064 |
| global-map/scalar | 205192 | 205192 | 0 | 2375680 |

Automatic work/routing counts summed once per family/shape/weight input at K1 (not multiplied by repetitions or K):

`{"accepted_hierarchies": 0, "baseline_components": 0, "components": 50, "dense_components": 50, "global_fallback_columns": 1, "hierarchy_attempts": 0, "hierarchy_rejections": 0, "large_components": 0, "quality_rejections": 0, "singleton_components": 0}`

## mac-development

15,120 isolated processes; 144,000/144,000 measured columns certified; 0 failed measured and 0 failed warmup attempts. Measured process sum 978.301945s; warmup sum 196.080228s; failed cost 0.000000s.

| Candidate | Control | All overall | All K1 | All repeated | Hard overall | Hard K1 | Hard repeated |
| --- | --- | --- | --- | --- | --- | --- | --- |
| automatic/scalar | global-map/scalar | 0.5585 | 0.3902 | 0.5929 | 0.5978 | 0.4273 | 0.6322 |
| automatic/scalar | global-map/all-row | 0.4487 | 0.3491 | 0.4679 | 0.4610 | 0.3698 | 0.4783 |
| automatic/fine-row | automatic/scalar | 1.1481 | 1.0799 | 1.1599 | 1.1911 | 1.1052 | 1.2061 |
| automatic/fine-row | global-map/scalar | 0.6412 | 0.4213 | 0.6877 | 0.7121 | 0.4722 | 0.7625 |
| automatic/fine-row | global-map/all-row | 0.5152 | 0.3769 | 0.5427 | 0.5491 | 0.4087 | 0.5769 |
| automatic/all-row | automatic/scalar | 1.1893 | 1.1101 | 1.2031 | 1.2322 | 1.1302 | 1.2500 |
| automatic/all-row | global-map/scalar | 0.6642 | 0.4331 | 0.7133 | 0.7366 | 0.4829 | 0.7903 |
| automatic/all-row | global-map/all-row | 0.5336 | 0.3875 | 0.5629 | 0.5681 | 0.4179 | 0.5979 |
| automatic/fine-image | automatic/scalar | 1.1816 | 1.1056 | 1.1947 | 1.2187 | 1.1247 | 1.2351 |
| automatic/fine-image | global-map/scalar | 0.6599 | 0.4314 | 0.7083 | 0.7286 | 0.4806 | 0.7809 |
| automatic/fine-image | global-map/all-row | 0.5302 | 0.3859 | 0.5590 | 0.5618 | 0.4159 | 0.5907 |
| automatic/all-image | automatic/scalar | 1.2348 | 1.1471 | 1.2501 | 1.2721 | 1.1649 | 1.2909 |
| automatic/all-image | global-map/scalar | 0.6896 | 0.4475 | 0.7412 | 0.7605 | 0.4978 | 0.8162 |
| automatic/all-image | global-map/all-row | 0.5541 | 0.4004 | 0.5849 | 0.5865 | 0.4308 | 0.6174 |
| components-map/scalar | global-map/scalar | 1.0010 | 0.9654 | 1.0071 | 0.9805 | 0.9857 | 0.9796 |
| components-map/all-row | global-map/all-row | 0.9860 | 0.9637 | 0.9897 | 0.9803 | 0.9971 | 0.9775 |
| global-map/all-row | global-map/scalar | 1.2447 | 1.1178 | 1.2673 | 1.2967 | 1.1555 | 1.3219 |

Each memory column is its own maximum over cells/repetitions; these maxima must not be added as if simultaneous. Requested bounds and actual retained capacities are scoped array accounting, while RSS includes runtime, allocator, stacks and decoder. Printed CPU zeros are below resolution; no CPU speedup is inferred.

| Arm | Max requested bytes | Max admitted bytes | Max grouping bytes | Max process RSS bytes |
| --- | --- | --- | --- | --- |
| automatic/all-image | 7816764 | 7816764 | 249528 | 10633216 |
| automatic/all-row | 7682084 | 7682084 | 249528 | 10928128 |
| automatic/fine-image | 7706484 | 7706484 | 139248 | 10616832 |
| automatic/fine-row | 7571804 | 7571804 | 139248 | 10338304 |
| automatic/scalar | 7432556 | 7432556 | 0 | 10354688 |
| components-map/all-row | 6204928 | 6204928 | 139184 | 8650752 |
| components-map/scalar | 6065744 | 6065744 | 0 | 8650752 |
| global-map/all-row | 6204928 | 6204928 | 139184 | 8617984 |
| global-map/scalar | 6065744 | 6065744 | 0 | 8404992 |

Family breakdown of the declared automatic/all-image versus grouped global MAP comparison. This exposes heterogeneity; it does not select a different arm per family.

| Family | Overall | K1 | Repeated |
| --- | --- | --- | --- |
| uniform | 0.3992 | 0.2697 | 0.4262 |
| communities | 0.6581 | 0.4236 | 0.7082 |
| chain | 0.6111 | 0.3826 | 0.6607 |
| pair-dominant | 0.5425 | 0.4202 | 0.5661 |
| nested | 0.7751 | 0.5697 | 0.8159 |
| hubs | 0.4654 | 0.3502 | 0.4879 |
| ragged | 0.8917 | 0.5811 | 0.9576 |
| tensor | 0.4044 | 0.3020 | 0.4246 |
| worker-firm-occupation | 0.5409 | 0.4584 | 0.5560 |
| exporter-importer-product | 0.4449 | 0.3592 | 0.4610 |

Automatic work/routing counts summed once per family/shape/weight input at K1 (not multiplied by repetitions or K):

`{"accepted_hierarchies": 29, "baseline_components": 9, "components": 108, "dense_components": 14, "global_fallback_columns": 0, "hierarchy_attempts": 38, "hierarchy_rejections": 9, "large_components": 38, "quality_rejections": 9, "singleton_components": 56}`

## linux-smoke

15,120 isolated processes; 144,000/144,000 measured columns certified; 0 failed measured and 0 failed warmup attempts. Measured process sum 31.030387s; warmup sum 6.255119s; failed cost 0.000000s.

| Candidate | Control | All overall | All K1 | All repeated | Hard overall | Hard K1 | Hard repeated |
| --- | --- | --- | --- | --- | --- | --- | --- |
| automatic/scalar | global-map/scalar | 1.3527 | 0.9928 | 1.4243 | 1.3826 | 0.9914 | 1.4615 |
| automatic/scalar | global-map/all-row | 1.2570 | 0.9808 | 1.3101 | 1.2844 | 0.9775 | 1.3442 |
| automatic/fine-row | automatic/scalar | 1.0029 | 1.0023 | 1.0030 | 1.0028 | 1.0037 | 1.0026 |
| automatic/fine-row | global-map/scalar | 1.3567 | 0.9951 | 1.4286 | 1.3865 | 0.9951 | 1.4653 |
| automatic/fine-row | global-map/all-row | 1.2607 | 0.9830 | 1.3141 | 1.2880 | 0.9811 | 1.3477 |
| automatic/all-row | automatic/scalar | 1.0040 | 0.9996 | 1.0048 | 1.0035 | 1.0008 | 1.0039 |
| automatic/all-row | global-map/scalar | 1.3582 | 0.9924 | 1.4311 | 1.3874 | 0.9922 | 1.4672 |
| automatic/all-row | global-map/all-row | 1.2621 | 0.9804 | 1.3164 | 1.2888 | 0.9783 | 1.3495 |
| automatic/fine-image | automatic/scalar | 1.0043 | 0.9988 | 1.0053 | 1.0040 | 1.0001 | 1.0047 |
| automatic/fine-image | global-map/scalar | 1.3586 | 0.9916 | 1.4318 | 1.3882 | 0.9915 | 1.4683 |
| automatic/fine-image | global-map/all-row | 1.2625 | 0.9796 | 1.3170 | 1.2896 | 0.9776 | 1.3505 |
| automatic/all-image | automatic/scalar | 1.0032 | 0.9995 | 1.0038 | 1.0030 | 1.0001 | 1.0035 |
| automatic/all-image | global-map/scalar | 1.3570 | 0.9923 | 1.4297 | 1.3868 | 0.9915 | 1.4666 |
| automatic/all-image | global-map/all-row | 1.2610 | 0.9803 | 1.3151 | 1.2883 | 0.9776 | 1.3489 |
| components-map/scalar | global-map/scalar | 1.3562 | 0.9937 | 1.4283 | 1.3884 | 0.9945 | 1.4678 |
| components-map/all-row | global-map/all-row | 1.2622 | 0.9815 | 1.3162 | 1.2902 | 0.9803 | 1.3506 |
| global-map/all-row | global-map/scalar | 1.0761 | 1.0122 | 1.0871 | 1.0765 | 1.0142 | 1.0872 |

Each memory column is its own maximum over cells/repetitions; these maxima must not be added as if simultaneous. Requested bounds and actual retained capacities are scoped array accounting, while RSS includes runtime, allocator, stacks and decoder. Printed CPU zeros are below resolution; no CPU speedup is inferred.

| Arm | Max requested bytes | Max admitted bytes | Max grouping bytes | Max process RSS bytes |
| --- | --- | --- | --- | --- |
| automatic/all-image | 205960 | 205960 | 2216 | 3522560 |
| automatic/all-row | 205960 | 205960 | 2216 | 3461120 |
| automatic/fine-image | 205960 | 205960 | 2216 | 3567616 |
| automatic/fine-row | 205960 | 205960 | 2216 | 3510272 |
| automatic/scalar | 205960 | 205960 | 0 | 3510272 |
| components-map/all-row | 205960 | 205960 | 2216 | 3485696 |
| components-map/scalar | 205960 | 205960 | 0 | 3510272 |
| global-map/all-row | 205192 | 205192 | 4456 | 3469312 |
| global-map/scalar | 205192 | 205192 | 0 | 3506176 |

Automatic work/routing counts summed once per family/shape/weight input at K1 (not multiplied by repetitions or K):

`{"accepted_hierarchies": 0, "baseline_components": 0, "components": 50, "dense_components": 50, "global_fallback_columns": 1, "hierarchy_attempts": 0, "hierarchy_rejections": 0, "large_components": 0, "quality_rejections": 0, "singleton_components": 0}`

## Completeness and interpretation

Total: 45,360 processes and 432,000/432,000 measured columns certified, with 0 failed measured and 0 failed warmup attempts. Warmup work remains separately charged. Full per-cell timing, native/certificate work, routing and capacity records remain in the linked summaries; all raw stdout/stderr, input checksums, journals, executables and build logs are preserved in the archives listed by `slices-preservation.json`.

All 45,360 Mac smoke/development and Linux smoke raw records exactly match the corresponding M6l schedule, input, mathematical, work, payload, layout and routing signatures. `slices-comparison.json` records this comparison. Timings across source revisions are unpaired and cannot establish a paired dense-change speedup. `slices-preservation.json` records independent copy validation, every archive member checksum and scoped cross-host rounding in derived geometric means. Scientific records and hashes remain exact.

This is a bounded development experiment: one seed, canonical unique tuples, no raw observation-collapse/multiplicity timing, the unchanged smoke/development input sizes, unpinned Apple Silicon execution, and Linux smoke only. It is not an out-of-cache study, SCC scaling result, Windows timing comparison or competitive qualification against immutable upstream within/Schwarz. M6 terminal/admission work and M7–M10 remain open.
