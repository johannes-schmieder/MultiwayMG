//! Bounded summary probes of the actual fixed recursive cycle.
use crate::{
    CycleQualityCriteria, CycleQualityOptions, MultiwayError, PreparedHierarchyWorkspace,
    PreparedMapHierarchy,
    cycle_probe::{
        dot, fill_deterministic, orient_deterministically, scale_in_place, tail_geometric_mean,
    },
};
use multiway_incidence::PreparedHierarchyBudget;

/// Entered outer actions, including a failing call; not internal tuple work.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PreparedCycleScreenWork {
    /// Entered outer Gramian calls, including range starts.
    pub gramian_applications: usize,
    /// Entered complete suffix-cycle calls; internal actions are not expanded.
    pub cycle_applications: usize,
    /// Entered direct compensated Gramian-energy evaluations.
    pub energy_evaluations: usize,
    /// Entered explicit probe projections, excluding projections inside each cycle.
    pub projections: usize,
    /// Entered explicit structural-shift defect evaluations.
    pub defect_evaluations: usize,
}
/// One measured recursive level, stored in bottom-up order.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct PreparedLevelCycleQuality {
    /// Original numerical level index; optional on a failure before level selection.
    pub level: usize,
    /// Coefficient count at this measured level.
    pub dimension: usize,
    /// Number of fully measured deterministic starts.
    pub completed_starts: usize,
    /// Starts whose energy reached the configured numerical zero gate.
    pub annihilated_starts: usize,
    /// Worst tail-geometric error-energy factor across starts.
    pub maximum_estimated_energy_factor: f64,
    /// Largest measured one-step error-energy factor.
    pub maximum_observed_energy_factor: f64,
    /// Largest absolute final signed Rayleigh quotient across starts.
    pub maximum_absolute_final_rayleigh: f64,
    /// Largest measured defect against the known structural shifts.
    pub maximum_structural_defect: f64,
    /// Whether every tested criterion passed; this is not a spectral proof or solve certificate.
    pub accepted: bool,
}
/// Array payload only; existing cycle and all direct owners are charged once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedCycleScreenSetup {
    /// Actual hierarchy/cycle owners plus caller-declared other live arrays.
    pub existing_live_payload_bytes: usize,
    /// Requested three-vector arena and bounded report-array bytes.
    pub new_arrays_payload_bytes: usize,
    /// Checked sum of existing live and requested new arrays; not process RSS.
    pub total_payload_bound: usize,
}
/// Borrowed completed reports cannot be mutated by a subsequent screen call.
#[derive(Debug)]
pub struct PreparedCycleScreenResult<'report> {
    /// Completed reports in bottom-up order; unvisited levels are excluded.
    pub levels: &'report [PreparedLevelCycleQuality],
    /// Entered outer actions, including those on failed attempts.
    pub work: PreparedCycleScreenWork,
    /// Whether every tested criterion passed; this is not a spectral proof or solve certificate.
    pub accepted: bool,
}
/// Numerical failure retains attempted work and the completed tail count.
#[derive(Debug, thiserror::Error)]
#[error("prepared recursive screen failed: {source}")]
pub struct PreparedCycleScreenFailure {
    /// Original typed numerical, owner, sizing or option error.
    pub source: MultiwayError,
    /// Original numerical level index; optional on a failure before level selection.
    pub level: Option<u8>,
    /// Current deterministic start, or None before selecting one.
    pub start: Option<u8>,
    /// Fully measured levels preceding a numerical failure.
    pub completed_tail_levels: usize,
    /// Entered outer actions, including those on failed attempts.
    pub work: PreparedCycleScreenWork,
}
/// A still-used borrowed result excludes another screen call:
/// ```compile_fail
/// use multiway_mg::{PreparedCycleScreenWorkspace, PreparedHierarchyWorkspace,
///     CycleQualityOptions, CycleQualityCriteria};
/// fn cannot_overwrite(screen: &mut PreparedCycleScreenWorkspace<'_>,
///     cycle: &mut PreparedHierarchyWorkspace<'_>, o: CycleQualityOptions,
///     c: CycleQualityCriteria) {
///     let result = screen.screen(o, c, cycle).unwrap();
///     screen.screen(o, c, cycle).unwrap();
///     assert!(result.accepted);
/// }
/// ```
///
/// Two arrays: three fine-dimension vectors and one bounded level-report array.
/// A caller lends an existing exact-owner cycle workspace during execution.
#[derive(Debug)]
pub struct PreparedCycleScreenWorkspace<'owner> {
    owner: &'owner PreparedMapHierarchy<'owner>,
    arena: Vec<f64>,
    reports: Vec<PreparedLevelCycleQuality>,
    setup: PreparedCycleScreenSetup,
    completed: usize,
}
#[derive(Default)]
struct ScreenProgress {
    work: PreparedCycleScreenWork,
    start: Option<u8>,
}
impl<'owner> PreparedCycleScreenWorkspace<'owner> {
    /// Size both new arrays with the exact cycle and all direct owners charged once.
    /// Caller other-live state includes old screens, outputs or concurrent workspaces.
    /// The fixed 64-factor stack history, inline roots and allocator metadata are excluded.
    pub fn setup_payload_report(
        owner: &PreparedMapHierarchy<'_>,
        cycle: &PreparedHierarchyWorkspace<'_>,
        budget: PreparedHierarchyBudget,
    ) -> Result<PreparedCycleScreenSetup, MultiwayError> {
        let existing = owner
            .payload_report(cycle, budget.additional_live_payload_bytes)?
            .total_payload_bytes;
        let new = add(
            bytes::<f64>(owner.dimension().checked_mul(3).ok_or_else(overflow)?)?,
            bytes::<PreparedLevelCycleQuality>(owner.frames().level_count())?,
        )?;
        Ok(PreparedCycleScreenSetup {
            existing_live_payload_bytes: existing,
            new_arrays_payload_bytes: new,
            total_payload_bound: add(existing, new)?,
        })
    }
    /// Admit requested live payload before either fallible allocation.
    /// Check actual retained capacities before publishing; no cycle is constructed here.
    pub fn try_new(
        owner: &'owner PreparedMapHierarchy<'_>,
        cycle: &PreparedHierarchyWorkspace<'_>,
        budget: PreparedHierarchyBudget,
    ) -> Result<Self, MultiwayError> {
        Self::build_with(owner, cycle, budget, &mut |_| Ok(()))
    }
    fn build_with<F>(
        owner: &'owner PreparedMapHierarchy<'_>,
        cycle: &PreparedHierarchyWorkspace<'_>,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<Self, MultiwayError>
    where
        F: FnMut(&'static str) -> Result<(), MultiwayError>,
    {
        let setup = Self::setup_payload_report(owner, cycle, budget)?;
        admit(setup.total_payload_bound, budget)?;
        let n = owner.dimension() * 3;
        let mut arena = reserve(n, "prepared screen arena", before)?;
        arena.resize(n, 0.);
        let n = owner.frames().level_count();
        let mut reports = reserve(n, "prepared screen reports", before)?;
        reports.resize(n, PreparedLevelCycleQuality::default());
        let result = Self {
            owner,
            arena,
            reports,
            setup,
            completed: 0,
        };
        admit(
            add(
                setup.existing_live_payload_bytes,
                result.retained_payload_bytes()?,
            )?,
            budget,
        )?;
        Ok(result)
    }
    /// Return the requested setup scope; actual new capacities are reported separately.
    pub fn setup_report(&self) -> PreparedCycleScreenSetup {
        self.setup
    }
    /// Actual exclusive arena/report capacities, excluding borrowed owners and stack.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        add(
            bytes::<f64>(self.arena.capacity())?,
            bytes::<PreparedLevelCycleQuality>(self.reports.capacity())?,
        )
    }
    /// Completed bottom-up reports from the last screen that passed static validation.
    /// Numerical failure leaves the successfully measured prefix available here.
    /// Invalid options or a wrong workspace do not replace the previous run's reports.
    pub fn completed_level_reports(&self) -> &[PreparedLevelCycleQuality] {
        &self.reports[..self.completed]
    }

