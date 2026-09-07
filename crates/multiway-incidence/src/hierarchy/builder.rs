//! Append-only unpublished structural preparation with explicit limits.
use super::{PreparedHierarchyBudget, PreparedHierarchyTopology, Transition, admit};
use crate::{
    FactorAggregation, IncidenceError, PreparedCoarseTupleMap, PreparedThreeWayTopology,
    construction::{array_bytes, reserve, sum_bytes},
};

/// Explicit structural limits; these do not select or certify a numerical policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedHierarchyLimits {
    /// Reserved transition/map descriptor capacity; zero permits only the fine level.
    pub maximum_transitions: usize,
    /// Maximum sum of unique tuple counts over the fine and all accepted levels.
    pub maximum_total_tuples: usize,
    /// Maximum sum of coefficient dimensions over the fine and all accepted levels.
    pub maximum_total_coefficients: usize,
    /// Require each proposed coefficient dimension to be smaller than its source.
    pub require_strict_dimension_reduction: bool,
}

/// One attempted structural extension, including a rejected extension's known sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedHierarchyAppendReport {
    /// Zero-based proposed transition position.
    pub transition: usize,
    /// Coefficient dimension of the accepted source level.
    pub source_coefficients: usize,
    /// Tuples submitted to the structural attempt, not an operator-work count.
    pub source_tuples: usize,
    /// Proposed dimension when checked cardinality arithmetic succeeds.
    pub proposed_coefficients: Option<usize>,
    /// Actual unique coarse tuple count after structural construction succeeds.
    pub coarse_tuples: Option<usize>,
    /// Conservative requested live peak when complete sizing succeeds.
    pub requested_peak_payload_bytes: Option<usize>,
    /// Whether the bounded structural constructor was entered after admission.
    pub structural_build_attempted: bool,
}

/// A failed append releases the candidate and preserves the accepted logical prefix.
#[derive(Debug, thiserror::Error)]
#[error("hierarchy append failed: {source}")]
pub struct PreparedHierarchyAppendFailure {
    /// Original typed structural, capacity or allocation failure.
    pub source: IncidenceError,
    /// Known submitted/result sizes and admission state for this attempt.
    pub report: PreparedHierarchyAppendReport,
}

