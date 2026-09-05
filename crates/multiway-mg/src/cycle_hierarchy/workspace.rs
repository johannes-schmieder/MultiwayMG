//! Caller-owned scratch for the complete recursive MAP hierarchy.

use super::{CycleScreenedMapHierarchy, add_assign};
use crate::{DensePseudoinverseWorkspace, MultiwayError, Preconditioner, ThreeWayProblem};

mod operators;
use operators::{LevelWorkspace, OperatorWorkspaces};

const FRAME_BUFFERS: usize = 7;
const CONTEXT: &str = "CycleScreenedMapHierarchyWorkspace";

/// Reusable vector and nested operator scratch for recursive MAP application.
///
/// Traversal vectors and terminal modal storage are anonymous. Component-bound
/// projection subworkspaces are explicitly refreshed during preparation. No
/// weights, factors, or numerical operators are retained here. One workspace
/// can serve independently built hierarchies of different sizes and depths.
///
/// After successful preparation, complete applications on that layout allocate
/// nothing, including MAP, structural projection and the dense terminal. The
/// ordinary convenience entry point still creates a temporary workspace.
/// Recursive frames use disjoint mutable borrows, with no global state or locks.
#[derive(Debug, Default)]
pub struct CycleScreenedMapHierarchyWorkspace {
    buffers: Vec<Vec<f64>>,
    operators: OperatorWorkspaces,
}

impl CycleScreenedMapHierarchyWorkspace {
    /// Construct empty scratch. The first application may prepare and grow it.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Prepare all traversal and operator scratch without numerical application.
    ///
    /// Component-bound subworkspaces are explicitly prepared for their new owners.
    /// Inactive storage remains retained. Failed setup may partially grow or
    /// reprepare scratch, but no caller output exists at this boundary. A later
    /// successful preparation restores all active lengths and bindings.
    pub fn try_prepare_for(
        &mut self,
        hierarchy: &CycleScreenedMapHierarchy,
    ) -> Result<(), MultiwayError> {
        hierarchy.workspace_required_bytes()?;
        self.prepare_vectors(&hierarchy.problems)?;
        self.operators.prepare(hierarchy)?;
        self.retained_bytes()?;
        Ok(())
    }

    /// Checked exclusive retained heap payload, including nested operator scratch.
    ///
    /// Includes all active/inactive vector capacities and heap descriptor arrays.
    /// Excludes the inline workspace, shared identity reference-count metadata,
    /// allocator overhead, immutable hierarchy and caller input/output. This is
    /// not process RSS or a complete solver peak-lifetime report.
    pub fn retained_bytes(&self) -> Result<usize, MultiwayError> {
        self.traversal_retained_bytes()?
            .checked_add(self.operator_retained_bytes()?)
            .ok_or_else(size_overflow)
    }

    /// Traversal vector capacities plus their heap-resident descriptor array.
    pub fn traversal_retained_bytes(&self) -> Result<usize, MultiwayError> {
        let descriptors = self
            .buffers
            .capacity()
            .checked_mul(core::mem::size_of::<Vec<f64>>())
            .ok_or_else(size_overflow)?;
        self.buffers.iter().try_fold(descriptors, |total, buffer| {
            let bytes = buffer
                .capacity()
                .checked_mul(core::mem::size_of::<f64>())
                .ok_or_else(size_overflow)?;
            total.checked_add(bytes).ok_or_else(size_overflow)
        })
    }

    /// Nested MAP/projection/modal payload and per-level heap descriptor storage.
    pub fn operator_retained_bytes(&self) -> Result<usize, MultiwayError> {
        self.operators.retained_bytes()
    }

    /// Number of retained traversal vectors, including inactive ones.
    ///
    /// Preserves the original traversal-only count; nested operator vectors are
    /// charged in bytes but do not change this count. This is not allocator calls.
    #[must_use]
    pub fn retained_buffer_count(&self) -> usize {
        self.buffers.len()
    }

    fn prepare_vectors(&mut self, problems: &[ThreeWayProblem]) -> Result<(), MultiwayError> {
        let count = required_buffer_count(problems.len() - 1)?;
        if count > self.buffers.len() {
            self.buffers
                .try_reserve_exact(count - self.buffers.len())
                .map_err(allocation_error)?;
            self.buffers.resize_with(count, Vec::new);
        }
        resize_buffer(&mut self.buffers[0], problems[0].dimension())?;
        for (level, pair) in problems.windows(2).enumerate() {
            let start = 1 + FRAME_BUFFERS * level;
            let fine = pair[0].dimension();
            let coarse = pair[1].dimension();
            for (buffer, length) in self.buffers[start..start + FRAME_BUFFERS]
                .iter_mut()
                .zip([fine, fine, coarse, coarse, fine, fine, fine])
            {
                resize_buffer(buffer, length)?;
            }
        }
        Ok(())
    }
}

