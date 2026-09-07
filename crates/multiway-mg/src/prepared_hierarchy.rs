//! Fixed serial MAP hierarchy attached to immutable supplied-map numerical frames.
use crate::{
    DensePseudoinverse, DensePseudoinverseWorkspace, MultiwayError, PreparedMapWorkspace,
    PreparedSymmetricMap,
    cycle_kernel::{self, CycleActions},
};
use multiway_incidence::{
    HierarchyWeightFrames, PreparedHierarchyGrouping, PreparedStructuralProjectionWorkspace,
    PreparedTupleGrouping, ThreeWayWeightFrame,
};

/// Hard bound for this serial prototype's whole dense terminal.
/// Component-local terminal routing is a later automatic-construction milestone.
pub const PREPARED_DENSE_TERMINAL_LIMIT: usize = 256;
/// Hard bound on recursive call depth, including the fine and terminal levels.
pub const PREPARED_HIERARCHY_LEVEL_LIMIT: usize = 64;

/// Explicit Gramian arithmetic for selected grouped hierarchy levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupedGramianMode {
    /// Stable row gather with no tuple-image scratch.
    RowGather,
    /// Compute a weighted tuple image once, then gather it by row.
    /// One maximum-E image is shared by all levels and outer PCG actions.
    TupleImage,
}

/// One immutable fixed symmetric V-cycle for the exact current numerical frames.
///
/// Owns only the dense terminal factorization; topology, maps, numerical levels
/// and per-level MAP actions are borrowed. One pre/post MAP sweep and exact
/// Galerkin correction share the ordinary hierarchy recurrence. Construction
/// admits at most 64 levels and a whole terminal of at most 256 coefficients
/// before dense assembly. This is supplied-map execution, not automatic quality
/// admission or permission to reuse an old factorization for changed weights.
///
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, PreparedHierarchyTopology,
///     PreparedHierarchyGrouping, HierarchyWeightFrames, ThreeWayWeightFrame, WeightFrameInput};
/// use multiway_mg::{PreparedMapHierarchy, GroupedGramianMode};
/// let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let h = PreparedHierarchyTopology::try_new(&t, vec![]).unwrap();
/// let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
/// let frames = HierarchyWeightFrames::try_new(&h, &f).unwrap();
/// let numerical = {
///     let groups = PreparedHierarchyGrouping::try_new(&h, 0).unwrap();
///     PreparedMapHierarchy::try_new_with_grouping(
///         &frames, &groups, GroupedGramianMode::RowGather, 1e-12).unwrap()
/// };
/// assert_eq!(numerical.dimension(), 3);
/// ```
#[derive(Debug)]
pub struct PreparedMapHierarchy<'state> {
    frames: &'state HierarchyWeightFrames<'state, 'state, 'state>,
    terminal: DensePseudoinverse,
    grouping: Option<(
        &'state PreparedHierarchyGrouping<'state, 'state>,
        GroupedGramianMode,
    )>,
}

/// Complete caller-owned scratch bound to an exact numerical hierarchy owner.
///
/// Owns one flat result/traversal/image arena, per-level projection/MAP scratch and modal
/// terminal storage. No pool, implicit preparation, mutation of the hierarchy or
/// allocation occurs on application. All active values are initialized each call.
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, PreparedHierarchyTopology,
///     HierarchyWeightFrames, ThreeWayWeightFrame, WeightFrameInput};
/// use multiway_mg::PreparedMapHierarchy;
/// let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let h = PreparedHierarchyTopology::try_new(&t, vec![]).unwrap();
/// let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
/// let frames = HierarchyWeightFrames::try_new(&h, &f).unwrap();
/// let w = {
///     let numerical = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
///     numerical.application_workspace().unwrap()
/// };
/// assert!(w.retained_payload_bytes().is_ok());
/// ```
#[derive(Debug)]
pub struct PreparedHierarchyWorkspace<'owner> {
    owner: &'owner PreparedMapHierarchy<'owner>,
    arena: Vec<f64>,
    levels: Vec<LevelScratch<'owner>>,
    terminal: DensePseudoinverseWorkspace,
}
#[derive(Debug)]
pub(crate) struct LevelScratch<'owner> {
    projection: PreparedStructuralProjectionWorkspace<'owner>,
    map: Option<PreparedMapWorkspace<'owner, 'owner>>,
}

