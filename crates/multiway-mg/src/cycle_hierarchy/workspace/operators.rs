//! Per-level projection/MAP scratch and anonymous terminal storage.

use multiway_incidence::StructuralProjectionWorkspace;

use super::{CycleScreenedMapHierarchy, allocation_error, size_overflow};
use crate::{DensePseudoinverseWorkspace, MultiwayError, SymmetricMapWorkspace};

#[derive(Debug)]
pub(super) struct LevelWorkspace {
    pub(super) projection: StructuralProjectionWorkspace,
    pub(super) map: Option<SymmetricMapWorkspace>,
}

#[derive(Debug, Default)]
pub(super) struct OperatorWorkspaces {
    pub(super) levels: Vec<LevelWorkspace>,
    pub(super) terminal: DensePseudoinverseWorkspace,
}

impl OperatorWorkspaces {
    pub(super) fn prepare(&mut self, hierarchy: &CycleScreenedMapHierarchy) -> Result<(), MultiwayError> {
        required_bytes(hierarchy)?;
        let count = hierarchy.problems.len();
        if count > self.levels.len() {
            self.levels.try_reserve_exact(count - self.levels.len()).map_err(allocation_error)?;
        }
        for (level, problem) in hierarchy.problems.iter().enumerate() {
            if level == self.levels.len() {
                self.levels.push(LevelWorkspace {
                    projection: problem.components().try_projection_workspace()?,
                    map: None,
                });
            }
            let state = &mut self.levels[level];
            // Deliberate setup boundary: application itself never bypasses binding.
            state.projection.try_prepare_for(problem.components())?;
            if let Some(smoother) = hierarchy.smoothers.get(level) {
                if let Some(workspace) = state.map.as_mut() {
                    workspace.try_prepare_for(smoother)?;
                } else {
                    state.map = Some(smoother.application_workspace()?);
                }
            }
            // Inactive levels and former nonterminal MAP scratch retain capacity.
        }
        self.terminal.try_prepare_for(&hierarchy.terminal)?;
        Ok(())
    }

    pub(super) fn retained_bytes(&self) -> Result<usize, MultiwayError> {
        let descriptors = self.levels.capacity().checked_mul(core::mem::size_of::<LevelWorkspace>()).ok_or_else(size_overflow)?;
        let initial = descriptors.checked_add(self.terminal.retained_bytes()?).ok_or_else(size_overflow)?;
        self.levels.iter().try_fold(initial, |total, level| {
            let map = level.map.as_ref().map_or(Ok(0), SymmetricMapWorkspace::retained_bytes)?;
            total.checked_add(level.projection.retained_bytes()).and_then(|n| n.checked_add(map)).ok_or_else(size_overflow)
        })
    }
}

pub(super) fn required_bytes(hierarchy: &CycleScreenedMapHierarchy) -> Result<usize, MultiwayError> {
    let descriptors = hierarchy.problems.len().checked_mul(core::mem::size_of::<LevelWorkspace>()).ok_or_else(size_overflow)?;
    let initial = descriptors.checked_add(hierarchy.terminal.workspace_required_bytes()?).ok_or_else(size_overflow)?;
    hierarchy.problems.iter().enumerate().try_fold(initial, |total, (level, problem)| {
        let projection = problem.components().projection_workspace_required_bytes()?;
        let map = hierarchy.smoothers.get(level).map_or(Ok(0), crate::SymmetricMapPreconditioner::workspace_required_bytes)?;
        total.checked_add(projection).and_then(|n| n.checked_add(map)).ok_or_else(size_overflow)
    })
}
