//! Canonical factor-local pair endpoints and grouped fine tuple IDs.

use super::{TupleMergeGroups, admit, groups};
use crate::{
    FactorPair, IncidenceError, PreparedThreeWayTopology, PreparedTopologyBinding,
    construction::{array_bytes, sum_bytes},
};

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
    pub fn try_new(
        source: &'a PreparedThreeWayTopology,
        pair: FactorPair,
    ) -> Result<Self, IncidenceError> {
        Self::try_new_with_budget(source, pair, usize::MAX)
    }

    /// Check the conservative additional requested-array bound before allocation.
    ///
    /// The bound excludes borrowed source storage, inline roots, sorting stack,
    /// allocator overhead and excess allocator-provided capacity; it is not RSS.
    pub fn try_new_with_budget(
        source: &'a PreparedThreeWayTopology,
        pair: FactorPair,
        maximum_setup_payload_bytes: usize,
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
        source: &'a PreparedThreeWayTopology,
        pair: FactorPair,
        budget: usize,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        admit("pair edge map", Self::setup_payload_bound(source)?, budget)?;
        let (first, second) = pair.factors();
        let (edges, groups) = groups::build(
            source.topology().tuple_count(),
            |index| {
                let tuple = source.topology().tuples()[index];
                [tuple[first], tuple[second]]
            },
            before,
        )?;
        let map = Self {
            source,
            pair,
            edges,
            groups,
        };
        map.retained_payload_bytes()?;
        Ok(map)
    }

    /// Exact borrowed source, not a value-equal replacement.
    #[must_use]
    pub const fn source(&self) -> &'a PreparedThreeWayTopology {
        self.source
    }

    /// Source identity, not numerical-weight generation.
    #[must_use]
    pub fn source_binding(&self) -> PreparedTopologyBinding<'a> {
        self.source.binding()
    }

    /// The explicit ordered factor pair represented by this map.
    #[must_use]
    pub const fn pair(&self) -> FactorPair {
        self.pair
    }

    /// Canonical pair edges in factor-local coordinates, without conductances.
    #[must_use]
    pub fn edges(&self) -> &[[u32; 2]] {
        &self.edges
    }

    /// Fine-tuple-to-edge IDs and deterministic canonical-source merge groups.
    #[must_use]
    pub const fn merge_groups(&self) -> &TupleMergeGroups {
        &self.groups
    }

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
    pub fn validate_for(
        &self,
        source: &PreparedThreeWayTopology,
        pair: FactorPair,
    ) -> Result<(), IncidenceError> {
        self.source.binding().validate_for(source)?;
        if pair != self.pair {
            return Err(IncidenceError::FactorPairMismatch);
        }
        Ok(())
    }

    /// Write graph-local endpoints `(left, left_count + right)` without allocation.
    ///
    /// This is not an edge-weight, connectivity or Laplacian constructor.
    pub fn write_local_endpoints_into(
        &self,
        source: &PreparedThreeWayTopology,
        pair: FactorPair,
        output: &mut [[usize; 2]],
    ) -> Result<(), IncidenceError> {
        self.validate_for(source, pair)?;
        if output.len() != self.edges.len() {
            return Err(crate::error::dimension(
                "pair local endpoints",
                self.edges.len(),
                output.len(),
            ));
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
        &self,
        source: &PreparedThreeWayTopology,
        pair: FactorPair,
        values: &[T],
        output: &mut [T],
    ) -> Result<(), IncidenceError> {
        self.validate_for(source, pair)?;
        self.groups.scatter(values, output)
    }

    /// Exclusive retained keys/groups by actual capacity, excluding borrowed source.
    ///
    /// Inline descriptors, setup stack and allocator metadata are not payload.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            array_bytes::<[u32; 2]>(self.edges.capacity())?,
            self.groups.retained_payload_bytes()?,
        ])
    }
}
