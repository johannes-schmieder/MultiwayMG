from pathlib import Path

def write(path, text):
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text)

def replace(path, old, new):
    p = Path(path)
    text = p.read_text()
    assert text.count(old) == 1, (path, old[:100])
    p.write_text(text.replace(old, new))

write('crates/multiway-incidence/src/construction.rs', r'''//! Checked, fallible array reservation at explicit construction boundaries.

use crate::IncidenceError;

pub(crate) fn array_bytes<T>(count: usize) -> Result<usize, IncidenceError> {
    count
        .checked_mul(core::mem::size_of::<T>())
        .filter(|&bytes| bytes <= isize::MAX as usize)
        .ok_or(IncidenceError::DimensionOverflow {
            context: "topology construction array",
        })
}

pub(crate) fn sum_bytes(parts: &[usize]) -> Result<usize, IncidenceError> {
    parts.iter().try_fold(0usize, |sum, &bytes| {
        sum.checked_add(bytes).ok_or(IncidenceError::DimensionOverflow {
            context: "topology construction payload",
        })
    })
}

pub(crate) fn reserve<T, F>(
    count: usize,
    context: &'static str,
    before_reservation: &mut F,
) -> Result<Vec<T>, IncidenceError>
where
    F: FnMut(&'static str) -> Result<(), IncidenceError>,
{
    array_bytes::<T>(count)?;
    let mut values = Vec::new();
    if count != 0 {
        before_reservation(context)?;
        values.try_reserve_exact(count).map_err(|_| IncidenceError::TopologyAllocation { context })?;
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impossible_arrays_reject_before_reservation() {
        assert!(array_bytes::<f64>(usize::MAX).is_err());
        assert!(array_bytes::<u8>(isize::MAX as usize + 1).is_err());
        assert!(sum_bytes(&[usize::MAX, 1]).is_err());
        assert!(reserve::<f64, _>(usize::MAX, "test", &mut |_| panic!("not admitted")).is_err());
    }
}
''')

write('crates/multiway-incidence/src/components/partition.rs', r'''//! One component-discovery implementation, independent of projection identities.

use super::{find_root, union_min_root};
use crate::{IncidenceError, ThreeWayTopology, construction::reserve};

#[derive(Debug)]
pub(crate) struct Partition {
    pub(crate) labels: Vec<usize>,
    pub(crate) factor_sizes: Vec<[usize; 3]>,
}

pub(crate) fn build<F>(topology: &ThreeWayTopology, before: &mut F) -> Result<Partition, IncidenceError>
where
    F: FnMut(&'static str) -> Result<(), IncidenceError>,
{
    let dimension = topology.total_levels();
    let mut labels = reserve(dimension, "component roots", before)?;
    labels.extend(0..dimension);
    for tuple in topology.tuples() {
        let a = topology.global_index(0, tuple[0]);
        let b = topology.global_index(1, tuple[1]);
        let c = topology.global_index(2, tuple[2]);
        union_min_root(&mut labels, a, b);
        union_min_root(&mut labels, a, c);
    }
    for vertex in 0..dimension {
        labels[vertex] = find_root(&mut labels, vertex);
    }
    let count = labels.iter().enumerate().filter(|&(vertex, root)| vertex == *root).count();
    let mut root_to_label = reserve(dimension, "component root labels", before)?;
    root_to_label.resize(dimension, usize::MAX);
    let mut factor_sizes = reserve(count, "component factor sizes", before)?;
    factor_sizes.resize(count, [0; 3]);
    let offsets = topology.offsets();
    let mut next = 0;
    // All roots were compressed above. Relabel the same array without chasing
    // a parent after an earlier root coordinate has been overwritten.
    for (vertex, root) in labels.iter_mut().enumerate() {
        let label = if root_to_label[*root] == usize::MAX {
            let label = next;
            root_to_label[*root] = label;
            next += 1;
            label
        } else {
            root_to_label[*root]
        };
        *root = label;
        let factor = if vertex < offsets[1] { 0 } else if vertex < offsets[2] { 1 } else { 2 };
        factor_sizes[label][factor] += 1;
    }
    Ok(Partition { labels, factor_sizes })
}
''')

