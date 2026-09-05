//! One factor-respecting, exact-component-preserving symbolic transition.

use super::{TupleMergeGroups, admit, groups};
use crate::{
    FactorAggregation, IncidenceError, PreparedThreeWayTopology, PreparedTopologyBinding,
    ThreeWayTopology,
    construction::{array_bytes, reserve, sum_bytes},
};

/// Canonical coarse keys and merge groups bound to a source and factor-map owner.
///
/// The source and aggregation are borrowed, never cloned. The coarse topology
/// owns its keys/components and can be used as the source of another transition.
/// Identity and relabeling maps are permitted here: structural validity is not
/// a hierarchy reduction, rank, smoothing, or changed-weight quality certificate.
/// There are no numerical weights, operator images or terminal factors.
///
/// An aggregation cannot be replaced while a transition borrowing it is live:
/// ```compile_fail
/// use multiway_incidence::{FactorAggregation, PreparedCoarseTupleMap, PreparedThreeWayTopology};
/// let source = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
/// let aggregation = FactorAggregation::identity([1; 3]).unwrap();
/// let map = PreparedCoarseTupleMap::try_new(&source, &aggregation).unwrap();
/// drop(aggregation);
/// assert_eq!(map.coarse().input_count(), 1);
/// ```
#[derive(Debug)]
pub struct PreparedCoarseTupleMap<'a> {
    source: &'a PreparedThreeWayTopology,
    aggregation: &'a FactorAggregation,
    coarse: PreparedThreeWayTopology,
    groups: TupleMergeGroups,
    coarse_to_fine_components: Vec<usize>,
    fine_to_coarse_components: Vec<usize>,
}

impl<'a> PreparedCoarseTupleMap<'a> {
    /// Prepare one symbolic transition, rejecting aggregation across components.
    pub fn try_new(
        source: &'a PreparedThreeWayTopology,
        aggregation: &'a FactorAggregation,
    ) -> Result<Self, IncidenceError> {
        Self::try_new_with_budget(source, aggregation, usize::MAX)
    }

    /// Admit additional requested construction-array payload before any reservation.
    ///
    /// Borrowed source/map storage, inline descriptors, stack and allocator overhead
    /// are excluded. No partially built transition escapes on failure.
    pub fn try_new_with_budget(
        source: &'a PreparedThreeWayTopology,
        aggregation: &'a FactorAggregation,
        maximum_setup_payload_bytes: usize,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(
            source,
            aggregation,
            maximum_setup_payload_bytes,
            &mut |_| Ok(()),
        )
    }

    /// Conservative additional requested-array upper bound, not total live memory.
    ///
    /// Includes keys/groups, coarse labels/component sizes/root scratch, coarse
    /// vertex ownership scratch and both component maps. Fine-tuple count bounds
    /// coarse keys; coarse dimension bounds coarse components. Every individual
    /// array and the sum are checked. Component-crossing checks occur during setup;
    /// this size query alone does not certify a map. Excess allocator capacity,
    /// caller inputs, sorting stack and allocator metadata are excluded.
    pub fn setup_payload_bound(
        source: &PreparedThreeWayTopology,
        aggregation: &FactorAggregation,
    ) -> Result<usize, IncidenceError> {
        if source.topology().level_counts() != aggregation.fine_counts() {
            return Err(IncidenceError::TopologyLayoutMismatch);
        }
        let shape = ThreeWayTopology::new(aggregation.coarse_counts(), Vec::new())?;
        let dimension = shape.total_levels();
        let one = array_bytes::<usize>(dimension)?;
        sum_bytes(&[
            groups::setup_bound::<[u32; 3]>(source.topology().tuple_count())?,
            one, // component roots, retained as labels
            one, // temporary component root-to-label map
            array_bytes::<[usize; 3]>(dimension)?,
            one, // temporary coarse vertex-to-fine-component ownership
            one, // retained coarse-to-fine component map (maximum count)
            array_bytes::<usize>(source.component_factor_sizes().len())?,
        ])
    }

