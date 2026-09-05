//! Weights-free preparation and exact borrowed symbolic ownership.

use crate::construction::{array_bytes, reserve, sum_bytes};
use crate::{
    IncidenceError, ThreeWayTopology,
    components::partition::{self, Partition},
};

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
    pub fn observation_to_tuple(&self) -> &[usize] {
        &self.observation_to_tuple
    }

    /// Original row indices grouped by tuple, increasing within every group.
    #[must_use]
    pub fn grouped_observations(&self) -> &[usize] {
        &self.grouped_observations
    }

    /// Group boundaries; length is unique-tuple count plus one.
    #[must_use]
    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }
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
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.owner, other.owner)
    }
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
        if !self.is_bound_to(owner) {
            return Err(IncidenceError::TopologyBindingMismatch);
        }
        Ok(())
    }
}

impl PreparedThreeWayTopology {
    /// Prepare raw observations without supplying or retaining weights.
    pub fn try_from_observations(
        counts: [usize; 3],
        rows: &[[u32; 3]],
    ) -> Result<Self, IncidenceError> {
        Self::try_from_observations_with_budget(counts, rows, usize::MAX)
    }

    /// Prepare observations after admitting the conservative requested-array bound.
    ///
    /// `maximum_setup_payload_bytes` is not an allocator quota or process-RSS cap.
    /// See [`Self::setup_payload_bound`]. All failures publish no partial owner.
    pub fn try_from_observations_with_budget(
        counts: [usize; 3],
        rows: &[[u32; 3]],
        maximum_setup_payload_bytes: usize,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(
            counts,
            rows,
            PreparedTopologySource::Observations,
            maximum_setup_payload_bytes,
            &mut |_| Ok(()),
        )
    }

    /// Prepare strictly increasing unique tuples, without inventing observation groups.
    pub fn try_from_collapsed(
        counts: [usize; 3],
        tuples: &[[u32; 3]],
    ) -> Result<Self, IncidenceError> {
        Self::try_from_collapsed_with_budget(counts, tuples, usize::MAX)
    }

    /// Prepare canonical collapsed tuples with a requested-array setup budget.
    pub fn try_from_collapsed_with_budget(
        counts: [usize; 3],
        tuples: &[[u32; 3]],
        maximum_setup_payload_bytes: usize,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(
            counts,
            tuples,
            PreparedTopologySource::Collapsed,
            maximum_setup_payload_bytes,
            &mut |_| Ok(()),
        )
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
        counts: [usize; 3],
        input_count: usize,
        source: PreparedTopologySource,
    ) -> Result<usize, IncidenceError> {
        let shape = ThreeWayTopology::new(counts, Vec::new())?;
        if input_count == 0 {
            return Err(IncidenceError::EmptyProblem);
        }
        let dimension = shape.total_levels();
        let base = sum_bytes(&[
            array_bytes::<[u32; 3]>(input_count)?,
            array_bytes::<usize>(dimension)?,
            array_bytes::<usize>(dimension)?,
            array_bytes::<[usize; 3]>(dimension)?,
        ])?;
        if source == PreparedTopologySource::Collapsed {
            return Ok(base);
        }
        let offsets = input_count
            .checked_add(1)
            .ok_or(IncidenceError::DimensionOverflow {
                context: "prepared group offsets",
            })?;
        sum_bytes(&[
            base,
            array_bytes::<usize>(input_count)?,
            array_bytes::<usize>(input_count)?,
            array_bytes::<usize>(offsets)?,
        ])
    }

    fn build_with<F>(
        counts: [usize; 3],
        rows: &[[u32; 3]],
        source: PreparedTopologySource,
        maximum_setup_payload_bytes: usize,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let bound = Self::setup_payload_bound(counts, rows.len(), source)?;
        for (tuple_index, tuple) in rows.iter().enumerate() {
            for factor in 0..3 {
                if tuple[factor] as usize >= counts[factor] {
                    return Err(IncidenceError::TupleOutOfBounds {
                        tuple_index,
                        factor,
                        level: tuple[factor],
                        level_count: counts[factor],
                    });
                }
            }
        }
        if source == PreparedTopologySource::Collapsed {
            if let Some(index) = rows.windows(2).position(|pair| pair[0] >= pair[1]) {
                return Err(IncidenceError::NonCanonicalTuples {
                    tuple_index: index + 1,
                });
            }
        }
        if bound > maximum_setup_payload_bytes {
            return Err(IncidenceError::TopologySetupBudgetExceeded {
                required: bound,
                budget: maximum_setup_payload_bytes,
            });
        }
        let (tuples, groups) = if source == PreparedTopologySource::Observations {
            let mut order = reserve(rows.len(), "prepared observation order", before)?;
            order.extend(0..rows.len());
            // Tuple plus original row is a total ordering: an in-place unstable
            // sort still gives deterministic increasing row order within groups.
            order.sort_unstable_by_key(|&row| (rows[row], row));
            let unique = 1 + order
                .windows(2)
                .filter(|pair| rows[pair[0]] != rows[pair[1]])
                .count();
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
            (
                tuples,
                Some(ObservationGroups {
                    observation_to_tuple: mapping,
                    grouped_observations: order,
                    offsets,
                }),
            )
        } else {
            let mut tuples = reserve(rows.len(), "prepared unique tuples", before)?;
            tuples.extend_from_slice(rows);
            (tuples, None)
        };
        let topology = ThreeWayTopology::new(counts, tuples)?;
        let partition = partition::build(&topology, before)?;
        for factor in 0..3 {
            for (level, &component) in partition.labels[topology.factor_range(factor)]
                .iter()
                .enumerate()
            {
                if partition.factor_sizes[component].contains(&0) {
                    return Err(IncidenceError::UnusedLevel { factor, level });
                }
            }
        }
        let prepared = Self {
            topology,
            partition,
            groups,
        };
        prepared.retained_payload_bytes()?;
        Ok(prepared)
    }