write('crates/multiway-incidence/src/prepared.rs', r'''//! Weights-free preparation and exact borrowed symbolic ownership.

use crate::{IncidenceError, ThreeWayTopology, components::partition::{self, Partition}};
use crate::construction::{array_bytes, reserve, sum_bytes};

/// Layout of the source rows used to construct a prepared topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedTopologySource {
    /// Raw observation rows; duplicates and arbitrary order are retained in groups.
    Observations,
    /// Strictly increasing unique tuples; source-to-tuple mapping is implicit identity.
    Collapsed,
}

/// Deterministic observation grouping, with immutable original-row indices.
#[derive(Debug)]
pub struct ObservationGroups {
    observation_to_tuple: Vec<usize>,
    grouped_observations: Vec<usize>,
    offsets: Vec<usize>,
}

impl ObservationGroups {
    /// Canonical tuple ID for every original observation row.
    #[must_use]
    pub fn observation_to_tuple(&self) -> &[usize] { &self.observation_to_tuple }

    /// Original row indices grouped by tuple, increasing within every group.
    #[must_use]
    pub fn grouped_observations(&self) -> &[usize] { &self.grouped_observations }

    /// Group boundaries; length is unique-tuple count plus one.
    #[must_use]
    pub fn offsets(&self) -> &[usize] { &self.offsets }
}

/// Immutable symbolic incidence state with no weights or numerical factors.
///
/// All construction arrays are reserved fallibly. There is no owning `Clone`:
/// share immutable references rather than implicitly duplicating the topology.
/// A binding borrows this exact owner and cannot outlive it or remain live while
/// the owner is moved. Independent equal builds always have distinct bindings.
/// This is not a numerical-weight generation or a serialization identifier.
#[derive(Debug)]
pub struct PreparedThreeWayTopology {
    topology: ThreeWayTopology,
    partition: Partition,
    groups: Option<ObservationGroups>,
}

/// Borrowed in-process identity of one prepared symbolic owner.
///
/// Copies refer to the same immutable owner and allocate nothing. No address or
/// hash is exposed as a persistent identifier. Downstream sample changes that
/// preserve identical coded tuples still require the caller to build a new owner.
///
/// A token cannot escape its owner's lifetime:
/// ```compile_fail
/// use multiway_incidence::PreparedThreeWayTopology;
/// let token = {
///     let owner = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
///     owner.binding()
/// };
/// assert_eq!(token, token);
/// ```
/// A live token prevents moving/dropping its owner:
/// ```compile_fail
/// use multiway_incidence::PreparedThreeWayTopology;
/// let owner = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
/// let token = owner.binding();
/// drop(owner);
/// assert_eq!(token, token);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PreparedTopologyBinding<'a> {
    owner: &'a PreparedThreeWayTopology,
}

impl PartialEq for PreparedTopologyBinding<'_> {
    fn eq(&self, other: &Self) -> bool { core::ptr::eq(self.owner, other.owner) }
}
impl Eq for PreparedTopologyBinding<'_> {}

impl PreparedTopologyBinding<'_> {
    /// Exact owner identity, not equality of dimensions, tuples or components.
    #[must_use]
    pub fn is_bound_to(self, owner: &PreparedThreeWayTopology) -> bool {
        core::ptr::eq(self.owner, owner)
    }

    /// Reject an unrelated owner before a symbolic operation mutates outputs.
    pub fn validate_for(self, owner: &PreparedThreeWayTopology) -> Result<(), IncidenceError> {
        if !self.is_bound_to(owner) { return Err(IncidenceError::TopologyBindingMismatch); }
        Ok(())
    }
}

impl PreparedThreeWayTopology {
    /// Prepare raw observations without supplying or retaining weights.
    pub fn try_from_observations(counts: [usize; 3], rows: &[[u32; 3]]) -> Result<Self, IncidenceError> {
        Self::try_from_observations_with_budget(counts, rows, usize::MAX)
    }

    /// Prepare observations after admitting the conservative requested-array bound.
    ///
    /// `maximum_setup_payload_bytes` is not an allocator quota or process-RSS cap.
    /// See [`Self::setup_payload_bound`]. All failures publish no partial owner.
    pub fn try_from_observations_with_budget(
        counts: [usize; 3], rows: &[[u32; 3]], maximum_setup_payload_bytes: usize,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(counts, rows, PreparedTopologySource::Observations, maximum_setup_payload_bytes, &mut |_| Ok(()))
    }

    /// Prepare strictly increasing unique tuples, without inventing observation groups.
    pub fn try_from_collapsed(counts: [usize; 3], tuples: &[[u32; 3]]) -> Result<Self, IncidenceError> {
        Self::try_from_collapsed_with_budget(counts, tuples, usize::MAX)
    }

    /// Prepare canonical collapsed tuples with a requested-array setup budget.
    pub fn try_from_collapsed_with_budget(
        counts: [usize; 3], tuples: &[[u32; 3]], maximum_setup_payload_bytes: usize,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(counts, tuples, PreparedTopologySource::Collapsed, maximum_setup_payload_bytes, &mut |_| Ok(()))
    }

    /// Conservative upper bound on simultaneously requested construction-array payload.
    ///
    /// Uses input count as the maximum unique-tuple count and coefficient dimension
    /// as the maximum component count, including invalid unused-level inputs. Counts
    /// retained tuples/groups/labels/factor sizes and temporary root labels. Every
    /// individual array is checked against `isize::MAX`; sums use checked arithmetic.
    /// Excludes caller inputs, inline objects, sorting stack, allocator metadata and
    /// any allocator-provided excess capacity. This is not an exact peak or RSS bound.
    pub fn setup_payload_bound(
        counts: [usize; 3], input_count: usize, source: PreparedTopologySource,
    ) -> Result<usize, IncidenceError> {
        let shape = ThreeWayTopology::new(counts, Vec::new())?;
        if input_count == 0 { return Err(IncidenceError::EmptyProblem); }
        let dimension = shape.total_levels();
        let base = sum_bytes(&[
            array_bytes::<[u32; 3]>(input_count)?,
            array_bytes::<usize>(dimension)?,
            array_bytes::<usize>(dimension)?,
            array_bytes::<[usize; 3]>(dimension)?,
        ])?;
        if source == PreparedTopologySource::Collapsed { return Ok(base); }
        let offsets = input_count.checked_add(1).ok_or(IncidenceError::DimensionOverflow { context: "prepared group offsets" })?;
        sum_bytes(&[base, array_bytes::<usize>(input_count)?, array_bytes::<usize>(input_count)?, array_bytes::<usize>(offsets)?])
    }

    fn build_with<F>(
        counts: [usize; 3], rows: &[[u32; 3]], source: PreparedTopologySource,
        maximum_setup_payload_bytes: usize, before: &mut F,
    ) -> Result<Self, IncidenceError>
    where F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let bound = Self::setup_payload_bound(counts, rows.len(), source)?;
        for (tuple_index, tuple) in rows.iter().enumerate() {
            for factor in 0..3 {
                if tuple[factor] as usize >= counts[factor] {
                    return Err(IncidenceError::TupleOutOfBounds { tuple_index, factor, level: tuple[factor], level_count: counts[factor] });
                }
            }
        }
        if source == PreparedTopologySource::Collapsed {
            if let Some(index) = rows.windows(2).position(|pair| pair[0] >= pair[1]) {
                return Err(IncidenceError::NonCanonicalTuples { tuple_index: index + 1 });
            }
        }
        if bound > maximum_setup_payload_bytes {
            return Err(IncidenceError::TopologySetupBudgetExceeded { required: bound, budget: maximum_setup_payload_bytes });
        }
        let (tuples, groups) = if source == PreparedTopologySource::Observations {
            let mut order = reserve(rows.len(), "prepared observation order", before)?;
            order.extend(0..rows.len());
            // Tuple plus original row is a total ordering: an in-place unstable
            // sort still gives deterministic increasing row order within groups.
            order.sort_unstable_by_key(|&row| (rows[row], row));
            let unique = 1 + order.windows(2).filter(|pair| rows[pair[0]] != rows[pair[1]]).count();
            let mut tuples = reserve(unique, "prepared unique tuples", before)?;
            let mut mapping = reserve(rows.len(), "prepared observation map", before)?;
            mapping.resize(rows.len(), 0);
            let mut offsets = reserve(unique + 1, "prepared group offsets", before)?;
            for (position, &row) in order.iter().enumerate() {
                if tuples.last() != Some(&rows[row]) {
                    tuples.push(rows[row]);
                    offsets.push(position);
                }
                mapping[row] = tuples.len() - 1;
            }
            offsets.push(rows.len());
            (tuples, Some(ObservationGroups { observation_to_tuple: mapping, grouped_observations: order, offsets }))
        } else {
            let mut tuples = reserve(rows.len(), "prepared unique tuples", before)?;
            tuples.extend_from_slice(rows);
            (tuples, None)
        };
        let topology = ThreeWayTopology::new(counts, tuples)?;
        let partition = partition::build(&topology, before)?;
        for factor in 0..3 {
            for (level, &component) in partition.labels[topology.factor_range(factor)].iter().enumerate() {
                if partition.factor_sizes[component].contains(&0) {
                    return Err(IncidenceError::UnusedLevel { factor, level });
                }
            }
        }
        let prepared = Self { topology, partition, groups };
        prepared.retained_payload_bytes()?;
        Ok(prepared)
    }

    /// Immutable, canonically sorted unique tuple topology.
    #[must_use]
    pub const fn topology(&self) -> &ThreeWayTopology { &self.topology }

    /// Original input layout; collapsed input has an implicit identity map.
    #[must_use]
    pub fn source(&self) -> PreparedTopologySource {
        if self.groups.is_some() { PreparedTopologySource::Observations } else { PreparedTopologySource::Collapsed }
    }

    /// Number of original source rows, not necessarily the unique-tuple count.
    #[must_use]
    pub fn input_count(&self) -> usize {
        self.groups.as_ref().map_or(self.topology.tuple_count(), |groups| groups.observation_to_tuple.len())
    }

    /// Explicit groups exist only for raw-observation preparation.
    #[must_use]
    pub const fn observation_groups(&self) -> Option<&ObservationGroups> { self.groups.as_ref() }

    /// Component label for each coefficient in global factor-block order.
    #[must_use]
    pub fn component_labels(&self) -> &[usize] { &self.partition.labels }

    /// Number of levels from each factor in each incidence component.
    ///
    /// These are structural components, not a certificate of full numerical rank.
    #[must_use]
    pub fn component_factor_sizes(&self) -> &[[usize; 3]] { &self.partition.factor_sizes }

    /// Borrow this exact symbolic owner without allocating or changing its state.
    #[must_use]
    pub const fn binding(&self) -> PreparedTopologyBinding<'_> { PreparedTopologyBinding { owner: self } }

    /// Check factor counts and every tuple in the original source-row order.
    ///
    /// Detects row reordering, support/cardinality changes and changed factor codes.
    /// Identical coded rows do not reveal external sample identities or semantics;
    /// callers must rebuild on such changes even if this structural check passes.
    pub fn validate_input_layout(&self, counts: [usize; 3], rows: &[[u32; 3]]) -> Result<(), IncidenceError> {
        if counts != self.topology.level_counts() || rows.len() != self.input_count() {
            return Err(IncidenceError::TopologyLayoutMismatch);
        }
        for (row, tuple) in rows.iter().enumerate() {
            let index = self.groups.as_ref().map_or(row, |groups| groups.observation_to_tuple[row]);
            if *tuple != self.topology.tuples()[index] { return Err(IncidenceError::TopologyLayoutMismatch); }
        }
        Ok(())
    }

    /// Scatter canonical tuple values into the original source-row layout.
    ///
    /// Binding and both dimensions are checked before modifying any output.
    /// This is an allocation-free symbolic copy, including all floating-point bit
    /// patterns; it neither validates weights nor certifies numerical values.
    pub fn scatter_tuple_values_into<T: Copy>(
        &self, binding: PreparedTopologyBinding<'_>, values: &[T], output: &mut [T],
    ) -> Result<(), IncidenceError> {
        binding.validate_for(self)?;
        if values.len() != self.topology.tuple_count() {
            return Err(crate::error::dimension("prepared scatter tuples", self.topology.tuple_count(), values.len()));
        }
        if output.len() != self.input_count() {
            return Err(crate::error::dimension("prepared scatter output", self.input_count(), output.len()));
        }
        if let Some(groups) = &self.groups {
            for (out, &tuple) in output.iter_mut().zip(&groups.observation_to_tuple) { *out = values[tuple]; }
        } else {
            output.copy_from_slice(values);
        }
        Ok(())
    }

    /// Actual exclusive retained array payload, including all unused capacities.
    ///
    /// No Arcs or identity control blocks are owned by this type. Excludes its
    /// inline root/descriptors, caller inputs, allocator overhead and setup scratch.
    /// Borrowed references/tokens add no heap payload. Not a process-RSS report.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        let grouped = if let Some(groups) = &self.groups {
            sum_bytes(&[
                array_bytes::<usize>(groups.observation_to_tuple.capacity())?,
                array_bytes::<usize>(groups.grouped_observations.capacity())?,
                array_bytes::<usize>(groups.offsets.capacity())?,
            ])?
        } else { 0 };
        sum_bytes(&[
            self.topology.retained_payload_bytes()?,
            array_bytes::<usize>(self.partition.labels.capacity())?,
            array_bytes::<[usize; 3]>(self.partition.factor_sizes.capacity())?,
            grouped,
        ])
    }
}

#[cfg(test)]
mod failure_tests;
''')

