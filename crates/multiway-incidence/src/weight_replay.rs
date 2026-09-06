//! Deterministic numerical replay through immutable symbolic maps.

mod coarse;
mod pair;

pub use coarse::CoarseWeightReplay;
pub use pair::PairConductanceReplay;

use crate::{
    IncidenceError, PreparedThreeWayTopology, ThreeWayWeightFrame, TupleMergeGroups,
    construction::sum_bytes,
    problem::CompensatedSum,
    weight_frame::reserve_frame,
};

/// Declared live requested-array payload budget for one replay construction.
///
/// This is not an allocator quota or process-memory cap. See
/// [`WeightReplaySetupReport`] for the charged and excluded categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightReplayPayloadBudget {
    /// Maximum checked sum of the setup-report categories.
    pub maximum_payload_bytes: usize,
    /// Other live payload, including older replay outputs and unlisted ancestors.
    /// Do not include direct owners already charged by the setup report.
    pub additional_live_payload_bytes: usize,
}

impl WeightReplayPayloadBudget {
    /// No explicit payload limit; callers still own full lifetime admission.
    pub const UNLIMITED: Self = Self {
        maximum_payload_bytes: usize::MAX,
        additional_live_payload_bytes: 0,
    };
}

/// Checked live requested-array inventory for a single replay construction.
///
/// Charges direct source topology, map, aggregation (coarse only), parent frame,
/// requested new arrays/scratch and caller-declared other live state. A coarse
/// map's report already includes its owned coarse topology. Parent numerical
/// arrays are not counted again as a submitted weight slice. Ancestors of a
/// chained source, older outputs and other caller storage are NOT discovered;
/// add them explicitly. Aliases included there may be conservatively overcounted.
/// Requested new arrays can have disjoint lifetimes. Inline descriptors, stack,
/// allocator headers/rounding and allocator-provided excess capacity are excluded.
/// This is neither an exact allocator peak nor a numerical-quality certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightReplaySetupReport {
    /// Actual retained payload of the borrowed fine topology.
    pub source_topology_payload_bytes: usize,
    /// Actual exclusive map payload, including owned coarse topology when present.
    pub symbolic_map_payload_bytes: usize,
    /// Actual parent-array payload of a borrowed factor aggregation, or zero.
    pub aggregation_payload_bytes: usize,
    /// Actual exclusive numerical payload of the direct source frame.
    pub source_frame_payload_bytes: usize,
    /// Requested output arrays plus transient numerical scratch.
    pub new_arrays_payload_bytes: usize,
    /// Other live payload explicitly declared by the caller.
    pub additional_live_payload_bytes: usize,
    /// Checked sum of all preceding categories.
    pub total_payload_bytes: usize,
}

impl WeightReplaySetupReport {
    fn build(
        source: &PreparedThreeWayTopology,
        parent: &ThreeWayWeightFrame<'_>,
        map_bytes: usize,
        aggregation_bytes: usize,
        new_arrays_payload_bytes: usize,
        additional_live_payload_bytes: usize,
    ) -> Result<Self, IncidenceError> {
        parent.validate_for(source)?;
        let source_topology_payload_bytes = source.retained_payload_bytes()?;
        let source_frame_payload_bytes = parent.retained_payload_bytes()?;
        let total_payload_bytes = sum_bytes(&[
            source_topology_payload_bytes,
            map_bytes,
            aggregation_bytes,
            source_frame_payload_bytes,
            new_arrays_payload_bytes,
            additional_live_payload_bytes,
        ])?;
        Ok(Self {
            source_topology_payload_bytes,
            symbolic_map_payload_bytes: map_bytes,
            aggregation_payload_bytes: aggregation_bytes,
            source_frame_payload_bytes,
            new_arrays_payload_bytes,
            additional_live_payload_bytes,
            total_payload_bytes,
        })
    }

    fn admit(self, budget: WeightReplayPayloadBudget) -> Result<(), IncidenceError> {
        if self.total_payload_bytes > budget.maximum_payload_bytes {
            return Err(IncidenceError::WeightReplayBudgetExceeded {
                required: self.total_payload_bytes,
                budget: budget.maximum_payload_bytes,
            });
        }
        Ok(())
    }
}

// Both replay kinds use the original compensated accumulator and the same group
// traversal. Internal callers have checked exact source identity, hence lengths.
fn reduce_groups<F>(
    groups: &TupleMergeGroups,
    weights: &[f64],
    context: &'static str,
    before: &mut F,
) -> Result<Vec<f64>, IncidenceError>
where
    F: FnMut(&'static str) -> Result<(), IncidenceError>,
{
    debug_assert_eq!(groups.source_to_group().len(), weights.len());
    let mut values = reserve_frame(groups.offsets().len() - 1, context, before)?;
    for (index, bounds) in groups.offsets().windows(2).enumerate() {
        let mut sum = CompensatedSum::default();
        for &source in &groups.grouped_sources()[bounds[0]..bounds[1]] {
            sum.add(weights[source]);
        }
        let value = sum.total();
        if !value.is_finite() || value <= 0.0 {
            return Err(IncidenceError::InvalidWeightFrameDerivedValue {
                context,
                index,
                value,
            });
        }
        values.push(value);
    }
    Ok(values)
}

fn validate_output(expected: usize, actual: usize) -> Result<(), IncidenceError> {
    if expected != actual {
        return Err(crate::error::dimension("numerical replay output", expected, actual));
    }
    Ok(())
}

#[cfg(test)]
mod failure_tests;
