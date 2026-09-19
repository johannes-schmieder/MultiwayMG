//! Explicit component-scoped grouping; numerical owners account for it once.
use super::*;
use crate::GroupedGramianMode;
use multiway_incidence::{
    PreparedHierarchyGrouping, PreparedHierarchyTopology, PreparedTupleGrouping,
};

/// Explicit application layout, independent of structural construction policy.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PreparedAutomaticLayout {
    /// Existing canonical tuple arithmetic with no grouping or image.
    #[default]
    Scalar,
    /// Group only the fine nonterminal hierarchy level, with no image.
    FineRow,
    /// Group every nonterminal hierarchy level, with no image.
    AllRow,
    /// Group the fine nonterminal level and retain one tuple image.
    FineImage,
    /// Group every nonterminal level and share one maximum-E image.
    AllImage,
}
impl PreparedAutomaticLayout {
    pub(super) fn mode(self) -> Option<GroupedGramianMode> {
        match self {
            Self::Scalar => None,
            Self::FineRow | Self::AllRow => Some(GroupedGramianMode::RowGather),
            Self::FineImage | Self::AllImage => Some(GroupedGramianMode::TupleImage),
        }
    }
    fn prefix(self, depth: usize) -> usize {
        match self {
            Self::Scalar => 0,
            Self::FineRow | Self::FineImage => depth.min(1),
            Self::AllRow | Self::AllImage => depth,
        }
    }
}

/// Numerical owner for which an explicit group build was attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedAutomaticGroupingScope {
    /// Nonterminal prefix of a component's proposed hierarchy.
    Hierarchy,
    /// Single fine level of a large-component fixed baseline.
    ComponentBaseline,
    /// Single fine level of the final full-original baseline.
    GlobalBaseline,
}
/// Last failed grouping location; its typed cause follows ordinary error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedAutomaticGroupingLocation {
    /// Original component label, or None for the whole original problem.
    pub component: Option<usize>,
    /// Owner whose group setup failed.
    pub scope: PreparedAutomaticGroupingScope,
}
/// Fixed-size layout inventory, separate from the original scalar progress ABI.
///
/// Completed groups may later be dropped after a rejected screen/solve. Counts
/// record construction, not accepted hierarchy count or hidden timing estimates.
/// A failed build retains its attempt/location; partially allocated unpublished
/// groups are not called completed. Full array peaks and typed failures remain
/// in ordinary progress/the returned error. No silent scalar layout recovery.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PreparedAutomaticLayoutProgress {
    /// Explicit requested layout, not a measured or automatic selection.
    pub requested_layout: PreparedAutomaticLayout,
    /// Entered group preparations, including failed admission/allocation.
    pub grouping_attempts: usize,
    /// Completely constructed grouping owners, including later discarded owners.
    pub completed_groupings: usize,
    /// Group preparations that returned an error.
    pub rejected_groupings: usize,
    /// Sum of published levels across completed grouping owners.
    pub grouped_levels_built: usize,
    /// Largest exclusive actual grouping array/descriptor capacity in bytes.
    pub maximum_grouping_payload_bytes: usize,
    /// Largest logical image length in a successfully constructed application
    /// workspace. Actual image capacity is included in ordinary array accounting.
    pub maximum_tuple_image_len: usize,
    /// Last failed group preparation, preserved through later successful fallback.
    pub last_rejected_grouping: Option<PreparedAutomaticGroupingLocation>,
}