/// Complete retained array payload for one hierarchy and its application scratch.
///
/// Counts direct borrowed owners exactly once. Additional caller state, including
/// inputs, RHS panels, solver/certificate scratch, other frames or workers, must
/// be declared. Construction temporaries, inline roots, allocator metadata and
/// process RSS are excluded. Dense factorization uses bounded nalgebra setup
/// allocations, which are separate from this retained-state inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedHierarchyPayloadReport {
    /// Fine structural keys, components and optional original observation groups.
    pub fine_topology_payload_bytes: usize,
    /// Owned coarse topology, factor maps, groups and heap descriptor capacities.
    pub coarse_topology_payload_bytes: usize,
    /// Fine weights, roots, degrees and component diagnostics.
    pub fine_frame_payload_bytes: usize,
    /// Owned coarse numerical arrays and frame descriptors.
    pub coarse_frame_payload_bytes: usize,
    /// Dense spectral factors and inverse eigenvalues.
    pub terminal_payload_bytes: usize,
    /// Borrowed optional grouping arrays/descriptors, counted once at that owner.
    pub grouping_payload_bytes: usize,
    /// Caller-owned traversal, projection, MAP, optional shared image and modal scratch plus descriptors.
    pub workspace_payload_bytes: usize,
    /// Other live array payload explicitly declared by the caller.
    pub additional_live_payload_bytes: usize,
    /// Checked sum of every preceding category.
    pub total_payload_bytes: usize,
}

impl<'state> PreparedMapHierarchy<'state> {
    /// Build a bounded dense terminal for these exact current numerical frames.
    ///
    /// No fine topology or numerical arrays are copied. Native column-major
    /// assembly avoids the ordinary row-array/flattening intermediates, then uses
    /// the same checked spectral factorization and rank policy. Large terminals
    /// reject before assembly; no hidden fallback or candidate build is attempted.
    pub fn try_new(
        frames: &'state HierarchyWeightFrames<'_, '_, '_>,
        relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        Self::build(frames, relative_tolerance, None)
    }

    /// Build with an explicit structural grouping and Gramian mode.
    ///
    /// Exact structural identity is checked before terminal construction. Groups
    /// may be shared across weight replays; the terminal always uses these current
    /// numerical frames. A zero-prefix grouping executes scalar actions. No
    /// automatic layout, fallback or stale numerical reuse is introduced.
    pub fn try_new_with_grouping(
        frames: &'state HierarchyWeightFrames<'_, '_, '_>,
        grouping: &'state PreparedHierarchyGrouping<'_, '_>,
        mode: GroupedGramianMode,
        relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        grouping.validate_for(frames.hierarchy())?;
        Self::build(frames, relative_tolerance, Some((grouping, mode)))
    }
    fn build(
        frames: &'state HierarchyWeightFrames<'_, '_, '_>,
        relative_tolerance: f64,
        grouping: Option<(
            &'state PreparedHierarchyGrouping<'state, 'state>,
            GroupedGramianMode,
        )>,
    ) -> Result<Self, MultiwayError> {
        if frames.level_count() > PREPARED_HIERARCHY_LEVEL_LIMIT {
            return Err(MultiwayError::InvalidOption {
                name: "prepared_hierarchy_levels",
                message: "must not exceed 64".to_owned(),
            });
        }
        let last = frames
            .frame(frames.level_count() - 1)
            .expect("nonempty frame inventory");
        if last.diagonal().len() > PREPARED_DENSE_TERMINAL_LIMIT {
            return Err(MultiwayError::HierarchyStagnated {
                dimension: last.diagonal().len(),
                tuples: last.weights().len(),
                limit: PREPARED_DENSE_TERMINAL_LIMIT,
            });
        }
        let terminal = DensePseudoinverse::from_frame(last, relative_tolerance)?;
        Ok(Self {
            frames,
            terminal,
            grouping,
        })
    }

