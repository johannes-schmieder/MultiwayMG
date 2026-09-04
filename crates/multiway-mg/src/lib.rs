//! Experimental multilevel solvers for weighted three-way incidence Gramians.
//!
//! The crate provides diagonal weighted Jacobi, a structure-preserving
//! three-way V-cycle, and optional pairwise graph-Laplacian corrections powered
//! by the `cmg` crate.

mod aggregation;
mod combine;
mod compatible;
mod compatible_gate;
mod dense;
mod dense_pair;
mod error;
mod hierarchy;
mod jacobi;
#[cfg(feature = "lsmr")]
mod lsmr;
mod map;
mod memory_estimate;
#[cfg(feature = "cmg")]
mod oracle_schedule;
#[cfg(feature = "cmg")]
mod pair_cmg;
mod pcg;
mod pcg_trace;
mod preconditioner;
#[cfg(feature = "cmg")]
mod research_pair;
mod spectral;
mod stationary;
mod two_grid;

pub use aggregation::{
    AffinityAggregationOptions, PairNeighborhoodAggregationOptions, build_affinity_aggregation,
    build_pair_neighborhood_aggregation,
};
pub use combine::WeightedSumPreconditioner;
pub use compatible::{
    CompatibleRelaxationOptions, CompatibleRelaxationReport, CompatibleRelaxationVectorReport,
    DiagonalAggregationProjector, analyze_compatible_relaxation,
};
pub use compatible_gate::{
    CompatibleRelaxationCriteria, CompatibleRelaxationDecision, CompatibleRelaxationRejection,
    evaluate_compatible_relaxation,
};
pub use dense::DensePseudoinverse;
pub use dense_pair::{DensePairOptions, DensePairSchwarzPreconditioner};
pub use error::MultiwayError;
pub use hierarchy::{
    AggregationKind, AggregationStrategy, HierarchyBuildReport, HierarchyOptions, ThreeWayHierarchy,
};
pub use jacobi::DiagonalPreconditioner;
#[cfg(feature = "lsmr")]
pub use lsmr::{
    LeastSquaresOptions, LeastSquaresResult, LeastSquaresStopReason, solve_weighted_least_squares,
};
pub use map::SymmetricMapPreconditioner;
#[cfg(feature = "cmg")]
pub use oracle_schedule::{
    OracleLevelSmootherSpec, ScheduledOracleHierarchy, ScheduledOracleHierarchyOptions,
    ScheduledOracleMemoryReport,
};
#[cfg(feature = "cmg")]
pub use pair_cmg::{HybridPairVcycle, PairCmgOptions, PairCmgPreconditioner};
pub use pcg::{PcgOptions, PcgResult, PcgStopReason, solve_projected_pcg};
pub use pcg_trace::{PcgTraceOptions, PcgTraceResult, PcgTraceSample, solve_projected_pcg_traced};
pub use preconditioner::Preconditioner;
#[cfg(feature = "cmg")]
pub use research_pair::{
    FactorPair, PairCmgMemoryReport, PairSubsetCmgPreconditioner, estimate_problem_bytes,
};
pub use spectral::{
    DenseRangeDecomposition, SpectralAnalysisOptions, SpectralAnalysisReport,
    analyze_preconditioner,
};
pub use stationary::{StationaryErrorReport, analyze_stationary_error};
pub use two_grid::{ExactCoarseCorrection, SymmetricTwoGridPreconditioner};

pub use multiway_incidence::{
    FactorAggregation, IncidenceComponents, IncidenceError, ThreeWayProblem, ThreeWayTopology,
};

#[cfg(test)]
mod tests;
