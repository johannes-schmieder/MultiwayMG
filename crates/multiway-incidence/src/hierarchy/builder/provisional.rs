//! Short-lived one-transition numerical state for structural map selection.
use super::PreparedHierarchyBuilder;
use crate::{
    IncidenceError, PreparedHierarchyBudget, ThreeWayWeightFrame, WeightFrameInput,
    WeightFrameInputKind,
    construction::{array_bytes, sum_bytes},
    weight_replay::reduce_groups,
};

/// Source numerical input for a provisional transition, not a replay certificate.
#[derive(Debug)]
pub enum ProvisionalWeightInput<'frame, 'topology> {
    /// Borrow a complete immutable frame; validate its exact source topology and
    /// charge its exclusive arrays throughout reduction and finishing.
    Frame(&'frame ThreeWayWeightFrame<'topology>),
    /// Consume raw positive tuple weights in the previous level's canonical order.
    /// Charge actual capacity, then release it immediately after reduction.
    Owned(Vec<f64>),
}
impl ProvisionalWeightInput<'_, '_> {
    fn weights(&self) -> &[f64] {
        match self {
            Self::Frame(f) => f.weights(),
            Self::Owned(w) => w,
        }
    }
    fn payload(&self) -> Result<usize, IncidenceError> {
        match self {
            Self::Frame(f) => f.retained_payload_bytes(),
            Self::Owned(w) => array_bytes::<f64>(w.capacity()),
        }
    }
    fn borrowed(&self) -> bool {
        matches!(self, Self::Frame(_))
    }
}
/// Distinct requested peaks for reduction and finishing; not measured RSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedProvisionalSetup {
    /// Previous accepted level supplying canonical weights.
    pub source_level: usize,
    /// Submitted source tuple count; not an exact visited-work counter.
    pub source_tuples: usize,
    /// Exact current-level output tuple count.
    pub output_tuples: usize,
    /// Actual fine topology plus all accepted structural capacities, counted once.
    pub structural_payload_bytes: usize,
    /// Borrowed frame payload or consumed predecessor Vec capacity.
    pub input_payload_bytes: usize,
    /// Other live state declared by the caller.
    pub additional_live_payload_bytes: usize,
    /// Whether the external input frame stays live after reduction.
    pub input_live_during_finishing: bool,
    /// Structural/other/input payload plus requested coarse weights.
    pub reduction_payload_bound: usize,
    /// Structural/other/new-frame/scratch payload plus still-live borrowed input.
    pub finishing_payload_bound: usize,
    /// Maximum of reduction and finishing requested bounds.
    pub total_payload_bound: usize,
}
/// Last entered setup stage, including a failing operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisionalFrameStage {
    /// Checked owner, shape and byte sizing.
    Sizing,
    /// Complete requested payload admission.
    Admission,
    /// Raw owned input numerical validation.
    InputValidation,
    /// Compensated reduction and input/output actual-capacity check.
    Reduction,
    /// Current roots, degrees, corrections and component extrema.
    Finishing,
    /// Final actual retained live-payload check.
    RetainedAdmission,
}
/// Failed unpublished provisional setup; structural owners remain unchanged.
#[derive(Debug, thiserror::Error)]
#[error("provisional frame setup failed: {source}")]
pub struct PreparedProvisionalFailure {
    /// Original typed failure; no allocated error string is required.
    pub source: IncidenceError,
    /// Last entered stage; submitted counts must not be read as exact CPU work.
    pub stage: ProvisionalFrameStage,
    /// Number of submitted input weights, including on early sizing failure.
    pub input_tuples: usize,
    /// Largest admitted requested/actual boundary, when admission succeeded.
    pub admitted_payload_bound: Option<usize>,
}
/// Temporary current-level values used only to choose another structural map.
///
/// Replays the latest accepted transition through existing compensated reduction
/// and frame finishing. The exposed frame borrows the builder; consume this
/// owner into raw weights before another structural append. This does not screen
/// a cycle or certify complete numerical replay from a submitted fine frame.
///
/// A live provisional owner excludes structural mutation:
/// ```compile_fail
/// use multiway_incidence::{PreparedHierarchyBuilder, PreparedHierarchyLimits,
///     PreparedHierarchyBudget, PreparedThreeWayTopology, FactorAggregation,
///     PreparedProvisionalFrame, ProvisionalWeightInput};
/// let t = PreparedThreeWayTopology::try_from_collapsed([2;3], &[[0;3],[1;3]]).unwrap();
/// let budget = PreparedHierarchyBudget::UNLIMITED;
/// let mut b = PreparedHierarchyBuilder::try_new(&t, PreparedHierarchyLimits {
///     maximum_transitions: 2, maximum_total_tuples: 10, maximum_total_coefficients: 20,
///     require_strict_dimension_reduction: false,
/// }, budget).unwrap();
/// b.try_append(FactorAggregation::identity([2;3]).unwrap(), budget).unwrap();
/// let p = PreparedProvisionalFrame::try_replay_last(&b,
///     ProvisionalWeightInput::Owned(vec![1.;2]), budget).unwrap();
/// b.try_append(FactorAggregation::identity([2;3]).unwrap(), budget).unwrap();
/// assert_eq!(p.frame().weights(), &[1.;2]);
/// ```
///
/// Publishing the builder cannot invalidate a provisional frame:
/// ```compile_fail
/// use multiway_incidence::{PreparedHierarchyBuilder, PreparedHierarchyLimits,
///     PreparedHierarchyBudget, PreparedThreeWayTopology, FactorAggregation,
///     PreparedProvisionalFrame, ProvisionalWeightInput};
/// let t = PreparedThreeWayTopology::try_from_collapsed([2;3], &[[0;3],[1;3]]).unwrap();
/// let budget = PreparedHierarchyBudget::UNLIMITED;
/// let mut b = PreparedHierarchyBuilder::try_new(&t, PreparedHierarchyLimits {
///     maximum_transitions: 2, maximum_total_tuples: 10, maximum_total_coefficients: 20,
///     require_strict_dimension_reduction: false,
/// }, budget).unwrap();
/// b.try_append(FactorAggregation::identity([2;3]).unwrap(), budget).unwrap();
/// let p = PreparedProvisionalFrame::try_replay_last(&b,
///     ProvisionalWeightInput::Owned(vec![1.;2]), budget).unwrap();
/// let finished = b.finish();
/// assert_eq!(p.frame().weights(), &[1.;2]);
/// ```
///
/// Consuming into raw weights cannot leave a usable numerical binding:
/// ```compile_fail
/// use multiway_incidence::{PreparedHierarchyBuilder, PreparedHierarchyLimits,
///     PreparedHierarchyBudget, PreparedThreeWayTopology, FactorAggregation,
///     PreparedProvisionalFrame, ProvisionalWeightInput};
/// let t = PreparedThreeWayTopology::try_from_collapsed([2;3], &[[0;3],[1;3]]).unwrap();
/// let budget = PreparedHierarchyBudget::UNLIMITED;
/// let mut b = PreparedHierarchyBuilder::try_new(&t, PreparedHierarchyLimits {
///     maximum_transitions: 2, maximum_total_tuples: 10, maximum_total_coefficients: 20,
///     require_strict_dimension_reduction: false,
/// }, budget).unwrap();
/// b.try_append(FactorAggregation::identity([2;3]).unwrap(), budget).unwrap();
/// let p = PreparedProvisionalFrame::try_replay_last(&b,
///     ProvisionalWeightInput::Owned(vec![1.;2]), budget).unwrap();
/// let binding = p.frame().binding();
/// let weights = p.into_tuple_weights();
/// assert_eq!(binding, binding);
/// ```
#[derive(Debug)]
pub struct PreparedProvisionalFrame<'builder, 'fine> {
    builder: &'builder PreparedHierarchyBuilder<'fine>,
    frame: ThreeWayWeightFrame<'builder>,
    setup: PreparedProvisionalSetup,
    admitted_payload_bound: usize,
}
impl<'builder, 'fine> PreparedProvisionalFrame<'builder, 'fine> {
    /// Size the latest transition and validate input owner/layout before allocation.
    pub fn setup_payload_report(
        builder: &PreparedHierarchyBuilder<'_>,
        input: &ProvisionalWeightInput<'_, '_>,
        budget: PreparedHierarchyBudget,
    ) -> Result<PreparedProvisionalSetup, IncidenceError> {
        let source_level = builder
            .level_count()
            .checked_sub(2)
            .ok_or(IncidenceError::HierarchyTransitionMissing)?;
        let source = builder.level(source_level).expect("accepted predecessor");
        if let ProvisionalWeightInput::Frame(frame) = input {
            frame.validate_for(source)?;
        }
        let source_tuples = source.topology().tuple_count();
        if source_tuples != input.weights().len() {
            return Err(crate::error::dimension(
                "provisional source tuple weights",
                source_tuples,
                input.weights().len(),
            ));
        }
        let current = builder.current_level();
        let output_tuples = current.topology().tuple_count();
        let structural_payload_bytes = sum_bytes(&[
            builder.hierarchy.fine.retained_payload_bytes()?,
            builder.retained_payload_bytes()?,
        ])?;
        let input_payload_bytes = input.payload()?;
        let base = sum_bytes(&[
            structural_payload_bytes,
            budget.additional_live_payload_bytes,
        ])?;
        let weights = array_bytes::<f64>(output_tuples)?;
        let finishing =
            ThreeWayWeightFrame::setup_payload_report(current, WeightFrameInput::UnitTuples, 0)?
                .new_arrays_payload_bytes;
        let reduction_payload_bound = sum_bytes(&[base, input_payload_bytes, weights])?;
        let finishing_payload_bound = sum_bytes(&[
            base,
            finishing,
            if input.borrowed() {
                input_payload_bytes
            } else {
                0
            },
        ])?;
        Ok(PreparedProvisionalSetup {
            source_level,
            source_tuples,
            output_tuples,
            structural_payload_bytes,
            input_payload_bytes,
            additional_live_payload_bytes: budget.additional_live_payload_bytes,
            input_live_during_finishing: input.borrowed(),
            reduction_payload_bound,
            finishing_payload_bound,
            total_payload_bound: reduction_payload_bound.max(finishing_payload_bound),
        })
    }
    /// Replay only the latest transition; consume/release an owned predecessor.
    pub fn try_replay_last(
        builder: &'builder PreparedHierarchyBuilder<'fine>,
        input: ProvisionalWeightInput<'_, '_>,
        budget: PreparedHierarchyBudget,
    ) -> Result<Self, PreparedProvisionalFailure> {
        Self::build_with(builder, input, budget, &mut |_| Ok(()))
    }
    fn build_with<F>(
        builder: &'builder PreparedHierarchyBuilder<'fine>,
        input: ProvisionalWeightInput<'_, '_>,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<Self, PreparedProvisionalFailure>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let input_tuples = input.weights().len();
        let mut stage = ProvisionalFrameStage::Sizing;
        let mut admitted_payload_bound = None;
        let result = (|| {
            let setup = Self::setup_payload_report(builder, &input, budget)?;
            stage = ProvisionalFrameStage::Admission;
            admit(setup.total_payload_bound, budget)?;
            admitted_payload_bound = Some(setup.total_payload_bound);
            stage = ProvisionalFrameStage::InputValidation;
            if let ProvisionalWeightInput::Owned(weights) = &input {
                for (tuple_index, &weight) in weights.iter().enumerate() {
                    if !weight.is_finite() || weight <= 0.0 {
                        return Err(IncidenceError::InvalidWeight {
                            tuple_index,
                            weight,
                        });
                    }
                }
            }
            stage = ProvisionalFrameStage::Reduction;
            let transition = &builder.hierarchy.transitions[setup.source_level];
            let weights = reduce_groups(
                &transition.groups,
                input.weights(),
                "provisional coarse tuple total",
                before,
            )?;
            let actual_reduction = sum_bytes(&[
                setup.structural_payload_bytes,
                setup.input_payload_bytes,
                budget.additional_live_payload_bytes,
                array_bytes::<f64>(weights.capacity())?,
            ])?;
            admit(actual_reduction, budget)?;
            admitted_payload_bound = Some(admitted_payload_bound.unwrap().max(actual_reduction));
            // Owned predecessors are dead before roots/diagonal scratch are built.
            // Dropping a Frame wrapper does not release its external owner.
            drop(input);
            stage = ProvisionalFrameStage::Finishing;
            let frame = ThreeWayWeightFrame::finish_with(
                builder.current_level(),
                weights,
                WeightFrameInputKind::Tuples,
                setup.output_tuples,
                before,
            )?;
            stage = ProvisionalFrameStage::RetainedAdmission;
            let actual = sum_bytes(&[
                setup.structural_payload_bytes,
                frame.retained_payload_bytes()?,
                budget.additional_live_payload_bytes,
                if setup.input_live_during_finishing {
                    setup.input_payload_bytes
                } else {
                    0
                },
            ])?;
            admit(actual, budget)?;
            let peak = admitted_payload_bound.unwrap().max(actual);
            admitted_payload_bound = Some(peak);
            Ok(Self {
                builder,
                frame,
                setup,
                admitted_payload_bound: peak,
            })
        })();
        result.map_err(|source| PreparedProvisionalFailure {
            source,
            stage,
            input_tuples,
            admitted_payload_bound,
        })
    }
    /// Borrow provisional values for map selection only.
    pub const fn frame(&self) -> &ThreeWayWeightFrame<'builder> {
        &self.frame
    }
    /// Complete requested array peaks for this transition's two setup stages.
    pub const fn setup_report(&self) -> PreparedProvisionalSetup {
        self.setup
    }
    /// Largest admitted requested or visible actual-capacity boundary.
    pub const fn admitted_payload_bound(&self) -> usize {
        self.admitted_payload_bound
    }
    /// Actual owned numerical capacities, excluding structural/input owners.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        self.frame.retained_payload_bytes()
    }
    /// Reject a different structural builder owner, even if value-equal.
    pub fn validate_for(
        &self,
        builder: &PreparedHierarchyBuilder<'_>,
    ) -> Result<(), IncidenceError> {
        if core::ptr::eq(self.builder, builder) {
            Ok(())
        } else {
            Err(IncidenceError::HierarchyBindingMismatch)
        }
    }
    /// End all frame bindings and release roots/degrees/extrema, retaining raw
    /// current-level tuple weights for the next provisional transition. The
    /// returned Vec has no numerical-generation or replay certificate attached.
    pub fn into_tuple_weights(self) -> Vec<f64> {
        self.frame.into_tuple_weights()
    }
}
fn admit(required: usize, budget: PreparedHierarchyBudget) -> Result<(), IncidenceError> {
    if required > budget.maximum_payload_bytes {
        Err(IncidenceError::HierarchyBudgetExceeded {
            required,
            budget: budget.maximum_payload_bytes,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "provisional_tests.rs"]
mod tests;