    /// Exact borrowed immutable numerical replay owner.
    #[must_use]
    pub const fn frames(&self) -> &'state HierarchyWeightFrames<'state, 'state, 'state> {
        self.frames
    }
    /// Optional borrowed structural grouping, with no numerical state inside it.
    #[must_use]
    pub fn grouping(&self) -> Option<&'state PreparedHierarchyGrouping<'state, 'state>> {
        self.grouping.map(|(g, _)| g)
    }
    /// Explicit requested Gramian mode; None denotes scalar execution.
    #[must_use]
    pub fn grouped_gramian_mode(&self) -> Option<GroupedGramianMode> {
        self.grouping.map(|(_, mode)| mode)
    }
    /// Actual borrowed grouping payload, charged separately from this owner's terminal.
    pub fn grouping_payload_bytes(&self) -> Result<usize, MultiwayError> {
        self.grouping()
            .map_or(Ok(0), |g| Ok(g.retained_payload_bytes()?))
    }
    /// Selected level grouping; terminal and unselected levels return None.
    pub(crate) fn level_grouping(&self, level: usize) -> Option<&PreparedTupleGrouping<'_>> {
        self.grouping().and_then(|g| g.level(level))
    }
    /// Maximum tuple-image length shared across all selected levels; zero in other modes.
    #[must_use]
    pub fn tuple_image_len(&self) -> usize {
        // Every nonempty selected prefix starts at the fine level, and mapping
        // tuples through a function then deduplicating cannot increase E. The
        // maximum is therefore the fine count, with no per-action level scan.
        match self.grouping {
            Some((groups, GroupedGramianMode::TupleImage)) if groups.grouped_levels() > 0 => {
                self.frames.fine().weights().len()
            }
            _ => 0,
        }
    }
    fn traversal_elements(&self) -> Result<usize, MultiwayError> {
        (0..self.depth()).try_fold(self.dimension(), |total, level| {
            add(
                total,
                frame_elements(
                    self.frame_at(level).diagonal().len(),
                    self.frame_at(level + 1).diagonal().len(),
                )?,
            )
        })
    }
    fn arena_elements(&self) -> Result<usize, MultiwayError> {
        add(self.traversal_elements()?, self.tuple_image_len())
    }
    /// Fine coefficient dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.frames.fine().diagonal().len()
    }
    /// Number of nonterminal transitions.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.frames.level_count() - 1
    }
    /// Numerical rank retained at the bounded dense terminal.
    #[must_use]
    pub fn terminal_rank(&self) -> usize {
        self.terminal.rank()
    }
    /// Exclusive immutable numerical payload, excluding all borrowed frames/structure.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        self.terminal.retained_payload_bytes()
    }

    /// Requested exclusive payload for a fresh complete application workspace.
    pub fn workspace_required_bytes(&self) -> Result<usize, MultiwayError> {
        let mut total = add(
            bytes::<LevelScratch<'_>>(self.frames.level_count())?,
            bytes::<f64>(self.arena_elements()?)?,
        )?;
        total = add(total, self.terminal.workspace_required_bytes()?)?;
        for level in 0..self.frames.level_count() {
            let frame = self.frame_at(level);
            total = add(
                total,
                frame.topology().projection_workspace_required_bytes()?,
            )?;
            if level < self.depth() {
                total = add(
                    total,
                    PreparedSymmetricMap::new(frame).workspace_required_bytes()?,
                )?;
            }
        }
        Ok(total)
    }

    /// Reserve all application arrays fallibly at an explicit setup boundary.
    pub fn application_workspace(&self) -> Result<PreparedHierarchyWorkspace<'_>, MultiwayError> {
        PreparedHierarchyWorkspace::build_with(self, &mut |_| Ok(()))
    }

    /// Apply the shared fixed V-cycle without allocation; output is transactional.
    ///
    /// Static validation and finite RHS checking precede any scratch mutation.
    /// Numerical failure publishes no output; later calls fully initialize the
    /// same scratch. Only a workspace prepared from this exact hierarchy is valid.
    pub fn apply_with_workspace(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut PreparedHierarchyWorkspace<'_>,
    ) -> Result<(), MultiwayError> {
        if rhs.len() != self.dimension() {
            return Err(crate::error::dimension(
                "prepared hierarchy rhs",
                self.dimension(),
                rhs.len(),
            ));
        }
        if output.len() != self.dimension() {
            return Err(crate::error::dimension(
                "prepared hierarchy output",
                self.dimension(),
                output.len(),
            ));
        }
        workspace.validate_for(self)?;
        finite(rhs, "prepared hierarchy rhs")?;
        let image_start = workspace.arena.len() - self.tuple_image_len();
        let (traversal, image) = workspace.arena.split_at_mut(image_start);
        let (solution, scratch) = traversal.split_at_mut(self.dimension());
        cycle_kernel::apply_level(
            workspace.owner,
            0,
            rhs,
            solution,
            scratch,
            &mut workspace.levels,
            &mut cycle_kernel::CycleSharedScratch {
                terminal: &mut workspace.terminal,
                image,
            },
        )?;
        finite(solution, "prepared hierarchy solution")?;
        output.copy_from_slice(solution);
        Ok(())
    }

    /// Validate exact fine numerical generation and structural hierarchy provenance.
    pub fn validate_for(
        &self,
        frames: &HierarchyWeightFrames<'_, '_, '_>,
    ) -> Result<(), MultiwayError> {
        if !core::ptr::eq(self.frames, frames) {
            return Err(MultiwayError::WorkspaceNotPrepared {
                context: "prepared hierarchy numerical frames",
            });
        }
        Ok(())
    }

    // Reuse the one hierarchy image for outer PCG; it is dead between actions.
    pub(crate) fn fine_gramian_with_workspace(
        &self,
        x: &[f64],
        out: &mut [f64],
        workspace: &mut PreparedHierarchyWorkspace<'_>,
    ) -> Result<(), MultiwayError> {
        workspace.validate_for(self)?;
        let image_start = workspace.arena.len() - self.tuple_image_len();
        let image = &mut workspace.arena[image_start..];
        self.gramian_at(0, x, out, image)
    }
    fn gramian_at(
        &self,
        level: usize,
        x: &[f64],
        out: &mut [f64],
        image: &mut [f64],
    ) -> Result<(), MultiwayError> {
        let original = self.frame_at(level).operator_view();
        if let Some(grouping) = self.level_grouping(level) {
            let grouped = original.with_grouping(grouping)?;
            if self.grouped_gramian_mode() == Some(GroupedGramianMode::TupleImage) {
                let required = original.tuple_count();
                if image.len() < required {
                    return Err(crate::error::dimension(
                        "shared hierarchy tuple image",
                        required,
                        image.len(),
                    ));
                }
                grouped.apply_gramian_with_image(x, out, &mut image[..required])?;
            } else {
                grouped.apply_gramian(x, out)?;
            }
        } else {
            original.apply_gramian(x, out)?;
        }
        Ok(())
    }

    /// Count every direct retained owner and declared other live payload once.
    pub fn payload_report(
        &self,
        workspace: &PreparedHierarchyWorkspace<'_>,
        additional_live_payload_bytes: usize,
    ) -> Result<PreparedHierarchyPayloadReport, MultiwayError> {
        workspace.validate_for(self)?;
        let mut report = PreparedHierarchyPayloadReport {
            fine_topology_payload_bytes: self.frames.hierarchy().fine().retained_payload_bytes()?,
            coarse_topology_payload_bytes: self.frames.hierarchy().retained_payload_bytes()?,
            fine_frame_payload_bytes: self.frames.fine().retained_payload_bytes()?,
            coarse_frame_payload_bytes: self.frames.retained_payload_bytes()?,
            terminal_payload_bytes: self.retained_payload_bytes()?,
            grouping_payload_bytes: self.grouping_payload_bytes()?,
            workspace_payload_bytes: workspace.retained_payload_bytes()?,
            additional_live_payload_bytes,
            total_payload_bytes: 0,
        };
        for part in [
            report.fine_topology_payload_bytes,
            report.coarse_topology_payload_bytes,
            report.fine_frame_payload_bytes,
            report.coarse_frame_payload_bytes,
            report.terminal_payload_bytes,
            report.grouping_payload_bytes,
            report.workspace_payload_bytes,
            additional_live_payload_bytes,
        ] {
            report.total_payload_bytes = add(report.total_payload_bytes, part)?;
        }
        Ok(report)
    }

    fn frame_at(&self, level: usize) -> &'state ThreeWayWeightFrame<'state> {
        self.frames.frame(level).expect("prepared numerical level")
    }
}