/// Unpublished owning hierarchy whose descriptors are reserved once.
///
/// No coarse numerical weights, smoother or dense factors are built here. Current
/// level borrows must end before append/finish. Failed attempts preserve accepted
/// levels; admitted attempted peaks still contribute to the construction bound.
/// Finish publishes the ordinary immutable PreparedHierarchyTopology, without
/// reallocating or silently shrinking unused descriptor capacity.
///
/// A borrowed current level excludes append:
/// ```compile_fail
/// use multiway_incidence::{PreparedHierarchyBuilder, PreparedHierarchyLimits,
///     PreparedHierarchyBudget, PreparedThreeWayTopology, FactorAggregation};
/// let t = PreparedThreeWayTopology::try_from_collapsed([2;3], &[[0;3],[1;3]]).unwrap();
/// let limits = PreparedHierarchyLimits { maximum_transitions: 2,
///     maximum_total_tuples: 10, maximum_total_coefficients: 20,
///     require_strict_dimension_reduction: false };
/// let mut b = PreparedHierarchyBuilder::try_new(&t, limits, PreparedHierarchyBudget::UNLIMITED).unwrap();
/// let level = b.current_level();
/// b.try_append(FactorAggregation::identity([2;3]).unwrap(), PreparedHierarchyBudget::UNLIMITED).unwrap();
/// assert_eq!(level.topology().tuple_count(), 2);
/// ```
/// A current-level frame likewise excludes publishing the hierarchy:
/// ```compile_fail
/// use multiway_incidence::{PreparedHierarchyBuilder, PreparedHierarchyLimits,
///     PreparedHierarchyBudget, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
/// let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let limits = PreparedHierarchyLimits { maximum_transitions: 0,
///     maximum_total_tuples: 1, maximum_total_coefficients: 3,
///     require_strict_dimension_reduction: true };
/// let b = PreparedHierarchyBuilder::try_new(&t, limits, PreparedHierarchyBudget::UNLIMITED).unwrap();
/// let frame = ThreeWayWeightFrame::try_new(b.current_level(), WeightFrameInput::UnitTuples).unwrap();
/// let finished = b.finish();
/// assert_eq!(frame.weights(), &[1.0]);
/// ```
#[derive(Debug)]
pub struct PreparedHierarchyBuilder<'fine> {
    hierarchy: PreparedHierarchyTopology<'fine>,
    limits: PreparedHierarchyLimits,
    total_tuples: usize,
    total_coefficients: usize,
}
impl<'fine> PreparedHierarchyBuilder<'fine> {
    /// Size initial descriptor reservation and validate the fine structural limits.
    pub fn initial_payload_bound(
        fine: &PreparedThreeWayTopology,
        limits: PreparedHierarchyLimits,
        budget: PreparedHierarchyBudget,
    ) -> Result<usize, IncidenceError> {
        limit(
            "total hierarchy tuples",
            fine.topology().tuple_count(),
            limits.maximum_total_tuples,
        )?;
        limit(
            "total hierarchy coefficients",
            fine.topology().total_levels(),
            limits.maximum_total_coefficients,
        )?;
        sum_bytes(&[
            fine.retained_payload_bytes()?,
            budget.additional_live_payload_bytes,
            array_bytes::<FactorAggregation>(limits.maximum_transitions)?,
            array_bytes::<Transition>(limits.maximum_transitions)?,
        ])
    }
    /// Reserve all map/transition descriptors after complete initial admission.
    pub fn try_new(
        fine: &'fine PreparedThreeWayTopology,
        limits: PreparedHierarchyLimits,
        budget: PreparedHierarchyBudget,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(fine, limits, budget, &mut |_| Ok(()))
    }
    fn build_with<F>(
        fine: &'fine PreparedThreeWayTopology,
        limits: PreparedHierarchyLimits,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let required = Self::initial_payload_bound(fine, limits, budget)?;
        admit(required, budget)?;
        let aggregations = reserve(
            limits.maximum_transitions,
            "builder aggregation descriptors",
            before,
        )?;
        let transitions = reserve(
            limits.maximum_transitions,
            "builder transition descriptors",
            before,
        )?;
        let hierarchy = PreparedHierarchyTopology {
            fine,
            aggregations,
            transitions,
            setup_peak_payload_bound: required,
        };
        let actual = sum_bytes(&[
            fine.retained_payload_bytes()?,
            hierarchy.retained_payload_bytes()?,
            budget.additional_live_payload_bytes,
        ])?;
        admit(actual, budget)?;
        Ok(Self {
            hierarchy: PreparedHierarchyTopology {
                setup_peak_payload_bound: required.max(actual),
                ..hierarchy
            },
            limits,
            total_tuples: fine.topology().tuple_count(),
            total_coefficients: fine.topology().total_levels(),
        })
    }
    /// Last accepted structural level, initially the borrowed fine owner.
    pub fn current_level(&self) -> &PreparedThreeWayTopology {
        self.hierarchy
            .level(self.hierarchy.level_count() - 1)
            .expect("nonempty hierarchy")
    }
    /// Borrow one accepted level; no partially constructed level is exposed.
    pub fn level(&self, index: usize) -> Option<&PreparedThreeWayTopology> {
        self.hierarchy.level(index)
    }
    /// Number of accepted levels including the fine level.
    pub fn level_count(&self) -> usize {
        self.hierarchy.level_count()
    }
    /// Current sum of fine/coarse unique tuple counts, excluding rejected attempts.
    pub const fn total_tuple_count(&self) -> usize {
        self.total_tuples
    }
    /// Current sum of fine/coarse dimensions, excluding rejected attempts.
    pub const fn total_coefficient_count(&self) -> usize {
        self.total_coefficients
    }
    /// Fixed structural limits, with no implicit timing-dependent choices.
    pub const fn limits(&self) -> PreparedHierarchyLimits {
        self.limits
    }
    /// Actual exclusive retained arrays, including all reserved descriptor capacity.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        self.hierarchy.retained_payload_bytes()
    }
    /// Largest admitted conservative bound, including failed structural attempts.
    pub const fn setup_peak_payload_bound(&self) -> usize {
        self.hierarchy.setup_peak_payload_bound
    }

    /// Consume and append one map only after structural and live-payload admission.
    ///
    /// Other live state is declared at every call, including provisional/current
    /// numerical frames or old solver generations. The new map's actual parent
    /// capacities are counted here; do not count them again in additional state.
    /// A tuple-complexity rejection follows bounded structural construction but
    /// precedes any numerical setup. Returned sizes are not measured CPU/RSS costs.
    pub fn try_append(
        &mut self,
        aggregation: FactorAggregation,
        budget: PreparedHierarchyBudget,
    ) -> Result<PreparedHierarchyAppendReport, PreparedHierarchyAppendFailure> {
        self.append_with(aggregation, budget, &mut |_| Ok(()))
    }
    fn append_with<F>(
        &mut self,
        aggregation: FactorAggregation,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<PreparedHierarchyAppendReport, PreparedHierarchyAppendFailure>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let source = self.current_level();
        let mut report = PreparedHierarchyAppendReport {
            transition: self.level_count() - 1,
            source_coefficients: source.topology().total_levels(),
            source_tuples: source.topology().tuple_count(),
            proposed_coefficients: aggregation
                .coarse_counts()
                .iter()
                .try_fold(0usize, |sum, &n| sum.checked_add(n)),
            coarse_tuples: None,
            requested_peak_payload_bytes: None,
            structural_build_attempted: false,
        };
        let result = (|| {
            limit(
                "hierarchy transitions",
                add(report.transition, 1)?,
                self.limits.maximum_transitions,
            )?;
            if self.current_level().topology().level_counts() != aggregation.fine_counts() {
                return Err(IncidenceError::TopologyLayoutMismatch);
            }
            let dimension = report.proposed_coefficients.ok_or_else(overflow)?;
            if self.limits.require_strict_dimension_reduction {
                limit(
                    "coarse coefficient dimension",
                    dimension,
                    report.source_coefficients - 1,
                )?;
            }
            let total_coefficients = add(self.total_coefficients, dimension)?;
            limit(
                "total hierarchy coefficients",
                total_coefficients,
                self.limits.maximum_total_coefficients,
            )?;
            let new_bound =
                PreparedCoarseTupleMap::setup_payload_bound(self.current_level(), &aggregation)?;
            let required = sum_bytes(&[
                self.hierarchy.fine.retained_payload_bytes()?,
                self.retained_payload_bytes()?,
                aggregation.retained_payload_bytes()?,
                budget.additional_live_payload_bytes,
                new_bound,
            ])?;
            report.requested_peak_payload_bytes = Some(required);
            admit(required, budget)?;
            // Attempted, released construction is still part of complete setup.
            self.hierarchy.setup_peak_payload_bound =
                self.hierarchy.setup_peak_payload_bound.max(required);
            report.structural_build_attempted = true;
            let transition = PreparedCoarseTupleMap::build_with(
                self.current_level(),
                &aggregation,
                new_bound,
                before,
            )?
            .into_hierarchy_parts();
            let count = transition.coarse.topology().tuple_count();
            report.coarse_tuples = Some(count);
            let total_tuples = add(self.total_tuples, count)?;
            limit(
                "total hierarchy tuples",
                total_tuples,
                self.limits.maximum_total_tuples,
            )?;
            let retained = sum_bytes(&[
                self.hierarchy.fine.retained_payload_bytes()?,
                self.retained_payload_bytes()?,
                aggregation.retained_payload_bytes()?,
                transition.retained_payload_bytes()?,
                budget.additional_live_payload_bytes,
            ])?;
            admit(retained, budget)?;
            self.hierarchy.setup_peak_payload_bound =
                self.hierarchy.setup_peak_payload_bound.max(retained);
            debug_assert!(
                self.hierarchy.aggregations.len() < self.hierarchy.aggregations.capacity()
            );
            debug_assert!(self.hierarchy.transitions.len() < self.hierarchy.transitions.capacity());
            self.hierarchy.aggregations.push(aggregation);
            self.hierarchy.transitions.push(transition);
            self.total_tuples = total_tuples;
            self.total_coefficients = total_coefficients;
            Ok(report)
        })();
        result.map_err(|source| PreparedHierarchyAppendFailure { source, report })
    }
    /// Publish accepted structure without allocating or shrinking descriptors.
    pub fn finish(self) -> PreparedHierarchyTopology<'fine> {
        self.hierarchy
    }
}
fn overflow() -> IncidenceError {
    IncidenceError::DimensionOverflow {
        context: "incremental hierarchy",
    }
}
fn add(a: usize, b: usize) -> Result<usize, IncidenceError> {
    a.checked_add(b).ok_or_else(overflow)
}
fn limit(context: &'static str, actual: usize, maximum: usize) -> Result<(), IncidenceError> {
    if actual > maximum {
        Err(IncidenceError::HierarchyStructureLimit {
            context,
            actual,
            maximum,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
