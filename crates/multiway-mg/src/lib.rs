//! Experimental multilevel solvers for weighted three-way incidence Gramians.
//!
//! The crate provides diagonal weighted Jacobi, a structure-preserving
//! three-way V-cycle, and optional pairwise graph-Laplacian corrections powered
//! by the `cmg` crate.

mod aggregation;
mod dense;
mod error;
mod hierarchy;
mod jacobi;
#[cfg(feature = "lsmr")]
mod lsmr;
#[cfg(feature = "cmg")]
mod pair_cmg;
mod pcg;
mod preconditioner;

pub use aggregation::{AffinityAggregationOptions, build_affinity_aggregation};
pub use dense::DensePseudoinverse;
pub use error::MultiwayError;
pub use hierarchy::{
    AggregationStrategy, HierarchyBuildReport, HierarchyOptions, ThreeWayHierarchy,
};
pub use jacobi::DiagonalPreconditioner;
#[cfg(feature = "lsmr")]
pub use lsmr::{
    LeastSquaresOptions, LeastSquaresResult, LeastSquaresStopReason, solve_weighted_least_squares,
};
#[cfg(feature = "cmg")]
pub use pair_cmg::{HybridPairVcycle, PairCmgOptions, PairCmgPreconditioner};
pub use pcg::{PcgOptions, PcgResult, PcgStopReason, solve_projected_pcg};
pub use preconditioner::Preconditioner;

pub use multiway_incidence::{
    FactorAggregation, IncidenceComponents, IncidenceError, ThreeWayProblem, ThreeWayTopology,
};

#[cfg(test)]
mod tests;
