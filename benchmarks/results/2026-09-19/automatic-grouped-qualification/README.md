# Automatic grouped ownership qualification (M6j)

[Records](records.json) preserve 39 exact debug/release peak-allocation controls:
11 original scalar controls and 28 explicit grouped controls. All eleven scalar
allocation records exactly match M6h. Two repetitions at K=1,2,4,8,16,17,32 give
6,240 certified columns per LSMR-enabled configuration. Minimal-feature builds
exercise root controls only. Four additional denied-grouping controls execute
component and final-global rejection with zero allocations and no accepted result.

Grouped controls cover dense terminals, two/three transitions, changed current
weights, forced construction fallback and mixed large/dense/singleton components.
Allocation counts, total requested bytes and peak bytes are identical across K,
repetition and debug/release. Every net allocation count/byte total returns to
zero. Existing owners/caller arrays plus measured new heap fit the declared
array admission. These are isolated serial setup/solve/drop checks; they do not
establish parallel allocator behavior or comparative speed.

LSMR baselines never apply the Gramian. Their FineImage/AllImage requests now
use exactly the same allocation record as FineRow/AllRow, with no unused tuple
image. Hierarchy image layouts retain one shared image. For the E=2048 baseline
control, avoiding dead storage removes one allocation and 16,384 peak bytes
relative to the initial, uncommitted M6j implementation; those development logs
remain outside the repository. No timing gain is inferred from this byte count.

Four scientific integration tests independently certify outputs and compare
coefficient bits, certificates and all structural/screen/native/gate work to
scalar references across K, changed weights, recursive depths, extra nullity,
multiple component kinds, explicit rejected screens, rank-truncation recovery,
static validation, exact budgets, caller live bytes and failure reuse. The final
explicit-screen assertion was rerun in debug/release and passes strict Clippy.
Required Rust1.85 checks, 105 Python validators and twelve release/scientific
groups pass. Debug/release automatic protocols each pass 241 checks; existing
layout protocol passes 120 comparisons. See [check receipts](checks.json) and
[the ownership contract](../../../../docs/ISSUE5_AUTOMATIC_GROUPED_LAYOUTS.md).

Logs and initial failed lint/development checks are preserved under
`$GIT_HOME/MultiwayMG-assessments/2026-09-19-m6j` (current machine
`/Users/johannes/Git/MultiwayMG-assessments/2026-09-19-m6j`). All exact-source/PR CI passes. [Cross-platform receipts](cross-platform.json)
match every control in Linux/macOS/Windows debug/release. The [frozen scalar
compatibility evidence](../prepared-automatic-grouped-compatibility/README.md)
retains 25,200 exact M6i numerical/work/payload records and its known negatives. New grouped performance needs a separate committed comparison policy.
M6 remains open and no default or competitive claim is made.