impl<'owner> PreparedHierarchyWorkspace<'owner> {
    fn build_with<F>(
        owner: &'owner PreparedMapHierarchy<'owner>,
        before: &mut F,
    ) -> Result<Self, MultiwayError>
    where
        F: FnMut(&'static str) -> Result<(), MultiwayError>,
    {
        owner.workspace_required_bytes()?;
        let arena = vector(owner.arena_elements()?, before)?;
        let mut levels = reserve(owner.frames.level_count(), before)?;
        for level in 0..owner.frames.level_count() {
            let frame = owner.frame_at(level);
            before("prepared cycle projection")?;
            let projection = frame.topology().try_projection_workspace()?;
            let map = if level < owner.depth() {
                Some(PreparedSymmetricMap::new(frame).workspace_with(before)?)
            } else {
                None
            };
            levels.push(LevelScratch { projection, map });
        }
        before("prepared cycle terminal workspace")?;
        Ok(Self {
            owner,
            arena,
            levels,
            terminal: owner.terminal.application_workspace()?,
        })
    }
    /// Exact numerical hierarchy owner, not shape or value equality.
    pub fn validate_for(&self, owner: &PreparedMapHierarchy<'_>) -> Result<(), MultiwayError> {
        if !core::ptr::eq(self.owner, owner) {
            return Err(MultiwayError::WorkspaceNotPrepared {
                context: "prepared hierarchy application",
            });
        }
        Ok(())
    }
    /// Complete exclusive scratch capacities, including heap descriptors.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        let mut total = add(
            bytes::<f64>(self.arena.capacity())?,
            bytes::<LevelScratch<'_>>(self.levels.capacity())?,
        )?;
        total = add(total, self.terminal.retained_bytes()?)?;
        for level in &self.levels {
            total = add(total, level.projection.retained_payload_bytes()?)?;
            if let Some(map) = &level.map {
                total = add(total, map.retained_payload_bytes()?)?;
            }
        }
        Ok(total)
    }
}

