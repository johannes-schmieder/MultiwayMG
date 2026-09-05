//! Strict payload admission for an already built and fully prepared MAP-PCG solve.

use super::{PcgTraceOptions, PcgTraceResultRef, PcgTraceWorkspace};
use crate::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, MapHierarchyPayloadReport,
    MultiwayError,
};

/// Explicit payload budget, not a process-RSS limit or an allocator quota.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcgPayloadBudget {
    /// Maximum counted payload for the prepared solve.
    pub maximum_bytes: usize,
    /// Additional disjoint live payload supplied by the caller.
    ///
    /// Charge other RHS columns, spare input capacity, owned result copies,
    /// retained plans and overlapping old state here. The submitted RHS slice,
    /// hierarchy and both workspaces are already counted. The library cannot
    /// discover external allocations or automatically deduplicate caller aliases.
    pub additional_live_bytes: usize,
}

/// Retained working-set payload for one fully prepared MAP-PCG solve.
///
/// Counts hierarchy payload once, both exclusive scratch payloads (including
/// inactive capacity and the complete trace budget), RHS slice and declared
/// extra live storage. Excludes Arc headers/padding, allocator overhead, inline
/// roots, setup transients, stack, unreported external storage and process RSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapPcgPayloadReport {
    /// Built fixed MAP hierarchy, with disjoint shared/exclusive categories.
    pub hierarchy: MapHierarchyPayloadReport,
    /// Outer coefficient, projection, solution and trace capacities.
    pub outer_workspace_bytes: usize,
    /// Complete recursive workspace, including inactive retained capacity.
    pub hierarchy_workspace_bytes: usize,
    /// Submitted RHS slice payload; unused owner capacity is caller-declared extra.
    pub rhs_bytes: usize,
    /// Additional disjoint live payload declared by the caller.
    pub additional_live_bytes: usize,
}

impl MapPcgPayloadReport {
    /// Checked total; this is not a promise about allocator or OS peak memory.
    pub fn total_bytes(self) -> Result<usize, MultiwayError> {
        [
            self.hierarchy.total_bytes()?,
            self.outer_workspace_bytes,
            self.hierarchy_workspace_bytes,
            self.rhs_bytes,
            self.additional_live_bytes,
        ]
        .into_iter()
        .try_fold(0usize, |total, bytes| {
            total
                .checked_add(bytes)
                .ok_or(MultiwayError::WorkspaceSizeOverflow {
                    context: "prepared MAP-PCG payload",
                })
        })
    }

    /// Admit equality and reject a smaller budget without allocation or mutation.
    pub fn check_budget(self, maximum_bytes: usize) -> Result<(), MultiwayError> {
        let required = self.total_bytes()?;
        if required > maximum_bytes {
            return Err(MultiwayError::PayloadBudgetExceeded {
                required,
                budget: maximum_bytes,
            });
        }
        Ok(())
    }
}

/// Validate preparation and report actual retained payload without mutation.
///
/// The submitted operator is always the hierarchy's finest problem. An unprepared
/// or rebound workspace is rejected, even for a zero RHS. Minimum scratch-size
/// queries are not substituted for actual capacities. All validation is read-only.
pub fn prepared_map_pcg_payload_report(
    hierarchy: &CycleScreenedMapHierarchy,
    rhs: &[f64],
    options: PcgTraceOptions,
    outer: &PcgTraceWorkspace,
    inner: &CycleScreenedMapHierarchyWorkspace,
    additional_live_bytes: usize,
) -> Result<MapPcgPayloadReport, MultiwayError> {
    let problem = hierarchy.finest_problem();
    super::validate_inputs(problem, rhs, hierarchy, options)?;
    outer.validate(problem, options)?;
    if !inner.is_prepared_for(hierarchy) {
        return Err(MultiwayError::WorkspaceNotPrepared {
            context: "MAP hierarchy",
        });
    }
    let report = MapPcgPayloadReport {
        hierarchy: hierarchy.retained_payload_report()?,
        outer_workspace_bytes: outer.retained_bytes()?,
        hierarchy_workspace_bytes: inner.retained_bytes()?,
        rhs_bytes: core::mem::size_of_val(rhs),
        additional_live_bytes,
    };
    report.total_bytes()?;
    Ok(report)
}

/// Admit a fully prepared working set, then run the existing borrowed PCG path.
///
/// Valid options/dimensions, both workspace layouts/bindings, checked arithmetic
/// and the retained payload budget are checked before either workspace changes.
/// No setup is performed on rejection. A successful prepared solve allocates
/// nothing, so its counted capacities cannot grow after admission. Numerical
/// errors may still allocate diagnostic strings and return no result view.
///
/// This admits only an already constructed working set. It neither reserves
/// memory nor limits construction/repreparation peaks, Arc/allocator overhead,
/// external allocations or OS RSS. Scientific acceptance still requires separate
/// certification against the submitted operator; no solver routing changes here.
pub fn solve_projected_pcg_traced_with_payload_budget<'a>(
    hierarchy: &CycleScreenedMapHierarchy,
    rhs: &[f64],
    options: PcgTraceOptions,
    outer: &'a mut PcgTraceWorkspace,
    inner: &mut CycleScreenedMapHierarchyWorkspace,
    budget: PcgPayloadBudget,
) -> Result<PcgTraceResultRef<'a>, MultiwayError> {
    prepared_map_pcg_payload_report(
        hierarchy,
        rhs,
        options,
        outer,
        inner,
        budget.additional_live_bytes,
    )?
    .check_budget(budget.maximum_bytes)?;
    super::solve_projected_pcg_traced_with_workspaces(
        hierarchy.finest_problem(),
        rhs,
        hierarchy,
        options,
        outer,
        inner,
    )
}
