//! Validated immutable numerical weights, separate from symbolic topology.

use crate::{
    IncidenceError, PreparedThreeWayTopology, PreparedTopologyBinding, PreparedTopologySource,
    construction::{array_bytes, reserve, sum_bytes},
    problem::{CompensatedSum, fill_weighted_degrees},
};

/// Explicit numerical input layout; lengths never determine its interpretation.
#[derive(Debug, Clone, Copy)]
pub enum WeightFrameInput<'a> {
    /// Weights in the original observation order retained by the prepared source.
    Observations(&'a [f64]),
    /// Weights in canonical unique-tuple order, for either prepared source kind.
    Tuples(&'a [f64]),
    /// One per original observation; duplicate multiplicities are retained.
    UnitObservations,
    /// One per unique tuple, regardless of original observation multiplicity.
    UnitTuples,
}

/// Provenance of the submitted weight values, not an identity certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightFrameInputKind {
    /// Explicit original-observation weights.
    Observations,
    /// Explicit canonical unique-tuple weights.
    Tuples,
    /// Implicit ones per original observation.
    UnitObservations,
    /// Implicit ones per unique tuple.
    UnitTuples,
}

impl WeightFrameInput<'_> {
    /// Explicit input provenance.
    #[must_use]
    pub const fn kind(self) -> WeightFrameInputKind {
        match self {
            Self::Observations(_) => WeightFrameInputKind::Observations,
            Self::Tuples(_) => WeightFrameInputKind::Tuples,
            Self::UnitObservations => WeightFrameInputKind::UnitObservations,
            Self::UnitTuples => WeightFrameInputKind::UnitTuples,
        }
    }

    fn is_observations(self) -> bool {
        matches!(self, Self::Observations(_) | Self::UnitObservations)
    }

    fn values(self) -> Option<&[f64]> {
        match self {
            Self::Observations(values) | Self::Tuples(values) => Some(values),
            Self::UnitObservations | Self::UnitTuples => None,
        }
    }

    fn value(self, index: usize) -> f64 {
        self.values().map_or(1.0, |values| values[index])
    }

    fn validate_layout(self, topology: &PreparedThreeWayTopology) -> Result<usize, IncidenceError> {
        let expected = if self.is_observations() {
            if topology.source() != PreparedTopologySource::Observations {
                return Err(IncidenceError::WeightFrameObservationLayoutRequired);
            }
            topology.input_count()
        } else {
            topology.topology().tuple_count()
        };
        if let Some(values) = self.values() {
            if values.len() != expected {
                return Err(crate::error::dimension(
                    if self.is_observations() {
                        "weight frame observation input"
                    } else {
                        "weight frame canonical tuple input"
                    },
                    expected,
                    values.len(),
                ));
            }
        }
        Ok(expected)
    }
}

/// Finite positive extrema for one exact incidence component.
///
/// Extrema are diagnostics of represented weights/degrees, not conditioning,
/// rank, an inverse-degree guarantee, or permission to reuse a hierarchy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentWeightRange {
    /// Smallest represented canonical tuple weight in this component.
    pub minimum_tuple_weight: f64,
    /// Largest represented canonical tuple weight in this component.
    pub maximum_tuple_weight: f64,
    /// Smallest represented weighted degree in this component.
    pub minimum_degree: f64,
    /// Largest represented weighted degree in this component.
    pub maximum_degree: f64,
}

impl ComponentWeightRange {
    const EMPTY: Self = Self {
        minimum_tuple_weight: f64::INFINITY,
        maximum_tuple_weight: 0.0,
        minimum_degree: f64::INFINITY,
        maximum_degree: 0.0,
    };
}

/// Validation metadata for one successfully published immutable frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightFrameValidationReport {
    /// Interpretation of the submitted values, including explicit versus unit input.
    pub input_kind: WeightFrameInputKind,
    /// Number of submitted or implicit weights in that input layout.
    pub input_count: usize,
    /// Number of canonical positive finite tuple weights and square roots.
    pub tuple_count: usize,
    /// Number of positive finite weighted degrees in global factor-block order.
    pub dimension: usize,
    /// Number of finite positive component-extrema records.
    pub component_count: usize,
}

/// Caller-declared live requested-array payload budget for frame construction.
///
/// This is not an allocator quota or process-RSS limit. See
/// [`ThreeWayWeightFrame::setup_payload_report`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightFramePayloadBudget {
    /// Maximum sum of the categories in the setup report.
    pub maximum_payload_bytes: usize,
    /// Other live array payload, for example an old frame retained during rebuild.
    /// Exclude the topology and input slice already counted by the report.
    pub additional_live_payload_bytes: usize,
}