impl CycleScreenedMapHierarchy {
    /// Minimum exclusive payload for a fresh complete application workspace.
    ///
    /// Includes traversal and per-level descriptor arrays, projection/MAP scratch
    /// and terminal modal storage. Actual retained capacities may be larger.
    pub fn workspace_required_bytes(&self) -> Result<usize, MultiwayError> {
        let count = required_buffer_count(self.depth())?;
        let descriptors = count
            .checked_mul(core::mem::size_of::<Vec<f64>>())
            .ok_or_else(size_overflow)?;
        let values = self
            .problems
            .windows(2)
            .try_fold(self.dimension(), |total, pair| {
                let fine = pair[0]
                    .dimension()
                    .checked_mul(5)
                    .ok_or_else(size_overflow)?;
                let coarse = pair[1]
                    .dimension()
                    .checked_mul(2)
                    .ok_or_else(size_overflow)?;
                total
                    .checked_add(fine)
                    .and_then(|n| n.checked_add(coarse))
                    .ok_or_else(size_overflow)
            })?;
        let vectors = values
            .checked_mul(core::mem::size_of::<f64>())
            .ok_or_else(size_overflow)?;
        let operator_bytes = operators::required_bytes(self)?;
        descriptors
            .checked_add(vectors)
            .and_then(|n| n.checked_add(operator_bytes))
            .ok_or_else(size_overflow)
    }

    /// Fallibly prepare all traversal, MAP, projection and terminal scratch.
    ///
    /// The outer workspace remains reusable with independent hierarchies; only
    /// semantic projection subworkspaces carry explicitly refreshed identities.
    pub fn application_workspace(
        &self,
    ) -> Result<CycleScreenedMapHierarchyWorkspace, MultiwayError> {
        let mut workspace = CycleScreenedMapHierarchyWorkspace::new();
        workspace.try_prepare_for(self)?;
        Ok(workspace)
    }

    /// Apply the same V-cycle as [`Preconditioner::apply`] with caller scratch.
    ///
    /// Both dimensions are checked before preparation or output mutation. This
    /// entry point retains automatic preparation for cross-instance/size reuse;
    /// callers may prepare explicitly beforehand. On a prepared layout the full
    /// operation allocates nothing. Output is copied only after complete success.
    pub fn apply_with_workspace(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut CycleScreenedMapHierarchyWorkspace,
    ) -> Result<(), MultiwayError> {
        if rhs.len() != self.dimension() {
            return Err(crate::error::dimension(
                "CycleScreenedMapHierarchy::apply rhs",
                self.dimension(),
                rhs.len(),
            ));
        }
        if out.len() != self.dimension() {
            return Err(crate::error::dimension(
                "CycleScreenedMapHierarchy::apply output",
                self.dimension(),
                out.len(),
            ));
        }
        workspace.try_prepare_for(self)?;
        let (solution, scratch) = workspace
            .buffers
            .split_first_mut()
            .expect("prepared result buffer");
        self.apply_level_into(
            0,
            rhs,
            solution,
            scratch,
            &mut workspace.operators.levels[..self.problems.len()],
            &mut workspace.operators.terminal,
        )?;
        out.copy_from_slice(solution);
        Ok(())
    }

    // One numerical recurrence; disjoint tails lend vector and operator scratch
    // to children without moving buffers, including during error or unwind.
    fn apply_level_into(
        &self,
        level: usize,
        rhs: &[f64],
        solution: &mut [f64],
        scratch: &mut [Vec<f64>],
        operator_levels: &mut [LevelWorkspace],
        terminal: &mut DensePseudoinverseWorkspace,
    ) -> Result<(), MultiwayError> {
        let problem = &self.problems[level];
        if rhs.len() != problem.dimension() {
            return Err(crate::error::dimension(
                "CycleScreenedMapHierarchy::apply_level",
                problem.dimension(),
                rhs.len(),
            ));
        }
        let (state, children) = operator_levels
            .split_first_mut()
            .expect("prepared operator level");
        solution.fill(0.0);
        if level == self.aggregations.len() {
            self.terminal
                .solve_into_with_workspace(rhs, solution, terminal)?;
            problem
                .components()
                .project_structural_range_with_workspace(solution, &mut state.projection)?;
            return Ok(());
        }
        let (frame, child_scratch) = scratch.split_at_mut(FRAME_BUFFERS);
        let [
            compatible_rhs,
            residual,
            coarse_rhs,
            coarse_solution,
            prolonged,
            post_residual,
            post,
        ] = frame
        else {
            unreachable!("a prepared nonterminal frame has seven buffers");
        };
        let map = state.map.as_mut().expect("prepared MAP level");
        compatible_rhs.fill(0.0);
        compatible_rhs.copy_from_slice(rhs);
        problem
            .components()
            .project_structural_range_with_workspace(compatible_rhs, &mut state.projection)?;
        self.smoothers[level].apply_with_workspace(compatible_rhs, solution, map)?;
        residual.fill(0.0);
        problem.residual_into(compatible_rhs, solution, residual)?;
        let coarse_problem = &self.problems[level + 1];
        coarse_rhs.fill(0.0);
        self.aggregations[level].restrict(residual, coarse_rhs)?;
        coarse_problem
            .components()
            .project_structural_range_with_workspace(coarse_rhs, &mut children[0].projection)?;
        self.apply_level_into(
            level + 1,
            coarse_rhs,
            coarse_solution,
            child_scratch,
            children,
            terminal,
        )?;
        prolonged.fill(0.0);
        self.aggregations[level].prolong(coarse_solution, prolonged)?;
        add_assign(solution, prolonged);
        post_residual.fill(0.0);
        problem.residual_into(compatible_rhs, solution, post_residual)?;
        post.fill(0.0);
        self.smoothers[level].apply_with_workspace(post_residual, post, map)?;
        add_assign(solution, post);
        problem
            .components()
            .project_structural_range_with_workspace(solution, &mut state.projection)?;
        Ok(())
    }
}