replace('crates/multiway-incidence/src/lib.rs', 'mod components;\n', 'mod components;\nmod construction;\nmod prepared;\n')
replace('crates/multiway-incidence/src/lib.rs', 'pub use topology::ThreeWayTopology;', 'pub use topology::ThreeWayTopology;\npub use prepared::{ObservationGroups, PreparedThreeWayTopology, PreparedTopologyBinding, PreparedTopologySource};')
replace('crates/multiway-incidence/src/components.rs', 'mod workspace;\n', 'mod workspace;\npub(crate) mod partition;\n')
p = Path('crates/multiway-incidence/src/components.rs')
s = p.read_text()
a = s.index('        let n = topology.total_levels();', s.index('pub fn from_topology'))
b = s.index('    /// Number of connected incidence components.', a)
s = s[:a] + '''        let partition::Partition { labels, factor_sizes } =
            partition::build(topology, &mut |_| Ok(()))
                .expect("incidence component array reservation failed");
        Self { labels, factor_sizes, offsets: topology.offsets(), binding: Arc::new(()) }
    }

''' + s[b:]
p.write_text(s)
replace('crates/multiway-incidence/src/error.rs', 'pub enum IncidenceError {\n', '''pub enum IncidenceError {
    /// A fallible immutable topology or component array reservation failed.
    #[error("topology array allocation failed in {context}")]
    TopologyAllocation {
        /// Array reservation boundary.
        context: &'static str,
    },
    /// Conservative requested-array setup payload exceeds the declared budget.
    #[error("topology setup requests at most {required} payload bytes, budget is {budget}")]
    TopologySetupBudgetExceeded {
        /// Conservative requested-array upper bound, not OS memory.
        required: usize,
        /// Declared requested-array payload budget.
        budget: usize,
    },
    /// A symbolic binding was issued by another prepared owner.
    #[error("prepared topology binding belongs to a different owner")]
    TopologyBindingMismatch,
    /// Factor counts or original coded source rows differ from preparation.
    #[error("source layout differs from the prepared topology")]
    TopologyLayoutMismatch,
    /// Collapsed source is not strictly increasing and unique.
    #[error("collapsed tuple {tuple_index} is not strictly greater than its predecessor")]
    NonCanonicalTuples {
        /// First offending zero-based tuple row.
        tuple_index: usize,
    },
''')

