from pathlib import Path

ROOT = Path('.')
def put(path, content):
    p = ROOT / path
    assert not p.exists(), path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(content, encoding='utf-8')
def replace(path, old, new):
    p = ROOT / path
    text = p.read_text()
    assert text.count(old) == 1, (path, old[:100], text.count(old))
    p.write_text(text.replace(old, new), encoding='utf-8')

# Move the existing pair enum unchanged; preserve its old public re-export.
pair_path = ROOT / 'crates/multiway-mg/src/research_pair.rs'
text = pair_path.read_text()
start = text.index('/// One of the three bipartite factor pairs')
end = text.index('/// Build-phase timing', start)
pair_definition = text[start:end]
put('crates/multiway-incidence/src/factor_pair.rs', '//! Weight-independent factor-pair identifiers.\n\n' + pair_definition)
pair_path.write_text(text[:start] + 'pub use multiway_incidence::FactorPair;\n\n' + text[end:])
replace('crates/multiway-incidence/src/lib.rs', 'mod error;\n', 'mod error;\nmod factor_pair;\nmod symbolic;\n')
replace('crates/multiway-incidence/src/lib.rs', 'pub use error::IncidenceError;\n', 'pub use error::IncidenceError;\npub use factor_pair::FactorPair;\npub use symbolic::{PreparedCoarseTupleMap, PreparedPairEdgeMap, TupleMergeGroups};\n')

# Finish an already-owned canonical tuple array through the same component path.
replace('crates/multiway-incidence/src/prepared.rs',
    '        let topology = ThreeWayTopology::new(counts, tuples)?;\n        let partition = partition::build(&topology, before)?;',
    '''        Self::finish_with(counts, tuples, groups, before)
    }

    // Internal callers supply canonical keys and a consistent optional source map.
    // The keys move into their owner rather than being copied during symbolic coarsening.
    pub(crate) fn finish_with<F>(
        counts: [usize; 3],
        tuples: Vec<[u32; 3]>,
        groups: Option<ObservationGroups>,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        if tuples.is_empty() {
            return Err(IncidenceError::EmptyProblem);
        }
        if let Some(index) = tuples.windows(2).position(|pair| pair[0] >= pair[1]) {
            return Err(IncidenceError::NonCanonicalTuples { tuple_index: index + 1 });
        }
        let topology = ThreeWayTopology::new(counts, tuples)?;
        let partition = partition::build(&topology, before)?;''')
replace('crates/multiway-incidence/src/error.rs', 'pub enum IncidenceError {\n', '''pub enum IncidenceError {
    /// A symbolic map was built from a different factor aggregation owner.
    #[error("symbolic map belongs to a different factor aggregation owner")]
    AggregationBindingMismatch,
    /// A factor aggregation would merge distinct exact incidence components.
    #[error("factor {factor} aggregate {parent} crosses incidence components")]
    CrossComponentAggregation {
        /// Zero-based factor index.
        factor: usize,
        /// Rejected factor-local coarse level.
        parent: usize,
    },
    /// A symbolic coarse/fine component correspondence is not a bijection.
    #[error("symbolic component correspondence is inconsistent")]
    ComponentMapMismatch,
    /// An operation submitted a different pair from the prepared pair map.
    #[error("factor pair differs from the prepared edge map")]
    FactorPairMismatch,
    /// Conservative additional requested-array payload exceeds the setup budget.
    #[error("{context}: symbolic setup requests at most {required} payload bytes, budget is {budget}")]
    SymbolicSetupBudgetExceeded {
        /// Symbolic construction boundary.
        context: &'static str,
        /// Conservative requested-array upper bound, not allocator or OS memory.
        required: usize,
        /// Declared additional requested-array payload budget.
        budget: usize,
    },
''')

put('crates/multiway-incidence/src/symbolic.rs', r'''//! Owner-bound symbolic maps; numerical weights and quality gates are separate.

mod coarse;
mod groups;
mod pair;

pub use coarse::PreparedCoarseTupleMap;
pub use groups::TupleMergeGroups;
pub use pair::PreparedPairEdgeMap;

use crate::IncidenceError;

fn admit(context: &'static str, required: usize, budget: usize) -> Result<(), IncidenceError> {
    if required > budget {
        return Err(IncidenceError::SymbolicSetupBudgetExceeded {
            context,
            required,
            budget,
        });
    }
    Ok(())
}

#[cfg(test)]
mod failure_tests;
''')

