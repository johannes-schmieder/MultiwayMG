//! Error types for incidence construction and operator application.

use thiserror::Error;

/// Failure while constructing or applying a weighted incidence problem.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum IncidenceError {
    /// A factor had no levels.
    #[error("factor {factor} must have at least one level")]
    EmptyFactor { factor: usize },
    /// A factor's level count cannot be represented by the compact index type.
    #[error("factor {factor} level count {count} exceeds the supported u32 range")]
    LevelCountTooWide { factor: usize, count: usize },
    /// Tuple and weight vectors had different lengths.
    #[error("tuple count {tuples} does not match weight count {weights}")]
    WeightLengthMismatch { tuples: usize, weights: usize },
    /// A tuple referenced a level outside its factor.
    #[error(
        "tuple {tuple_index} factor {factor} level {level} is outside level count {level_count}"
    )]
    TupleOutOfBounds {
        tuple_index: usize,
        factor: usize,
        level: u32,
        level_count: usize,
    },
    /// A submitted tuple weight was not finite and strictly positive.
    #[error("tuple {tuple_index} weight must be finite and positive, got {weight}")]
    InvalidWeight { tuple_index: usize, weight: f64 },
    /// Duplicate aggregation produced a non-finite weight.
    #[error("collapsed tuple {tuple:?} has invalid accumulated weight {weight}")]
    InvalidCollapsedWeight { tuple: [u32; 3], weight: f64 },
    /// The problem had no positive-weight tuples.
    #[error("a three-way incidence problem must contain at least one tuple")]
    EmptyProblem,
    /// A declared level never appeared in a positive-weight tuple.
    #[error("factor {factor} level {level} is unused")]
    UnusedLevel { factor: usize, level: usize },
    /// An input or output vector had the wrong dimension.
    #[error("{context}: expected length {expected}, got {actual}")]
    DimensionMismatch {
        context: &'static str,
        expected: usize,
        actual: usize,
    },
    /// An aggregation parent vector did not cover every fine level.
    #[error("factor {factor} parent count {actual} does not match fine level count {expected}")]
    ParentLengthMismatch {
        factor: usize,
        expected: usize,
        actual: usize,
    },
    /// An aggregation parent index was invalid.
    #[error("factor {factor} parent index {parent} is invalid")]
    InvalidParent { factor: usize, parent: u32 },
    /// Aggregation labels skipped an intermediate parent.
    #[error("factor {factor} aggregation has an empty coarse level {parent}")]
    EmptyAggregate { factor: usize, parent: usize },
    /// Checked dimension arithmetic overflowed.
    #[error("dimension arithmetic overflowed at {context}")]
    DimensionOverflow { context: &'static str },
}

pub(crate) fn dimension(context: &'static str, expected: usize, actual: usize) -> IncidenceError {
    IncidenceError::DimensionMismatch {
        context,
        expected,
        actual,
    }
}