fn tracked<T>(
    scope: PreparedAutomaticGroupingScope,
    progress: &mut Progress<'_>,
    build: impl FnOnce(&mut Progress<'_>) -> Result<(T, usize, usize), MultiwayError>,
) -> Result<T, MultiwayError> {
    #[cfg(feature = "profiling")]
    let _span = crate::automatic_profiling::span(match scope {
        PreparedAutomaticGroupingScope::Hierarchy => {
            crate::automatic_profiling::Phase::HierarchyGrouping
        }
        PreparedAutomaticGroupingScope::ComponentBaseline => {
            crate::automatic_profiling::Phase::ComponentGrouping
        }
        PreparedAutomaticGroupingScope::GlobalBaseline => {
            crate::automatic_profiling::Phase::GlobalGrouping
        }
    });
    increment(&mut progress.layout.grouping_attempts, 1)?;
    match build(progress) {
        Ok((groups, bytes, levels)) => {
            increment(&mut progress.layout.completed_groupings, 1)?;
            increment(&mut progress.layout.grouped_levels_built, levels)?;
            progress.layout.maximum_grouping_payload_bytes =
                progress.layout.maximum_grouping_payload_bytes.max(bytes);
            Ok(groups)
        }
        Err(source) => {
            increment(&mut progress.layout.rejected_groupings, 1)?;
            progress.layout.last_rejected_grouping = Some(PreparedAutomaticGroupingLocation {
                component: progress.component,
                scope,
            });
            Err(source)
        }
    }
}

pub(super) fn hierarchy_groups<'hierarchy, 'fine>(
    hierarchy: &'hierarchy PreparedHierarchyTopology<'fine>,
    frame: &ThreeWayWeightFrame<'_>,
    maximum: usize,
    other: usize,
    progress: &mut Progress<'_>,
) -> Result<Option<PreparedHierarchyGrouping<'hierarchy, 'fine>>, MultiwayError> {
    let prefix = progress
        .layout
        .requested_layout
        .prefix(hierarchy.level_count() - 1);
    if prefix == 0 {
        return Ok(None);
    }
    tracked(PreparedAutomaticGroupingScope::Hierarchy, progress, |p| {
        // Group construction counts fine/coarse topology itself; the fine
        // numerical frame and external original/local/caller owners are extra.
        let budget = payload_budget(maximum, add(other, frame.retained_payload_bytes()?)?);
        admit(
            PreparedHierarchyGrouping::setup_payload_bound(
                hierarchy,
                prefix,
                budget.additional_live_payload_bytes,
            )?,
            maximum,
            p,
        )?;
        let groups = PreparedHierarchyGrouping::try_new_with_budget(hierarchy, prefix, budget)?;
        admit(groups.setup_peak_payload_bound(), maximum, p)?;
        let bytes = groups.retained_payload_bytes()?;
        Ok((groups, bytes, prefix))
    })
    .map(Some)
}

pub(super) fn baseline_groups<'fine>(
    frame: &ThreeWayWeightFrame<'fine>,
    maximum: usize,
    other: usize,
    scope: PreparedAutomaticGroupingScope,
    progress: &mut Progress<'_>,
) -> Result<Option<PreparedTupleGrouping<'fine>>, MultiwayError> {
    if progress.layout.requested_layout == PreparedAutomaticLayout::Scalar {
        return Ok(None);
    }
    tracked(scope, progress, |p| {
        let other = add(other, frame.retained_payload_bytes()?)?;
        admit(
            PreparedTupleGrouping::setup_payload_bound(frame.topology(), other)?,
            maximum,
            p,
        )?;
        let groups = PreparedTupleGrouping::try_new_with_budget(frame.topology(), maximum, other)?;
        let bytes = groups.retained_payload_bytes()?;
        admit(
            add(
                add(frame.topology().retained_payload_bytes()?, other)?,
                bytes,
            )?,
            maximum,
            p,
        )?;
        Ok((groups, bytes, 1))
    })
    .map(Some)
}

pub(super) fn record_image(owner: &crate::PreparedMapHierarchy<'_>, progress: &mut Progress<'_>) {
    // Only hierarchy cycles apply grouped Gramians during this LSMR route.
    // The existing application arena shares the image across selected levels.
    progress.layout.maximum_tuple_image_len = progress
        .layout
        .maximum_tuple_image_len
        .max(owner.tuple_image_len());
}
