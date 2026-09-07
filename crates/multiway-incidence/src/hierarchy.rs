//! Owning, index-linked supplied-map structure and immutable numerical replay.

use crate::{
    FactorAggregation, IncidenceError, PreparedCoarseTupleMap, PreparedThreeWayTopology,
    ThreeWayWeightFrame, TupleMergeGroups, WeightFrameInput, WeightFrameInputKind,
    construction::{array_bytes, reserve, sum_bytes},
    weight_replay::reduce_groups,
};

mod builder;
pub use builder::{
    PreparedHierarchyAppendFailure, PreparedHierarchyAppendReport, PreparedHierarchyBuilder,
    PreparedHierarchyLimits, PreparedProvisionalFailure, PreparedProvisionalFrame,
    PreparedProvisionalSetup, ProvisionalFrameStage, ProvisionalWeightInput,
};

/// Live array-payload admission for a complete structural or replay build.
///
/// Charges the fine topology once, all owned hierarchy arrays, and (for replay)
/// the fine frame and requested new numerical arrays. Add other live objects,
/// including old numerical hierarchies, through `additional_live_payload_bytes`.
/// Excludes inline roots, stack, allocator metadata and new excess capacity;
/// this is not a process-RSS cap. Owned vector descriptor arrays are included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedHierarchyBudget {
    /// Maximum charged live payload before each new construction reservation.
    pub maximum_payload_bytes: usize,
    /// Other live array payload not already named by this construction.
    pub additional_live_payload_bytes: usize,
}
impl PreparedHierarchyBudget {
    /// No explicit payload limit; integer overflow is still rejected.
    pub const UNLIMITED: Self = Self {
        maximum_payload_bytes: usize::MAX,
        additional_live_payload_bytes: 0,
    };
}

#[derive(Debug)]
pub(crate) struct Transition {
    pub(crate) coarse: PreparedThreeWayTopology,
    pub(crate) groups: TupleMergeGroups,
    pub(crate) coarse_to_fine: Vec<usize>,
    pub(crate) fine_to_coarse: Vec<usize>,
}
impl Transition {
    fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            self.coarse.retained_payload_bytes()?,
            self.groups.retained_payload_bytes()?,
            array_bytes::<usize>(self.coarse_to_fine.capacity())?,
            array_bytes::<usize>(self.fine_to_coarse.capacity())?,
        ])
    }
}

/// Immutable supplied-map hierarchy with one borrowed fine structural owner.
///
/// Owns factor maps and all coarse keys/components/groups. Transitions link by
/// index, with no self-references, copied fine topology, reference-counted token,
/// numerical weights or terminal factors. Identity/relabeling transitions are
/// valid here; reduction and solver-quality admission are separate decisions.
/// No owning `Clone` or mutation API is provided.
///
/// The fine owner cannot be dropped while the hierarchy remains in use:
/// ```compile_fail
/// use multiway_incidence::{PreparedHierarchyTopology, PreparedThreeWayTopology};
/// let h = {
///     let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
///     PreparedHierarchyTopology::try_new(&t, vec![]).unwrap()
/// };
/// assert_eq!(h.level_count(), 1);
/// ```
#[derive(Debug)]
pub struct PreparedHierarchyTopology<'fine> {
    fine: &'fine PreparedThreeWayTopology,
    aggregations: Vec<FactorAggregation>,
    transitions: Vec<Transition>,
    setup_peak_payload_bound: usize,
}
impl<'fine> PreparedHierarchyTopology<'fine> {
    /// Consume supplied maps, preserving exact incidence components at every level.
    pub fn try_new(
        fine: &'fine PreparedThreeWayTopology,
        aggregations: Vec<FactorAggregation>,
    ) -> Result<Self, IncidenceError> {
        Self::try_new_with_budget(fine, aggregations, PreparedHierarchyBudget::UNLIMITED)
    }

