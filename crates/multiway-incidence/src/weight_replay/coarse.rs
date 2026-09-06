//! One immutable coarse numerical frame with exact parent/map provenance.

use super::{WeightReplayPayloadBudget, WeightReplaySetupReport, reduce_groups, validate_output};
use crate::{
    IncidenceError, PreparedCoarseTupleMap, ThreeWayWeightFrame, WeightFrameBinding,
    WeightFrameInput, WeightFrameInputKind,
};

/// A coarse frame rebuilt from one exact source frame and symbolic transition.
///
/// Tuple totals follow increasing canonical fine-tuple order within each stored
/// group. The resulting weights move into the ordinary frame finishing path,
/// which recomputes roots, degrees and component diagnostics without sorting or
/// copying topology. No old numerical state is reused. This is not map-quality,
/// rank, terminal-factor or complete-hierarchy admission.
///
/// The parent numerical frame cannot be replaced while a replay still uses it:
/// ```compile_fail
/// use multiway_incidence::{CoarseWeightReplay, FactorAggregation, PreparedCoarseTupleMap,
///     PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
/// let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
/// let a = FactorAggregation::identity([1; 3]).unwrap();
/// let m = PreparedCoarseTupleMap::try_new(&t, &a).unwrap();
/// let mut f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
/// let replay = CoarseWeightReplay::try_new(&m, &f).unwrap();
/// f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[2.0])).unwrap();
/// replay.validate_for(&m, &f).unwrap();
/// ```
#[derive(Debug)]
pub struct CoarseWeightReplay<'map, 'frame, 'topology> {
    map: &'map PreparedCoarseTupleMap<'topology>,
    parent: &'frame ThreeWayWeightFrame<'topology>,
    frame: ThreeWayWeightFrame<'map>,
}

impl<'map, 'frame, 'topology> CoarseWeightReplay<'map, 'frame, 'topology> {
    /// Rebuild coarse numerical state from the current source frame.
    pub fn try_new(
        map: &'map PreparedCoarseTupleMap<'topology>,
        parent: &'frame ThreeWayWeightFrame<'topology>,
    ) -> Result<Self, IncidenceError> {
        Self::try_new_with_budget(map, parent, WeightReplayPayloadBudget::UNLIMITED)
    }

    /// Check live requested-array admission before any new reservation.
    pub fn try_new_with_budget(
        map: &'map PreparedCoarseTupleMap<'topology>,
        parent: &'frame ThreeWayWeightFrame<'topology>,
        budget: WeightReplayPayloadBudget,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(map, parent, budget, &mut |_| Ok(()))
    }

    /// Checked direct-owner and new-array ledger; not a quality/replay certificate.
    ///
    /// The coarse topology is charged in the map, not again with the new frame.
    /// Ancestor owners, old outputs and unrelated caller storage must be declared.
    pub fn setup_payload_report(
        map: &PreparedCoarseTupleMap<'_>,
        parent: &ThreeWayWeightFrame<'_>,
        additional_live_payload_bytes: usize,
    ) -> Result<WeightReplaySetupReport, IncidenceError> {
        parent.validate_for(map.source())?;
        let new_arrays = ThreeWayWeightFrame::setup_payload_report(
            map.coarse(),
            WeightFrameInput::UnitTuples,
            0,
        )?
        .new_arrays_payload_bytes;
        WeightReplaySetupReport::build(
            map.source(),
            parent,
            map.retained_payload_bytes()?,
            map.aggregation().retained_payload_bytes()?,
            new_arrays,
            additional_live_payload_bytes,
        )
    }

    pub(super) fn build_with<F>(
        map: &'map PreparedCoarseTupleMap<'topology>,
        parent: &'frame ThreeWayWeightFrame<'topology>,
        budget: WeightReplayPayloadBudget,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        Self::setup_payload_report(map, parent, budget.additional_live_payload_bytes)?
            .admit(budget)?;
        let weights = reduce_groups(
            map.merge_groups(),
            parent.weights(),
            "coarse tuple total",
            before,
        )?;
        let frame = ThreeWayWeightFrame::finish_with(
            map.coarse(),
            weights,
            WeightFrameInputKind::Tuples,
            map.coarse().topology().tuple_count(),
            before,
        )?;
        Ok(Self { map, parent, frame })
    }

    /// Exact borrowed symbolic transition, not a value-equal rebuilt map.
    #[must_use]
    pub const fn map(&self) -> &'map PreparedCoarseTupleMap<'topology> {
        self.map
    }

    /// Exact direct source numerical generation.
    #[must_use]
    pub const fn source_frame(&self) -> &'frame ThreeWayWeightFrame<'topology> {
        self.parent
    }

    /// Borrowed identity of the direct source numerical generation.
    #[must_use]
    pub fn source_binding(&self) -> WeightFrameBinding<'frame, 'topology> {
        self.parent.binding()
    }

    /// Owned coarse numerical frame, exposed only by immutable borrow.
    ///
    /// This can supply the parent frame of another replay. Its validation input
    /// kind is canonical `Tuples`; the enclosing record retains replay provenance.
    /// There is no consuming extractor that detaches the frame from that record.
    #[must_use]
    pub const fn frame(&self) -> &ThreeWayWeightFrame<'map> {
        &self.frame
    }

    /// Reject another map owner or parent numerical generation before any writes.
    pub fn validate_for(
        &self,
        map: &PreparedCoarseTupleMap<'_>,
        parent: &ThreeWayWeightFrame<'_>,
    ) -> Result<(), IncidenceError> {
        if !core::ptr::eq(self.map, map) {
            return Err(IncidenceError::WeightReplayMapMismatch);
        }
        self.parent.binding().validate_for(parent)
    }

    /// Copy coarse weights after exact map/frame and dimension validation.
    pub fn copy_weights_into(
        &self,
        map: &PreparedCoarseTupleMap<'_>,
        parent: &ThreeWayWeightFrame<'_>,
        output: &mut [f64],
    ) -> Result<(), IncidenceError> {
        self.validate_for(map, parent)?;
        validate_output(self.frame.weights().len(), output.len())?;
        output.copy_from_slice(self.frame.weights());
        Ok(())
    }

    /// Actual exclusive numerical capacities; all borrowed owners are excluded.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        self.frame.retained_payload_bytes()
    }
}
