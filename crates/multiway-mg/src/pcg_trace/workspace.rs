//! Caller-owned coefficient, projection and trace storage for traced PCG.

use multiway_incidence::StructuralProjectionWorkspace;

use super::{PcgTraceOptions, PcgTraceResult, PcgTraceResultRef, PcgTraceSample, PcgTraceSummary};
use crate::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, MultiwayError, Preconditioner,
    ThreeWayProblem,
};

const CONTEXT: &str = "PcgTraceWorkspace";

/// Reusable traced-PCG storage, including the solution and all residual samples.
///
/// This stores no numerical operator or weights. Projection scratch is bound to
/// the submitted problem's immutable components; independent builds require
/// explicit [`Self::try_prepare_for`]. Every solve starts from zero, not from a
/// cached solution. A returned [`PcgTraceResultRef`] borrows this storage, so it
/// cannot remain live across another mutable workspace use.
#[derive(Debug, Default)]
pub struct PcgTraceWorkspace {
    pub(super) projected_rhs: Vec<f64>,
    pub(super) solution: Vec<f64>,
    pub(super) residual: Vec<f64>,
    pub(super) preconditioned: Vec<f64>,
    pub(super) direction: Vec<f64>,
    pub(super) applied: Vec<f64>,
    pub(super) projection: Option<StructuralProjectionWorkspace>,
    pub(super) samples: Vec<PcgTraceSample>,
}

impl PcgTraceWorkspace {
    /// Empty storage; explicitly prepare it before a borrowed-result solve.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fallibly prepare coefficient and trace storage for the full iteration budget.
    pub fn try_new(
        problem: &ThreeWayProblem,
        options: PcgTraceOptions,
    ) -> Result<Self, MultiwayError> {
        let mut workspace = Self::new();
        workspace.try_prepare_for(problem, options)?;
        Ok(workspace)
    }

    /// Minimum exclusive heap payload for this dimension and iteration budget.
    ///
    /// Includes six coefficient vectors, component-projection scratch, and
    /// `max_iterations + 1` trace records. Excludes inline descriptors, shared
    /// identity control blocks, allocator overhead, operators and caller input.
    /// Rejects unrepresentable byte sizes and worst-case work counters.
    pub fn required_bytes(
        problem: &ThreeWayProblem,
        options: PcgTraceOptions,
    ) -> Result<usize, MultiwayError> {
        super::validate_options(options)?;
        let vectors = bytes::<f64>(problem.dimension())?
            .checked_mul(6)
            .ok_or_else(overflow)?;
        let trace = bytes::<PcgTraceSample>(required_samples(options.max_iterations)?)?;
        let projection = problem.components().projection_workspace_required_bytes()?;
        vectors
            .checked_add(trace)
            .and_then(|n| n.checked_add(projection))
            .ok_or_else(overflow)
    }

    /// Explicit setup boundary for a new problem or a larger iteration budget.
    ///
    /// Retains capacities. All vector/trace reservations precede publication of
    /// new lengths and component binding. Failed setup may grow capacities, but
    /// leaves previous lengths, contents and binding usable. Successful setup
    /// clears the old trace. Scratch is reinitialized by the next solve.
    pub fn try_prepare_for(
        &mut self,
        problem: &ThreeWayProblem,
        options: PcgTraceOptions,
    ) -> Result<(), MultiwayError> {
        Self::required_bytes(problem, options)?;
        let dimension = problem.dimension();
        let samples = required_samples(options.max_iterations)?;
        for vector in [
            &mut self.projected_rhs,
            &mut self.solution,
            &mut self.residual,
            &mut self.preconditioned,
            &mut self.direction,
            &mut self.applied,
        ] {
            reserve(vector, dimension)?;
        }
        reserve(&mut self.samples, samples)?;
        if let Some(projection) = self.projection.as_mut() {
            projection.try_prepare_for(problem.components())?;
        } else {
            self.projection = Some(problem.components().try_projection_workspace()?);
        }
        for vector in [
            &mut self.projected_rhs,
            &mut self.solution,
            &mut self.residual,
            &mut self.preconditioned,
            &mut self.direction,
            &mut self.applied,
        ] {
            vector.resize(dimension, 0.0);
        }
        self.samples.clear();
        Ok(())
    }