    /// Probe actual tails from the terminal upward, stopping on the first quality rejection.
    /// Options are explicit: at most 16 starts and 64 iterations, with a valid tail.
    /// Exact owner and all options are checked before resetting report/scratch state.
    /// This uses a bounded 64-f64 stack history and no allocations after preparation.
    /// Numerical failure reports attempted actions and completed levels; original
    /// frames and the fixed cycle remain immutable and usable for recovery.
    pub fn screen(
        &mut self,
        options: CycleQualityOptions,
        criteria: CycleQualityCriteria,
        cycle: &mut PreparedHierarchyWorkspace<'_>,
    ) -> Result<PreparedCycleScreenResult<'_>, PreparedCycleScreenFailure> {
        let mut progress = ScreenProgress::default();
        let mut current = None;
        let mut completed = 0;
        let result = (|| {
            cycle.validate_for(self.owner)?;
            validate(options, criteria)?;
            self.reports.fill(PreparedLevelCycleQuality::default());
            self.completed = 0;
            for level in (0..self.owner.frames().level_count()).rev() {
                current = Some(u8::try_from(level).expect("at most 64 levels"));
                progress.start = None;
                let report = analyze_level(
                    self.owner,
                    level,
                    options,
                    criteria,
                    cycle,
                    &mut self.arena,
                    &mut progress,
                )?;
                self.reports[completed] = report;
                completed += 1;
                self.completed = completed;
                if !report.accepted {
                    return Ok(false);
                }
            }
            Ok(true)
        })();
        match result {
            Ok(accepted) => Ok(PreparedCycleScreenResult {
                levels: &self.reports[..completed],
                work: progress.work,
                accepted,
            }),
            Err(source) => Err(PreparedCycleScreenFailure {
                source,
                level: current,
                start: progress.start,
                completed_tail_levels: completed,
                work: progress.work,
            }),
        }
    }
}
fn analyze_level(
    owner: &PreparedMapHierarchy<'_>,
    level: usize,
    options: CycleQualityOptions,
    criteria: CycleQualityCriteria,
    cycle: &mut PreparedHierarchyWorkspace<'_>,
    arena: &mut [f64],
    progress: &mut ScreenProgress,
) -> Result<PreparedLevelCycleQuality, MultiwayError> {
    let work = &mut progress.work;
    let frame = owner.frames().frame(level).expect("bounded screen level");
    let n = frame.diagonal().len();
    let (mut error, rest) = arena[..3 * n].split_at_mut(n);
    let (gradient, mut next) = rest.split_at_mut(n);
    let mut report = PreparedLevelCycleQuality {
        level,
        dimension: n,
        ..Default::default()
    };
    for vector in 0..options.test_vectors {
        progress.start = Some(u8::try_from(vector).expect("at most 16 starts"));
        let mut generated = false;
        for attempt in 0..16_u64 {
            fill_deterministic(
                gradient,
                options.seed
                    ^ (vector as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                    ^ attempt.wrapping_mul(0xbf58_476d_1ce4_e5b9),
            );
            work.gramian_applications += 1;
            owner.gramian_tail_with_workspace(level, gradient, error, cycle)?;
            work.projections += 1;
            owner.project_tail_with_workspace(level, error, cycle)?;
            work.energy_evaluations += 1;
            let energy = energy_norm(frame, error)?;
            if energy > options.relative_zero_tolerance {
                scale_in_place(error, 1. / energy);
                finite(error, "screen normalized range start")?;
                orient_deterministically(error);
                generated = true;
                break;
            }
        }
        if !generated {
            return Err(numerical("screen nonzero range start"));
        }
        let mut factors = [0.; 64];
        let mut length = 0;
        let mut final_rayleigh = 0.;
        let mut annihilated = false;
        for step in 0..options.power_iterations {
            work.gramian_applications += 1;
            owner.gramian_tail_with_workspace(level, error, gradient, cycle)?;
            work.cycle_applications += 1;
            owner.apply_tail_with_workspace(level, gradient, next, cycle)?;
            finite(next, "screen correction")?;
            work.projections += 1;
            owner.project_tail_with_workspace(level, next, cycle)?;
            for (value, &old) in next.iter_mut().zip(error.iter()) {
                *value = (-options.correction_damping).mul_add(*value, old);
            }
            work.projections += 1;
            owner.project_tail_with_workspace(level, next, cycle)?;
            finite(next, "screen error")?;
            work.defect_evaluations += 1;
            let defect = owner.defect_tail_with_workspace(level, next, cycle)?;
            scalar(defect, "screen structural defect")?;
            report.maximum_structural_defect = report.maximum_structural_defect.max(defect);
            final_rayleigh = dot(gradient, next);
            scalar(final_rayleigh, "screen Rayleigh quotient")?;
            work.energy_evaluations += 1;
            let energy = energy_norm(frame, next)?;
            factors[step] = energy;
            length = step + 1;
            report.maximum_observed_energy_factor =
                report.maximum_observed_energy_factor.max(energy);
            if energy <= options.relative_zero_tolerance {
                annihilated = true;
                break;
            }
            scale_in_place(next, 1. / energy);
            finite(next, "screen normalized error")?;
            core::mem::swap(&mut error, &mut next);
        }
        let estimate = if annihilated {
            0.
        } else {
            tail_geometric_mean(&factors[..length], options.tail_iterations)
        };
        scalar(estimate, "screen estimated energy factor")?;
        report.maximum_estimated_energy_factor =
            report.maximum_estimated_energy_factor.max(estimate);
        report.maximum_absolute_final_rayleigh = report
            .maximum_absolute_final_rayleigh
            .max(final_rayleigh.abs());
        report.completed_starts += 1;
        report.annihilated_starts += usize::from(annihilated);
    }
    report.accepted = report.maximum_estimated_energy_factor
        <= criteria.maximum_estimated_energy_factor
        && criteria
            .maximum_observed_energy_factor
            .is_none_or(|limit| report.maximum_observed_energy_factor <= limit)
        && report.maximum_structural_defect <= criteria.maximum_structural_defect;
    Ok(report)
}
fn energy_norm(
    frame: &multiway_incidence::ThreeWayWeightFrame<'_>,
    x: &[f64],
) -> Result<f64, MultiwayError> {
    let energy = frame.operator_view().energy(x)?;
    scalar(energy, "screen Gramian energy")?;
    if energy < -64. * f64::EPSILON {
        return Err(numerical("screen negative Gramian energy"));
    }
    Ok(energy.max(0.).sqrt())
}
fn validate(o: CycleQualityOptions, c: CycleQualityCriteria) -> Result<(), MultiwayError> {
    if o.test_vectors == 0
        || o.test_vectors > 16
        || o.power_iterations == 0
        || o.power_iterations > 64
        || o.tail_iterations == 0
        || o.tail_iterations > o.power_iterations
    {
        return Err(invalid("screen bounded start/iteration counts"));
    }
    for (value, context) in [
        (o.correction_damping, "screen damping"),
        (o.relative_zero_tolerance, "screen zero tolerance"),
        (
            c.maximum_estimated_energy_factor,
            "screen estimated-factor criterion",
        ),
    ] {
        if !value.is_finite() || value <= 0. {
            return Err(invalid(context));
        }
    }
    if c.maximum_observed_energy_factor
        .is_some_and(|v| !v.is_finite() || v <= 0.)
    {
        return Err(invalid("screen observed-factor criterion"));
    }
    if !c.maximum_structural_defect.is_finite() || c.maximum_structural_defect < 0. {
        return Err(invalid("screen structural-defect criterion"));
    }
    Ok(())
}
fn invalid(context: &'static str) -> MultiwayError {
    MultiwayError::InvalidCycleScreenInput { context }
}
fn numerical(context: &'static str) -> MultiwayError {
    MultiwayError::NumericalFailure { context }
}
fn scalar(value: f64, context: &'static str) -> Result<(), MultiwayError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(numerical(context))
    }
}
fn finite(values: &[f64], context: &'static str) -> Result<(), MultiwayError> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(numerical(context))
    }
}
fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow {
        context: "prepared cycle screen",
    }
}
fn bytes<T>(n: usize) -> Result<usize, MultiwayError> {
    n.checked_mul(size_of::<T>())
        .filter(|&n| n <= isize::MAX as usize)
        .ok_or_else(overflow)
}
fn add(a: usize, b: usize) -> Result<usize, MultiwayError> {
    a.checked_add(b).ok_or_else(overflow)
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
fn reserve<T, F>(n: usize, context: &'static str, before: &mut F) -> Result<Vec<T>, MultiwayError>
where
    F: FnMut(&'static str) -> Result<(), MultiwayError>,
{
    bytes::<T>(n)?;
    if n > 0 {
        before(context)?;
    }
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|source| MultiwayError::WorkspaceAllocation { context, source })?;
    Ok(v)
}

#[cfg(test)]
mod tests;