    /// Check each next transition's conservative live payload before reserving it.
    ///
    /// All map layouts are checked first. A failure drops the unpublished owned
    /// state; the fine owner and any older hierarchies remain usable. Consumed
    /// input maps are not returned on failure. Their actual capacities are charged.
    pub fn try_new_with_budget(
        fine: &'fine PreparedThreeWayTopology,
        aggregations: Vec<FactorAggregation>,
        budget: PreparedHierarchyBudget,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(fine, aggregations, budget, &mut |_| Ok(()))
    }

    fn build_with<F>(
        fine: &'fine PreparedThreeWayTopology,
        aggregations: Vec<FactorAggregation>,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let mut counts = fine.topology().level_counts();
        for aggregation in &aggregations {
            if aggregation.fine_counts() != counts {
                return Err(IncidenceError::TopologyLayoutMismatch);
            }
            counts = aggregation.coarse_counts();
        }
        let maps_payload = aggregations.iter().try_fold(
            array_bytes::<FactorAggregation>(aggregations.capacity())?,
            |total, map| sum_bytes(&[total, map.retained_payload_bytes()?]),
        )?;
        let base = sum_bytes(&[
            fine.retained_payload_bytes()?,
            maps_payload,
            budget.additional_live_payload_bytes,
        ])?;
        let initial = sum_bytes(&[base, array_bytes::<Transition>(aggregations.len())?])?;
        admit(initial, budget)?;
        let transitions = reserve(
            aggregations.len(),
            "hierarchy transition descriptors",
            before,
        )?;
        let mut result = Self {
            fine,
            aggregations,
            transitions,
            setup_peak_payload_bound: initial,
        };
        for index in 0..result.aggregations.len() {
            let source = result
                .level(index)
                .expect("previous level is already built");
            let aggregation = &result.aggregations[index];
            let new_bound = PreparedCoarseTupleMap::setup_payload_bound(source, aggregation)?;
            let live = sum_bytes(&[
                fine.retained_payload_bytes()?,
                result.retained_payload_bytes()?,
                budget.additional_live_payload_bytes,
                new_bound,
            ])?;
            admit(live, budget)?;
            let transition =
                PreparedCoarseTupleMap::build_with(source, aggregation, new_bound, before)?
                    .into_hierarchy_parts();
            result.transitions.push(transition);
            result.setup_peak_payload_bound = result.setup_peak_payload_bound.max(live);
        }
        let retained_live = sum_bytes(&[
            fine.retained_payload_bytes()?,
            result.retained_payload_bytes()?,
            budget.additional_live_payload_bytes,
        ])?;
        admit(retained_live, budget)?;
        result.setup_peak_payload_bound = result.setup_peak_payload_bound.max(retained_live);
        Ok(result)
    }

