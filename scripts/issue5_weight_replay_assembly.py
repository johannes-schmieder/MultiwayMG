#!/usr/bin/env python3
"""One-time source integration; excluded from the clean PR and its ancestry."""
from pathlib import Path

def replace_once(path, old, new):
    p = Path(path)
    text = p.read_text()
    assert text.count(old) == 1, (path, old[:80], text.count(old))
    p.write_text(text.replace(old, new, 1))

wf = 'crates/multiway-incidence/src/weight_frame.rs'
anchor = '        let mut square_root_weights = reserve_frame(count, "frame square roots", before)?;'
replace_once(wf, anchor, '''        Self::finish_with(
            topology,
            weights,
            input.kind(),
            input.validate_layout(topology)?,
            before,
        )
    }

    // Both submitted weights and replayed coarse weights finish through this
    // checked path. Ownership transfers without an extra numerical-array copy.
    pub(crate) fn finish_with<F>(
        topology: &'topology PreparedThreeWayTopology,
        weights: Vec<f64>,
        input_kind: WeightFrameInputKind,
        input_count: usize,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let shape = topology.topology();
        let count = shape.tuple_count();
        if weights.len() != count {
            return Err(crate::error::dimension("frame finishing weights", count, weights.len()));
        }
        for (tuple_index, &weight) in weights.iter().enumerate() {
            if !weight.is_finite() || weight <= 0.0 {
                return Err(IncidenceError::InvalidWeight { tuple_index, weight });
            }
        }
''' + anchor)
replace_once(wf, '''                input_kind: input.kind(),
                input_count: input.validate_layout(topology)?,''', '''                input_kind,
                input_count,''')
replace_once(wf, 'fn reserve_frame<T, F>(', 'pub(crate) fn reserve_frame<T, F>(')
replace_once('crates/multiway-incidence/src/error.rs', 'pub enum IncidenceError {', '''pub enum IncidenceError {
    /// A replay was supplied with a different symbolic map owner.
    #[error("numerical replay belongs to a different symbolic map owner")]
    WeightReplayMapMismatch,
    /// Declared live requested-array payload is too small for replay construction.
    #[error("weight replay setup requires {required} payload bytes, budget is {budget}")]
    WeightReplayBudgetExceeded {
        /// Checked direct-owner plus requested-new-array payload.
        required: usize,
        /// Caller-declared payload limit, not an allocator quota.
        budget: usize,
    },''')
replace_once('crates/multiway-incidence/src/lib.rs', 'mod weight_frame;', 'mod weight_frame;\nmod weight_replay;')
replace_once('crates/multiway-incidence/src/lib.rs', '#[cfg(test)]\nmod tests;', '''pub use weight_replay::{
    CoarseWeightReplay, PairConductanceReplay, WeightReplayPayloadBudget, WeightReplaySetupReport,
};

#[cfg(test)]
mod tests;''')
replace_once('crates/multiway-mg/tests/workspace_allocations.rs', 'mod weight_frame_allocations;', '''mod weight_frame_allocations;
#[path = "support/weight_replay_allocations.rs"]
mod weight_replay_allocations;''')
replace_once('crates/multiway-mg/tests/workspace_allocations.rs', '    weight_frame_allocations::run()?;', '    weight_frame_allocations::run()?;\n    weight_replay_allocations::run()?;')
replace_once('CHANGELOG.md', '### Added\n', '''### Added

- Compensated coarse-weight and pair-conductance replay with exact parent-frame
  and symbolic-map provenance, rebuilt finite degrees, checked live payload
  admission and allocation/failure tests; see `docs/ISSUE5_WEIGHT_REPLAY.md`.
''')
p = Path('crates/multiway-mg/tests/support/weight_replay_allocations.rs')
s = p.read_text()
assert s.count('    drop(error);') == 2
p.write_text(s.replace('let error = ', 'let _error = ').replace('    drop(error);\n', ''))