put('crates/multiway-incidence/src/symbolic/groups.rs', r'''//! Common canonical grouping for coarse tuples and pair endpoints.

use crate::{IncidenceError, construction::{array_bytes, reserve, sum_bytes}};

/// Deterministic partition of canonical fine-tuple IDs into mapped-key groups.
///
/// These are unique fine-tuple IDs, not original observation rows. Each group
/// contains increasing IDs, preserving the source order needed for a later
/// deterministic numerical reduction. This type neither sums nor retains weights.
/// A borrowed group view has no independent owner/generation authorization.
#[derive(Debug)]
pub struct TupleMergeGroups {
    source_to_group: Vec<usize>,
    grouped_sources: Vec<usize>,
    offsets: Vec<usize>,
}

impl TupleMergeGroups {
    /// Mapped-key ID for every canonical fine tuple.
    #[must_use]
    pub fn source_to_group(&self) -> &[usize] {
        &self.source_to_group
    }

    /// Canonical fine-tuple IDs grouped by mapped key and increasing within groups.
    #[must_use]
    pub fn grouped_sources(&self) -> &[usize] {
        &self.grouped_sources
    }

    /// Group boundaries; length is mapped-key count plus one.
    #[must_use]
    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    /// Exclusive array payload by actual capacity, excluding inline descriptors.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            array_bytes::<usize>(self.source_to_group.capacity())?,
            array_bytes::<usize>(self.grouped_sources.capacity())?,
            array_bytes::<usize>(self.offsets.capacity())?,
        ])
    }

    pub(super) fn scatter<T: Copy>(&self, values: &[T], out: &mut [T]) -> Result<(), IncidenceError> {
        let expected = self.offsets.len() - 1;
        if values.len() != expected {
            return Err(crate::error::dimension("symbolic scatter groups", expected, values.len()));
        }
        if out.len() != self.source_to_group.len() {
            return Err(crate::error::dimension(
                "symbolic scatter source", self.source_to_group.len(), out.len(),
            ));
        }
        for (output, &group) in out.iter_mut().zip(&self.source_to_group) {
            *output = values[group];
        }
        Ok(())
    }
}

pub(super) fn setup_bound<K>(count: usize) -> Result<usize, IncidenceError> {
    if count == 0 {
        return Err(IncidenceError::EmptyProblem);
    }
    let offsets = count.checked_add(1).ok_or(IncidenceError::DimensionOverflow {
        context: "symbolic group offsets",
    })?;
    sum_bytes(&[
        array_bytes::<K>(count)?,
        array_bytes::<usize>(count)?,
        array_bytes::<usize>(count)?,
        array_bytes::<usize>(offsets)?,
    ])
}

pub(super) fn build<K, Key, F>(
    count: usize,
    key_of: Key,
    before: &mut F,
) -> Result<(Vec<K>, TupleMergeGroups), IncidenceError>
where
    K: Copy + Ord,
    Key: Fn(usize) -> K,
    F: FnMut(&'static str) -> Result<(), IncidenceError>,
{
    setup_bound::<K>(count)?;
    let mut order = reserve(count, "symbolic source order", before)?;
    order.extend(0..count);
    // Original canonical source ID breaks all ties; no stable-sort scratch is needed.
    order.sort_unstable_by_key(|&index| (key_of(index), index));
    let unique = 1 + order.windows(2).filter(|pair| key_of(pair[0]) != key_of(pair[1])).count();
    let mut keys = reserve(unique, "symbolic mapped keys", before)?;
    let mut source_to_group = reserve(count, "symbolic source map", before)?;
    source_to_group.resize(count, 0);
    let mut offsets = reserve(unique + 1, "symbolic group offsets", before)?;
    for (position, &index) in order.iter().enumerate() {
        let key = key_of(index);
        if keys.last() != Some(&key) {
            keys.push(key);
            offsets.push(position);
        }
        source_to_group[index] = keys.len() - 1;
    }
    offsets.push(count);
    Ok((keys, TupleMergeGroups { source_to_group, grouped_sources: order, offsets }))
}
''')