    pub(super) fn build_with<F>(
        source: &'a PreparedThreeWayTopology,
        aggregation: &'a FactorAggregation,
        budget: usize,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let required = Self::setup_payload_bound(source, aggregation)?;
        admit("coarse tuple map", required, budget)?;
        let shape = ThreeWayTopology::new(aggregation.coarse_counts(), Vec::new())?;
        let mut owners = reserve(shape.total_levels(), "coarse vertex components", before)?;
        owners.resize(shape.total_levels(), usize::MAX);
        for factor in 0..3 {
            let fine_offset = source.topology().offsets()[factor];
            for (level, &parent) in aggregation.parents(factor).iter().enumerate() {
                let index = shape.global_index(factor, parent);
                let component = source.component_labels()[fine_offset + level];
                if owners[index] != usize::MAX && owners[index] != component {
                    return Err(IncidenceError::CrossComponentAggregation {
                        factor,
                        parent: parent as usize,
                    });
                }
                owners[index] = component;
            }
        }
        let (keys, groups) = groups::build(
            source.topology().tuple_count(),
            |index| {
                let tuple = source.topology().tuples()[index];
                core::array::from_fn(|factor| aggregation.parents(factor)[tuple[factor] as usize])
            },
            before,
        )?;
        // Move the unique keys into the same finishing path as raw preparation.
        let coarse =
            PreparedThreeWayTopology::finish_with(aggregation.coarse_counts(), keys, None, before)?;
        let fine_count = source.component_factor_sizes().len();
        let coarse_count = coarse.component_factor_sizes().len();
        let mut coarse_to_fine = reserve(coarse_count, "coarse-to-fine components", before)?;
        coarse_to_fine.resize(coarse_count, usize::MAX);
        let mut fine_to_coarse = reserve(fine_count, "fine-to-coarse components", before)?;
        fine_to_coarse.resize(fine_count, usize::MAX);
        for (&fine, &coarse_component) in owners.iter().zip(coarse.component_labels()) {
            if fine >= fine_count
                || (coarse_to_fine[coarse_component] != usize::MAX
                    && coarse_to_fine[coarse_component] != fine)
                || (fine_to_coarse[fine] != usize::MAX && fine_to_coarse[fine] != coarse_component)
            {
                return Err(IncidenceError::ComponentMapMismatch);
            }
            coarse_to_fine[coarse_component] = fine;
            fine_to_coarse[fine] = coarse_component;
        }
        if fine_count != coarse_count
            || coarse_to_fine.contains(&usize::MAX)
            || fine_to_coarse.contains(&usize::MAX)
        {
            return Err(IncidenceError::ComponentMapMismatch);
        }
        let map = Self {
            source,
            aggregation,
            coarse,
            groups,
            coarse_to_fine_components: coarse_to_fine,
            fine_to_coarse_components: fine_to_coarse,
        };
        map.retained_payload_bytes()?;
        Ok(map)
    }

    /// Exact borrowed source owner; its observation groups are not duplicated.
    #[must_use]
    pub const fn source(&self) -> &'a PreparedThreeWayTopology {
        self.source
    }

    /// Exact borrowed factor aggregation; a value-equal clone is another owner.
    #[must_use]
    pub const fn aggregation(&self) -> &'a FactorAggregation {
        self.aggregation
    }

    /// Source identity, not numerical-weight generation.
    #[must_use]
    pub fn source_binding(&self) -> PreparedTopologyBinding<'a> {
        self.source.binding()
    }

    /// Owned canonical coarse topology with implicit collapsed-source layout.
    #[must_use]
    pub const fn coarse(&self) -> &PreparedThreeWayTopology {
        &self.coarse
    }

    /// Fine-tuple-to-coarse-tuple map and deterministic merge groups.
    #[must_use]
    pub const fn merge_groups(&self) -> &TupleMergeGroups {
        &self.groups
    }

    /// Fine component ID for every coarse component, including renumbering.
    #[must_use]
    pub fn coarse_to_fine_components(&self) -> &[usize] {
        &self.coarse_to_fine_components
    }

    /// Coarse component ID for every fine component; inverse of the other map.
    #[must_use]
    pub fn fine_to_coarse_components(&self) -> &[usize] {
        &self.fine_to_coarse_components
    }

    /// Reject mismatched source or aggregation owners without mutation or allocation.
    pub fn validate_for(
        &self,
        source: &PreparedThreeWayTopology,
        aggregation: &FactorAggregation,
    ) -> Result<(), IncidenceError> {
        self.source.binding().validate_for(source)?;
        if !core::ptr::eq(self.aggregation, aggregation) {
            return Err(IncidenceError::AggregationBindingMismatch);
        }
        Ok(())
    }

    /// Copy coarse tuple values to canonical fine tuples, not raw observations.
    ///
    /// Exact owners and both dimensions are checked before output mutation.
    /// This is a symbolic copy of arbitrary bit patterns, not a numerical reduction.
    pub fn scatter_coarse_values_into<T: Copy>(
        &self,
        source: &PreparedThreeWayTopology,
        aggregation: &FactorAggregation,
        values: &[T],
        output: &mut [T],
    ) -> Result<(), IncidenceError> {
        self.validate_for(source, aggregation)?;
        self.groups.scatter(values, output)
    }

    /// Exclusive retained array payload, including all actual unused capacity.
    ///
    /// Includes coarse topology, merge groups and both component maps. Borrowed
    /// fine topology and aggregation, inline roots, construction scratch and
    /// allocator overhead are excluded and must be charged separately by callers.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            self.coarse.retained_payload_bytes()?,
            self.groups.retained_payload_bytes()?,
            array_bytes::<usize>(self.coarse_to_fine_components.capacity())?,
            array_bytes::<usize>(self.fine_to_coarse_components.capacity())?,
        ])
    }
}
