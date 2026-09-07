//! Explicit scalable fixed actions for the shared certified prepared solvers.
use crate::{
    GroupedGramianMode, MultiwayError, PreparedHierarchyPayloadReport, PreparedMapWorkspace,
    PreparedSolverAction, PreparedSymmetricMap,
    certificate::{bytes, ensure_finite, vector},
    prepared_action::sealed,
};
use multiway_incidence::{
    PreparedHierarchyBudget, PreparedStructuralProjectionWorkspace, PreparedTupleGrouping,
    ThreeWayWeightFrame,
};

/// Explicit fixed baseline; construction does not select a route automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedBaselineKind {
    /// Literal identity, with no action arrays or implicit preconditioning.
    /// PCG retains its outer projections; LSMR projects its final candidate.
    Identity,
    /// P D^-1 P, using positive degrees from this current frame.
    InverseDiagonal,
    /// Existing symmetric factor sweep with its structural projections.
    SymmetricMap,
}

/// A fixed baseline borrowing a current frame and optional structural grouping.
///
/// No topology, weights, factor, or dense terminal is copied or constructed.
/// These are candidate preconditioners, not convergence or numerical-rank claims.
/// The shared solver must certify every result against this original fine frame.
#[derive(Debug, Clone, Copy)]
pub struct PreparedBaseline<'frame, 'topology> {
    frame: &'frame ThreeWayWeightFrame<'topology>,
    kind: PreparedBaselineKind,
    grouping: Option<(&'frame PreparedTupleGrouping<'topology>, GroupedGramianMode)>,
}

/// Complete mutable baseline action scratch, tied to the exact baseline owner.
/// A workspace cannot outlive its selected action owner:
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
/// use multiway_mg::{PreparedBaseline, PreparedBaselineKind, PreparedSolverAction};
/// let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
/// let w = {
///     let owner = PreparedBaseline::new(&f, PreparedBaselineKind::SymmetricMap);
///     owner.application_workspace().unwrap()
/// };
/// assert!(w.retained_payload_bytes().is_ok());
/// ```
#[derive(Debug)]
pub struct PreparedBaselineWorkspace<'owner> {
    owner: &'owner PreparedBaseline<'owner, 'owner>,
    action: BaselineScratch<'owner>,
    image: Vec<f64>,
}
#[derive(Debug)]
enum BaselineScratch<'owner> {
    Identity,
    Map(PreparedMapWorkspace<'owner, 'owner>),
    Pointwise {
        projection: PreparedStructuralProjectionWorkspace<'owner>,
        result: Vec<f64>,
    },
}