/// Checked construction-payload inventory, not a numerical validation certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightFrameSetupReport {
    /// Actual retained payload of the borrowed prepared topology, counted once.
    pub topology_payload_bytes: usize,
    /// Submitted slice length in bytes; implicit unit inputs contribute zero.
    pub input_payload_bytes: usize,
    /// Requested arrays for the new frame plus its degree-correction scratch.
    pub new_arrays_payload_bytes: usize,
    /// Additional storage explicitly declared by the caller.
    pub additional_live_payload_bytes: usize,
    /// Checked sum of the preceding four categories.
    pub total_payload_bytes: usize,
}

/// One immutable numerical generation borrowing an exact prepared topology.
///
/// Construction does not sort tuples, rediscover components or copy symbolic
/// topology. It owns canonical weights, square roots, weighted degrees and
/// component extrema. All are finite and strictly positive when published.
/// There is no owning `Clone` or mutation API. Rebuilding, even with identical
/// values, creates a distinct numerical owner; it does not invalidate a still
/// live old frame, whose downstream objects must continue to name that frame.
///
/// No operator, numerical hierarchy, rank certificate, norm bound, inverse degree
/// or map-quality admission is supplied by this type. Existing solvers are not
/// retroactively generation-checked. Getter slices are read-only inspection views,
/// not evidence that a downstream factorization was built for this generation.
#[derive(Debug)]
pub struct ThreeWayWeightFrame<'topology> {
    topology: &'topology PreparedThreeWayTopology,
    weights: Vec<f64>,
    square_root_weights: Vec<f64>,
    diagonal: Vec<f64>,
    component_ranges: Vec<ComponentWeightRange>,
    validation: WeightFrameValidationReport,
}

/// Borrowed identity of one immutable numerical frame, not just its topology.
///
/// There is no hash shortcut, global counter, hidden allocation or serialized ID.
/// Copies allocate nothing. The binding cannot outlive its frame:
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
/// let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
/// let token = {
///     let frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
///     frame.binding()
/// };
/// assert_eq!(token, token);
/// ```
/// Nor can a frame be replaced while its binding remains in use:
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
/// let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
/// let mut frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
/// let token = frame.binding();
/// frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[2.0])).unwrap();
/// token.validate_for(&frame).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct WeightFrameBinding<'frame, 'topology> {
    owner: &'frame ThreeWayWeightFrame<'topology>,
}

impl PartialEq for WeightFrameBinding<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.owner, other.owner)
    }
}
impl Eq for WeightFrameBinding<'_, '_> {}

impl WeightFrameBinding<'_, '_> {
    /// Exact frame identity, even when another frame has identical weights/topology.
    #[must_use]
    pub fn is_bound_to(self, frame: &ThreeWayWeightFrame<'_>) -> bool {
        core::ptr::eq(self.owner, frame)
    }

    /// Reject a different numerical owner without allocating or mutating anything.
    pub fn validate_for(self, frame: &ThreeWayWeightFrame<'_>) -> Result<(), IncidenceError> {
        if !self.is_bound_to(frame) {
            return Err(IncidenceError::WeightFrameBindingMismatch);
        }
        Ok(())
    }
}

impl<'topology> ThreeWayWeightFrame<'topology> {
    /// Build and validate a new immutable numerical generation.
    ///
    /// Observation inputs must use the prepared original-row order; tuple inputs
    /// must use its canonical unique-tuple order. A numerical slice alone cannot
    /// reveal reordered observations or changed sample semantics. Callers remain
    /// responsible for those identities and for rebuilding changed topology.
    pub fn try_new(
        topology: &'topology PreparedThreeWayTopology,
        input: WeightFrameInput<'_>,
    ) -> Result<Self, IncidenceError> {
        Self::try_new_with_budget(topology, input, WeightFramePayloadBudget {
            maximum_payload_bytes: usize::MAX,
            additional_live_payload_bytes: 0,
        })
    }

    /// Build after checking the live requested-array budget, before any reservation.
    ///
    /// Invalid input, a failed reservation or numerical overflow publishes no
    /// partial frame and leaves the topology, submitted values and old frames alone.
    pub fn try_new_with_budget(
        topology: &'topology PreparedThreeWayTopology,
        input: WeightFrameInput<'_>,
        budget: WeightFramePayloadBudget,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(topology, input, budget, &mut |_| Ok(()))
    }

