//! Caller-owned scratch for the recursive MAP hierarchy.

use super::{CycleScreenedMapHierarchy, add_assign};
use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

const FRAME_BUFFERS: usize = 7;
const CONTEXT: &str = "CycleScreenedMapHierarchyWorkspace";

/// Reusable anonymous vector storage for recursive hierarchy application.
///
/// This workspace contains no topology, weights, factors, or component binding.
/// It can serve independently built hierarchies with different dimensions.
/// Buffers are resized during preparation and initialized before every lease.
/// Recursive calls borrow disjoint tails of a stack arena; no buffer leaves the
/// arena, including on an error or unwind. No global state or locks are used.
///
/// Only traversal-owned scratch is covered. The existing MAP and structural
/// projection implementations still allocate internally; this type does not
/// yet make the complete hierarchy application allocation-free.
#[derive(Debug, Default)]
pub struct CycleScreenedMapHierarchyWorkspace {
    buffers: Vec<Vec<f64>>,
}

impl CycleScreenedMapHierarchyWorkspace {
    /// Construct an empty workspace. The first application prepares its layout.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Exact capacity-based retained heap bytes, with checked arithmetic.
    ///
    /// Includes the outer vector's descriptor storage and all retained f64
    /// buffers, including inactive buffers from a previously larger hierarchy.
    /// Excludes the inline workspace object, allocator metadata, the immutable
    /// hierarchy, caller input/output, and allocations in MAP/projection calls.
    pub fn retained_bytes(&self) -> Result<usize, MultiwayError> {
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

    /// Number of retained vectors, including currently inactive buffers.
    #[must_use]
    pub fn retained_buffer_count(&self) -> usize {
        self.buffers.len()
    }

    fn prepare(&mut self, problems: &[ThreeWayProblem]) -> Result<(), MultiwayError> {
        // A successfully constructed hierarchy always contains its finest level.
        let depth = problems.len() - 1;
        let count = required_buffer_count(depth)?;
        if count > self.buffers.len() {
            self.buffers
                .try_reserve_exact(count - self.buffers.len())
                .map_err(allocation_error)?;
            self.buffers.resize_with(count, Vec::new);
        }
        resize_buffer(&mut self.buffers[0], problems[0].dimension())?;
        for (level, pair) in problems.windows(2).enumerate() {
            // The checked total buffer count bounds these index operations.
            let start = 1 + FRAME_BUFFERS * level;
            let fine = pair[0].dimension();
            let coarse = pair[1].dimension();
            let lengths = [fine, fine, coarse, coarse, fine, fine, fine];
            for (buffer, length) in self.buffers[start..start + FRAME_BUFFERS]
                .iter_mut()
                .zip(lengths)
            {
                resize_buffer(buffer, length)?;
            }
        }
        self.retained_bytes()?;
        Ok(())
    }
}

impl CycleScreenedMapHierarchy {
    /// Prepare a caller-owned workspace for this hierarchy's traversal.
    ///
    /// Preparation is fallible and does not run numerical operators. Later
    /// applications with this layout reuse all traversal buffer capacities.
    /// The returned workspace is not bound to this hierarchy.
    pub fn application_workspace(
        &self,
    ) -> Result<CycleScreenedMapHierarchyWorkspace, MultiwayError> {
        let mut workspace = CycleScreenedMapHierarchyWorkspace::new();
        workspace.prepare(&self.problems)?;
        Ok(workspace)
    }

    /// Apply the same V-cycle as [`Preconditioner::apply`] using retained scratch.
    ///
    /// Both vector dimensions are checked before preparation or output mutation.
    /// A changed layout may grow the workspace; an allocation failure can leave
    /// partial extra capacity retained, but never changes caller output. The
    /// numerical result is copied to `out` only after the full cycle succeeds.
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
        workspace.prepare(&self.problems)?;
        let (solution, scratch) = workspace
            .buffers
            .split_first_mut()
            .expect("preparation retains a result buffer");
        self.apply_level_into(0, rhs, solution, scratch)?;
        out.copy_from_slice(solution);
        Ok(())
    }

    // One numerical recurrence for both public entry points. Each frame leases
    // a fixed prefix and passes the remaining arena to its child. Rust's scoped
    // mutable borrows enforce non-aliasing and LIFO release even during unwind.
    fn apply_level_into(
        &self,
        level: usize,
        rhs: &[f64],
        solution: &mut [f64],
        scratch: &mut [Vec<f64>],
    ) -> Result<(), MultiwayError> {
        let problem = &self.problems[level];
        if rhs.len() != problem.dimension() {
            return Err(crate::error::dimension(
                "CycleScreenedMapHierarchy::apply_level",
                problem.dimension(),
                rhs.len(),
            ));
        }
        solution.fill(0.0);
        if level == self.aggregations.len() {
            self.terminal.solve_into(rhs, solution)?;
            problem.components().project_structural_range(solution)?;
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
        compatible_rhs.fill(0.0);
        compatible_rhs.copy_from_slice(rhs);
        problem
            .components()
            .project_structural_range(compatible_rhs)?;
        self.smoothers[level].apply(compatible_rhs, solution)?;

        residual.fill(0.0);
        problem.residual_into(compatible_rhs, solution, residual)?;
        let coarse_problem = &self.problems[level + 1];
        coarse_rhs.fill(0.0);
        self.aggregations[level].restrict(residual, coarse_rhs)?;
        coarse_problem
            .components()
            .project_structural_range(coarse_rhs)?;
        self.apply_level_into(level + 1, coarse_rhs, coarse_solution, child_scratch)?;
        prolonged.fill(0.0);
        self.aggregations[level].prolong(coarse_solution, prolonged)?;
        add_assign(solution, prolonged);

        post_residual.fill(0.0);
        problem.residual_into(compatible_rhs, solution, post_residual)?;
        post.fill(0.0);
        self.smoothers[level].apply(post_residual, post)?;
        add_assign(solution, post);
        problem.components().project_structural_range(solution)?;
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
        .and_then(|count| count.checked_add(1))
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
        let expected = workspace.buffers.capacity() * core::mem::size_of::<Vec<f64>>()
            + workspace
                .buffers
                .iter()
                .map(|v| v.capacity() * 8)
                .sum::<usize>();
        assert_eq!(workspace.retained_bytes().unwrap(), expected);
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
        for buffer in &mut workspace.buffers {
            buffer.fill(f64::NAN);
        }
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (solution, scratch) = workspace.buffers.split_first_mut().unwrap();
            // Intentionally violate the private prepared-layout precondition to
            // panic in a child while its parent holds live scratch leases.
            hierarchy
                .apply_level_into(0, &rhs, solution, &mut scratch[..FRAME_BUFFERS])
                .unwrap();
        }));
        assert!(caught.is_err());
        let mut actual = vec![f64::NAN; rhs.len()];
        hierarchy
            .apply_with_workspace(&rhs, &mut actual, &mut workspace)
            .unwrap();
        assert_eq!(actual, expected);
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