impl<'state> CycleActions for PreparedMapHierarchy<'state> {
    type LevelScratch = LevelScratch<'state>;
    fn level_count(&self) -> usize {
        self.frames.level_count()
    }
    fn dimension_at(&self, level: usize) -> usize {
        self.frame_at(level).diagonal().len()
    }
    fn project(
        &self,
        level: usize,
        values: &mut [f64],
        scratch: &mut Self::LevelScratch,
    ) -> Result<(), MultiwayError> {
        finite(values, "prepared hierarchy projection input")?;
        self.frame_at(level)
            .topology()
            .project_structural_range_with_workspace(values, &mut scratch.projection)?;
        finite(values, "prepared hierarchy projection output")
    }
    fn smooth(
        &self,
        level: usize,
        rhs: &[f64],
        out: &mut [f64],
        scratch: &mut Self::LevelScratch,
    ) -> Result<(), MultiwayError> {
        let map = PreparedSymmetricMap::new(self.frame_at(level));
        let workspace = scratch.map.as_mut().expect("prepared MAP level");
        if let Some(grouping) = self.level_grouping(level) {
            map.with_grouping(grouping)?
                .apply_with_workspace(rhs, out, workspace)
        } else {
            map.apply_with_workspace(rhs, out, workspace)
        }
    }
    fn residual(
        &self,
        level: usize,
        rhs: &[f64],
        x: &[f64],
        out: &mut [f64],
        image: &mut [f64],
    ) -> Result<(), MultiwayError> {
        if rhs.len() != self.frame_at(level).diagonal().len() {
            return Err(crate::error::dimension(
                "prepared grouped residual RHS",
                self.frame_at(level).diagonal().len(),
                rhs.len(),
            ));
        }
        self.gramian_at(level, x, out, image)?;
        for (value, &right) in out.iter_mut().zip(rhs) {
            *value = right - *value;
        }
        finite(out, "prepared hierarchy residual")
    }
    fn restrict(
        &self,
        level: usize,
        fine: &[f64],
        coarse: &mut [f64],
    ) -> Result<(), MultiwayError> {
        self.frames
            .hierarchy()
            .aggregation(level)
            .expect("prepared transition")
            .restrict(fine, coarse)?;
        finite(coarse, "prepared hierarchy restriction")
    }
    fn prolong_add(
        &self,
        level: usize,
        coarse: &[f64],
        fine: &mut [f64],
    ) -> Result<(), MultiwayError> {
        self.frames
            .hierarchy()
            .aggregation(level)
            .expect("prepared transition")
            .prolong_add(coarse, fine)?;
        Ok(())
    }
    fn terminal(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut DensePseudoinverseWorkspace,
    ) -> Result<(), MultiwayError> {
        self.terminal
            .solve_into_with_workspace(rhs, out, workspace)?;
        finite(out, "prepared hierarchy terminal")
    }
}

