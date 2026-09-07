# One prepared cycle arena (M5g)

M5f fused prolong-add is merged. Prepared hierarchy scratch now owns one
contiguous arena containing the transactional fine solution, the four live
traversal slices per transition, and the optional maximum-E tuple image at its
tail. All floating-point lengths and arithmetic are unchanged. The arena has
`V0 + sum(2*V_level + 2*V_child) + max_selected_E_if_image` elements. Its full
length, bytes and allocation limit are checked before the first reservation.

A private monomorphized scratch-splitting trait lends disjoint frame slices and
the child tail through safe `split_at_mut`. No offsets vector, pointer table,
self references, unsafe aliasing or runtime storage enum is needed. The ordinary
hierarchy keeps separate vectors and uses the same recurrence through the other
trait implementation. The image tail is reborrowed across all levels and outer
PCG; it is not live across another image action. Since a nonempty selected
prefix begins at the fine level and mapped tuple counts cannot increase, its
maximum E is the fine count; tail lookup needs no per-action level scan. Public owner/dimension/finite
validation still precedes mutation, and failed numerical actions publish no
output. Every later call fully initializes all live scratch.

The original outer vector descriptor list is eliminated, as are all separate
traversal/image reservations. Relative to M5f, a depth-d workspace removes
`1 + 4*d + image_enabled` allocations and
`24*(1 + 4*d + image_enabled)` descriptor bytes on the qualified 64-bit targets.
The inline arena vector replaces the previous inline vector-of-vectors, so
its descriptor size does not grow. All numeric element capacities remain the
same. MAP, per-level projection, terminal modal scratch, outer Krylov arrays
and local LSMR history remain separately owned where their bindings require it.

| Two-transition complete workspace | M5f scalar/rows | M5f image | M5g any layout |
| --- | ---: | ---: | ---: |
| Cycle | 23 | 24 | 14 |
| PCG | 34 | 35 | 25 |
| Native or gated LSMR | 40 | 41 | 31 |

These are exact heap-array allocation counts, not peak-RSS estimates. The image
adds only `8*max(E_selected)` values inside the arena, with zero additional
descriptor bytes or allocations. Scalar remains default. The removed descriptor
bytes are modest; fewer allocations do not prove a wall-clock improvement.

## Evidence protocol and gates

[Layout policy v2](../benchmarks/policies/prepared-layout-v2.json) changes the
reported image descriptor ABI from 24 to 0 and declares the shared arena. Inputs,
five explicit layouts, native/certificate tolerances, process/RSS budgets,
rotation and profile sizes remain unchanged. The v1 policy is retained verbatim.
Historical validation chooses the committed source/policy and memory scopes by
the explicit policy revision; v1 and v2 cannot silently exchange descriptor
claims. Ordinary scalar schema2 and the canonical input generator are unchanged.

Initial tests caught a fixture mismatch: the ordinary supplied hierarchy requires
strict dimension reduction and rejects identity maps. Ragged/relabelled integration
tests still use that ordinary comparison. A private identity/ragged test instead
compares safe arena storage against separately allocated legacy traversal vectors
through the same fixed recurrence, including image mode and poisoned scratch.
The ordinary acceptance rule was not changed. An adversarial descriptor mutation
was also updated because zero is now the correct v2 value; both policy versions
remain independently tested.

Required Rust1.85 format, strict Clippy, all/minimal tests and warning-free docs
pass, together with all 90 Python evidence tests. Reservation-error/unwind,
budget-minus-one, exact bytes/release, zero first/repeat32 action allocation,
foreign owner, numerical failure and recovery gates pass with the new counts.
Release and actual probe checks precede source freeze. Then run Mac smoke/
development/expanded and exact-source Linux smoke under committed v2. Compare
every numerical/work/fingerprint result to M5f, allowing only the derived
hierarchy/total descriptor delta and the declared image ABI/storage changes.
Preserve all attempted costs and failures. No M5g measurement or timing claim
exists at this source checkpoint; campaign calibration/holdout remain untouched.

Final release all/minimal complete solver and allocator gates pass, as do 120
actual release v2 protocol comparisons. Preserved Mac/Linux M5f v1 artifacts
revalidate and their summaries reproduce exactly. These checks precede source
freeze and performance collection.