    /// Report requested construction-array payload together with declared live state.
    ///
    /// Checks layout and integer arithmetic, but not numerical input values. The
    /// new arrays include both retained numerical state and temporary degree
    /// corrections. Counts the borrowed topology once, the supplied slice by length,
    /// and caller-declared other live storage. Add unused input-vector capacity and
    /// old frames there as needed. A slice aliasing an already charged object is
    /// conservatively double-counted; no general alias deduplication is attempted.
    /// Excludes inline objects, allocator headers/rounding/excess capacity and stack.
    /// No existing hierarchy/PCG payload report is silently extended by this API.
    pub fn setup_payload_report(
        topology: &PreparedThreeWayTopology,
        input: WeightFrameInput<'_>,
        additional_live_payload_bytes: usize,
    ) -> Result<WeightFrameSetupReport, IncidenceError> {
        input.validate_layout(topology)?;
        let tuples = topology.topology().tuple_count();
        let dimension = topology.topology().total_levels();
        let new_arrays_payload_bytes = sum_bytes(&[
            array_bytes::<f64>(tuples)?,
            array_bytes::<f64>(tuples)?,
            array_bytes::<f64>(dimension)?,
            array_bytes::<f64>(dimension)?,
            array_bytes::<ComponentWeightRange>(topology.component_factor_sizes().len())?,
        ])?;
        let topology_payload_bytes = topology.retained_payload_bytes()?;
        let input_payload_bytes = input.values().map_or(0, core::mem::size_of_val);
        let total_payload_bytes = sum_bytes(&[
            topology_payload_bytes,
            input_payload_bytes,
            new_arrays_payload_bytes,
            additional_live_payload_bytes,
        ])?;
        Ok(WeightFrameSetupReport {
            topology_payload_bytes,
            input_payload_bytes,
            new_arrays_payload_bytes,
            additional_live_payload_bytes,
            total_payload_bytes,
        })
    }

    fn build_with<F>(
        topology: &'topology PreparedThreeWayTopology,
        input: WeightFrameInput<'_>,
        budget: WeightFramePayloadBudget,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let report = Self::setup_payload_report(topology, input, budget.additional_live_payload_bytes)?;
        if report.total_payload_bytes > budget.maximum_payload_bytes {
            return Err(IncidenceError::WeightFrameBudgetExceeded {
                required: report.total_payload_bytes,
                budget: budget.maximum_payload_bytes,
            });
        }
        if let Some(values) = input.values() {
            // Validate in submitted order so the reported index names the input row.
            for (tuple_index, &weight) in values.iter().enumerate() {
                if !weight.is_finite() || weight <= 0.0 {
                    return Err(IncidenceError::InvalidWeight { tuple_index, weight });
                }
            }
        }
        let shape = topology.topology();
        let count = shape.tuple_count();
        let mut weights = reserve_frame(count, "frame tuple weights", before)?;
        if input.is_observations() {
            let groups = topology.observation_groups().ok_or(IncidenceError::WeightFrameObservationLayoutRequired)?;
            for (index, range) in groups.offsets().windows(2).enumerate() {
                let mut sum = CompensatedSum::default();
                for &row in &groups.grouped_observations()[range[0]..range[1]] {
                    sum.add(input.value(row));
                }
                let weight = sum.total();
                if !weight.is_finite() || weight <= 0.0 {
                    return Err(IncidenceError::InvalidCollapsedWeight { tuple: shape.tuples()[index], weight });
                }
                weights.push(weight);
            }
        } else {
            for index in 0..count {
                weights.push(input.value(index));
            }
        }
        let mut square_root_weights = reserve_frame(count, "frame square roots", before)?;
        for (index, &weight) in weights.iter().enumerate() {
            let value = weight.sqrt();
            if !value.is_finite() || value <= 0.0 {
                return Err(IncidenceError::InvalidWeightFrameDerivedValue { context: "square root", index, value });
            }
            square_root_weights.push(value);
        }
        let dimension = shape.total_levels();
        let mut diagonal = reserve_frame(dimension, "frame weighted degrees", before)?;
        diagonal.resize(dimension, 0.0);
        let mut correction = reserve_frame(dimension, "frame degree corrections", before)?;
        correction.resize(dimension, 0.0);
        fill_weighted_degrees(shape, &weights, &mut diagonal, &mut correction);
        drop(correction);
        for factor in 0..3 {
            for (level, &value) in diagonal[shape.factor_range(factor)].iter().enumerate() {
                if !value.is_finite() || value <= 0.0 {
                    return Err(IncidenceError::InvalidWeightedDegree { factor, level, value });
                }
            }
        }
        let component_count = topology.component_factor_sizes().len();
        let mut ranges = reserve_frame(component_count, "frame component ranges", before)?;
        ranges.resize(component_count, ComponentWeightRange::EMPTY);
        for (&tuple, &weight) in shape.tuples().iter().zip(&weights) {
            let component = topology.component_labels()[shape.global_index(0, tuple[0])];
            let range = &mut ranges[component];
            range.minimum_tuple_weight = range.minimum_tuple_weight.min(weight);
            range.maximum_tuple_weight = range.maximum_tuple_weight.max(weight);
        }
        for (&component, &value) in topology.component_labels().iter().zip(&diagonal) {
            let range = &mut ranges[component];
            range.minimum_degree = range.minimum_degree.min(value);
            range.maximum_degree = range.maximum_degree.max(value);
        }
        for (index, range) in ranges.iter().enumerate() {
            for value in [range.minimum_tuple_weight, range.maximum_tuple_weight, range.minimum_degree, range.maximum_degree] {
                if !value.is_finite() || value <= 0.0 {
                    return Err(IncidenceError::InvalidWeightFrameDerivedValue { context: "component range", index, value });
                }
            }
        }
        let frame = Self {
            topology,
            weights,
            square_root_weights,
            diagonal,
            component_ranges: ranges,
            validation: WeightFrameValidationReport {
                input_kind: input.kind(),
                input_count: input.validate_layout(topology)?,
                tuple_count: count,
                dimension,
                component_count,
            },
        };
        frame.retained_payload_bytes()?;
        Ok(frame)
    }