fn frame_elements(fine: usize, coarse: usize) -> Result<usize, MultiwayError> {
    fine.checked_add(coarse)
        .and_then(|n| n.checked_mul(2))
        .ok_or_else(overflow)
}
fn bytes<T>(count: usize) -> Result<usize, MultiwayError> {
    count
        .checked_mul(core::mem::size_of::<T>())
        .filter(|&n| n <= isize::MAX as usize)
        .ok_or_else(overflow)
}
fn add(a: usize, b: usize) -> Result<usize, MultiwayError> {
    a.checked_add(b).ok_or_else(overflow)
}
fn reserve<T, F>(count: usize, before: &mut F) -> Result<Vec<T>, MultiwayError>
where
    F: FnMut(&'static str) -> Result<(), MultiwayError>,
{
    bytes::<T>(count)?;
    if count > 0 {
        before("prepared hierarchy scratch")?;
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|source| MultiwayError::WorkspaceAllocation {
            context: "prepared hierarchy scratch",
            source,
        })?;
    Ok(values)
}
fn vector<F>(count: usize, before: &mut F) -> Result<Vec<f64>, MultiwayError>
where
    F: FnMut(&'static str) -> Result<(), MultiwayError>,
{
    let mut values = reserve(count, before)?;
    values.resize(count, 0.0);
    Ok(values)
}
fn finite(values: &[f64], context: &'static str) -> Result<(), MultiwayError> {
    if values.iter().any(|x| !x.is_finite()) {
        return Err(MultiwayError::NumericalFailure { context });
    }
    Ok(())
}
fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow {
        context: "prepared hierarchy payload",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use multiway_incidence::{
        FactorAggregation, PreparedHierarchyTopology, PreparedThreeWayTopology, WeightFrameInput,
    };
    #[test]
    fn identity_and_ragged_frames_match_independent_vector_storage() {
        let counts = [3, 2, 4];
        let tuples: Vec<_> = (0..3)
            .flat_map(|i| (0..2).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
            .collect();
        let t = PreparedThreeWayTopology::try_from_collapsed(counts, &tuples).unwrap();
        let h = PreparedHierarchyTopology::try_new(
            &t,
            vec![
                FactorAggregation::identity(counts).unwrap(),
                FactorAggregation::consecutive_halving(counts).unwrap(),
            ],
        )
        .unwrap();
        let fine = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
        let frames = HierarchyWeightFrames::try_new(&h, &fine).unwrap();
        let groups = PreparedHierarchyGrouping::try_new(&h, 2).unwrap();
        for mode in [
            GroupedGramianMode::RowGather,
            GroupedGramianMode::TupleImage,
        ] {
            let owner =
                PreparedMapHierarchy::try_new_with_grouping(&frames, &groups, mode, 1e-12).unwrap();
            let mut actual = owner.application_workspace().unwrap();
            let mut reference = owner.application_workspace().unwrap();
            let mut vectors = Vec::new();
            for level in 0..owner.depth() {
                let n = owner.frame_at(level).diagonal().len();
                let m = owner.frame_at(level + 1).diagonal().len();
                for count in [n, n, m, m] {
                    vectors.push(vec![f64::NAN; count]);
                }
            }
            let mut image = vec![f64::NAN; owner.tuple_image_len()];
            for column in 0..4 {
                let rhs: Vec<_> = (0..owner.dimension())
                    .map(|i| ((i + column) as f64 * 0.23).sin())
                    .collect();
                let mut a = vec![f64::NAN; rhs.len()];
                let mut b = a.clone();
                actual.arena.fill(f64::NAN);
                owner
                    .apply_with_workspace(&rhs, &mut a, &mut actual)
                    .unwrap();
                cycle_kernel::apply_level(
                    &owner,
                    0,
                    &rhs,
                    &mut b,
                    vectors.as_mut_slice(),
                    &mut reference.levels,
                    &mut cycle_kernel::CycleSharedScratch {
                        terminal: &mut reference.terminal,
                        image: &mut image,
                    },
                )
                .unwrap();
                assert_eq!(
                    a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                    b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                );
            }
        }
    }
    #[test]
    fn all_workspace_reservation_failures_and_poisoned_scratch_recover() {
        let tuples: Vec<_> = (0..4)
            .flat_map(|i| (0..4).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
            .collect();
        let topology = PreparedThreeWayTopology::try_from_collapsed([4; 3], &tuples).unwrap();
        let structural = PreparedHierarchyTopology::try_new(
            &topology,
            vec![
                FactorAggregation::consecutive_halving([4; 3]).unwrap(),
                FactorAggregation::consecutive_halving([2; 3]).unwrap(),
            ],
        )
        .unwrap();
        let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
        let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
        let groups = PreparedHierarchyGrouping::try_new(&structural, 2).unwrap();
        for hierarchy in [
            PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap(),
            PreparedMapHierarchy::try_new_with_grouping(
                &frames,
                &groups,
                GroupedGramianMode::RowGather,
                1e-12,
            )
            .unwrap(),
            PreparedMapHierarchy::try_new_with_grouping(
                &frames,
                &groups,
                GroupedGramianMode::TupleImage,
                1e-12,
            )
            .unwrap(),
        ] {
            let mut old = hierarchy.application_workspace().unwrap();
            let rhs = [1.0; 12];
            let mut expected = [0.0; 12];
            hierarchy
                .apply_with_workspace(&rhs, &mut expected, &mut old)
                .unwrap();
            let mut calls = 0;
            PreparedHierarchyWorkspace::build_with(&hierarchy, &mut |_| {
                calls += 1;
                Ok(())
            })
            .unwrap();
            assert_eq!(calls, 14);
            for unwind in [false, true] {
                for fail_at in 0..calls {
                    let mut reached = 0;
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        PreparedHierarchyWorkspace::build_with(&hierarchy, &mut |context| {
                            reached += 1;
                            if reached == fail_at + 1 {
                                assert!(!unwind, "injected complete cycle setup unwind");
                                return Err(MultiwayError::WorkspaceNotPrepared { context });
                            }
                            Ok(())
                        })
                    }));
                    assert_eq!(reached, fail_at + 1);
                    if unwind {
                        assert!(result.is_err());
                    } else {
                        assert!(result.unwrap().is_err());
                    }
                    old.arena.fill(f64::NAN);
                    let mut actual = [0.0; 12];
                    hierarchy
                        .apply_with_workspace(&rhs, &mut actual, &mut old)
                        .unwrap();
                    assert_eq!(actual, expected);
                }
            }
        }
        assert!(frame_elements(usize::MAX, 1).is_err());
        assert!(frame_elements(usize::MAX / 2, 1).is_err());
        assert!(bytes::<f64>(isize::MAX as usize / 8 + 1).is_err());
        assert!(bytes::<f64>(usize::MAX).is_err());
    }
}
