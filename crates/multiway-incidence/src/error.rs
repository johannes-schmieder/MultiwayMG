//! Error types for incidence construction and operator application.

use thiserror::Error;

/// Failure while constructing or applying a weighted incidence problem.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum IncidenceError {
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