    /// Exact borrowed symbolic owner; never implicitly cloned or rebuilt.
    #[must_use]
    pub const fn topology(&self) -> &'topology PreparedThreeWayTopology { self.topology }

    /// Identity of the symbolic owner, distinct from this frame's numerical binding.
    #[must_use]
    pub fn topology_binding(&self) -> PreparedTopologyBinding<'topology> { self.topology.binding() }

    /// Borrow this exact immutable numerical generation without allocation.
    #[must_use]
    pub const fn binding(&self) -> WeightFrameBinding<'_, 'topology> { WeightFrameBinding { owner: self } }

    /// Reject a different prepared topology owner, even if its contents match.
    pub fn validate_for(&self, topology: &PreparedThreeWayTopology) -> Result<(), IncidenceError> {
        self.topology.binding().validate_for(topology)
    }

    /// Positive finite weights in canonical unique-tuple order.
    #[must_use]
    pub fn weights(&self) -> &[f64] { &self.weights }

    /// Positive finite square roots in canonical unique-tuple order.
    #[must_use]
    pub fn square_root_weights(&self) -> &[f64] { &self.square_root_weights }

    /// Positive finite weighted degrees in global factor-block order.
    #[must_use]
    pub fn diagonal(&self) -> &[f64] { &self.diagonal }

    /// Component extrema, in the exact symbolic owner's component order.
    #[must_use]
    pub fn component_ranges(&self) -> &[ComponentWeightRange] { &self.component_ranges }

    /// Validated counts and input interpretation; not an outer-solver certificate.
    #[must_use]
    pub const fn validation_report(&self) -> WeightFrameValidationReport { self.validation }

    /// Copy weights after exact topology/frame identity and output-length checks.
    ///
    /// Rejection leaves output unchanged. Copies allocate nothing; the caller
    /// supplies storage. This does not validate any downstream numerical factors.
    pub fn copy_weights_into(
        &self,
        topology: &PreparedThreeWayTopology,
        binding: WeightFrameBinding<'_, '_>,
        output: &mut [f64],
    ) -> Result<(), IncidenceError> {
        self.validate_for(topology)?;
        binding.validate_for(self)?;
        if output.len() != self.weights.len() {
            return Err(crate::error::dimension("frame weight copy", self.weights.len(), output.len()));
        }
        output.copy_from_slice(&self.weights);
        Ok(())
    }

    /// Actual exclusive retained array capacities, excluding the borrowed topology.
    ///
    /// Unit inputs avoid allocating an input ones-vector, but retained weights and
    /// roots are materialized in this first implementation. No hidden identity
    /// allocation is owned. Inline objects, allocator overhead and setup scratch
    /// are excluded; old/new frame overlap must charge both exclusive reports.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            array_bytes::<f64>(self.weights.capacity())?,
            array_bytes::<f64>(self.square_root_weights.capacity())?,
            array_bytes::<f64>(self.diagonal.capacity())?,
            array_bytes::<ComponentWeightRange>(self.component_ranges.capacity())?,
        ])
    }
}

fn reserve_frame<T, F>(count: usize, context: &'static str, before: &mut F) -> Result<Vec<T>, IncidenceError>
where F: FnMut(&'static str) -> Result<(), IncidenceError>,
{
    reserve(count, context, before).map_err(|error| match error {
        IncidenceError::TopologyAllocation { context } => IncidenceError::WeightFrameAllocation { context },
        other => other,
    })
}

#[cfg(test)]
mod failure_tests;