put('crates/multiway-incidence/src/symbolic/coarse.rs', r'''//! One factor-respecting, exact-component-preserving symbolic transition.

use super::{TupleMergeGroups, admit, groups};
use crate::{
    FactorAggregation, IncidenceError, PreparedThreeWayTopology, PreparedTopologyBinding,
    ThreeWayTopology, construction::{array_bytes, reserve, sum_bytes},
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
        Self::build_with(source, aggregation, maximum_setup_payload_bytes, &mut |_| Ok(()))
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
                    return Err(IncidenceError::CrossComponentAggregation { factor, parent: parent as usize });
                }
                owners[index] = component;
            }
        }
        let (keys, groups) = groups::build(source.topology().tuple_count(), |index| {
            let tuple = source.topology().tuples()[index];
            core::array::from_fn(|factor| aggregation.parents(factor)[tuple[factor] as usize])
        }, before)?;
        // Move the unique keys into the same finishing path as raw preparation.
        let coarse = PreparedThreeWayTopology::finish_with(
            aggregation.coarse_counts(), keys, None, before,
        )?;
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
                || (fine_to_coarse[fine] != usize::MAX
                    && fine_to_coarse[fine] != coarse_component)
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
            source, aggregation, coarse, groups,
            coarse_to_fine_components: coarse_to_fine,
            fine_to_coarse_components: fine_to_coarse,
        };
        map.retained_payload_bytes()?;
        Ok(map)
    }

    /// Exact borrowed source owner; its observation groups are not duplicated.
    #[must_use]
    pub const fn source(&self) -> &'a PreparedThreeWayTopology { self.source }

    /// Exact borrowed factor aggregation; a value-equal clone is another owner.
    #[must_use]
    pub const fn aggregation(&self) -> &'a FactorAggregation { self.aggregation }

    /// Source identity, not numerical-weight generation.
    #[must_use]
    pub fn source_binding(&self) -> PreparedTopologyBinding<'a> { self.source.binding() }

    /// Owned canonical coarse topology with implicit collapsed-source layout.
    #[must_use]
    pub const fn coarse(&self) -> &PreparedThreeWayTopology { &self.coarse }

    /// Fine-tuple-to-coarse-tuple map and deterministic merge groups.
    #[must_use]
    pub const fn merge_groups(&self) -> &TupleMergeGroups { &self.groups }

    /// Fine component ID for every coarse component, including renumbering.
    #[must_use]
    pub fn coarse_to_fine_components(&self) -> &[usize] { &self.coarse_to_fine_components }

    /// Coarse component ID for every fine component; inverse of the other map.
    #[must_use]
    pub fn fine_to_coarse_components(&self) -> &[usize] { &self.fine_to_coarse_components }

    /// Reject mismatched source or aggregation owners without mutation or allocation.
    pub fn validate_for(
        &self, source: &PreparedThreeWayTopology, aggregation: &FactorAggregation,
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
        &self, source: &PreparedThreeWayTopology, aggregation: &FactorAggregation,
        values: &[T], output: &mut [T],
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
''')