    /// Exclusive retained vector/trace/projection payload, using actual capacities.
    ///
    /// Includes unused capacity after larger solves. The exclusions are the same
    /// as [`Self::required_bytes`]; this is not total peak memory or process RSS.
    pub fn retained_bytes(&self) -> Result<usize, MultiwayError> {
        let initial = self
            .trace_retained_bytes()?
            .checked_add(
                self.projection
                    .as_ref()
                    .map_or(0, StructuralProjectionWorkspace::retained_bytes),
            )
            .ok_or_else(overflow)?;
        [
            &self.projected_rhs,
            &self.solution,
            &self.residual,
            &self.preconditioned,
            &self.direction,
            &self.applied,
        ]
        .into_iter()
        .try_fold(initial, |total, vector| {
            total
                .checked_add(bytes::<f64>(vector.capacity())?)
                .ok_or_else(overflow)
        })
    }

    /// Retained trace payload, already included in [`Self::retained_bytes`].
    pub fn trace_retained_bytes(&self) -> Result<usize, MultiwayError> {
        bytes::<PcgTraceSample>(self.samples.capacity())
    }

    /// Number of trace records that fit without growing storage.
    #[must_use]
    pub fn trace_capacity(&self) -> usize {
        self.samples.capacity()
    }

    pub(super) fn validate(
        &self,
        problem: &ThreeWayProblem,
        options: PcgTraceOptions,
    ) -> Result<(), MultiwayError> {
        Self::required_bytes(problem, options)?;
        for vector in [
            &self.projected_rhs,
            &self.solution,
            &self.residual,
            &self.preconditioned,
            &self.direction,
            &self.applied,
        ] {
            if vector.len() != problem.dimension() {
                return Err(crate::error::dimension(
                    CONTEXT,
                    problem.dimension(),
                    vector.len(),
                ));
            }
        }
        if !self
            .projection
            .as_ref()
            .is_some_and(|p| p.is_compatible_with(problem.components()))
        {
            return Err(
                multiway_incidence::IncidenceError::WorkspaceBindingMismatch { context: CONTEXT }
                    .into(),
            );
        }
        let required = required_samples(options.max_iterations)?;
        if self.samples.capacity() < required {
            return Err(crate::error::dimension(
                "PcgTraceWorkspace trace capacity",
                required,
                self.samples.capacity(),
            ));
        }
        Ok(())
    }

    pub(super) fn result_ref(&self, summary: PcgTraceSummary) -> PcgTraceResultRef<'_> {
        PcgTraceResultRef::new(&self.solution, &self.samples, summary)
    }

    pub(super) fn into_result(self, summary: PcgTraceSummary) -> PcgTraceResult {
        PcgTraceResult::from_parts(self.solution, self.samples, summary)
    }
}

/// Solve traced PCG with explicitly prepared outer storage and a generic preconditioner.
///
/// Options, dimensions, component binding and full trace capacity are validated
/// before any workspace mutation. No outer storage is allocated on a valid call;
/// the supplied preconditioner remains responsible for its own allocations.
/// Numerical errors may overwrite internal scratch but return no result view.
/// The same workspace can be used again after an error or caught unwind.
pub fn solve_projected_pcg_traced_with_workspace<'a, P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    rhs: &[f64],
    preconditioner: &P,
    options: PcgTraceOptions,
    workspace: &'a mut PcgTraceWorkspace,
) -> Result<PcgTraceResultRef<'a>, MultiwayError> {
    let summary = super::run(
        problem,
        rhs,
        preconditioner,
        options,
        workspace,
        |right, out| preconditioner.apply(right, out),
    )?;
    Ok(workspace.result_ref(summary))
}

