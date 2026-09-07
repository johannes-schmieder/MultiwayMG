//! One fixed symmetric V-cycle recurrence for ordinary and prepared owners.
use crate::{DensePseudoinverseWorkspace, MultiwayError};

pub(crate) const FRAME_BUFFERS: usize = 4;

// Neither modal terminal scratch nor the tuple image is live across another
// operator action. One shared image is reborrowed throughout recursion.
pub(crate) struct CycleSharedScratch<'a> {
    pub(crate) terminal: &'a mut DensePseudoinverseWorkspace,
    pub(crate) image: &'a mut [f64],
}

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
        image: &mut [f64],
    ) -> Result<(), MultiwayError>;
    fn restrict(&self, level: usize, fine: &[f64], coarse: &mut [f64])
    -> Result<(), MultiwayError>;
    fn prolong_add(
        &self,
        level: usize,
        coarse: &[f64],
        fine: &mut [f64],
    ) -> Result<(), MultiwayError>;
    fn terminal(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut DensePseudoinverseWorkspace,
    ) -> Result<(), MultiwayError>;
}

// A private storage adapter keeps the arithmetic recurrence common while the
// prepared owner uses one flat arena and the ordinary owner retains its vectors.
// Splitting consumes the borrow; frame slices and child tails cannot overlap.
pub(crate) trait CycleScratch<'a>: Sized {
    fn split_frame(self, fine: usize, coarse: usize) -> ([&'a mut [f64]; 4], Self);
}
impl<'a> CycleScratch<'a> for &'a mut [Vec<f64>] {
    fn split_frame(self, fine: usize, coarse: usize) -> ([&'a mut [f64]; 4], Self) {
        let (frame, children) = self.split_at_mut(FRAME_BUFFERS);
        let [rhs, residual, coarse_rhs, coarse_solution] = frame else {
            unreachable!("prepared ordinary cycle frame");
        };
        debug_assert_eq!(
            [
                rhs.len(),
                residual.len(),
                coarse_rhs.len(),
                coarse_solution.len()
            ],
            [fine, fine, coarse, coarse]
        );
        (
            [
                rhs.as_mut_slice(),
                residual.as_mut_slice(),
                coarse_rhs.as_mut_slice(),
                coarse_solution.as_mut_slice(),
            ],
            children,
        )
    }
}
impl<'a> CycleScratch<'a> for &'a mut [f64] {
    fn split_frame(self, fine: usize, coarse: usize) -> ([&'a mut [f64]; 4], Self) {
        let (rhs, tail) = self.split_at_mut(fine);
        let (residual, tail) = tail.split_at_mut(fine);
        let (coarse_rhs, tail) = tail.split_at_mut(coarse);
        let (coarse_solution, tail) = tail.split_at_mut(coarse);
        ([rhs, residual, coarse_rhs, coarse_solution], tail)
    }
}

// Disjoint tails lend scratch to children without moving buffers on errors/unwind.
pub(crate) fn apply_level<'a, A: CycleActions, S: CycleScratch<'a>>(
    actions: &A,
    level: usize,
    rhs: &[f64],
    solution: &mut [f64],
    scratch: S,
    operator_levels: &mut [A::LevelScratch],
    shared: &mut CycleSharedScratch<'_>,
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
        actions.terminal(rhs, solution, shared.terminal)?;
        actions.project(level, solution, state)?;
        return Ok(());
    }
    let ([compatible_rhs, residual, coarse_rhs, coarse_solution], child_scratch) =
        scratch.split_frame(actions.dimension_at(level), actions.dimension_at(level + 1));
    compatible_rhs.copy_from_slice(rhs);
    actions.project(level, compatible_rhs, state)?;
    actions.smooth(level, compatible_rhs, solution, state)?;
    actions.residual(level, compatible_rhs, solution, residual, shared.image)?;
    actions.restrict(level, residual, coarse_rhs)?;
    actions.project(level + 1, coarse_rhs, &mut children[0])?;
    apply_level(
        actions,
        level + 1,
        coarse_rhs,
        coarse_solution,
        child_scratch,
        children,
        shared,
    )?;
    // Add the correction with the original coefficient-wise addition, avoiding
    // a temporary prolongation store/read. The residual stays dead until the
    // following operator overwrites it for post-smoothing.
    actions.prolong_add(level, coarse_solution, solution)?;
    actions.residual(level, compatible_rhs, solution, residual, shared.image)?;
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
