//! Deterministic operators for weighted three-way incidence Gramians.
//!
//! A problem contains one categorical level from each of three factors per
//! tuple. If `B` is the tuple-by-level incidence matrix and `W` is diagonal,
//! this crate represents and applies `B`, `sqrt(W) B`, and `G = B^T W B`
//! without materializing a sparse matrix.

mod aggregation;
mod components;
mod error;
mod problem;
mod topology;

pub use aggregation::FactorAggregation;
pub use components::{IncidenceComponents, StructuralProjectionWorkspace};
pub use error::IncidenceError;
pub use problem::ThreeWayProblem;
pub use topology::ThreeWayTopology;

#[cfg(test)]
mod tests;