/// Solve traced PCG with retained outer storage and caller-owned MAP hierarchy scratch.
///
/// Returns a borrowed solution and trace without output cloning. After both
/// workspaces have been prepared, the entire successful solve allocates nothing.
/// Outer storage requires explicit preparation; hierarchy storage retains its
/// existing automatic preparation semantics and may allocate if unprepared or
/// reused with a different layout. No hierarchy application/preparation occurs
/// for a zero projected RHS or an invalid initial numerical state.
///
/// No previous result is returned on error. This is a candidate solve, not a
/// replacement for independent certification against the submitted operator.
pub fn solve_projected_pcg_traced_with_workspaces<'a>(
    problem: &ThreeWayProblem,
    rhs: &[f64],
    hierarchy: &CycleScreenedMapHierarchy,
    options: PcgTraceOptions,
    workspace: &'a mut PcgTraceWorkspace,
    hierarchy_workspace: &mut CycleScreenedMapHierarchyWorkspace,
) -> Result<PcgTraceResultRef<'a>, MultiwayError> {
    let summary = super::run(problem, rhs, hierarchy, options, workspace, |right, out| {
        hierarchy.apply_with_workspace(right, out, hierarchy_workspace)
    })?;
    Ok(workspace.result_ref(summary))
}

fn required_samples(iterations: usize) -> Result<usize, MultiwayError> {
    // The original loop uses two Gramian calls per completed iteration and a
    // final nonconverged preconditioner call. Never allow those counters to wrap.
    iterations.checked_mul(2).ok_or_else(overflow)?;
    iterations.checked_add(1).ok_or_else(overflow)
}

fn bytes<T>(count: usize) -> Result<usize, MultiwayError> {
    count
        .checked_mul(core::mem::size_of::<T>())
        .ok_or_else(overflow)
}

fn reserve<T>(vector: &mut Vec<T>, length: usize) -> Result<(), MultiwayError> {
    bytes::<T>(length)?;
    if length > vector.len() {
        vector
            .try_reserve_exact(length - vector.len())
            .map_err(|source| MultiwayError::WorkspaceAllocation {
                context: CONTEXT,
                source,
            })?;
    }
    Ok(())
}

fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow { context: CONTEXT }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisoned_vectors_and_trace_are_overwritten_before_use() {
        let problem =
            ThreeWayProblem::from_observations([2; 3], &[[0, 0, 0], [1, 1, 1]], &[1.0, 2.0])
                .unwrap();
        let hierarchy =
            CycleScreenedMapHierarchy::from_maps(problem.clone(), vec![], 1.0e-12).unwrap();
        let options = PcgTraceOptions::default();
        let mut input = vec![0.0; problem.dimension()];
        problem
            .apply_gramian(&[1.0, -2.0, 3.0, -4.0, 5.0, -6.0], &mut input)
            .unwrap();
        let expected =
            crate::solve_projected_pcg_traced(&problem, &input, &hierarchy, options).unwrap();
        let mut workspace = PcgTraceWorkspace::try_new(&problem, options).unwrap();
        let bytes = workspace.retained_bytes().unwrap();
        for vector in [
            &mut workspace.projected_rhs,
            &mut workspace.solution,
            &mut workspace.residual,
            &mut workspace.preconditioned,
            &mut workspace.direction,
            &mut workspace.applied,
        ] {
            vector.fill(f64::NAN);
        }
        workspace.samples.push(PcgTraceSample {
            iteration: 999,
            residual_norm: f64::NAN,
            relative_residual: f64::NAN,
        });
        let actual = solve_projected_pcg_traced_with_workspace(
            &problem,
            &input,
            &hierarchy,
            options,
            &mut workspace,
        )
        .unwrap()
        .to_owned();
        assert_eq!(actual, expected);
        assert_eq!(workspace.retained_bytes().unwrap(), bytes);
    }

    #[test]
    fn counter_and_capacity_overflow_preserve_existing_storage() {
        assert!(required_samples(usize::MAX).is_err());
        assert!(required_samples(usize::MAX / 2 + 1).is_err());
        let mut values = vec![1.0, -2.0];
        let pointer = values.as_ptr();
        let capacity = values.capacity();
        assert!(reserve(&mut values, usize::MAX).is_err());
        assert!(reserve(&mut values, isize::MAX as usize / 8 + 1).is_err());
        assert_eq!(values, [1.0, -2.0]);
        assert_eq!(values.as_ptr(), pointer);
        assert_eq!(values.capacity(), capacity);
    }
}
