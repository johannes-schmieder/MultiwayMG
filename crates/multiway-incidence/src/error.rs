//! Error types for incidence construction and operator application.

use thiserror::Error;

/// Failure while constructing or applying a weighted incidence problem.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum IncidenceError {
    /// A component index is outside the exact structural partition.
    #[error("component index {component} is outside component count {count}")]
    ComponentIndexOutOfBounds {
        /// Submitted zero-based component ID.
        component: usize,
        /// Number of components in the exact owner.
        count: usize,
    },
    /// A proposed hierarchy exceeds an explicitly declared structural limit.
    #[error("{context}: structural size {actual} exceeds limit {maximum}")]
    HierarchyStructureLimit {
        /// Structural quantity being limited.
        context: &'static str,
        /// Checked proposed size.
        actual: usize,
        /// Caller-declared inclusive limit.
        maximum: usize,
    },
    /// A numerical hierarchy names another supplied-map structural owner.
    #[error("prepared hierarchy binding belongs to a different owner")]
    HierarchyBindingMismatch,
    /// Complete structural/replay live payload exceeds its declared budget.
    #[error("prepared hierarchy requires {required} payload bytes, budget is {budget}")]
    HierarchyBudgetExceeded {
        /// Checked live array payload, not process memory.
        required: usize,
        /// Declared maximum live array payload.
        budget: usize,
    },
    /// A replay was supplied with a different symbolic map owner.
    #[error("numerical replay belongs to a different symbolic map owner")]
    WeightReplayMapMismatch,
    /// Declared live requested-array payload is too small for replay construction.
    #[error("weight replay setup requires {required} payload bytes, budget is {budget}")]
    WeightReplayBudgetExceeded {
        /// Checked direct-owner plus requested-new-array payload.
        required: usize,
        /// Caller-declared payload limit, not an allocator quota.
        budget: usize,
    },
    /// Observation weights require a prepared source with original row groups.
    #[error("observation weight input requires an observation-prepared topology")]
    WeightFrameObservationLayoutRequired,
    /// A numerical token names another immutable weight frame.
    #[error("weight binding belongs to a different numerical frame")]
    WeightFrameBindingMismatch,
    /// Checked frame-array reservation failed.
    #[error("weight frame array allocation failed in {context}")]
    WeightFrameAllocation {
        /// Array reservation boundary.
        context: &'static str,
    },
    /// The declared live requested-array payload budget is insufficient.
    #[error("weight frame setup requires {required} payload bytes, budget is {budget}")]
    WeightFrameBudgetExceeded {
        /// Sum of requested new arrays and declared live payload, not process memory.
        required: usize,
        /// Caller-declared payload budget.
        budget: usize,
    },
    /// A weighted degree is not representable as a finite strictly positive value.
    #[error("factor {factor} level {level} has invalid weighted degree {value}")]
    InvalidWeightedDegree {
        /// Zero-based factor index.
        factor: usize,
        /// Factor-local level.
        level: usize,
        /// Rejected accumulated value.
        value: f64,
    },
    /// Another derived frame value is not finite and strictly positive.
    #[error("invalid weight frame {context} at index {index}: {value}")]
    InvalidWeightFrameDerivedValue {
        /// Derived quantity being checked.
        context: &'static str,
        /// Zero-based tuple or component index, as named by context.
        index: usize,
        /// Rejected numerical value.
        value: f64,
    },
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
    #[error(
        "{context}: symbolic setup requests at most {required} payload bytes, budget is {budget}"
    )]
    SymbolicSetupBudgetExceeded {
        /// Symbolic construction boundary.
        context: &'static str,
        /// Conservative requested-array upper bound, not allocator or OS memory.
        required: usize,
        /// Declared additional requested-array payload budget.
        budget: usize,
    },
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
    /// A factor had no levels.
    #[error("factor {factor} must have at least one level")]
    EmptyFactor {
        /// Zero-based factor index.
        factor: usize,
    },
    /// A factor's level count cannot be represented by the compact index type.
    #[error("factor {factor} level count {count} exceeds the supported u32 range")]
    LevelCountTooWide {
        /// Zero-based factor index.
        factor: usize,
        /// Rejected level count.
        count: usize,
    },
    /// Tuple and weight vectors had different lengths.
    #[error("tuple count {tuples} does not match weight count {weights}")]
    WeightLengthMismatch {
        /// Number of submitted tuples.
        tuples: usize,
        /// Number of submitted weights.
        weights: usize,
    },
    /// A tuple referenced a level outside its factor.
    #[error(
        "tuple {tuple_index} factor {factor} level {level} is outside level count {level_count}"
    )]
    TupleOutOfBounds {
        /// Zero-based tuple index.
        tuple_index: usize,
        /// Zero-based factor index.
        factor: usize,
        /// Rejected factor-local level.
        level: u32,
        /// Declared number of levels in the factor.
        level_count: usize,
    },
    /// A submitted tuple weight was not finite and strictly positive.
    #[error("tuple {tuple_index} weight must be finite and positive, got {weight}")]
    InvalidWeight {
        /// Zero-based tuple index.
        tuple_index: usize,
        /// Rejected weight.
        weight: f64,
    },
    /// Duplicate aggregation produced a non-finite weight.
    #[error("collapsed tuple {tuple:?} has invalid accumulated weight {weight}")]
    InvalidCollapsedWeight {
        /// Collapsed tuple key.
        tuple: [u32; 3],
        /// Rejected accumulated weight.
        weight: f64,
    },
    /// The problem had no positive-weight tuples.
    #[error("a three-way incidence problem must contain at least one tuple")]
    EmptyProblem,
    /// A declared level never appeared in a positive-weight tuple.
    #[error("factor {factor} level {level} is unused")]
    UnusedLevel {
        /// Zero-based factor index.
        factor: usize,
        /// Zero-based factor-local level.
        level: usize,
    },
    /// An input or output vector had the wrong dimension.
    #[error("{context}: expected length {expected}, got {actual}")]
    DimensionMismatch {
        /// Operation that detected the mismatch.
        context: &'static str,
        /// Required vector length.
        expected: usize,
        /// Submitted vector length.
        actual: usize,
    },
    /// Fallible projection scratch reservation failed.
    #[error("workspace allocation failed in {context}")]
    WorkspaceAllocation {
        /// Setup operation whose reservation failed.
        context: &'static str,
    },
    /// Projection scratch belongs to a different component decomposition.
    #[error("{context}: workspace belongs to a different component decomposition")]
    WorkspaceBindingMismatch {
        /// Operation that rejected the incompatible workspace.
        context: &'static str,
    },
    /// An aggregation parent vector did not cover every fine level.
    #[error("factor {factor} parent count {actual} does not match fine level count {expected}")]
    ParentLengthMismatch {
        /// Zero-based factor index.
        factor: usize,
        /// Number of fine levels requiring parents.
        expected: usize,
        /// Number of submitted parent labels.
        actual: usize,
    },
    /// An aggregation parent index was invalid.
    #[error("factor {factor} parent index {parent} is invalid")]
    InvalidParent {
        /// Zero-based factor index.
        factor: usize,
        /// Rejected parent label.
        parent: u32,
    },
    /// Aggregation labels skipped an intermediate parent.
    #[error("factor {factor} aggregation has an empty coarse level {parent}")]
    EmptyAggregate {
        /// Zero-based factor index.
        factor: usize,
        /// Missing zero-based coarse parent label.
        parent: usize,
    },
    /// Checked dimension arithmetic overflowed.
    #[error("dimension arithmetic overflowed at {context}")]
    DimensionOverflow {
        /// Operation whose checked arithmetic overflowed.
        context: &'static str,
    },
}

pub(crate) fn dimension(context: &'static str, expected: usize, actual: usize) -> IncidenceError {
    IncidenceError::DimensionMismatch {
        context,
        expected,
        actual,
    }
}