    /// Immutable, canonically sorted unique tuple topology.
    #[must_use]
    pub const fn topology(&self) -> &ThreeWayTopology {
        &self.topology
    }

    /// Original input layout; collapsed input has an implicit identity map.
    #[must_use]
    pub fn source(&self) -> PreparedTopologySource {
        if self.groups.is_some() {
            PreparedTopologySource::Observations
        } else {
            PreparedTopologySource::Collapsed
        }
    }

    /// Number of original source rows, not necessarily the unique-tuple count.
    #[must_use]
    pub fn input_count(&self) -> usize {
        self.groups
            .as_ref()
            .map_or(self.topology.tuple_count(), |groups| {
                groups.observation_to_tuple.len()
            })
    }

    /// Explicit groups exist only for raw-observation preparation.
    #[must_use]
    pub const fn observation_groups(&self) -> Option<&ObservationGroups> {
        self.groups.as_ref()
    }

    /// Component label for each coefficient in global factor-block order.
    #[must_use]
    pub fn component_labels(&self) -> &[usize] {
        &self.partition.labels
    }

    /// Number of levels from each factor in each incidence component.
    ///
    /// These are structural components, not a certificate of full numerical rank.
    #[must_use]
    pub fn component_factor_sizes(&self) -> &[[usize; 3]] {
        &self.partition.factor_sizes
    }

    /// Borrow this exact symbolic owner without allocating or changing its state.
    #[must_use]
    pub const fn binding(&self) -> PreparedTopologyBinding<'_> {
        PreparedTopologyBinding { owner: self }
    }

    /// Check factor counts and every tuple in the original source-row order.
    ///
    /// Detects row reordering, support/cardinality changes and changed factor codes.
    /// Identical coded rows do not reveal external sample identities or semantics;
    /// callers must rebuild on such changes even if this structural check passes.
    pub fn validate_input_layout(
        &self,
        counts: [usize; 3],
        rows: &[[u32; 3]],
    ) -> Result<(), IncidenceError> {
        if counts != self.topology.level_counts() || rows.len() != self.input_count() {
            return Err(IncidenceError::TopologyLayoutMismatch);
        }
        for (row, tuple) in rows.iter().enumerate() {
            let index = self
                .groups
                .as_ref()
                .map_or(row, |groups| groups.observation_to_tuple[row]);
            if *tuple != self.topology.tuples()[index] {
                return Err(IncidenceError::TopologyLayoutMismatch);
            }
        }
        Ok(())
    }

    /// Scatter canonical tuple values into the original source-row layout.
    ///
    /// Binding and both dimensions are checked before modifying any output.
    /// This is an allocation-free symbolic copy, including all floating-point bit
    /// patterns; it neither validates weights nor certifies numerical values.
    pub fn scatter_tuple_values_into<T: Copy>(
        &self,
        binding: PreparedTopologyBinding<'_>,
        values: &[T],
        output: &mut [T],
    ) -> Result<(), IncidenceError> {
        binding.validate_for(self)?;
        if values.len() != self.topology.tuple_count() {
            return Err(crate::error::dimension(
                "prepared scatter tuples",
                self.topology.tuple_count(),
                values.len(),
            ));
        }
        if output.len() != self.input_count() {
            return Err(crate::error::dimension(
                "prepared scatter output",
                self.input_count(),
                output.len(),
            ));
        }
        if let Some(groups) = &self.groups {
            for (out, &tuple) in output.iter_mut().zip(&groups.observation_to_tuple) {
                *out = values[tuple];
            }
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
        } else {
            0
        };
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