put('crates/multiway-incidence/src/symbolic/pair.rs', r'''//! Canonical factor-local pair endpoints and grouped fine tuple IDs.

use super::{TupleMergeGroups, admit, groups};
use crate::{FactorPair, IncidenceError, PreparedThreeWayTopology, PreparedTopologyBinding,
    construction::{array_bytes, sum_bytes}};

/// Weights-free pair-edge structure borrowing one exact prepared source.
///
/// Keys are `(left factor-local level, right factor-local level)` in lexicographic
/// order. The bipartite local graph numbers left nodes first and right nodes after
/// `left_count`. Duplicate edges group canonical fine tuples, not observation rows.
/// No pair connected components, conductances, gauge, rank or solver state are
/// supplied. A pair graph can have more components than the three-way incidence.
///
/// A map cannot outlive its source:
/// ```compile_fail
/// use multiway_incidence::{FactorPair, PreparedPairEdgeMap, PreparedThreeWayTopology};
/// let map = {
///     let source = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
///     PreparedPairEdgeMap::try_new(&source, FactorPair::OneTwo).unwrap()
/// };
/// assert_eq!(map.edges().len(), 1);
/// ```
#[derive(Debug)]
pub struct PreparedPairEdgeMap<'a> {
    source: &'a PreparedThreeWayTopology,
    pair: FactorPair,
    edges: Vec<[u32; 2]>,
    groups: TupleMergeGroups,
}

impl<'a> PreparedPairEdgeMap<'a> {
    /// Prepare deterministic pair endpoint keys and fine-tuple groups.
    pub fn try_new(source: &'a PreparedThreeWayTopology, pair: FactorPair) -> Result<Self, IncidenceError> {
        Self::try_new_with_budget(source, pair, usize::MAX)
    }

    /// Check the conservative additional requested-array bound before allocation.
    ///
    /// The bound excludes borrowed source storage, inline roots, sorting stack,
    /// allocator overhead and excess allocator-provided capacity; it is not RSS.
    pub fn try_new_with_budget(
        source: &'a PreparedThreeWayTopology, pair: FactorPair, maximum_setup_payload_bytes: usize,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(source, pair, maximum_setup_payload_bytes, &mut |_| Ok(()))
    }

    /// Conservative additional requested-array payload, bounding edges by fine tuples.
    ///
    /// All three factor pairs have the same conservative size bound. No source
    /// storage is copied or charged by this query; add it to a caller's live ledger.
    pub fn setup_payload_bound(source: &PreparedThreeWayTopology) -> Result<usize, IncidenceError> {
        groups::setup_bound::<[u32; 2]>(source.topology().tuple_count())
    }

    pub(super) fn build_with<F>(
        source: &'a PreparedThreeWayTopology, pair: FactorPair, budget: usize, before: &mut F,
    ) -> Result<Self, IncidenceError>
    where F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        admit("pair edge map", Self::setup_payload_bound(source)?, budget)?;
        let (first, second) = pair.factors();
        let (edges, groups) = groups::build(source.topology().tuple_count(), |index| {
            let tuple = source.topology().tuples()[index];
            [tuple[first], tuple[second]]
        }, before)?;
        let map = Self { source, pair, edges, groups };
        map.retained_payload_bytes()?;
        Ok(map)
    }

    /// Exact borrowed source, not a value-equal replacement.
    #[must_use]
    pub const fn source(&self) -> &'a PreparedThreeWayTopology { self.source }

    /// Source identity, not numerical-weight generation.
    #[must_use]
    pub fn source_binding(&self) -> PreparedTopologyBinding<'a> { self.source.binding() }

    /// The explicit ordered factor pair represented by this map.
    #[must_use]
    pub const fn pair(&self) -> FactorPair { self.pair }

    /// Canonical pair edges in factor-local coordinates, without conductances.
    #[must_use]
    pub fn edges(&self) -> &[[u32; 2]] { &self.edges }

    /// Fine-tuple-to-edge IDs and deterministic canonical-source merge groups.
    #[must_use]
    pub const fn merge_groups(&self) -> &TupleMergeGroups { &self.groups }

    /// Left and right factor cardinalities, in the declared pair order.
    #[must_use]
    pub fn level_counts(&self) -> [usize; 2] {
        let (left, right) = self.pair.factors();
        let counts = self.source.topology().level_counts();
        [counts[left], counts[right]]
    }

    /// Bipartite local dimension; bounded by the source's checked total dimension.
    #[must_use]
    pub fn local_dimension(&self) -> usize {
        let [left, right] = self.level_counts();
        left + right
    }

    /// Reject a different source owner or pair before output mutation.
    pub fn validate_for(&self, source: &PreparedThreeWayTopology, pair: FactorPair) -> Result<(), IncidenceError> {
        self.source.binding().validate_for(source)?;
        if pair != self.pair { return Err(IncidenceError::FactorPairMismatch); }
        Ok(())
    }

    /// Write graph-local endpoints `(left, left_count + right)` without allocation.
    ///
    /// This is not an edge-weight, connectivity or Laplacian constructor.
    pub fn write_local_endpoints_into(
        &self, source: &PreparedThreeWayTopology, pair: FactorPair, output: &mut [[usize; 2]],
    ) -> Result<(), IncidenceError> {
        self.validate_for(source, pair)?;
        if output.len() != self.edges.len() {
            return Err(crate::error::dimension("pair local endpoints", self.edges.len(), output.len()));
        }
        let left_count = self.level_counts()[0];
        for (out, &[left, right]) in output.iter_mut().zip(&self.edges) {
            *out = [left as usize, left_count + right as usize];
        }
        Ok(())
    }

    /// Copy edge values to canonical fine tuples; exact owner/pair and lengths first.
    ///
    /// Arbitrary bit patterns are copied, not validated or numerically accumulated.
    pub fn scatter_edge_values_into<T: Copy>(
        &self, source: &PreparedThreeWayTopology, pair: FactorPair, values: &[T], output: &mut [T],
    ) -> Result<(), IncidenceError> {
        self.validate_for(source, pair)?;
        self.groups.scatter(values, output)
    }

    /// Exclusive retained keys/groups by actual capacity, excluding borrowed source.
    ///
    /// Inline descriptors, setup stack and allocator metadata are not payload.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[array_bytes::<[u32; 2]>(self.edges.capacity())?, self.groups.retained_payload_bytes()?])
    }
}
''')

