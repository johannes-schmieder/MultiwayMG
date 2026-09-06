//! One fixed symmetric V-cycle recurrence for ordinary and prepared owners.
use crate::{DensePseudoinverseWorkspace, MultiwayError};

pub(crate) const FRAME_BUFFERS: usize = 4;

pub(crate) trait CycleActions {
    type LevelScratch;
    fn level_count(&self) -> usize;
    fn dimension_at(&self, level: usize) -> usize;
    fn project(
        &self,
        level: usize,
        values: &mut [f64],
        scratch: &mut Self::LevelScratch,
    ) -> Result<(), MultiwayError>;
    fn smooth(
        &self,
        level: usize,
        rhs: &[f64],
        out: &mut [f64],
        scratch: &mut Self::LevelScratch,
    ) -> Result<(), MultiwayError>;
    fn residual(
        &self,
        level: usize,
        rhs: &[f64],
        x: &[f64],
        out: &mut [f64],
    ) -> Result<(), MultiwayError>;
    fn restrict(&self, level: usize, fine: &[f64], coarse: &mut [f64])
    -> Result<(), MultiwayError>;
    fn prolong(&self, level: usize, coarse: &[f64], fine: &mut [f64]) -> Result<(), MultiwayError>;
    fn terminal(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut DensePseudoinverseWorkspace,
    ) -> Result<(), MultiwayError>;
}

// Disjoint tails lend scratch to children without moving buffers on errors/unwind.
pub(crate) fn apply_level<A: CycleActions>(
    actions: &A,
    level: usize,
    rhs: &[f64],
    solution: &mut [f64],
    scratch: &mut [Vec<f64>],
    operator_levels: &mut [A::LevelScratch],
    terminal: &mut DensePseudoinverseWorkspace,
) -> Result<(), MultiwayError> {
    #[cfg(feature = "profiling")]
    let _profile_span =
        multiway_incidence::profiling::span(multiway_incidence::profiling::Phase::Cycle);

    if rhs.len() != actions.dimension_at(level) {
        return Err(crate::error::dimension(
            "CycleScreenedMapHierarchy::apply_level",
            actions.dimension_at(level),
            rhs.len(),
        ));
    }
    let (state, children) = operator_levels
        .split_first_mut()
        .expect("prepared operator level");
    if level + 1 == actions.level_count() {
        actions.terminal(rhs, solution, terminal)?;
        actions.project(level, solution, state)?;
        return Ok(());
    }
    let (frame, child_scratch) = scratch.split_at_mut(FRAME_BUFFERS);
    let [compatible_rhs, residual, coarse_rhs, coarse_solution] = frame else {
        unreachable!("a prepared nonterminal frame has four buffers");
    };
    compatible_rhs.copy_from_slice(rhs);
    actions.project(level, compatible_rhs, state)?;
    actions.smooth(level, compatible_rhs, solution, state)?;
    actions.residual(level, compatible_rhs, solution, residual)?;
    actions.restrict(level, residual, coarse_rhs)?;
    actions.project(level + 1, coarse_rhs, &mut children[0])?;
    apply_level(
        actions,
        level + 1,
        coarse_rhs,
        coarse_solution,
        child_scratch,
        children,
        terminal,
    )?;
    // The pre-residual is dead after restriction. Reuse its storage for the
    // prolongated correction, then overwrite it with the post-residual.
    actions.prolong(level, coarse_solution, residual)?;
    add_assign(solution, residual);
    actions.residual(level, compatible_rhs, solution, residual)?;
    // The compatible RHS dies after that residual. The post smoother copies
    // its input into private scratch before publishing its correction here.
    actions.smooth(level, residual, compatible_rhs, state)?;
    add_assign(solution, compatible_rhs);
    actions.project(level, solution, state)?;
    Ok(())
}
fn add_assign(destination: &mut [f64], source: &[f64]) {
    for (left, &right) in destination.iter_mut().zip(source) {
        *left += right;
    }
}
