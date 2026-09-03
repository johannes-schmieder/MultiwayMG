//! Error type for multilevel construction and solves.

use thiserror::Error;

use multiway_incidence::IncidenceError;

/// Failure while building or applying a MultiwayMG solver.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MultiwayError {
    /// Incidence construction or operator application failed.
    #[error(transparent)]
    Incidence(#[from] IncidenceError),
    /// A numerical option was invalid.
    #[error("invalid option {name}: {message}")]
    InvalidOption {
        /// Option name.
        name: &'static str,
        /// Explanation of the rejected value.
        message: String,
    },
    /// Automatic coarsening stopped while the dense terminal remained too large.
    #[error(
        "hierarchy stagnated at dimension {dimension} with {tuples} tuples; dense terminal limit is {limit}"
    )]
    HierarchyStagnated {
        /// Coefficient dimension at the stalled level.
        dimension: usize,
        /// Unique tuple count at the stalled level.
        tuples: usize,
        /// Maximum coefficient dimension admitted to the dense terminal.
        limit: usize,
    },
    /// A supplied oracle aggregation did not match the current hierarchy level.
    #[error("supplied aggregation {level} does not match the current fine level counts")]
    InvalidSuppliedAggregation {
        /// Zero-based hierarchy level.
        level: usize,
    },
    /// The dense terminal detected a materially negative eigenvalue.
    #[error("terminal Gramian has negative eigenvalue {value} below tolerance {tolerance}")]
    NegativeEigenvalue {
        /// Rejected eigenvalue.
        value: f64,
        /// Absolute eigenvalue tolerance.
        tolerance: f64,
    },
    /// Dense research spectral analysis failed.
    #[error("spectral analysis failed: {message}")]
    SpectralAnalysis {
        /// Failure description.
        message: String,
    },
    /// A vector had the wrong dimension.
    #[error("{context}: expected length {expected}, got {actual}")]
    DimensionMismatch {
        /// Operation that detected the mismatch.
        context: &'static str,
        /// Required vector length.
        expected: usize,
        /// Submitted vector length.
        actual: usize,
    },
    /// Preconditioned conjugate gradients encountered a nonpositive metric.
    #[error("PCG breakdown at iteration {iteration}: {message}")]
    PcgBreakdown {
        /// Number of completed iterations before the breakdown.
        iteration: usize,
        /// Numerical breakdown description.
        message: String,
    },
    /// A CMG pair solver failed.
    #[cfg(feature = "cmg")]
    #[error("CMG pair solver failed: {0}")]
    Cmg(String),
    /// The external modified-LSMR driver failed.
    #[cfg(feature = "lsmr")]
    #[error("modified LSMR failed: {0}")]
    Lsmr(String),
}

pub(crate) fn dimension(context: &'static str, expected: usize, actual: usize) -> MultiwayError {
    MultiwayError::DimensionMismatch {
        context,
        expected,
        actual,
    }
}