put('crates/multiway-incidence/src/symbolic/failure_tests.rs', r'''//! Error/unwind injection before every new reservation, not allocator-null tests.
use super::*;
use crate::{FactorAggregation, FactorPair, IncidenceError, PreparedThreeWayTopology};

#[test]
fn all_reservation_boundaries_release_partial_owners_and_allow_retry() {
    let rows: Vec<_> = (0..2).flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i,j,k]))).collect();
    let source = PreparedThreeWayTopology::try_from_observations([2;3], &rows).unwrap();
    let aggregation = FactorAggregation::consecutive_halving([2;3]).unwrap();
    let snapshot = format!("{source:?}{aggregation:?}");
    for unwind in [false, true] {
        for fail_at in 0..10 {
            let mut reached = 0;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                PreparedCoarseTupleMap::build_with(&source, &aggregation, usize::MAX, &mut |context| {
                    let index = reached; reached += 1;
                    if index == fail_at {
                        assert!(!unwind, "injected coarse setup unwind");
                        return Err(IncidenceError::TopologyAllocation { context });
                    }
                    Ok(())
                })
            }));
            assert_eq!(reached, fail_at + 1);
            if unwind { assert!(result.is_err()); }
            else { assert!(matches!(result.unwrap(), Err(IncidenceError::TopologyAllocation { .. }))); }
            assert_eq!(format!("{source:?}{aggregation:?}"), snapshot);
            PreparedCoarseTupleMap::try_new(&source, &aggregation).unwrap();
        }
        for pair in FactorPair::ALL {
            for fail_at in 0..4 {
                let mut reached = 0;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    PreparedPairEdgeMap::build_with(&source, pair, usize::MAX, &mut |context| {
                        let index = reached; reached += 1;
                        if index == fail_at {
                            assert!(!unwind, "injected pair setup unwind");
                            return Err(IncidenceError::TopologyAllocation { context });
                        }
                        Ok(())
                    })
                }));
                assert_eq!(reached, fail_at + 1);
                if unwind { assert!(result.is_err()); }
                else { assert!(matches!(result.unwrap(), Err(IncidenceError::TopologyAllocation { .. }))); }
                assert_eq!(format!("{source:?}{aggregation:?}"), snapshot);
                PreparedPairEdgeMap::try_new(&source, pair).unwrap();
            }
        }
    }
}

#[test]
fn size_and_budget_errors_precede_reservations() {
    assert!(groups::setup_bound::<[u32;3]>(usize::MAX).is_err());
    assert!(groups::setup_bound::<[u32;2]>(0).is_err());
    assert!(groups::setup_bound::<[u32;2]>(isize::MAX as usize).is_err());
    let source = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
    let aggregation = FactorAggregation::identity([1;3]).unwrap();
    let bound = PreparedCoarseTupleMap::setup_payload_bound(&source, &aggregation).unwrap();
    assert!(matches!(PreparedCoarseTupleMap::build_with(&source, &aggregation, bound-1,
        &mut |_| panic!("budget must reject first")), Err(IncidenceError::SymbolicSetupBudgetExceeded { .. })));
    let wrong = FactorAggregation::identity([2;3]).unwrap();
    assert!(PreparedCoarseTupleMap::build_with(&source, &wrong, usize::MAX,
        &mut |_| panic!("shape must reject first")).is_err());
    let bound = PreparedPairEdgeMap::setup_payload_bound(&source).unwrap();
    assert!(PreparedPairEdgeMap::build_with(&source, FactorPair::OneTwo, bound-1,
        &mut |_| panic!("budget must reject first")).is_err());
}
''')

