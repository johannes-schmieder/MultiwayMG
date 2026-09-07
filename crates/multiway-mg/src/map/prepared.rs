//! Immutable current-frame MAP and caller-owned mutable scratch.
use super::kernel::{MapData, sweep, sweep_grouped};
use crate::MultiwayError;
use multiway_incidence::{
    PreparedStructuralProjectionWorkspace, PreparedTupleGrouping, ThreeWayWeightFrame,
    WeightFrameBinding,
};

/// One fixed symmetric MAP correction borrowing an exact numerical frame.
///
/// No topology, weights, degrees or components are copied. The underlying factor
/// sweep and structural projection arithmetic is shared with the ordinary MAP
/// implementation. This action uses caller-owned scratch and serial kernels.
/// It is not itself an original-operator convergence or rank certificate.
#[derive(Debug, Clone, Copy)]
pub struct PreparedSymmetricMap<'frame, 'topology> {
    frame: &'frame ThreeWayWeightFrame<'topology>,
}

/// Explicit stable-row MAP alternative for an exact immutable numerical frame.
///
/// Borrows grouping and numerical state separately. Reuses the same caller-owned
/// MAP workspace; row order, tuple subtraction order, mul_add association,
/// rounded middle and structural projections match the scalar sweep. No layout
/// is selected implicitly and no extra application arrays are required.
#[derive(Debug, Clone, Copy)]
pub struct PreparedGroupedSymmetricMap<'groups, 'frame, 'topology> {
    scalar: PreparedSymmetricMap<'frame, 'topology>,
    grouping: &'groups PreparedTupleGrouping<'topology>,
}
impl<'groups, 'frame, 'topology> PreparedGroupedSymmetricMap<'groups, 'frame, 'topology> {
    /// Exact current numerical frame.
    #[must_use]
    pub const fn frame(&self) -> &'frame ThreeWayWeightFrame<'topology> {
        self.scalar.frame
    }
    /// Borrowed weights-free grouping owner, counted separately from scratch.
    #[must_use]
    pub const fn grouping(&self) -> &'groups PreparedTupleGrouping<'topology> {
        self.grouping
    }
    /// Same exclusive scratch requirement as scalar MAP.
    pub fn workspace_required_bytes(&self) -> Result<usize, MultiwayError> {
        self.scalar.workspace_required_bytes()
    }
    /// Prepare the existing three-vector/projector MAP scratch, without layout copies.
    pub fn application_workspace(
        &self,
    ) -> Result<PreparedMapWorkspace<'frame, 'topology>, MultiwayError> {
        self.scalar.application_workspace()
    }
    /// Apply ordered grouped MAP, with the scalar validation/transactional boundary.
    pub fn apply_with_workspace(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut PreparedMapWorkspace<'_, '_>,
    ) -> Result<(), MultiwayError> {
        self.scalar
            .apply_impl(rhs, output, workspace, Some(self.grouping))
    }
}

/// All mutable storage for one exact current-frame MAP application.
///
/// Binding validation is exact even though the vectors contain only scratch.
/// No implicit preparation, rebind, pool, locking or allocation occurs on apply.
/// A workspace cannot outlive or permit replacement of its numerical frame.
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
/// use multiway_mg::PreparedSymmetricMap;
/// let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let w = {
///     let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
///     PreparedSymmetricMap::new(&f).application_workspace().unwrap()
/// };
/// assert!(w.retained_payload_bytes().is_ok());
/// ```
#[derive(Debug)]
pub struct PreparedMapWorkspace<'frame, 'topology> {
    binding: WeightFrameBinding<'frame, 'topology>,
    projection: PreparedStructuralProjectionWorkspace<'topology>,
    compatible_rhs: Vec<f64>,
    forward: Vec<f64>,
    solution: Vec<f64>,
}

impl<'frame, 'topology> PreparedSymmetricMap<'frame, 'topology> {
    /// Borrow an immutable validated frame without allocating numerical state.
    #[must_use]
    pub const fn new(frame: &'frame ThreeWayWeightFrame<'topology>) -> Self {
        Self { frame }
    }

