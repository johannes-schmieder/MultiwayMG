//! Deterministic operators for weighted three-way incidence Gramians.
//!
//! A problem contains one categorical level from each of three factors per
//! tuple. If `B` is the tuple-by-level incidence matrix and `W` is diagonal,
//! this crate represents and applies `B`, `sqrt(W) B`, and `G = B^T W B`
//! without materializing a sparse matrix.

mod aggregation;
mod components;
mod construction;
mod error;
mod factor_pair;
mod grouped_operator;
mod grouping;
mod hierarchy;
mod hierarchy_grouping;
mod kernels;
mod operator_view;
mod prepared;
mod problem;
/// Explicit serial diagnostic timing, absent unless the profiling feature is enabled.
#[cfg(feature = "profiling")]
pub mod profiling;
mod symbolic;
mod topology;
mod weight_frame;
mod weight_replay;

pub use aggregation::FactorAggregation;
pub use components::{
    IncidenceComponents, PreparedStructuralProjectionWorkspace, StructuralProjectionWorkspace,
};
pub use error::IncidenceError;
pub use factor_pair::FactorPair;
pub use hierarchy::{
    HierarchyWeightFrames, PreparedHierarchyAppendFailure, PreparedHierarchyAppendReport,
    PreparedHierarchyBudget, PreparedHierarchyBuilder, PreparedHierarchyLimits,
    PreparedHierarchyTopology,
};
pub use operator_view::ThreeWayOperatorView;
pub use prepared::{
    ObservationGroups, PreparedThreeWayTopology, PreparedTopologyBinding, PreparedTopologySource,
};
pub use problem::ThreeWayProblem;
pub use symbolic::{PreparedCoarseTupleMap, PreparedPairEdgeMap, TupleMergeGroups};
pub use topology::ThreeWayTopology;
pub use weight_frame::{
    ComponentWeightRange, ThreeWayWeightFrame, WeightFrameBinding, WeightFrameInput,
    WeightFrameInputKind, WeightFramePayloadBudget, WeightFrameSetupReport,
    WeightFrameValidationReport,
};

pub use weight_replay::{
    CoarseWeightReplay, PairConductanceReplay, WeightReplayPayloadBudget, WeightReplaySetupReport,
};

#[cfg(test)]
mod tests;

pub use grouping::{GroupedIndexWidth, GroupedTupleRow, PreparedTupleGrouping};

pub use grouped_operator::ThreeWayGroupedOperatorView;

pub use hierarchy_grouping::PreparedHierarchyGrouping;

mod component_layout;
pub use component_layout::{
    ComponentSourceIds, PreparedComponentLayout, PreparedComponentRecoding, PreparedComponentView,
};
