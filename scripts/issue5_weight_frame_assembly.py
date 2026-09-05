from pathlib import Path

root = Path('crates/multiway-incidence/src')

def replace_once(path, old, new):
    text = path.read_text()
    assert text.count(old) == 1, (str(path), old[:80], text.count(old))
    path.write_text(text.replace(old, new, 1))

replace_once(root / 'lib.rs', 'mod topology;\n', 'mod topology;\nmod weight_frame;\n')
replace_once(root / 'lib.rs', 'pub use topology::ThreeWayTopology;\n', '''pub use topology::ThreeWayTopology;
pub use weight_frame::{
    ComponentWeightRange, ThreeWayWeightFrame, WeightFrameBinding, WeightFrameInput,
    WeightFrameInputKind, WeightFramePayloadBudget, WeightFrameSetupReport,
    WeightFrameValidationReport,
};
''')
replace_once(root / 'error.rs', 'pub enum IncidenceError {\n', '''pub enum IncidenceError {
    /// Observation weights require a prepared source with original row groups.
    #[error("observation weight input requires an observation-prepared topology")]
    WeightFrameObservationLayoutRequired,
    /// A numerical token names another immutable weight frame.
    #[error("weight binding belongs to a different numerical frame")]
    WeightFrameBindingMismatch,
    /// Checked frame-array reservation failed.
    #[error("weight frame array allocation failed in {context}")]
    WeightFrameAllocation {
        /// Array reservation boundary.
        context: &'static str,
    },
    /// The declared live requested-array payload budget is insufficient.
    #[error("weight frame setup requires {required} payload bytes, budget is {budget}")]
    WeightFrameBudgetExceeded {
        /// Sum of requested new arrays and declared live payload, not process memory.
        required: usize,
        /// Caller-declared payload budget.
        budget: usize,
    },
    /// A weighted degree is not representable as a finite strictly positive value.
    #[error("factor {factor} level {level} has invalid weighted degree {value}")]
    InvalidWeightedDegree {
        /// Zero-based factor index.
        factor: usize,
        /// Factor-local level.
        level: usize,
        /// Rejected accumulated value.
        value: f64,
    },
    /// Another derived frame value is not finite and strictly positive.
    #[error("invalid weight frame {context} at index {index}: {value}")]
    InvalidWeightFrameDerivedValue {
        /// Derived quantity being checked.
        context: &'static str,
        /// Zero-based tuple or component index, as named by context.
        index: usize,
        /// Rejected numerical value.
        value: f64,
    },
''')
old_degree = '''        for (&tuple, &weight) in topology.tuples().iter().zip(&weights) {
            for factor in 0..3 {
                let index = topology.global_index(factor, tuple[factor]);
                neumaier_add(
                    &mut diagonal[index],
                    &mut diagonal_correction[index],
                    weight,
                );
            }
        }
        for (value, correction) in diagonal.iter_mut().zip(diagonal_correction) {
            *value += correction;
        }
'''
replace_once(root / 'problem.rs', old_degree, '''        fill_weighted_degrees(&topology, &weights, &mut diagonal, &mut diagonal_correction);
        drop(diagonal_correction);
''')
replace_once(root / 'problem.rs', '#[derive(Debug, Clone, Copy, Default)]\npub(crate) struct CompensatedSum', '''// Both owned problems and prepared frames use the original degree recurrence.
// Internal callers validate all dimensions and numerical inputs before entry.
pub(crate) fn fill_weighted_degrees(
    topology: &ThreeWayTopology,
    weights: &[f64],
    diagonal: &mut [f64],
    correction: &mut [f64],
) {
    debug_assert_eq!(weights.len(), topology.tuple_count());
    debug_assert_eq!(diagonal.len(), topology.total_levels());
    debug_assert_eq!(correction.len(), diagonal.len());
    diagonal.fill(0.0);
    correction.fill(0.0);
    for (&tuple, &weight) in topology.tuples().iter().zip(weights) {
        for factor in 0..3 {
            let index = topology.global_index(factor, tuple[factor]);
            neumaier_add(&mut diagonal[index], &mut correction[index], weight);
        }
    }
    for (value, &adjustment) in diagonal.iter_mut().zip(correction.iter()) {
        *value += adjustment;
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CompensatedSum''')
replace_once(root / 'weight_frame.rs', "impl WeightFrameInput<'_> {", "impl<'a> WeightFrameInput<'a> {")
replace_once(root / 'weight_frame.rs', 'fn values(self) -> Option<&[f64]> {', "fn values(self) -> Option<&'a [f64]> {")
replace_once(root / 'weight_frame.rs', '''/// not evidence that a downstream factorization was built for this generation.
#[derive(Debug)]''', '''/// not evidence that a downstream factorization was built for this generation.
///
/// A frame cannot outlive its symbolic owner:
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
/// let frame = {
///     let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
///     ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap()
/// };
/// assert_eq!(frame.weights(), &[1.0]);
/// ```
#[derive(Debug)]''')
path = Path('crates/multiway-incidence/tests/weight_frames.rs')
path.write_text(path.read_text() + '''
#[test]
fn published_frame_owns_its_values_independently_of_the_input_slice() {
    let topology = PreparedThreeWayTopology::try_from_observations([1; 3], &[[0; 3]; 2]).unwrap();
    let mut values = [2.0, 3.0];
    let frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Observations(&values)).unwrap();
    values.fill(f64::NAN);
    bits(frame.weights(), &[5.0]);
    bits(frame.diagonal(), &[5.0; 3]);
    assert!(values.iter().all(|value| value.is_nan()));
}
''')
path = Path('crates/multiway-mg/tests/workspace_allocations.rs')
replace_once(path, 'mod symbolic_map_allocations;\n', '''mod symbolic_map_allocations;
#[path = "support/weight_frame_allocations.rs"]
mod weight_frame_allocations;
''')
replace_once(path, '    symbolic_map_allocations::run()?;\n', '    symbolic_map_allocations::run()?;\n    weight_frame_allocations::run()?;\n')
replace_once(Path('CHANGELOG.md'), '### Added\n', '''### Added

- Immutable validated weight frames with explicit input layouts, numerical-owner
  bindings, compensated duplicate weights/degrees, component diagnostics and
  checked live setup-payload accounting; see `docs/ISSUE5_WEIGHT_FRAMES.md`.
''')
Path('docs/ISSUE5_WEIGHT_FRAMES.md').write_text('''# Issue 5: validated immutable numerical weight frames

## Implemented boundary

`ThreeWayWeightFrame` borrows an exact `PreparedThreeWayTopology` and owns one
immutable numerical generation: canonical positive finite tuple weights, their
positive finite square roots, positive finite weighted degrees, and per-component
minimum/maximum tuple weights and degrees. Construction never sorts tuple keys,
rediscovers components or copies symbolic topology. Getters and validation reports
are allocation-free. The frame owns its numerical arrays independently of the
submitted slice, which may be changed or discarded after successful construction.

`WeightFrameInput` explicitly distinguishes original `Observations`, canonical
`Tuples`, `UnitObservations` and `UnitTuples`. Lengths are checked, not used to infer
meaning. Observation variants require an observation-prepared topology; a collapsed
source has no original physical-row groups. Tuple variants work with either source
kind and always refer to canonical unique tuples. Unit observation input includes
duplicate multiplicity; unit tuple input does not. Implicit ones avoid allocating an
input ones-vector, but the frame's retained weight/root arrays are materialized.

Observation duplicate sums use the existing `CompensatedSum` in increasing original
row order within each prepared group. The degree recurrence is extracted unchanged
into one private kernel shared with existing `ThreeWayProblem` construction. It
visits canonical tuples and factors in the original order. Neither solver/kernel
application paths nor floating-point stopping rules change. The ordinary problem
constructor keeps its prior acceptance behavior; only this new frame path adds
explicit finite/positive checks on every published derived quantity.

## Numerical acceptance and deliberate limits

Explicit inputs reject nonpositive or nonfinite values before array reservation;
`InvalidWeight.tuple_index` names the submitted observation or canonical-tuple row
according to the explicit input variant. Duplicate overflow rejects with its tuple
key. Individually finite tuple totals can still overflow a degree shared by several
tuples; the frame rejects and reports the factor/level. Square roots and all component
extrema must also pass finite/positive validation before a frame is published.

No positive weight is thresholded, deleted, rescaled or floored. Positive subnormals
and a singleton `f64::MAX` weight are retained exactly. Min/max diagnostics avoid
forming an overflow-prone ratio. A valid frame does NOT ensure reciprocal degrees,
operator images, norm products or solves are representable, well-conditioned or
identified. Safe scaled norm-bound reporting, rank diagnostics and downstream
operator certification remain separate work; this first frame stores no inverse
degrees, floating-point norm bound, numerical factors or hierarchy quality decision.

## Numerical identity versus symbolic identity

`frame.topology_binding()` names the prepared symbolic owner. `frame.binding()`
returns `WeightFrameBinding`, which borrows this exact immutable numerical frame.
Independent frames have different bindings even if every numerical value is equal.
Two frames under the same topology share its symbolic binding, not numerical identity.
An old frame may remain valid and live alongside a new frame; it is not globally
revoked. Any future derived factorization must bind to the specific frame used to
construct it and reject a different frame, rather than infer validity from topology.

There is no owning Clone, in-place reweighting API, global generation counter, hash
identity, hidden heap token, unsafe state, interior mutability or serialization ID.
Bindings cannot outlive a frame or remain usable after moving/replacing its owner;
frames cannot outlive topology. Immutable references support concurrent use. Three
compile-fail examples protect these lifetime boundaries. `copy_weights_into` checks
exact topology and frame ownership plus output length before touching caller output.
Raw getter slices remain inspection views, not downstream-generation certificates.

Numerical slices cannot reveal external sample IDs, reordered rows or changed factor
meanings. Callers must still preserve the declared layout and rebuild topology when
those semantics change. Existing hierarchy/solver APIs do not accept these frames
and are NOT retroactively protected by the new numerical binding. No production
coarse-weight or pair-conductance replay is included in this increment.

## Construction memory and failure behavior

All five new arrays use checked fallible reservation: weights, roots, degrees,
temporary degree corrections, and component ranges. `setup_payload_report` counts
new requested arrays (including scratch), actual borrowed-topology payload, the
submitted numerical slice by length, and caller-declared other live payload.
Unit inputs contribute no input-slice bytes. Additional live payload must include,
for example, an older frame kept alive during replacement and unused input-vector
capacity. Equality admits and one byte short rejects before any new reservation.
Integer overflow rejects. Reports check layout/size, not numerical input validity.

This is a conservative live REQUESTED-ARRAY ledger, not exact allocator peak memory
or an RSS cap. Some requested arrays have disjoint lifetimes. Allocator headers,
rounding/excess capacity, inline objects and stack are excluded. A submitted slice
aliasing an already charged older frame is conservatively counted twice; no general
alias deduplication is attempted. Existing PCG/hierarchy payload reports do not
silently discover these external objects. After construction, `retained_payload_bytes`
counts actual exclusive capacities, excluding borrowed topology and released scratch.
Charge each distinct old/new numerical owner separately and shared topology once.

Failures publish no partial frame. Source topology, submitted weights and old frames
are unchanged; owned partial arrays are dropped normally. Local private callbacks
exercise every reservation boundary with errors and caught unwinds, but these are
not OS allocator-null tests. Actual duplicate/degree-overflow cleanup is separately
measured by the isolated allocator executable.

## Qualification

Require exact-head Rust 1.85 GitHub Actions and all existing permanent scientific
checks, plus the unchanged 12-configuration allocator matrix: Linux/macOS/Windows,
debug/release, minimal/all features. Independent test-only compensated references
check duplicate totals and degrees without calling the shared production kernel;
valid values also match fresh ordinary construction bit-for-bit. Unit variants,
input ownership, disconnected extreme weights, malformed values, duplicate/degree
overflow, topology/frame mismatch, budget/overflow, concurrency and lifetimes are
covered. No tolerance or frozen scientific evidence is weakened.

The isolated allocator process adds 24 frame cases (four datasets, explicit/unit
observation input, and explicit/unit tuple input for raw/collapsed sources). It checks
first/64-repeated report/identity/copy calls and static rejection for zero allocations,
reallocations and deallocations. Five setup allocations minus released correction
scratch must equal the retained report; destroying a frame releases exactly that
payload while topology and an old frame remain alive. Sixteen rebuild/drop cycles
per case balance their individual ledgers. Numeric failures must release all partial
arrays, not merely recover at the end of a process. Previous allocation tests remain.

## Next

Replay coarse weights and pair conductances through the stored deterministic groups,
with explicit parent-frame and map provenance. Then integrate generation-bound
numerical hierarchies, rebuilding every numerical quantity and re-screening map
quality. The old pair constructors' ordinary accumulation versus compensated replay
must be compared explicitly. Owning multi-level prepared containers, pair-component
metadata, scaled norms, full construction-lifetime admission, LSMR storage, panels,
pools and performance/changing-weight qualification remain open. ADR 0002 is unchanged;
no fresh holdout, timing win, changed-weight solve or closure of issue #5 is implied.
''')