replace('CHANGELOG.md', '### Added\n', '''### Added

- Owner-bound symbolic coarse-tuple and pair-edge maps with deterministic merge
  groups, component correspondence and checked fallible setup; see
  `docs/ISSUE5_SYMBOLIC_MAPS.md`.
''')
put('docs/ISSUE5_SYMBOLIC_MAPS.md', '''# Issue 5: symbolic coarse-tuple and pair-edge maps

## Boundary

`PreparedCoarseTupleMap` represents one transition from a borrowed prepared
source through a borrowed `FactorAggregation`. `PreparedPairEdgeMap` represents
one explicit factor pair of a borrowed source. Both retain canonical mapped keys,
fine-tuple-to-group IDs, grouped fine tuple IDs and offsets, but no numerical
weights, degrees, conductances, smoother coefficients or terminal factors.

Grouping is over unique canonical fine tuples, not original observations. A
single grouping builder orders by (mapped key, canonical fine tuple ID), so every
merge group preserves increasing fine tuple order for a future deterministic
compensated reduction. No reduction or weight-frame API is provided here.

The unchanged `FactorPair` definition now lives in `multiway-incidence`, keeping
this structural layer independent of CMG/within. The prior CMG-feature research
re-export in `multiway-mg` remains available; no numerical pair route changes.

## Owners and component preservation

Coarse maps borrow both source and exact factor aggregation. `validate_for` and
symbolic scattering reject a source from another owner or a value-equal cloned
aggregation. Borrowing prevents replacement or movement while a derived map is
used; no global counter, hash identity, unsafe state or owning clone is added.
Pair maps check the exact source plus the explicit factor pair. Raw getter slices
are inspection views, not independent generation-bearing certificates.

Factor-local aggregation alone is insufficient: two fine levels in distinct
incidence components must not have the same parent. Construction checks this in
linear vertex work using coarse-vertex ownership scratch and rejects crossings.
The coarse topology is finished through the same existing component routine,
with keys moved rather than copied. Both directions of component correspondence
are retained and checked as a bijection, including possible component renumbering.

Identity maps and pure relabelings are structurally legal. A successful symbolic
constructor is NOT hierarchy admission, proof of dimension reduction, numerical
rank, smoother quality or a license to reuse maps after arbitrary weight changes.
The owned coarse topology can be borrowed for another transition; this is not yet
an owning multi-level prepared-hierarchy container.

Pair endpoints are lexicographically sorted factor-local `[left, right]` keys.
Graph-local endpoints are `[left, left_count + right]`, with dimension
`left_count + right_count`. The explicit endpoint writer and both symbolic
scatters validate owners/pair/dimensions before any output mutation. Values are
copied bit-for-bit, not validated as weights. A pair graph can have more connected
components than its parent three-way incidence. This API does not supply pair
components, gauges or rank and must not substitute three-way labels for them.

## Allocation and memory

Every new array uses checked fallible reservation. The budgeted constructors
check conservative ADDITIONAL requested-array setup payload before allocating.
Fine tuple count bounds unique mapped keys; coarse dimension bounds component
arrays. Coarse bounds include merge arrays, component construction scratch,
coarse-vertex ownership scratch and both correspondence arrays. Pair bounds
include endpoint keys and grouping arrays. Equality is admitted; one byte short
rejects. The bound may exceed actual requirements and excludes caller/borrowed
source and aggregation payload, inline descriptors, sorting stack, allocator
metadata and excess allocator-provided capacity. It is not OS/RSS admission.

Retained reports count actual exclusive capacities. Coarse maps include owned
coarse topology, groups and both component maps; pair maps include keys/groups.
Borrowed source/maps add no exclusive heap allocation but must still be charged
once in a caller's full live-memory ledger. No hierarchy payload-budget API is
silently extended to count these external objects.

All partial construction state is ordinarily owned and dropped on error/unwind.
Private local callbacks test reservation boundaries without retained hooks or
global state. This is boundary injection, not an actual OS allocator returning
null. Existing infallible FactorAggregation constructors remain outside this new
fallible construction boundary.

## Qualification

Require exact-head GitHub Actions on Rust 1.85, full existing scientific gates,
and the Linux/macOS/Windows x debug/release x minimal/all-feature allocation
matrix. Add independent key/group references, fresh Galerkin comparisons,
source/map/pair mismatch rejection, component renumbering/crossing, chaining,
borrowed lifetime tests, all ten coarse and four per-pair reservation failures
and unwinds, plus construction/destruction accounting and allocation-free reads,
scattering, endpoint writes and static rejection.

## Next

Explicit numerical-weight frames need their own validated generation identity.
Then implement compensated fine/coarse/pair weight replay and generation-safe
numerical hierarchies, rebuilding every weight-dependent quantity and re-screening
map quality. Existing solver numerics, original-operator certification, frozen
scientific evidence and ADR 0002 remain unchanged. No speedup, production-routing
change, new holdout or closure of issue #5 follows from this symbolic increment.
''')