write('crates/multiway-incidence/src/prepared/failure_tests.rs', r'''//! Local construction-boundary failure tests, not OS allocator fault injection.
use super::*;

#[test]
fn every_preparation_boundary_recovers_after_error_and_unwind() {
    let rows = [[0, 0, 0], [1, 1, 1], [0, 1, 1], [1, 0, 0]];
    let old = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    let token = old.binding();
    let mut sorted = rows;
    sorted.sort_unstable();
    for source in [PreparedTopologySource::Observations, PreparedTopologySource::Collapsed] {
        let input = if source == PreparedTopologySource::Observations { &rows } else { &sorted };
        let count = if source == PreparedTopologySource::Observations { 7 } else { 4 };
        for unwind in [false, true] {
            for fail_at in 0..count {
                let mut reached = 0;
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    PreparedThreeWayTopology::build_with([2; 3], input, source, usize::MAX, &mut |context| {
                        let index = reached;
                        reached += 1;
                        if index == fail_at {
                            assert!(!unwind, "injected topology construction unwind");
                            return Err(IncidenceError::TopologyAllocation { context });
                        }
                        Ok(())
                    })
                }));
                assert_eq!(reached, fail_at + 1);
                if unwind { assert!(outcome.is_err()); } else {
                    assert!(matches!(outcome.unwrap(), Err(IncidenceError::TopologyAllocation { .. })));
                }
                old.validate_input_layout([2; 3], &rows).unwrap();
                assert!(token.is_bound_to(&old));
                let fresh = PreparedThreeWayTopology::build_with([2; 3], input, source, usize::MAX, &mut |_| Ok(())).unwrap();
                fresh.validate_input_layout([2; 3], input).unwrap();
                assert!(!token.is_bound_to(&fresh));
                let mut output = [99; 4];
                old.scatter_tuple_values_into(token, &[10, 20, 30, 40], &mut output).unwrap();
                assert_eq!(output, [10, 40, 20, 30]);
            }
        }
    }
}

#[test]
fn preflight_rejections_do_not_reach_array_reservations() {
    let valid = [[0, 0, 0], [1, 1, 1]];
    let source = PreparedTopologySource::Observations;
    let bound = PreparedThreeWayTopology::setup_payload_bound([2; 3], valid.len(), source).unwrap();
    for (counts, input, source, budget) in [
        ([2; 3], valid.as_slice(), source, bound - 1),
        ([2; 3], &[][..], source, usize::MAX),
        ([0, 2, 2], valid.as_slice(), source, usize::MAX),
        ([1; 3], valid.as_slice(), source, usize::MAX),
        ([2; 3], &[[1; 3], [0; 3]][..], PreparedTopologySource::Collapsed, usize::MAX),
    ] {
        assert!(PreparedThreeWayTopology::build_with(counts, input, source, budget, &mut |_| panic!("preflight failed to reject")).is_err());
    }
}
''')