    /// Exact borrowed fine topology, charged separately from exclusive payload.
    #[must_use]
    pub const fn fine(&self) -> &'fine PreparedThreeWayTopology {
        self.fine
    }

    /// Number of levels including the fine level and final terminal candidate.
    #[must_use]
    pub fn level_count(&self) -> usize {
        self.transitions.len() + 1
    }

    /// Level zero is the fine owner; subsequent levels are owned coarse topologies.
    #[must_use]
    pub fn level(&self, index: usize) -> Option<&PreparedThreeWayTopology> {
        if index == 0 {
            Some(self.fine)
        } else {
            self.transitions.get(index - 1).map(|t| &t.coarse)
        }
    }

    /// Factor map from level `index` to level `index + 1`.
    #[must_use]
    pub fn aggregation(&self, index: usize) -> Option<&FactorAggregation> {
        self.aggregations.get(index)
    }

    /// Canonical fine-tuple merge groups for a transition.
    #[must_use]
    pub fn merge_groups(&self, index: usize) -> Option<&TupleMergeGroups> {
        self.transitions.get(index).map(|t| &t.groups)
    }

    /// Fine component ID for each coarse component at a transition.
    #[must_use]
    pub fn coarse_to_fine_components(&self, index: usize) -> Option<&[usize]> {
        self.transitions
            .get(index)
            .map(|t| t.coarse_to_fine.as_slice())
    }

    /// Coarse component ID for each fine component at a transition.
    #[must_use]
    pub fn fine_to_coarse_components(&self, index: usize) -> Option<&[usize]> {
        self.transitions
            .get(index)
            .map(|t| t.fine_to_coarse.as_slice())
    }

    /// Reject another fine structural owner, including a value-equal reconstruction.
    pub fn validate_for(&self, fine: &PreparedThreeWayTopology) -> Result<(), IncidenceError> {
        self.fine.binding().validate_for(fine)
    }

    /// Largest charged construction bound, including the declared other live state.
    #[must_use]
    pub const fn setup_peak_payload_bound(&self) -> usize {
        self.setup_peak_payload_bound
    }

    /// Exclusive retained capacities, including heap-allocated level/map descriptors.
    ///
    /// Excludes the borrowed fine topology, inline root and allocator overhead.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        let mut total = sum_bytes(&[
            array_bytes::<FactorAggregation>(self.aggregations.capacity())?,
            array_bytes::<Transition>(self.transitions.capacity())?,
        ])?;
        for map in &self.aggregations {
            total = sum_bytes(&[total, map.retained_payload_bytes()?])?;
        }
        for transition in &self.transitions {
            total = sum_bytes(&[total, transition.retained_payload_bytes()?])?;
        }
        Ok(total)
    }
}

/// Complete immutable weight replay tied to one exact fine-frame generation.
///
/// All coarse numerical frames borrow the stable structural hierarchy. They are
/// owned here and cannot be detached. Rebuilding recomputes every weight, root,
/// degree and diagnostic through the same compensated reduction as single-map
/// replay. No prior numerical state or component discovery is reused. This is
/// structural replay, not permission to reuse old smoothers or terminal factors.
///
/// A numerical replay cannot outlive its exact fine frame:
/// ```compile_fail
/// use multiway_incidence::{PreparedHierarchyTopology, HierarchyWeightFrames,
///     PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
/// let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let h = PreparedHierarchyTopology::try_new(&t, vec![]).unwrap();
/// let replay = {
///     let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
///     HierarchyWeightFrames::try_new(&h, &f).unwrap()
/// };
/// assert_eq!(replay.level_count(), 1);
/// ```
#[derive(Debug)]
pub struct HierarchyWeightFrames<'hierarchy, 'frame, 'fine> {
    hierarchy: &'hierarchy PreparedHierarchyTopology<'fine>,
    fine: &'frame ThreeWayWeightFrame<'fine>,
    coarse: Vec<ThreeWayWeightFrame<'hierarchy>>,
}
impl<'hierarchy, 'frame, 'fine> HierarchyWeightFrames<'hierarchy, 'frame, 'fine> {
    /// Replay all supplied transitions from this exact current fine frame.
    pub fn try_new(
        hierarchy: &'hierarchy PreparedHierarchyTopology<'fine>,
        fine: &'frame ThreeWayWeightFrame<'fine>,
    ) -> Result<Self, IncidenceError> {
        Self::try_new_with_budget(hierarchy, fine, PreparedHierarchyBudget::UNLIMITED)
    }

    /// Admit complete requested numerical payload before the first reservation.
    pub fn try_new_with_budget(
        hierarchy: &'hierarchy PreparedHierarchyTopology<'fine>,
        fine: &'frame ThreeWayWeightFrame<'fine>,
        budget: PreparedHierarchyBudget,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(hierarchy, fine, budget, &mut |_| Ok(()))
    }

