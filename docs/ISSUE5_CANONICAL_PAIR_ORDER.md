# Canonical pair ordering (M6n)

M6m PR66 merged as `98ad3e5836a3f8374711a2e4195f64e9238d7214`, with actual
clean main matching reviewed tree `59802754ae0d694ff2b529c1273cf3553ee10d9c`.
All36 final source/PR jobs pass in35448848847/35448848827 and
35448850898/35448850891. Its application improvement and complete-solver limits
remain in [dense reconstruction](ISSUE5_DENSE_RECONSTRUCTION.md). Development
all-image still trails grouped MAP by1.81x; M6 and subsequent milestones are open.

## Exact ordering boundary

The six directed pair marginals currently sort a reusable source-ID array by
`(tuple[neighbor], tuple[factor], source_id)`. Prepared topology guarantees
strictly increasing unique `(a,b,c)` tuples. Consequently, for neighbor0/factor1,
`(a,b,source_id)` is exactly the identity order. Replace this one comparison sort
with an ascending reset of the existing compact-u32 or wide-usize array.

This is the third pass in the unchanged six-pass schedule. Earlier passes have
permuted the array, so merely skipping the call would be incorrect. The other
five sorts are unchanged and receive the same ID order as before. Pair masses
still add weights in increasing original-ID order. Mass/level tie resolution,
proposal emission and ordinal order, overlap sums, normalization and greedy
matching are unchanged for both legacy top-k and adjacent-pair policies.

The private method now takes `&PreparedThreeWayTopology` instead of a generic
tuple slice. Raw observations are sorted/collapsed before that owner is published;
collapsed and symbolic coarse owners reject noncanonical keys. Component-local
roots preserve canonical order by monotone recoding of ascending original tuples.
Generic `ThreeWayTopology::new` does not provide this guarantee and is not an
eligible boundary for the shortcut. No repeated canonical validation scan or
public API is added.

There is no second index array, histogram, cached pair table, changed lifetime,
new allocation or admission category. The reset performs E sequential writes;
it removes one comparison sort without changing the six marginal tuple visits.
Existing work counters count those visits and proposals, not sorting comparisons.
Requested/admitted payload bounds and failure/ownership semantics remain fixed.
No asymptotic claim about the entire six-sort builder or full solver is implied.

## Qualification fixed before collection

Compare exact visitation against the independent old total-key sort for all six
pairs and two successive cycles. Cover compact and forced-wide IDs, identity/
reverse/rotated starts, tensor and irregular keys with tied first-two coordinates,
and singleton input. Enter through unsorted duplicated raw observations to verify
the prepared boundary. Preserve integer-width/overflow checks and reservation
failure, unwind, one-byte-under admission and reconstruction tests.

Existing independent legacy/adjacent ordered-map references, provisional coarse
frames, disconnected component roots, actual recursive screening, PCG/LSMR and
full driver/allocation gates remain required. Run Rust1.85 formatting, strict
Clippy, all/minimal workspace tests and rustdoc;135 Python tests; release
scientific gates and actual debug/release diagnostic/legacy/layout protocols.

After a clean committed source freeze, rerun the unchanged complete automatic
layout policy on Mac smoke/development and Linux smoke. Compare every scheduled
raw mathematical/work/payload/layout/routing signature with the corresponding
retained M6m collection and all six platform/build allocation controls. Preserve
all17 within-source cost comparisons, all/hard, single/repeated, every family,
failed work, process costs and memory scopes. Cross-source times are unpaired;
do not infer a paired sort-removal speedup. No new microbenchmark or adaptive
size threshold is used. Expanded reruns are required if failures, changed work/
capacity/routing or size-specific concerns arise.

Raw originals, independent copies and every archive member must validate, and
GitHub archive SHA256s must match provider metadata. Preserve original summaries;
only derived Linux geometric-mean archival recomputation may differ within the
already declared1e-14 relative bound. Raw timings/scientific fields remain exact.

Commit full evidence, review it, require every final source/PR CI job green,
merge with exact head/base guards and verify actual clean main before the next
increment. This does not change numerical policy, scalar default, compiler flags,
calibration, holdout or competitive status. Sparse-terminal bounded admission,
M7 explicit CPU pools, M8 independent panels, M9 current-weight replay and M10
qualification remain open. Forward-transform interleaving is a separate possible
application experiment; a changed smoother requires its own numerical proof and
screening/cost policy.

All required Rust1.85/135 Python checks and eight release scientific/protocol
groups pass before source freeze. Both actual protocols retain1,125 exact pairs.
See [source qualification](../benchmarks/results/2026-09-19/canonical-pair-order-qualification/README.md).
M6m post-merge CI35449392966/35449392967 passes all18 jobs.


## Complete frozen result

Measured source `f4a611d61287e2375c1812232f1df1ac451605a1`, tree
`4f7376c270d919a4108259e7ae6576b936357e48`, passes all 36 source/PR jobs:
35449932557/35449932569 and 35449934477/35449934536. Three complete collections
retain 45,360 processes and 432,000 certified measured columns, with zero failed
warmups or measurements. Every scheduled input, mathematical, work, payload,
layout and routing signature matches retained M6m exactly. All six Linux/macOS/
Windows debug/release logs preserve 39 allocator controls, four zero-allocation
denials and 6,240 certified columns per configuration.

Originals, independent copies and every deterministic archive member validate.
Provider ZIP sizes and SHA256 digests match fresh GitHub metadata. Mac summaries
recompute exactly; 15 Linux derived geometric means differ only within the
predeclared 1e-14 archival bound. Original summaries and all raw/scientific fields
are preserved. See [all 17 comparisons and full memory/family results](../benchmarks/results/2026-09-19/canonical-pair-order-v1/README.md).

On Mac development, automatic all-image versus automatic scalar is 1.247244x.
Against grouped MAP it is 0.556642x overall, 0.406319x for one RHS and 0.586626x
for repeated RHS: about 1.80x slower overall. Every whole-family comparison is
negative. Nine internal quality rejections retain their work and baseline
fallback. Smoke has only small direct terminals and does not exercise candidate
construction. Cross-revision timings are unpaired and establish no sort-removal
speedup. No expanded rerun was triggered: numerical/work/routing/capacity fields
are unchanged and canonical order/admission/failure and size-boundary gates pass.

The change removes one provably redundant comparison sort without adding memory;
it does not establish competitiveness or finish M6. Final evidence-head review,
all source/PR CI and guarded actual-main verification remain the delivery gates.
Next follow the [sparse-terminal boundary plan](ISSUE5_SPARSE_TERMINAL_BOUNDARY.md).