write('crates/multiway-incidence/tests/prepared_topology.rs', r'''//! Independent symbolic references and source/owner invalidation.
use multiway_incidence::{IncidenceComponents, IncidenceError, PreparedThreeWayTopology, PreparedTopologySource, ThreeWayProblem, ThreeWayTopology};

#[test]
fn exact_original_row_groups_and_bit_preserving_scatter() {
    let rows = [[1,0,1], [0,1,0], [1,0,1], [0,0,0], [0,1,0], [1,1,1]];
    let prepared = PreparedThreeWayTopology::try_from_observations([2;3], &rows).unwrap();
    assert_eq!(prepared.topology().tuples(), &[[0,0,0], [0,1,0], [1,0,1], [1,1,1]]);
    let groups = prepared.observation_groups().unwrap();
    assert_eq!(groups.observation_to_tuple(), &[2,1,2,0,1,3]);
    assert_eq!(groups.grouped_observations(), &[3,1,4,0,2,5]);
    assert_eq!(groups.offsets(), &[0,1,3,5,6]);
    let values = [3.0, 5.0, f64::from_bits(0x7ff8_0000_0000_0123), -0.0];
    let mut output = [0.0;6];
    prepared.scatter_tuple_values_into(prepared.binding(), &values, &mut output).unwrap();
    for (row, &tuple) in groups.observation_to_tuple().iter().enumerate() {
        assert_eq!(output[row].to_bits(), values[tuple].to_bits());
    }
    for (tuple, range) in groups.offsets().windows(2).enumerate() {
        let expected: Vec<_> = rows.iter().enumerate().filter_map(|(row, key)| (*key == prepared.topology().tuples()[tuple]).then_some(row)).collect();
        assert_eq!(&groups.grouped_observations()[range[0]..range[1]], expected);
    }
}

#[test]
fn collapsed_layout_is_explicit_and_strict() {
    let rows = [[0,0,0],[1,1,1]];
    let prepared = PreparedThreeWayTopology::try_from_collapsed([2;3], &rows).unwrap();
    assert_eq!(prepared.source(), PreparedTopologySource::Collapsed);
    assert_eq!(prepared.input_count(), 2);
    assert!(prepared.observation_groups().is_none());
    assert_eq!(prepared.component_labels(), &[0,1,0,1,0,1]);
    assert_eq!(prepared.component_factor_sizes(), &[[1;3],[1;3]]);
    let mut output = [0;2];
    prepared.scatter_tuple_values_into(prepared.binding(), &[31,42], &mut output).unwrap();
    assert_eq!(output, [31,42]);
    for invalid in [[[1;3],[0;3]], [[0;3],[0;3]]] {
        assert!(matches!(PreparedThreeWayTopology::try_from_collapsed([2;3], &invalid), Err(IncidenceError::NonCanonicalTuples{tuple_index:1})));
    }
}

#[test]
fn exact_owner_and_layout_rejection_precedes_output_mutation() {
    let rows = [[0,0,0],[1,1,1],[0,1,1],[1,0,0]];
    let first = PreparedThreeWayTopology::try_from_observations([2;3], &rows).unwrap();
    let second = PreparedThreeWayTopology::try_from_observations([2;3], &rows).unwrap();
    assert_eq!(first.topology(), second.topology());
    assert_ne!(first.binding(), second.binding());
    let alias = &first;
    assert_eq!(first.binding(), alias.binding());
    let mut out = [23;4];
    assert!(matches!(first.scatter_tuple_values_into(second.binding(), &[1,2,3,4], &mut out), Err(IncidenceError::TopologyBindingMismatch)));
    assert!(first.scatter_tuple_values_into(first.binding(), &[1,2,3], &mut out).is_err());
    assert!(first.scatter_tuple_values_into(first.binding(), &[1,2,3,4], &mut out[..3]).is_err());
    assert_eq!(out, [23;4]);
    first.validate_input_layout([2;3], &rows).unwrap();
    let mut reordered = rows;
    reordered.swap(0,1);
    assert!(first.validate_input_layout([2;3], &reordered).is_err());
    let mut changed = rows;
    changed[0] = [0,0,1];
    assert!(first.validate_input_layout([2;3], &changed).is_err());
    assert!(first.validate_input_layout([2,2,3], &rows).is_err());
    assert!(first.validate_input_layout([2;3], &rows[..3]).is_err());
    first.scatter_tuple_values_into(first.binding(), &[1,2,3,4], &mut out).unwrap();
    assert_eq!(out, [1,4,2,3]);
}

fn reference_labels(topology: &ThreeWayTopology) -> Vec<usize> {
    let mut labels = vec![usize::MAX; topology.total_levels()];
    let mut component = 0;
    for start in 0..labels.len() {
        if labels[start] != usize::MAX { continue; }
        labels[start] = component;
        let mut pending = vec![start];
        while let Some(vertex) = pending.pop() {
            for tuple in topology.tuples() {
                let adjacent = core::array::from_fn::<_,3,_>(|factor| topology.global_index(factor, tuple[factor]));
                if adjacent.contains(&vertex) {
                    for neighbor in adjacent {
                        if labels[neighbor] == usize::MAX { labels[neighbor] = component; pending.push(neighbor); }
                    }
                }
            }
        }
        component += 1;
    }
    labels
}

#[test]
fn exhaustive_small_supports_match_independent_graph_search() {
    let universe: Vec<_> = (0..2).flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i,j,k]))).collect();
    for mask in 1..256 {
        let tuples: Vec<_> = universe.iter().enumerate().filter_map(|(i,tuple)| ((mask >> i) & 1 == 1).then_some(*tuple)).collect();
        let topology = ThreeWayTopology::new([2;3], tuples.clone()).unwrap();
        let reference = reference_labels(&topology);
        let legacy = IncidenceComponents::from_topology(&topology);
        assert_eq!(legacy.labels(), reference);
        let unused = (0..3).any(|f| (0..2).any(|level| !tuples.iter().any(|t| t[f] == level)));
        let result = PreparedThreeWayTopology::try_from_collapsed([2;3], &tuples);
        if unused {
            assert!(matches!(result, Err(IncidenceError::UnusedLevel {..})));
        } else {
            let prepared = result.unwrap();
            assert_eq!(prepared.component_labels(), reference);
            assert_eq!(prepared.component_factor_sizes(), legacy.factor_sizes());
            let problem = ThreeWayProblem::from_observations([2;3], &tuples, &vec![1.0;tuples.len()]).unwrap();
            assert_eq!(prepared.topology(), problem.topology());
            let mut raw = tuples.clone();
            raw.extend(tuples.iter().rev());
            raw.reverse();
            let observed = PreparedThreeWayTopology::try_from_observations([2;3], &raw).unwrap();
            assert_eq!(observed.topology(), prepared.topology());
            assert_eq!(observed.component_labels(), reference);
            observed.validate_input_layout([2;3], &raw).unwrap();
        }
    }
    // Existing component-only construction still permits isolated/unused vertices.
    let empty = ThreeWayTopology::new([2;3], vec![]).unwrap();
    assert_eq!(IncidenceComponents::from_topology(&empty).labels(), reference_labels(&empty));
}

#[test]
fn invalid_inputs_and_checked_setup_budgets() {
    assert!(matches!(PreparedThreeWayTopology::try_from_observations([1;3], &[]), Err(IncidenceError::EmptyProblem)));
    assert!(matches!(PreparedThreeWayTopology::try_from_observations([0,1,1], &[[0;3]]), Err(IncidenceError::EmptyFactor {factor:0})));
    assert!(matches!(PreparedThreeWayTopology::try_from_observations([1;3], &[[1,0,0]]), Err(IncidenceError::TupleOutOfBounds {..})));
    assert!(matches!(PreparedThreeWayTopology::try_from_observations([2,1,1], &[[0;3]]), Err(IncidenceError::UnusedLevel {factor:0, level:1})));
    assert!(PreparedThreeWayTopology::setup_payload_bound([1;3], usize::MAX, PreparedTopologySource::Observations).is_err());
    for source in [PreparedTopologySource::Observations, PreparedTopologySource::Collapsed] {
        let bound = PreparedThreeWayTopology::setup_payload_bound([1;3], 1, source).unwrap();
        let build = |budget| match source {
            PreparedTopologySource::Observations => PreparedThreeWayTopology::try_from_observations_with_budget([1;3], &[[0;3]], budget),
            PreparedTopologySource::Collapsed => PreparedThreeWayTopology::try_from_collapsed_with_budget([1;3], &[[0;3]], budget),
        };
        assert!(matches!(build(bound - 1), Err(IncidenceError::TopologySetupBudgetExceeded { required, budget }) if required == bound && budget == bound-1));
        let prepared = build(bound).unwrap();
        assert!(prepared.retained_payload_bytes().unwrap() <= bound);
    }
}

#[test]
fn shared_borrows_work_across_threads_without_mutable_owner_state() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<PreparedThreeWayTopology>();
    let prepared = PreparedThreeWayTopology::try_from_observations([2;3], &[[1;3],[0;3],[1;3]]).unwrap();
    std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            let mut out = [0;3];
            prepared.scatter_tuple_values_into(prepared.binding(), &[10,20], &mut out).unwrap();
            out
        });
        let second = scope.spawn(|| {
            let mut out = [0;3];
            prepared.scatter_tuple_values_into(prepared.binding(), &[30,40], &mut out).unwrap();
            out
        });
        assert_eq!(first.join().unwrap(), [20,10,20]);
        assert_eq!(second.join().unwrap(), [40,30,40]);
    });
}
''')