    /// Conservative complete live bound, including all coarse degree scratch.
    ///
    /// Each temporary degree-correction array is charged even though those arrays
    /// have disjoint lifetimes. The borrowed fine topology/frame are counted once.
    pub fn setup_payload_bound(
        hierarchy: &PreparedHierarchyTopology<'_>,
        fine: &ThreeWayWeightFrame<'_>,
        additional_live_payload_bytes: usize,
    ) -> Result<usize, IncidenceError> {
        fine.validate_for(hierarchy.fine)?;
        let mut total = sum_bytes(&[
            hierarchy.fine.retained_payload_bytes()?,
            hierarchy.retained_payload_bytes()?,
            fine.retained_payload_bytes()?,
            additional_live_payload_bytes,
            array_bytes::<ThreeWayWeightFrame<'_>>(hierarchy.transitions.len())?,
        ])?;
        for transition in &hierarchy.transitions {
            total = sum_bytes(&[
                total,
                ThreeWayWeightFrame::setup_payload_report(
                    &transition.coarse,
                    WeightFrameInput::UnitTuples,
                    0,
                )?
                .new_arrays_payload_bytes,
            ])?;
        }
        Ok(total)
    }

    fn build_with<F>(
        hierarchy: &'hierarchy PreparedHierarchyTopology<'fine>,
        fine: &'frame ThreeWayWeightFrame<'fine>,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        admit(
            Self::setup_payload_bound(hierarchy, fine, budget.additional_live_payload_bytes)?,
            budget,
        )?;
        let mut coarse: Vec<ThreeWayWeightFrame<'hierarchy>> = reserve(
            hierarchy.transitions.len(),
            "hierarchy frame descriptors",
            before,
        )?;
        for (index, transition) in hierarchy.transitions.iter().enumerate() {
            let source = if index == 0 { fine } else { &coarse[index - 1] };
            let weights = reduce_groups(
                &transition.groups,
                source.weights(),
                "hierarchy coarse total",
                before,
            )?;
            coarse.push(ThreeWayWeightFrame::finish_with(
                &transition.coarse,
                weights,
                WeightFrameInputKind::Tuples,
                transition.coarse.topology().tuple_count(),
                before,
            )?);
        }
        Ok(Self {
            hierarchy,
            fine,
            coarse,
        })
    }

    /// Exact structural owner of all levels and supplied maps.
    #[must_use]
    pub const fn hierarchy(&self) -> &'hierarchy PreparedHierarchyTopology<'fine> {
        self.hierarchy
    }

    /// Exact fine numerical generation; no numerical arrays are copied here.
    #[must_use]
    pub const fn fine(&self) -> &'frame ThreeWayWeightFrame<'fine> {
        self.fine
    }

    /// Number of current numerical levels including the borrowed fine frame.
    #[must_use]
    pub fn level_count(&self) -> usize {
        self.coarse.len() + 1
    }

    /// Immutable frame at a level; its borrow cannot outlive this replay owner.
    #[must_use]
    pub fn frame(&self, index: usize) -> Option<&ThreeWayWeightFrame<'_>> {
        if index == 0 {
            Some(self.fine)
        } else {
            self.coarse.get(index - 1)
        }
    }

    /// Reject another structural owner or fine numerical generation.
    pub fn validate_for(
        &self,
        hierarchy: &PreparedHierarchyTopology<'_>,
        fine: &ThreeWayWeightFrame<'_>,
    ) -> Result<(), IncidenceError> {
        if !core::ptr::eq(self.hierarchy, hierarchy) {
            return Err(IncidenceError::HierarchyBindingMismatch);
        }
        self.fine.binding().validate_for(fine)
    }

    /// Exclusive coarse numerical capacities and frame descriptors.
    ///
    /// Excludes borrowed fine arrays, all structural owners, and allocator overhead.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        self.coarse.iter().try_fold(
            array_bytes::<ThreeWayWeightFrame<'_>>(self.coarse.capacity())?,
            |total, frame| sum_bytes(&[total, frame.retained_payload_bytes()?]),
        )
    }
}

fn admit(required: usize, budget: PreparedHierarchyBudget) -> Result<(), IncidenceError> {
    if required > budget.maximum_payload_bytes {
        return Err(IncidenceError::HierarchyBudgetExceeded {
            required,
            budget: budget.maximum_payload_bytes,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
