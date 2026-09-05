from pathlib import Path
import subprocess

BASE = 'cafed1bd2f2ee744eed4b16a458d168bf02547b3'
assert subprocess.check_output(['git', 'rev-parse', 'HEAD^'], text=True).strip() == BASE

def append(path, text):
    p = Path(path)
    p.write_text(p.read_text() + '\n' + text)

def replace(path, old, new):
    p = Path(path)
    text = p.read_text()
    assert text.count(old) == 1, (path, old, text.count(old))
    p.write_text(text.replace(old, new))

def new(path, text):
    p = Path(path)
    assert not p.exists(), path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text)

append('crates/multiway-incidence/src/topology.rs', r'''
impl ThreeWayTopology {
    /// Retained tuple-array payload, using capacity rather than length.
    ///
    /// Excludes this inline object and allocator overhead.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        self.tuples.capacity().checked_mul(core::mem::size_of::<[u32; 3]>())
            .ok_or(IncidenceError::DimensionOverflow { context: "topology payload" })
    }
}
''')
append('crates/multiway-incidence/src/components.rs', r'''
impl IncidenceComponents {
    /// Retained labels and factor-size array payload, including unused capacity.
    ///
    /// Excludes this inline object, the zero-sized identity token's Arc header,
    /// allocator overhead and any separately owned projection workspace.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        let overflow = || IncidenceError::DimensionOverflow { context: "component payload" };
        let labels = self.labels.capacity().checked_mul(core::mem::size_of::<usize>())
            .ok_or_else(overflow)?;
        let sizes = self.factor_sizes.capacity().checked_mul(core::mem::size_of::<[usize; 3]>())
            .ok_or_else(overflow)?;
        labels.checked_add(sizes).ok_or_else(overflow)
    }
}
''')
append('crates/multiway-incidence/src/problem.rs', r'''
impl ThreeWayProblem {
    /// Payload reachable through the five shared problem allocations, counted once.
    ///
    /// Counts topology/component objects behind Arc, their vector capacities,
    /// and the three numerical Arc slices. Excludes the inline problem handle,
    /// Arc headers/alignment padding, identity headers and allocator overhead.
    /// Ordinary problem clones share this payload; do not sum their reports.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        let parts = [
            core::mem::size_of::<ThreeWayTopology>(),
            self.topology.retained_payload_bytes()?,
            core::mem::size_of::<IncidenceComponents>(),
            self.components.retained_payload_bytes()?,
            core::mem::size_of_val(self.weights.as_ref()),
            core::mem::size_of_val(self.square_root_weights.as_ref()),
            core::mem::size_of_val(self.diagonal.as_ref()),
        ];
        parts.into_iter().try_fold(0usize, |total, bytes| {
            total.checked_add(bytes).ok_or(IncidenceError::DimensionOverflow {
                context: "shared problem payload",
            })
        })
    }

    /// Whether all five immutable backing allocations are shared with `other`.
    ///
    /// This is storage identity, not value equality or authorization to reuse
    /// numerical state under changed weights. Independent equal builds return false.
    #[must_use]
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.topology, &other.topology)
            && Arc::ptr_eq(&self.weights, &other.weights)
            && Arc::ptr_eq(&self.square_root_weights, &other.square_root_weights)
            && Arc::ptr_eq(&self.diagonal, &other.diagonal)
            && Arc::ptr_eq(&self.components, &other.components)
    }
}
''')
append('crates/multiway-incidence/src/aggregation.rs', r'''
impl FactorAggregation {
    /// Checked parent-array payload, including unused capacities.
    ///
    /// Excludes the inline aggregation descriptor and allocator overhead.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        self.parents.iter().try_fold(0usize, |total, parents| {
            parents.capacity().checked_mul(core::mem::size_of::<u32>())
                .and_then(|bytes| total.checked_add(bytes))
                .ok_or(IncidenceError::DimensionOverflow { context: "aggregation payload" })
        })
    }
}
''')
append('crates/multiway-mg/src/dense.rs', r'''
impl DensePseudoinverse {
    /// Exclusive retained matrix and inverse-eigenvalue payload, by capacity.
    ///
    /// Excludes the inline descriptor, temporary factorization storage and
    /// allocator overhead. This does not estimate factorization peak memory.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        self.eigenvectors.data.as_vec().capacity()
            .checked_add(self.inverse_eigenvalues.capacity())
            .and_then(|values| values.checked_mul(core::mem::size_of::<f64>()))
            .ok_or(MultiwayError::WorkspaceSizeOverflow { context: "dense terminal payload" })
    }

    pub(crate) fn workspace_is_prepared(&self, workspace: &DensePseudoinverseWorkspace) -> bool {
        workspace.modal.len() == self.dimension()
    }
}
''')
append('crates/multiway-mg/src/map/workspace.rs', r'''
impl SymmetricMapWorkspace {
    pub(crate) fn is_prepared_for(&self, operator: &SymmetricMapPreconditioner) -> bool {
        self.validate(operator).is_ok()
    }
}
''')
append('crates/multiway-mg/src/cycle_hierarchy/workspace/operators.rs', r'''
impl OperatorWorkspaces {
    pub(super) fn is_prepared_for(&self, hierarchy: &CycleScreenedMapHierarchy) -> bool {
        self.levels.len() >= hierarchy.problems.len()
            && hierarchy.terminal.workspace_is_prepared(&self.terminal)
            && hierarchy.problems.iter().enumerate().all(|(level, problem)| {
                let state = &self.levels[level];
                state.projection.is_compatible_with(problem.components())
                    && hierarchy.smoothers.get(level).is_none_or(|smoother| {
                        state.map.as_ref().is_some_and(|map| map.is_prepared_for(smoother))
                    })
            })
    }
}
''')
append('crates/multiway-mg/src/cycle_hierarchy/workspace.rs', r'''
impl CycleScreenedMapHierarchyWorkspace {
    /// Read-only validation of every active vector, modal buffer and binding.
    ///
    /// Does not prepare, allocate, mutate or compare numerical weights. Inactive
    /// storage is allowed and remains included in retained-byte accounting.
    #[must_use]
    pub fn is_prepared_for(&self, hierarchy: &CycleScreenedMapHierarchy) -> bool {
        let Ok(count) = required_buffer_count(hierarchy.depth()) else { return false; };
        if self.buffers.len() < count
            || self.buffers[0].len() != hierarchy.finest_problem().dimension()
            || !self.operators.is_prepared_for(hierarchy)
        {
            return false;
        }
        hierarchy.problems.windows(2).enumerate().all(|(level, pair)| {
            let start = 1 + FRAME_BUFFERS * level;
            let fine = pair[0].dimension();
            let coarse = pair[1].dimension();
            self.buffers[start..start + FRAME_BUFFERS].iter()
                .zip([fine, fine, coarse, coarse, fine, fine, fine])
                .all(|(buffer, expected)| buffer.len() == expected)
        })
    }
}
''')
replace('crates/multiway-mg/src/cycle_hierarchy.rs', 'mod workspace;', 'mod workspace;\nmod payload;\npub use payload::MapHierarchyPayloadReport;')
new('crates/multiway-mg/src/cycle_hierarchy/payload.rs', r'''//! Retained ownership inventory for one fixed MAP hierarchy, not setup peak RSS.

use super::CycleScreenedMapHierarchy;
use crate::{FactorAggregation, MultiwayError, SymmetricMapPreconditioner, ThreeWayProblem};

/// Disjoint payload categories reachable from one built MAP hierarchy.
///
/// Shared level problems are counted once, not again through smoother clones.
/// Excludes the inline hierarchy root, Arc headers/padding, allocator overhead,
/// construction plans/transients and all caller/workspace storage. Reports for
/// two hierarchies cannot simply be summed when they share problem allocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapHierarchyPayloadReport {
    /// Immutable fine/coarse problem payload, potentially shared with other owners.
    pub shared_problem_bytes: usize,
    /// Heap descriptor arrays for problems, aggregations and smoothers.
    pub descriptor_bytes: usize,
    /// Exclusively owned factor-parent array capacities.
    pub aggregation_bytes: usize,
    /// Exclusively owned terminal matrix and inverse-eigenvalue capacities.
    pub terminal_bytes: usize,
}

impl MapHierarchyPayloadReport {
    /// Disjoint exclusive payload, excluding shared problem state.
    pub fn exclusive_bytes(self) -> Result<usize, MultiwayError> {
        sum(&[self.descriptor_bytes, self.aggregation_bytes, self.terminal_bytes])
    }

    /// Total retained payload for this ownership boundary.
    pub fn total_bytes(self) -> Result<usize, MultiwayError> {
        sum(&[self.shared_problem_bytes, self.exclusive_bytes()?])
    }
}

impl CycleScreenedMapHierarchy {
    /// Inventory one built hierarchy without allocating, cloning or applying it.
    ///
    /// Each strictly smaller level owns independently constructed problem state.
    /// The smoother at that level must share all of its immutable backing state;
    /// fail closed if this private construction invariant is ever broken.
    pub fn retained_payload_report(&self) -> Result<MapHierarchyPayloadReport, MultiwayError> {
        if self.smoothers.len() != self.aggregations.len()
            || self.problems.len() != self.aggregations.len() + 1
            || self.smoothers.iter().zip(&self.problems)
                .any(|(smoother, problem)| !smoother.problem().shares_storage_with(problem))
            || self.problems.windows(2).any(|pair| pair[1].dimension() >= pair[0].dimension())
        {
            return Err(MultiwayError::PayloadInventoryMismatch);
        }
        let shared_problem_bytes = self.problems.iter().try_fold(0usize, |total, problem| {
            sum(&[total, problem.retained_payload_bytes()?])
        })?;
        let descriptor_bytes = sum(&[
            bytes::<ThreeWayProblem>(self.problems.capacity())?,
            bytes::<FactorAggregation>(self.aggregations.capacity())?,
            bytes::<SymmetricMapPreconditioner>(self.smoothers.capacity())?,
        ])?;
        let aggregation_bytes = self.aggregations.iter().try_fold(0usize, |total, map| {
            sum(&[total, map.retained_payload_bytes()?])
        })?;
        let report = MapHierarchyPayloadReport {
            shared_problem_bytes,
            descriptor_bytes,
            aggregation_bytes,
            terminal_bytes: self.terminal.retained_payload_bytes()?,
        };
        report.total_bytes()?;
        Ok(report)
    }
}

fn bytes<T>(count: usize) -> Result<usize, MultiwayError> {
    count.checked_mul(core::mem::size_of::<T>()).ok_or_else(overflow)
}
fn sum(parts: &[usize]) -> Result<usize, MultiwayError> {
    parts.iter().try_fold(0usize, |total, &part| total.checked_add(part).ok_or_else(overflow))
}
fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow { context: "MAP hierarchy payload" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_arithmetic_never_wraps() {
        assert!(bytes::<f64>(usize::MAX).is_err());
        assert!(sum(&[usize::MAX, 1]).is_err());
        let report = MapHierarchyPayloadReport {
            shared_problem_bytes: usize::MAX, descriptor_bytes: 1,
            aggregation_bytes: 0, terminal_bytes: 0,
        };
        assert!(report.total_bytes().is_err());
    }
}
''')
replace('crates/multiway-mg/src/error.rs', 'pub enum MultiwayError {', r'''pub enum MultiwayError {
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
    PayloadInventoryMismatch,''')
