//! Current pair conductances and degrees without graph-solver state.

use super::{WeightReplayPayloadBudget, WeightReplaySetupReport, reduce_groups, validate_output};
use crate::{
    IncidenceError, PreparedPairEdgeMap, ThreeWayWeightFrame, WeightFrameBinding,
    construction::{array_bytes, sum_bytes},
    problem::CompensatedSum,
    weight_frame::reserve_frame,
};

/// Immutable compensated pair conductances bound to an exact map and parent frame.
///
/// Conductances follow the symbolic edge order. Weighted degrees are rebuilt
/// from those represented conductances in canonical edge order; they are NOT
/// copied from fine-frame degrees, whose rounding can differ after grouping.
/// All published conductances and degrees are finite and strictly positive.
/// No pair components, gauge, inverse degrees, Laplacian factorization, rank
/// certificate or hierarchy-quality decision is provided.
///
/// A replay cannot outlive its symbolic map, even while its source remains live:
/// ```compile_fail
/// use multiway_incidence::{FactorPair, PairConductanceReplay, PreparedPairEdgeMap,
///     PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
/// let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
/// let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
/// let replay = {
///     let map = PreparedPairEdgeMap::try_new(&t, FactorPair::OneTwo).unwrap();
///     PairConductanceReplay::try_new(&map, &f).unwrap()
/// };
/// assert_eq!(replay.conductances(), &[1.0]);
/// ```
#[derive(Debug)]
pub struct PairConductanceReplay<'map, 'frame, 'topology> {
    map: &'map PreparedPairEdgeMap<'topology>,
    parent: &'frame ThreeWayWeightFrame<'topology>,
    conductances: Vec<f64>,
    diagonal: Vec<f64>,
}

impl<'map, 'frame, 'topology> PairConductanceReplay<'map, 'frame, 'topology> {
    /// Recompute conductances and degrees without sorting or rebuilding edge keys.
    pub fn try_new(
        map: &'map PreparedPairEdgeMap<'topology>,
        parent: &'frame ThreeWayWeightFrame<'topology>,
    ) -> Result<Self, IncidenceError> {
        Self::try_new_with_budget(map, parent, WeightReplayPayloadBudget::UNLIMITED)
    }

    /// Admit live requested-array payload before any new reservation.
    pub fn try_new_with_budget(
        map: &'map PreparedPairEdgeMap<'topology>,
        parent: &'frame ThreeWayWeightFrame<'topology>,
        budget: WeightReplayPayloadBudget,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(map, parent, budget, &mut |_| Ok(()))
    }

    /// Direct-owner inventory plus conductances, degrees and compensated scratch.
    ///
    /// No endpoint array is copied. Caller-declared storage must include older
    /// replay outputs, any unlisted ancestors and unrelated live caller arrays.
    pub fn setup_payload_report(
        map: &PreparedPairEdgeMap<'_>,
        parent: &ThreeWayWeightFrame<'_>,
        additional_live_payload_bytes: usize,
    ) -> Result<WeightReplaySetupReport, IncidenceError> {
        parent.validate_for(map.source())?;
        let dimension = map.local_dimension();
        let new_arrays = sum_bytes(&[
            array_bytes::<f64>(map.edges().len())?,
            array_bytes::<CompensatedSum>(dimension)?,
            array_bytes::<f64>(dimension)?,
        ])?;
        WeightReplaySetupReport::build(
            map.source(),
            parent,
            map.retained_payload_bytes()?,
            0,
            new_arrays,
            additional_live_payload_bytes,
        )
    }

    pub(super) fn build_with<F>(
        map: &'map PreparedPairEdgeMap<'topology>,
        parent: &'frame ThreeWayWeightFrame<'topology>,
        budget: WeightReplayPayloadBudget,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        Self::setup_payload_report(map, parent, budget.additional_live_payload_bytes)?.admit(budget)?;
        let conductances = reduce_groups(map.merge_groups(), parent.weights(), "pair conductance", before)?;
        let dimension = map.local_dimension();
        let mut sums = reserve_frame(dimension, "pair degree accumulators", before)?;
        sums.resize(dimension, CompensatedSum::default());
        let left_count = map.level_counts()[0];
        for (&[left, right], &value) in map.edges().iter().zip(&conductances) {
            sums[left as usize].add(value);
            sums[left_count + right as usize].add(value);
        }
        for (index, sum) in sums.iter().enumerate() {
            let value = sum.total();
            if !value.is_finite() || value <= 0.0 {
                return Err(IncidenceError::InvalidWeightFrameDerivedValue {
                    context: "pair weighted degree",
                    index,
                    value,
                });
            }
        }
        let mut diagonal = reserve_frame(dimension, "pair weighted degrees", before)?;
        diagonal.extend(sums.iter().map(|sum| sum.total()));
        drop(sums);
        let replay = Self { map, parent, conductances, diagonal };
        replay.retained_payload_bytes()?;
        Ok(replay)
    }

    /// Exact symbolic edge-map owner, which also names the factor pair.
    #[must_use]
    pub const fn map(&self) -> &'map PreparedPairEdgeMap<'topology> {
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

    /// Positive finite conductances in the borrowed map's canonical edge order.
    #[must_use]
    pub fn conductances(&self) -> &[f64] {
        &self.conductances
    }

    /// Positive finite degrees in left-then-right bipartite local order.
    #[must_use]
    pub fn diagonal(&self) -> &[f64] {
        &self.diagonal
    }

    /// Reject another exact map or numerical parent, including equal-value builds.
    pub fn validate_for(
        &self,
        map: &PreparedPairEdgeMap<'_>,
        parent: &ThreeWayWeightFrame<'_>,
    ) -> Result<(), IncidenceError> {
        if !core::ptr::eq(self.map, map) {
            return Err(IncidenceError::WeightReplayMapMismatch);
        }
        self.parent.binding().validate_for(parent)
    }

    /// Copy conductances after exact owner and output-dimension checks.
    pub fn copy_conductances_into(
        &self,
        map: &PreparedPairEdgeMap<'_>,
        parent: &ThreeWayWeightFrame<'_>,
        output: &mut [f64],
    ) -> Result<(), IncidenceError> {
        self.validate_for(map, parent)?;
        validate_output(self.conductances.len(), output.len())?;
        output.copy_from_slice(&self.conductances);
        Ok(())
    }

    /// Write `(left, left_count + right, conductance)` after all checks.
    ///
    /// The caller owns the output allocation. This does not build a graph,
    /// discover its components or validate a downstream numerical factorization.
    pub fn write_edges_into(
        &self,
        map: &PreparedPairEdgeMap<'_>,
        parent: &ThreeWayWeightFrame<'_>,
        output: &mut [(usize, usize, f64)],
    ) -> Result<(), IncidenceError> {
        self.validate_for(map, parent)?;
        validate_output(self.conductances.len(), output.len())?;
        let left_count = self.map.level_counts()[0];
        for ((out, &[left, right]), &value) in output.iter_mut().zip(self.map.edges()).zip(&self.conductances) {
            *out = (left as usize, left_count + right as usize, value);
        }
        Ok(())
    }

    /// Exclusive retained array capacities, excluding all borrowed owners/scratch.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            array_bytes::<f64>(self.conductances.capacity())?,
            array_bytes::<f64>(self.diagonal.capacity())?,
        ])
    }
}