    /// Bind optional grouping from this frame's exact structural owner.
    pub fn with_grouping<'groups>(
        self,
        grouping: &'groups PreparedTupleGrouping<'topology>,
    ) -> Result<PreparedGroupedSymmetricMap<'groups, 'frame, 'topology>, MultiwayError> {
        grouping.validate_for(self.frame.topology())?;
        Ok(PreparedGroupedSymmetricMap {
            scalar: self,
            grouping,
        })
    }

    /// Exact borrowed numerical owner.
    #[must_use]
    pub const fn frame(&self) -> &'frame ThreeWayWeightFrame<'topology> {
        self.frame
    }

    /// Coefficient dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.frame.diagonal().len()
    }

    /// Checked exclusive requested scratch payload; excludes all immutable owners.
    pub fn workspace_required_bytes(&self) -> Result<usize, MultiwayError> {
        let vectors = self
            .dimension()
            .checked_mul(3)
            .and_then(|n| n.checked_mul(8))
            .ok_or_else(overflow)?;
        vectors
            .checked_add(
                self.frame
                    .topology()
                    .projection_workspace_required_bytes()?,
            )
            .ok_or_else(overflow)
    }

    /// Fallibly allocate every scratch array at an explicit setup boundary.
    pub fn application_workspace(
        &self,
    ) -> Result<PreparedMapWorkspace<'frame, 'topology>, MultiwayError> {
        self.workspace_with(&mut |_| Ok(()))
    }

    pub(crate) fn workspace_with<F>(
        &self,
        before: &mut F,
    ) -> Result<PreparedMapWorkspace<'frame, 'topology>, MultiwayError>
    where
        F: FnMut(&'static str) -> Result<(), MultiwayError>,
    {
        self.workspace_required_bytes()?;
        before("prepared MAP projection")?;
        let projection = self.frame.topology().try_projection_workspace()?;
        let dimension = self.dimension();
        Ok(PreparedMapWorkspace {
            binding: self.frame.binding(),
            projection,
            compatible_rhs: vector(dimension, before)?,
            forward: vector(dimension, before)?,
            solution: vector(dimension, before)?,
        })
    }

    /// Apply shared serial MAP arithmetic without allocating, with transactional output.
    ///
    /// Both dimensions, the exact frame, and finite RHS are checked before scratch
    /// mutation. Every active vector is initialized on each application. A finite
    /// input whose projection or sweep is not representable returns a typed error
    /// without publishing the output. A later valid application reuses the storage.
    pub fn apply_with_workspace(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut PreparedMapWorkspace<'_, '_>,
    ) -> Result<(), MultiwayError> {
        self.apply_impl(rhs, output, workspace, None)
    }
    fn apply_impl(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut PreparedMapWorkspace<'_, '_>,
        grouping: Option<&PreparedTupleGrouping<'_>>,
    ) -> Result<(), MultiwayError> {
        if rhs.len() != self.dimension() {
            return Err(crate::error::dimension(
                "prepared MAP rhs",
                self.dimension(),
                rhs.len(),
            ));
        }
        if output.len() != self.dimension() {
            return Err(crate::error::dimension(
                "prepared MAP output",
                self.dimension(),
                output.len(),
            ));
        }
        workspace.validate_for(self.frame)?;
        finite(rhs, "prepared MAP rhs")?;
        let PreparedMapWorkspace {
            compatible_rhs,
            forward,
            solution,
            projection,
            ..
        } = workspace;
        compatible_rhs.copy_from_slice(rhs);
        self.frame
            .topology()
            .project_structural_range_with_workspace(compatible_rhs, projection)?;
        finite(compatible_rhs, "prepared MAP projected rhs")?;
        let data = MapData {
            topology: self.frame.topology().topology(),
            weights: self.frame.weights(),
            diagonal: self.frame.diagonal(),
        };
        if let Some(grouping) = grouping {
            sweep_grouped(data, grouping, compatible_rhs, forward, solution);
        } else {
            sweep(data, compatible_rhs, forward, solution);
        }
        finite(solution, "prepared MAP sweep")?;
        self.frame
            .topology()
            .project_structural_range_with_workspace(solution, projection)?;
        finite(solution, "prepared MAP projected solution")?;
        output.copy_from_slice(solution);
        Ok(())
    }
}

impl PreparedMapWorkspace<'_, '_> {
    /// Reject a different immutable numerical generation before any writes.
    pub fn validate_for(&self, frame: &ThreeWayWeightFrame<'_>) -> Result<(), MultiwayError> {
        self.binding.validate_for(frame)?;
        self.projection.validate_for(frame.topology())?;
        Ok(())
    }

    /// Exclusive retained vector/projection capacities, including spare capacity.
    ///
    /// Excludes the inline root, borrowed owners and allocator overhead.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        [&self.compatible_rhs, &self.forward, &self.solution]
            .iter()
            .try_fold(
                self.projection.retained_payload_bytes()?,
                |total, values| {
                    values
                        .capacity()
                        .checked_mul(8)
                        .and_then(|n| total.checked_add(n))
                        .ok_or_else(overflow)
                },
            )
    }
}

fn vector<F>(dimension: usize, before: &mut F) -> Result<Vec<f64>, MultiwayError>
where
    F: FnMut(&'static str) -> Result<(), MultiwayError>,
{
    let bytes = dimension
        .checked_mul(8)
        .filter(|&n| n <= isize::MAX as usize)
        .ok_or_else(overflow)?;
    if bytes > 0 {
        before("prepared MAP vector")?;
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(dimension)
        .map_err(|source| MultiwayError::WorkspaceAllocation {
            context: "prepared MAP vector",
            source,
        })?;
    result.resize(dimension, 0.0);
    Ok(result)
}
fn finite(values: &[f64], context: &'static str) -> Result<(), MultiwayError> {
    if values.iter().any(|x| !x.is_finite()) {
        return Err(MultiwayError::NumericalFailure { context });
    }
    Ok(())
}
fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow {
        context: "prepared MAP workspace",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use multiway_incidence::{PreparedThreeWayTopology, WeightFrameInput};
    #[test]
    fn all_four_setup_boundaries_fail_without_invalidating_existing_scratch() {
        let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
        let frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
        let map = PreparedSymmetricMap::new(&frame);
        let mut old = map.application_workspace().unwrap();
        for fail_at in 0..4 {
            let mut reached = 0;
            assert!(
                map.workspace_with(&mut |context| {
                    reached += 1;
                    if reached == fail_at + 1 {
                        return Err(MultiwayError::WorkspaceNotPrepared { context });
                    }
                    Ok(())
                })
                .is_err()
            );
            assert_eq!(reached, fail_at + 1);
            let mut out = [0.0; 3];
            map.apply_with_workspace(&[1.0; 3], &mut out, &mut old)
                .unwrap();
            assert!(out.iter().all(|x| (x - 1.0 / 3.0).abs() < 1e-14));
        }
    }
}