replace('crates/multiway-mg/src/pcg_trace.rs', 'mod finite;', 'mod finite;\nmod admission;\npub use admission::{MapPcgPayloadReport, PcgPayloadBudget, prepared_map_pcg_payload_report, solve_projected_pcg_traced_with_payload_budget};')
new('crates/multiway-mg/src/pcg_trace/admission.rs', r'''//! Strict payload admission for an already built and fully prepared MAP-PCG solve.

use super::{PcgTraceOptions, PcgTraceResultRef, PcgTraceWorkspace};
use crate::{CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace,
    MapHierarchyPayloadReport, MultiwayError};

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
        [self.hierarchy.total_bytes()?, self.outer_workspace_bytes,
            self.hierarchy_workspace_bytes, self.rhs_bytes, self.additional_live_bytes]
            .into_iter().try_fold(0usize, |total, bytes| total.checked_add(bytes)
                .ok_or(MultiwayError::WorkspaceSizeOverflow { context: "prepared MAP-PCG payload" }))
    }

    /// Admit equality and reject a smaller budget without allocation or mutation.
    pub fn check_budget(self, maximum_bytes: usize) -> Result<(), MultiwayError> {
        let required = self.total_bytes()?;
        if required > maximum_bytes {
            return Err(MultiwayError::PayloadBudgetExceeded { required, budget: maximum_bytes });
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
        return Err(MultiwayError::WorkspaceNotPrepared { context: "MAP hierarchy" });
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
    prepared_map_pcg_payload_report(hierarchy, rhs, options, outer, inner,
        budget.additional_live_bytes)?.check_budget(budget.maximum_bytes)?;
    super::solve_projected_pcg_traced_with_workspaces(
        hierarchy.finest_problem(), rhs, hierarchy, options, outer, inner)
}
''')
append('crates/multiway-mg/src/pcg_trace/result_ref.rs', r'''
impl PcgTraceResult {
    /// Exclusive retained solution and trace payload, using actual capacities.
    ///
    /// Useful for charging live owned copies as additional caller payload during
    /// another solve. Excludes the inline result object and allocator overhead.
    pub fn retained_payload_bytes(&self) -> Result<usize, crate::MultiwayError> {
        self.solution.capacity().checked_mul(core::mem::size_of::<f64>())
            .and_then(|solution| self.samples.capacity()
                .checked_mul(core::mem::size_of::<PcgTraceSample>())
                .and_then(|trace| solution.checked_add(trace)))
            .ok_or(crate::MultiwayError::WorkspaceSizeOverflow { context: "owned PCG result payload" })
    }
}
''')
replace('crates/multiway-mg/src/lib.rs', '    CycleScreenedMapHierarchyWorkspace,', '    CycleScreenedMapHierarchyWorkspace, MapHierarchyPayloadReport,')
replace('crates/multiway-mg/src/lib.rs', '    PcgTraceOptions, PcgTraceResult, PcgTraceResultRef, PcgTraceSample, PcgTraceWorkspace,', '    MapPcgPayloadReport, PcgPayloadBudget, PcgTraceOptions, PcgTraceResult, PcgTraceResultRef, PcgTraceSample, PcgTraceWorkspace, prepared_map_pcg_payload_report, solve_projected_pcg_traced_with_payload_budget,')
replace('CHANGELOG.md', '### Added\n', '### Added\n\n- Checked fixed-MAP hierarchy payload inventories and strict prepared traced-PCG\n  payload-budget admission; see `docs/ISSUE5_PAYLOAD_ADMISSION.md`.\n')
new('docs/ISSUE5_PAYLOAD_ADMISSION.md', r'''# Issue 5: resident payload inventory and prepared-solve admission

## Scope and ownership

This increment inventories one **already built fixed MAP hierarchy** and admits
one **already prepared traced-PCG working set**. It does not claim total peak
memory or solve the separate hierarchy-build allocation-admission problem.

`ThreeWayProblem::retained_payload_bytes` includes the objects behind its topology
and component Arcs, their tuple/label/component-size vector capacities, and its
weight, square-root-weight and diagonal Arc slices. An ordinary problem clone
shares all five backing allocations. `shares_storage_with` checks those allocation
identities; value equality is not storage identity or a numerical replay contract.

`CycleScreenedMapHierarchy::retained_payload_report` separates shared problem
payload from exclusive problem/map/smoother descriptor arrays, aggregation parent
arrays and dense terminal factors. Smoother problem clones are not counted again.
Every level is strictly smaller and independently constructed; the inventory checks
the private shape and smoother-sharing invariant rather than silently assuming a
future representation still satisfies it. The terminal matrix uses actual storage
capacity, not its logical matrix dimension. A retained construction plan is outside
this boundary and must be charged separately by a caller that keeps it alive.

One cloned hierarchy shares the level problems but owns another set of descriptors,
parent arrays and terminal factors. When both remain live, charge shared problem
payload once and each hierarchy's exclusive payload once. Independent equal builds
are distinct allocations. These are not general graph-deduplicating reports for an
arbitrary collection of partially shared objects.

## Strict prepared execution

`prepared_map_pcg_payload_report` validates options and RHS dimensions, outer
workspace lengths/component identity/full trace capacity and every active recursive
scratch length and binding. It reports actual capacities, including unused and
inactive scratch, rather than substituting minimum `required_bytes` estimates.
Read-only `CycleScreenedMapHierarchyWorkspace::is_prepared_for` never rebinds or grows.

`solve_projected_pcg_traced_with_payload_budget` accepts the hierarchy, RHS, options,
both workspaces and `PcgPayloadBudget { maximum_bytes, additional_live_bytes }`.
It checks the report and rejects before mutation when the total exceeds the budget;
equality is admitted. The original shared borrowed-solve path is then used without
changing arithmetic, counters, stopping criteria or certification responsibilities.
The numerical operator is the hierarchy's finest problem. Prepared scratch cannot
grow during a successful solve. Even a zero RHS requires both workspaces prepared
for this strict API; existing APIs retain their previous automatic-prepare behavior.

The report charges the hierarchy once, both complete workspace payloads, the RHS
slice, and caller-declared disjoint extra live bytes. The outer workspace already
contains its solution and full trace budget. Borrowed views add no heap payload;
`PcgTraceResult::retained_payload_bytes` charges an explicitly retained owned result.
Use the extra-live category for other RHS columns, input capacity beyond the slice,
retained result copies, old-state overlap and other independent retained buffers.
Do not charge an alias of already-counted storage twice. The library cannot discover
or verify unreported external allocations.

## Exclusions and lifetime boundaries

All counts are checked **payload bytes**, not allocation footprints or RSS. They
exclude inline root objects, Arc control headers/alignment padding (including old
identity tokens held by inactive scratch), allocator metadata/rounding, stack,
construction/repreparation transients and unreported external storage. Heap-resident
descriptor arrays and topology/component objects behind Arc ARE included.

The strict API performs admission after construction and preparation, before
numerical mutation. It does not promise to prevent an out-of-memory failure during
setup, enforce a malloc quota, or cap OS memory. Existing minimum workspace queries
are useful for planning but cannot replace retained-capacity checks after setup.
Numerical-error strings may allocate; static budget/binding/dimension rejection and
successful prepared execution do not. An external original-operator certificate may
need additional storage and remains a separate charged phase.

## Qualification and remaining work

Require the complete Rust 1.85 Actions suite, original numerical regressions and
existing 12-configuration allocator matrix. Instrument hierarchy-clone allocation
and destruction against exclusive payload, owned copies against their reports,
strict report/admission/solve regions, equality and one-byte-short budgets, external
live charges, overflow, inactive scratch, wrong owners and post-rejection reuse.

Boundary-failure tests must distinguish deterministic injected errors from real OS
allocation failures. Full setup-lifetime/allocator admission, exhaustive failure
injection, other hierarchy/pair routes and LSMR remain separate increments. No
numerical replay, fresh holdout, speedup or production-routing change is implied;
ADR 0002 and frozen scientific evidence remain unchanged.
''')
print('Permanent payload source assembled. Tests are added by the companion script.')