impl Preconditioner for CycleScreenedMapHierarchy {
    fn dimension(&self) -> usize {
        self.finest_problem().dimension()
    }
    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.apply_with_workspace(rhs, out, &mut CycleScreenedMapHierarchyWorkspace::new())
    }
}

fn required_buffer_count(depth: usize) -> Result<usize, MultiwayError> {
    depth
        .checked_mul(FRAME_BUFFERS)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(size_overflow)
}

fn resize_buffer(buffer: &mut Vec<f64>, length: usize) -> Result<(), MultiwayError> {
    if length > buffer.len() {
        buffer
            .try_reserve_exact(length - buffer.len())
            .map_err(allocation_error)?;
    }
    buffer.resize(length, 0.0);
    Ok(())
}

fn allocation_error(source: std::collections::TryReserveError) -> MultiwayError {
    MultiwayError::WorkspaceAllocation {
        context: CONTEXT,
        source,
    }
}
fn size_overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow { context: CONTEXT }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FactorAggregation;

    fn hierarchy() -> CycleScreenedMapHierarchy {
        let tuples: Vec<_> = (0..4)
            .flat_map(|i| (0..4).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
            .collect();
        let problem =
            ThreeWayProblem::from_observations([4; 3], &tuples, &vec![1.0; tuples.len()]).unwrap();
        let maps = vec![
            FactorAggregation::consecutive_halving([4; 3]).unwrap(),
            FactorAggregation::consecutive_halving([2; 3]).unwrap(),
        ];
        CycleScreenedMapHierarchy::from_maps(problem, maps, 1.0e-12).unwrap()
    }

    #[test]
    fn accounting_includes_descriptors_and_growth_is_checked() {
        let hierarchy = hierarchy();
        let workspace = hierarchy.application_workspace().unwrap();
        let traversal = workspace.buffers.capacity() * core::mem::size_of::<Vec<f64>>()
            + workspace
                .buffers
                .iter()
                .map(|v| v.capacity() * 8)
                .sum::<usize>();
        assert_eq!(workspace.traversal_retained_bytes().unwrap(), traversal);
        assert_eq!(
            workspace.retained_bytes().unwrap(),
            traversal + workspace.operators.retained_bytes().unwrap()
        );
        assert!(
            workspace.retained_bytes().unwrap() >= hierarchy.workspace_required_bytes().unwrap()
        );
        assert_eq!(workspace.retained_buffer_count(), 1 + 2 * FRAME_BUFFERS);
        assert!(required_buffer_count(usize::MAX).is_err());
        let mut buffer = vec![1.0, 2.0];
        let capacity = buffer.capacity();
        assert!(resize_buffer(&mut buffer, usize::MAX).is_err());
        assert_eq!(buffer, [1.0, 2.0]);
        assert_eq!(buffer.capacity(), capacity);
    }

    #[test]
    fn poisoned_scratch_and_nested_unwind_do_not_lose_buffers() {
        let hierarchy = hierarchy();
        let rhs = vec![1.0; hierarchy.dimension()];
        let mut expected = vec![0.0; rhs.len()];
        hierarchy.apply(&rhs, &mut expected).unwrap();
        let mut workspace = hierarchy.application_workspace().unwrap();
        let capacities: Vec<_> = workspace.buffers.iter().map(Vec::capacity).collect();
        let pointers: Vec<_> = workspace.buffers.iter().map(Vec::as_ptr).collect();
        let bytes = workspace.retained_bytes().unwrap();
        for buffer in &mut workspace.buffers {
            buffer.fill(f64::NAN);
        }
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (solution, scratch) = workspace.buffers.split_first_mut().unwrap();
            // Panic inside a child while the parent holds live disjoint leases.
            hierarchy
                .apply_level_into(
                    0,
                    &rhs,
                    solution,
                    &mut scratch[..FRAME_BUFFERS],
                    &mut workspace.operators.levels,
                    &mut workspace.operators.terminal,
                )
                .unwrap();
        }));
        assert!(caught.is_err());
        let mut actual = vec![f64::NAN; rhs.len()];
        hierarchy
            .apply_with_workspace(&rhs, &mut actual, &mut workspace)
            .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(workspace.retained_bytes().unwrap(), bytes);
        assert_eq!(
            workspace
                .buffers
                .iter()
                .map(Vec::capacity)
                .collect::<Vec<_>>(),
            capacities
        );
        assert_eq!(
            workspace
                .buffers
                .iter()
                .map(Vec::as_ptr)
                .collect::<Vec<_>>(),
            pointers
        );
    }
}