write('crates/multiway-mg/tests/support/prepared_topology_allocations.rs', r'''//! Exact array lifetime and allocation-free symbolic operations in the existing process.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{IncidenceError, PreparedThreeWayTopology, PreparedTopologySource, ThreeWayProblem};
use std::hint::black_box;

pub(super) fn run() -> Result<()> {
    let cases = [
        ([1,1,1], vec![[0,0,0];5]),
        ([2,2,2], vec![[1,1,1],[0,0,0],[1,1,1]]),
        ([2,3,4], (0..2).flat_map(|i| (0..3).flat_map(move |j| (0..4).map(move |k| [i,j,k]))).collect()),
        ([3,3,3], (0..108).map(|i| [(i%3) as u32, ((i/3)%3) as u32, ((i/9)%3) as u32]).collect()),
    ];
    let mut tested = 0;
    for (counts, rows) in cases {
        let original = ThreeWayProblem::from_observations(counts, &rows, &vec![1.0;rows.len()])?;
        for source in [PreparedTopologySource::Observations, PreparedTopologySource::Collapsed] {
            let input = if source == PreparedTopologySource::Observations { rows.as_slice() } else { original.topology().tuples() };
            let bound = PreparedThreeWayTopology::setup_payload_bound(counts, input.len(), source)?;
            let build = |budget| match source {
                PreparedTopologySource::Observations => PreparedThreeWayTopology::try_from_observations_with_budget(counts, black_box(input), budget),
                PreparedTopologySource::Collapsed => PreparedThreeWayTopology::try_from_collapsed_with_budget(counts, black_box(input), budget),
            };
            let before = GLOBAL.stats();
            let error = build(bound - 1).unwrap_err();
            no_events(GLOBAL.stats() - before);
            assert!(matches!(error, IncidenceError::TopologySetupBudgetExceeded {..}));
            let before = GLOBAL.stats();
            let prepared = black_box(build(bound)?);
            let setup = GLOBAL.stats() - before;
            let retained = prepared.retained_payload_bytes()?;
            assert_eq!(setup.reallocations, 0);
            assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, retained);
            assert!(setup.bytes_allocated <= bound);
            assert_eq!(setup.allocations, if source == PreparedTopologySource::Observations {7} else {4});
            assert_eq!(prepared.topology(), original.topology());
            assert_eq!(prepared.component_labels(), original.components().labels());
            let foreign = build(bound)?;
            let values: Vec<_> = (0..prepared.topology().tuple_count()).collect();
            let mut output = vec![usize::MAX;input.len()];
            let before = GLOBAL.stats();
            let token = prepared.binding();
            for _ in 0..64 {
                prepared.validate_input_layout(counts, black_box(input))?;
                token.validate_for(&prepared)?;
                assert_eq!(prepared.retained_payload_bytes()?, retained);
                prepared.scatter_tuple_values_into(token, black_box(&values), black_box(&mut output))?;
            }
            no_events(GLOBAL.stats() - before);
            for (row, &tuple) in output.iter().enumerate() { assert_eq!(prepared.topology().tuples()[tuple], input[row]); }
            let snapshot = output.clone();
            let before = GLOBAL.stats();
            let wrong_owner = prepared.scatter_tuple_values_into(foreign.binding(), &values, &mut output).unwrap_err();
            let wrong_dimension = prepared.scatter_tuple_values_into(token, &values[..values.len()-1], &mut output).unwrap_err();
            no_events(GLOBAL.stats() - before);
            assert!(matches!(wrong_owner, IncidenceError::TopologyBindingMismatch));
            assert!(matches!(wrong_dimension, IncidenceError::DimensionMismatch {..}));
            assert_eq!(output, snapshot);
            let before = GLOBAL.stats();
            drop(prepared);
            let released = GLOBAL.stats() - before;
            assert_eq!(released.allocations, 0);
            assert_eq!(released.reallocations, 0);
            assert_eq!(released.bytes_deallocated, retained);
            println!("prepared-topology source={source:?} rows={} setup_bound={bound} allocated={} freed_setup={} retained={retained} release=exact read/scatter/reject_allocations=0", input.len(), setup.bytes_allocated, setup.bytes_deallocated);
            tested += 1;
        }
    }
    assert_eq!(tested, 8);
    println!("PASS prepared-topology cases=8 borrowed-owner checks and array lifetime accounting");
    Ok(())
}
''')
replace('crates/multiway-mg/tests/workspace_allocations.rs', 'mod pcg_allocations;\n', 'mod pcg_allocations;\n#[path = "support/prepared_topology_allocations.rs"]\nmod prepared_topology_allocations;\n')
replace('crates/multiway-mg/tests/workspace_allocations.rs', '    operator_checks()?;\n', '    operator_checks()?;\n    prepared_topology_allocations::run()?;\n')
replace('CHANGELOG.md', '### Added\n', '### Added\n\n- Weights-free prepared incidence topology with deterministic observation groups,\n  borrowed owner bindings, fallible arrays and checked setup-payload admission;\n  see `docs/ISSUE5_PREPARED_TOPOLOGY.md`.\n')
write('docs/ISSUE5_PREPARED_TOPOLOGY.md', '''# Issue 5: prepared immutable incidence topology

## Qualified boundary

`PreparedThreeWayTopology` in `multiway-incidence` retains only canonical tuple
keys, deterministic observation groups and structural component metadata. It
contains no weights, diagonals, operator images, smoother parameters or factors.
Existing weighted-problem and hierarchy constructors remain the numerical path;
this increment does not implement a weight frame or numerical hierarchy replay.

Raw observations are sorted by (tuple key, original row index), with an in-place
unstable sort on that total order. The retained arrays are original-row-to-tuple,
grouped original rows and tuple-group offsets. Every group's rows are increasing
in original input order, matching the order needed by subsequent deterministic
compensated duplicate aggregation. This increment does not itself sum weights.

The collapsed-input constructor requires strictly increasing unique tuple keys.
It rejects duplicates or unsorted input rather than silently changing the caller's
weight layout. Its source-to-tuple mapping is implicit identity: it allocates no
observation-group arrays and makes no claim to recover original physical rows.
Both constructors reject empty inputs, invalid codes/counts and unused levels.
Extra null directions beyond structural factor shifts are not rank-certified here.

## Owner identity and invalidation

`PreparedTopologyBinding` borrows the exact immutable prepared owner. Copies and
shared references are cheap and allocation-free. Rust prevents a token outliving
its owner, and prevents moving/replacing that owner while its binding remains live.
Independent equal builds have different bindings. There is no owning Clone, global
ID generator, hash-equality shortcut, unsafe pointer storage, Arc allocation or
serialized token. The identity is an in-process symbolic lifetime boundary only.

`validate_input_layout` checks factor cardinalities and every coded source tuple
in its original row order. It catches changed support, row count, factor codes and
reordering of distinguishable rows. It cannot detect external observation IDs,
reordering of identical coded rows, changed factor meanings or estimator sample
policy when the supplied tuples are identical. Such changes must build a new owner.
Retained-sample, aggregation-map and numerical-weight generations are still separate
future responsibilities; passing the structural check authorizes no numerical reuse.

`scatter_tuple_values_into` is a symbolic copy into original source order. It
checks the supplied binding and both dimensions before output mutation. It preserves
all bit patterns and is intentionally not a positive-weight or finite-value check.
No output or previous owner is modified by a failed constructor or rejected scatter.

## Construction and memory

All new arrays use checked `try_reserve_exact` at explicit setup boundaries. The
prepared owner has no hidden infallible identity allocation. A private per-call
no-op callback allows tests to inject failure or unwinding before each reservation;
no callback, allocator override or mutable global is retained in production state.

`setup_payload_bound(counts, input_count, source)` conservatively charges the maximum
unique-tuple count (input count), maximum component count (coefficient dimension),
retained arrays and component root-label scratch together. This is deliberately an
upper bound even for invalid inputs with unused levels. The `_with_budget`
constructors reject when that requested-array bound exceeds the budget, before any
array reservation. Equality is admitted. The bound may reject a duplicate-heavy
input whose eventual retained representation would be smaller.

This is not OS memory admission: caller input storage, inline descriptors, sorting
stack, allocator metadata/rounding and excess allocator-provided capacity are outside
the requested-array bound. All individual arrays and combined arithmetic are checked.
`retained_payload_bytes` instead counts actual retained capacities after construction.
The isolated allocator gate reconciles construction allocations minus released setup
scratch to that report and destruction to the same payload, with no Arc exclusions.

Component discovery is one shared fallible array routine used by prepared topology
and the existing `IncidenceComponents::from_topology`. Minimum-root union and
first-global-vertex label ordering are unchanged. Compressed roots become labels in
the same array, reducing scratch duplication; component-size storage reserves its
exact discovered count. The old public component-only API remains infallible and
may still construct isolated components; it unwraps reservation errors and keeps
its existing Arc projection identity. Prepared construction does not call that
infallible wrapper. Projection arithmetic, weighted kernels and solver recurrences
are untouched. Historical exact byte figures may differ because metadata capacities
are now sized by discovered component count; reports use actual capacities.

## Tests and limitations

Require exact-head Rust 1.85 Actions and all permanent scientific gates. New tests
cover exact row groups and bit-preserving scatter, canonical collapsed layout,
wrong owner/dimensions/source layout, exhaustive nonempty supports on a 2x2x2 domain
against independent graph search, empty legacy component topology, unused levels,
setup overflow and one-byte-short budgets, shared concurrent references and two
compile-fail binding lifetime examples.

Error and unwind injection covers all seven observation-construction reservations
and all four collapsed-construction reservations, including shared component setup.
It is deterministic boundary injection, not an OS allocator returning null. Older
owners and source rows remain reusable; partial arrays are ordinary RAII-owned Vecs.
The existing 12-configuration allocation executable adds eight prepared-source cases
and verifies first/64-repeated symbolic calls, static rejection and exact array
allocation/destruction accounting. Existing complete-cycle/PCG gates stay intact.

No weight-generation safety, numerical replay, setup peak-RSS cap, production solver
choice, timing win, fresh holdout or closure of issue #5 is implied. Next: symbolic
coarse-tuple/pair-edge grouping, explicit validated weight frames and complete numeric
replay with map quality screening in separately reviewed increments. ADR 0002 remains
unchanged.
''')
print('Prepared topology permanent source assembled; validation requires clean-head Actions.')