impl<'frame, 'topology> PreparedBaseline<'frame, 'topology> {
    /// Borrow a validated frame with an explicit action and scalar tuple kernels.
    #[must_use]
    pub const fn new(
        frame: &'frame ThreeWayWeightFrame<'topology>,
        kind: PreparedBaselineKind,
    ) -> Self {
        Self {
            frame,
            kind,
            grouping: None,
        }
    }
    /// Bind exact structural grouping and an explicit fine Gramian mode.
    pub fn with_grouping(
        mut self,
        grouping: &'frame PreparedTupleGrouping<'topology>,
        mode: GroupedGramianMode,
    ) -> Result<Self, MultiwayError> {
        grouping.validate_for(self.frame.topology())?;
        self.grouping = Some((grouping, mode));
        Ok(self)
    }
    /// Explicit fixed action, with no adaptive stopping or hidden preparation.
    #[must_use]
    pub const fn kind(&self) -> PreparedBaselineKind {
        self.kind
    }
    /// Exact current numerical frame.
    #[must_use]
    pub const fn frame(&self) -> &'frame ThreeWayWeightFrame<'topology> {
        self.frame
    }
    fn image_len(&self) -> usize {
        if matches!(self.grouping, Some((_, GroupedGramianMode::TupleImage))) {
            self.frame.weights().len()
        } else {
            0
        }
    }
    /// Requested exclusive mutable array payload; all immutable owners are extra.
    pub fn workspace_required_bytes(&self) -> Result<usize, MultiwayError> {
        let action = if self.kind == PreparedBaselineKind::Identity {
            0
        } else if self.kind == PreparedBaselineKind::SymmetricMap {
            PreparedSymmetricMap::new(self.frame).workspace_required_bytes()?
        } else {
            add(
                bytes(self.dimension())?,
                self.frame
                    .topology()
                    .projection_workspace_required_bytes()?,
            )?
        };
        add(action, bytes(self.image_len())?)
    }
    /// Complete requested live payload, including direct owners and caller state.
    pub fn workspace_setup_payload_bound(
        &self,
        additional_live_payload_bytes: usize,
    ) -> Result<usize, MultiwayError> {
        self.requested_payload_parts()?
            .into_iter()
            .try_fold(additional_live_payload_bytes, add)
    }
    /// Admit complete requested arrays before allocation and actual retained bytes
    /// before publication. New allocator excess and RSS are outside this bound.
    pub fn application_workspace_with_budget(
        &self,
        budget: PreparedHierarchyBudget,
    ) -> Result<PreparedBaselineWorkspace<'_>, MultiwayError> {
        self.workspace_with(budget, &mut |_| Ok(()))
    }
    fn workspace_with<F>(
        &self,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<PreparedBaselineWorkspace<'_>, MultiwayError>
    where
        F: FnMut(&'static str) -> Result<(), MultiwayError>,
    {
        admit(
            self.workspace_setup_payload_bound(budget.additional_live_payload_bytes)?,
            budget,
        )?;
        let action = if self.kind == PreparedBaselineKind::Identity {
            BaselineScratch::Identity
        } else if self.kind == PreparedBaselineKind::SymmetricMap {
            BaselineScratch::Map(PreparedSymmetricMap::new(self.frame).workspace_with(before)?)
        } else {
            before("baseline projection")?;
            let projection = self.frame.topology().try_projection_workspace()?;
            before("baseline result")?;
            let result = vector(self.dimension())?;
            BaselineScratch::Pointwise { projection, result }
        };
        if self.image_len() > 0 {
            before("baseline tuple image")?;
        }
        let result = PreparedBaselineWorkspace {
            owner: self,
            action,
            image: vector(self.image_len())?,
        };
        admit(
            self.payload_report(&result, budget.additional_live_payload_bytes)?
                .total_payload_bytes,
            budget,
        )?;
        Ok(result)
    }
}
impl PreparedBaselineWorkspace<'_> {
    /// Exact owner validation, including the chosen fixed action and layout.
    pub fn validate_for(&self, owner: &PreparedBaseline<'_, '_>) -> Result<(), MultiwayError> {
        if !core::ptr::eq(self.owner, owner) {
            return Err(MultiwayError::WorkspaceNotPrepared {
                context: "prepared baseline owner",
            });
        }
        Ok(())
    }
    /// Actual exclusive mutable array capacities, excluding the borrowed owner.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        let action = match &self.action {
            BaselineScratch::Identity => 0,
            BaselineScratch::Map(w) => w.retained_payload_bytes()?,
            BaselineScratch::Pointwise { projection, result } => add(
                projection.retained_payload_bytes()?,
                bytes(result.capacity())?,
            )?,
        };
        add(action, bytes(self.image.capacity())?)
    }
}
impl sealed::Sealed for PreparedBaseline<'_, '_> {}
impl PreparedSolverAction for PreparedBaseline<'_, '_> {
    type Workspace<'owner>
        = PreparedBaselineWorkspace<'owner>
    where
        Self: 'owner;
    fn fine_frame(&self) -> &ThreeWayWeightFrame<'_> {
        self.frame
    }
    fn fine_grouping(&self) -> Option<&PreparedTupleGrouping<'_>> {
        self.grouping.map(|(g, _)| g)
    }
    fn application_workspace(&self) -> Result<Self::Workspace<'_>, MultiwayError> {
        self.application_workspace_with_budget(PreparedHierarchyBudget::UNLIMITED)
    }
    fn apply_with_workspace<'owner>(
        &'owner self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut Self::Workspace<'owner>,
    ) -> Result<(), MultiwayError> {
        workspace.validate_for(self)?;
        let n = self.dimension();
        if rhs.len() != n {
            return Err(crate::error::dimension("baseline RHS", n, rhs.len()));
        }
        if out.len() != n {
            return Err(crate::error::dimension("baseline output", n, out.len()));
        }
        ensure_finite(rhs, "baseline RHS")?;
        match &mut workspace.action {
            BaselineScratch::Identity => {
                out.copy_from_slice(rhs);
                Ok(())
            }
            BaselineScratch::Map(w) => {
                let map = PreparedSymmetricMap::new(self.frame);
                if let Some((grouping, _)) = self.grouping {
                    map.with_grouping(grouping)?
                        .apply_with_workspace(rhs, out, w)
                } else {
                    map.apply_with_workspace(rhs, out, w)
                }
            }
            BaselineScratch::Pointwise { projection, result } => {
                result.copy_from_slice(rhs);
                self.frame
                    .topology()
                    .project_structural_range_with_workspace(result, projection)?;
                ensure_finite(result, "baseline projected RHS")?;
                for (x, degree) in result.iter_mut().zip(self.frame.diagonal()) {
                    *x /= degree;
                }
                ensure_finite(result, "baseline inverse diagonal")?;
                self.frame
                    .topology()
                    .project_structural_range_with_workspace(result, projection)?;
                ensure_finite(result, "baseline correction")?;
                out.copy_from_slice(result);
                Ok(())
            }
        }
    }
    fn fine_gramian_with_workspace<'owner>(
        &'owner self,
        x: &[f64],
        out: &mut [f64],
        workspace: &mut Self::Workspace<'owner>,
    ) -> Result<(), MultiwayError> {
        workspace.validate_for(self)?;
        let view = self.frame.operator_view();
        if let Some((grouping, mode)) = self.grouping {
            let grouped = view.with_grouping(grouping)?;
            if mode == GroupedGramianMode::TupleImage {
                grouped.apply_gramian_with_image(x, out, &mut workspace.image)?;
            } else {
                grouped.apply_gramian(x, out)?;
            }
        } else {
            view.apply_gramian(x, out)?;
        }
        Ok(())
    }
    fn payload_report<'owner>(
        &'owner self,
        workspace: &Self::Workspace<'owner>,
        additional_live_payload_bytes: usize,
    ) -> Result<PreparedHierarchyPayloadReport, MultiwayError> {
        workspace.validate_for(self)?;
        let p = self.requested_payload_parts()?;
        let workspace_payload_bytes = workspace.retained_payload_bytes()?;
        let total_payload_bytes = p[..6]
            .iter()
            .copied()
            .chain([workspace_payload_bytes])
            .try_fold(additional_live_payload_bytes, add)?;
        Ok(PreparedHierarchyPayloadReport {
            fine_topology_payload_bytes: p[0],
            coarse_topology_payload_bytes: 0,
            fine_frame_payload_bytes: p[2],
            coarse_frame_payload_bytes: 0,
            terminal_payload_bytes: 0,
            grouping_payload_bytes: p[5],
            workspace_payload_bytes,
            additional_live_payload_bytes,
            total_payload_bytes,
        })
    }
    fn workspace_payload_bytes<'owner>(
        &'owner self,
        workspace: &Self::Workspace<'owner>,
    ) -> Result<usize, MultiwayError> {
        workspace.validate_for(self)?;
        workspace.retained_payload_bytes()
    }
    fn requested_payload_parts(&self) -> Result<[usize; 7], MultiwayError> {
        Ok([
            self.frame.topology().retained_payload_bytes()?,
            0,
            self.frame.retained_payload_bytes()?,
            0,
            0,
            self.grouping
                .map_or(Ok(0), |(g, _)| g.retained_payload_bytes())?,
            self.workspace_required_bytes()?,
        ])
    }
}
fn add(a: usize, b: usize) -> Result<usize, MultiwayError> {
    a.checked_add(b)
        .ok_or(MultiwayError::WorkspaceSizeOverflow {
            context: "prepared baseline payload",
        })
}
fn admit(required: usize, budget: PreparedHierarchyBudget) -> Result<(), MultiwayError> {
    if required > budget.maximum_payload_bytes {
        Err(MultiwayError::PayloadBudgetExceeded {
            required,
            budget: budget.maximum_payload_bytes,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use multiway_incidence::{PreparedThreeWayTopology, WeightFrameInput};
    #[test]
    fn every_baseline_reservation_admission_unwind_and_owner_boundary_recovers() {
        let keys: Vec<_> = (0..2)
            .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
            .collect();
        let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &keys).unwrap();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
        let g = PreparedTupleGrouping::try_new(&t).unwrap();
        for kind in [
            PreparedBaselineKind::Identity,
            PreparedBaselineKind::InverseDiagonal,
            PreparedBaselineKind::SymmetricMap,
        ] {
            for mode in [
                None,
                Some(GroupedGramianMode::RowGather),
                Some(GroupedGramianMode::TupleImage),
            ] {
                let mut owner = PreparedBaseline::new(&f, kind);
                if let Some(mode) = mode {
                    owner = owner.with_grouping(&g, mode).unwrap();
                }
                let mut calls = 0;
                let mut old = owner
                    .workspace_with(PreparedHierarchyBudget::UNLIMITED, &mut |_| {
                        calls += 1;
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(
                    calls,
                    if kind == PreparedBaselineKind::Identity {
                        0
                    } else if kind == PreparedBaselineKind::SymmetricMap {
                        4
                    } else {
                        2
                    } + usize::from(mode == Some(GroupedGramianMode::TupleImage))
                );
                let required = owner.workspace_setup_payload_bound(71).unwrap();
                let budget = PreparedHierarchyBudget {
                    maximum_payload_bytes: required,
                    additional_live_payload_bytes: 71,
                };
                let exact = owner.application_workspace_with_budget(budget).unwrap();
                assert_eq!(
                    owner
                        .payload_report(&exact, 71)
                        .unwrap()
                        .total_payload_bytes,
                    required
                );
                assert!(
                    owner
                        .workspace_with(
                            PreparedHierarchyBudget {
                                maximum_payload_bytes: required - 1,
                                ..budget
                            },
                            &mut |_| panic!("budget must precede allocation")
                        )
                        .is_err()
                );
                assert!(
                    owner
                        .workspace_with(
                            PreparedHierarchyBudget {
                                maximum_payload_bytes: usize::MAX,
                                additional_live_payload_bytes: usize::MAX
                            },
                            &mut |_| panic!("overflow must precede allocation")
                        )
                        .is_err()
                );
                let rhs = [0.25, -1., 2., 0.75, 0., 1.];
                let mut expected = [0.; 6];
                owner
                    .apply_with_workspace(&rhs, &mut expected, &mut old)
                    .unwrap();
                for unwind in [false, true] {
                    for fail_at in 0..calls {
                        let mut reached = 0;
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            owner.workspace_with(PreparedHierarchyBudget::UNLIMITED,&mut |context| {
                        let at=reached;reached+=1;
                        if at==fail_at { if unwind {panic!("injected baseline setup unwind");} return Err(multiway_incidence::IncidenceError::TopologyAllocation {context}.into()); }
                        Ok(())
                    })
                        }));
                        assert_eq!(reached, fail_at + 1);
                        if unwind {
                            assert!(result.is_err());
                        } else {
                            assert!(result.unwrap().is_err());
                        }
                        let mut fresh = owner.application_workspace().unwrap();
                        let mut out = [f64::NAN; 6];
                        owner
                            .apply_with_workspace(&rhs, &mut out, &mut fresh)
                            .unwrap();
                        assert_eq!(out, expected);
                        owner
                            .apply_with_workspace(&rhs, &mut out, &mut old)
                            .unwrap();
                        assert_eq!(out, expected);
                    }
                }
                let other = owner;
                let mut out = [17.; 6];
                assert!(
                    other
                        .apply_with_workspace(&rhs, &mut out, &mut old)
                        .is_err()
                );
                assert_eq!(out, [17.; 6]);
                assert!(
                    other
                        .fine_gramian_with_workspace(&rhs, &mut out, &mut old)
                        .is_err()
                );
                assert_eq!(out, [17.; 6]);
                for bad in [&[0.; 5][..], &[f64::NAN; 6][..]] {
                    assert!(owner.apply_with_workspace(bad, &mut out, &mut old).is_err());
                    assert_eq!(out, [17.; 6]);
                }
                assert!(
                    owner
                        .apply_with_workspace(&rhs, &mut out[..5], &mut old)
                        .is_err()
                );
                assert_eq!(out, [17.; 6]);
            }
        }
    }
    #[test]
    fn representable_frames_can_fail_corrections_without_publishing_or_poisoning() {
        let keys: Vec<_> = (0..2)
            .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
            .collect();
        let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &keys).unwrap();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[1e-300; 8])).unwrap();
        for kind in [
            PreparedBaselineKind::Identity,
            PreparedBaselineKind::InverseDiagonal,
            PreparedBaselineKind::SymmetricMap,
        ] {
            let owner = PreparedBaseline::new(&f, kind);
            let mut w = owner.application_workspace().unwrap();
            let bad = if kind == PreparedBaselineKind::Identity {
                f64::NAN
            } else {
                1e20
            };
            let mut out = [17.; 6];
            if kind == PreparedBaselineKind::Identity {
                owner
                    .apply_with_workspace(&[f64::MAX; 6], &mut out, &mut w)
                    .unwrap();
                assert_eq!(out, [f64::MAX; 6]);
                out.fill(17.);
            }
            assert!(
                owner
                    .apply_with_workspace(&[bad; 6], &mut out, &mut w)
                    .is_err()
            );
            assert_eq!(out, [17.; 6]);
            owner
                .apply_with_workspace(&[0.; 6], &mut out, &mut w)
                .unwrap();
            assert_eq!(out, [0.; 6]);
            let mut fresh = owner.application_workspace().unwrap();
            let mut expected = [0.; 6];
            owner
                .apply_with_workspace(&[1e-310; 6], &mut expected, &mut fresh)
                .unwrap();
            owner
                .apply_with_workspace(&[1e-310; 6], &mut out, &mut w)
                .unwrap();
            assert_eq!(out, expected);
        }
    }
}
