//! Error type for multilevel construction and solves.

use thiserror::Error;

use multiway_incidence::IncidenceError;

/// Failure while building or applying a MultiwayMG solver.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MultiwayError {
    /// A prepared working-set payload exceeds the explicitly supplied budget.
    #[error("prepared payload requires {required} bytes, budget is {budget}")]
    PayloadBudgetExceeded {
        /// Counted retained and declared caller payload.
        required: usize,
        /// Maximum payload admitted by this call.
        budget: usize,
    },
    /// Strict prepared execution found unprepared hierarchy scratch.
    #[error("workspace is not prepared for {context}")]
    WorkspaceNotPrepared {
        /// Rejected workspace boundary.
        context: &'static str,
    },
    /// The private hierarchy ownership invariant no longer matches its inventory.
    #[error("MAP hierarchy problem/smoother ownership invariant is inconsistent")]
    PayloadInventoryMismatch,
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
    /// A hard aggregation violated a problem or component invariant.
    #[error("invalid aggregation: {message}")]
    InvalidAggregation {
        /// Invariant violation.
        message: String,
    },
    /// Projected compatible relaxation could not produce a valid diagnostic.
    #[error("compatible relaxation failed: {message}")]
    CompatibleRelaxation {
        /// Failure description.
        message: String,
    },
    /// Complete-cycle matrix-free quality probing failed.
    #[error("cycle quality analysis failed: {message}")]
    CycleQuality {
        /// Failure description.
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
    /// A caller-owned workspace size or byte count overflowed.
    #[error("workspace size overflow in {context}")]
    WorkspaceSizeOverflow {
        /// Operation whose checked size calculation failed.
        context: &'static str,
    },
    /// A caller-owned workspace could not reserve requested storage.
    #[error("workspace allocation failed in {context}: {source}")]
    WorkspaceAllocation {
        /// Operation that attempted the reservation.
        context: &'static str,
        /// Allocation or capacity-overflow error from the standard library.
        source: std::collections::TryReserveError,
    },
    /// A CMG pair solver failed.
    #[cfg(feature = "cmg")]
    #[error("CMG pair solver failed: {0}")]
    Cmg(String),
    /// The external modified-LSMR driver failed.
    #[cfg(feature = "lsmr")]
    #[error("modified LSMR failed: {0}")]
    Lsmr(String),
    /// The frozen `within` approximate-Cholesky comparator failed.
    #[cfg(feature = "within-comparator")]
    #[error("within comparator failed: {0}")]
    Within(String),
}

pub(crate) fn dimension(context: &'static str, expected: usize, actual: usize) -> MultiwayError {
    MultiwayError::DimensionMismatch {
        context,
        expected,
        actual,
    }
}
