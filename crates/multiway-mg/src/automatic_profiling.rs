//! Opt-in, current-thread, disjoint automatic-driver diagnostics.
//!
//! Timers perturb execution. These records are never authoritative speed samples.
//! The fixed kernel profiler has its own inventory; its inclusive times must not
//! be added to these regions. Neither collector propagates to worker threads.
use std::{cell::RefCell, marker::PhantomData, rc::Rc, time::Instant};

/// Disjoint coarse regions. Calls include failed attempts, not just accepted work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Phase {
    /// Original component layout and recoding construction.
    Partition,
    /// Local canonical component topology construction.
    LocalRoot,
    /// Local current-weight numerical frame construction.
    LocalFrame,
    /// Small-component dense assembly and factorization.
    DirectFactor,
    /// Singleton or small-component direct column solves.
    DirectSolve,
    /// Hierarchy candidates, structural admission and provisional weight replay.
    StructuralPreparation,
    /// Selected nonterminal hierarchy grouping construction.
    HierarchyGrouping,
    /// Large-component baseline grouping construction.
    ComponentGrouping,
    /// Whole-original baseline grouping construction.
    GlobalGrouping,
    /// Complete current-weight hierarchy replay after structure is prepared.
    NumericalReplay,
    /// Coarse terminal assembly, factorization and numerical owner construction.
    CoarseFactor,
    /// Screen setup, all actual-tail checks, bookkeeping and scratch destruction.
    Screening,
    /// Gathered-target and complete local Krylov workspace preparation.
    LocalWorkspace,
    /// All local Krylov columns, including gather/scatter and rejected work.
    LocalSolve,
    /// Original whole-problem certificate, including its scratch lifecycle.
    OriginalCertificate,
    /// Whole-original baseline Krylov workspace preparation.
    GlobalWorkspace,
    /// Whole-original baseline columns, including rejected certificates.
    GlobalSolve,
}
/// Complete fixed inventory, including unvisited regions.
pub const PHASES: [Phase; 17] = [
    Phase::Partition,
    Phase::LocalRoot,
    Phase::LocalFrame,
    Phase::DirectFactor,
    Phase::DirectSolve,
    Phase::StructuralPreparation,
    Phase::HierarchyGrouping,
    Phase::ComponentGrouping,
    Phase::GlobalGrouping,
    Phase::NumericalReplay,
    Phase::CoarseFactor,
    Phase::Screening,
    Phase::LocalWorkspace,
    Phase::LocalSolve,
    Phase::OriginalCertificate,
    Phase::GlobalWorkspace,
    Phase::GlobalSolve,
];
impl Phase {
    /// Stable diagnostic name; distinct from the frozen kernel inventory.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Partition => "partition",
            Self::LocalRoot => "local_root",
            Self::LocalFrame => "local_frame",
            Self::DirectFactor => "direct_factor",
            Self::DirectSolve => "direct_solve",
            Self::StructuralPreparation => "structural_preparation",
            Self::HierarchyGrouping => "hierarchy_grouping",
            Self::ComponentGrouping => "component_grouping",
            Self::GlobalGrouping => "global_grouping",
            Self::NumericalReplay => "numerical_replay",
            Self::CoarseFactor => "coarse_factor",
            Self::Screening => "screening",
            Self::LocalWorkspace => "local_workspace",
            Self::LocalSolve => "local_solve",
            Self::OriginalCertificate => "original_certificate",
            Self::GlobalWorkspace => "global_workspace",
            Self::GlobalSolve => "global_solve",
        }
    }
}
/// Aggregate for one nonoverlapping region; no event history is retained.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// Completed span count, including spans exited through errors or unwinding.
    pub calls: u64,
    /// Sum of complete span durations, including observer overhead within them.
    pub elapsed_ns: u128,
}
const EMPTY: Region = Region {
    calls: 0,
    elapsed_ns: 0,
};
struct State {
    active: bool,
    valid: bool,
    generation: u64,
    open: Option<Phase>,
    regions: [Region; PHASES.len()],
}
impl State {
    const fn new() -> Self {
        Self {
            active: false,
            valid: true,
            generation: 0,
            open: None,
            regions: [EMPTY; PHASES.len()],
        }
    }
}
thread_local! { static STATE: RefCell<State> = const { RefCell::new(State::new()) }; }
/// Fixed inline current-thread state; not admitted heap or allocator metadata.
#[must_use]
pub const fn thread_local_payload_bytes() -> usize {
    std::mem::size_of::<RefCell<State>>()
}
/// A bounded diagnostic result. Reject invalid records rather than summing them.
#[derive(Debug, Clone, Copy)]
pub struct Report {
    /// False for overlap, unfinished spans, counter overflow or inconsistent time.
    pub valid: bool,
    /// Whole callback time, excluding collector initialization/extraction.
    pub elapsed_ns: u128,
    /// Whole callback less disjoint regions; includes untimed bookkeeping/drop.
    /// This value has meaning only when `valid` is true.
    pub unattributed_ns: u128,
    /// Complete inventory in [`PHASES`] order.
    pub regions: [Region; PHASES.len()],
}
/// Starting a nested collection never invokes its callback or resets its parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectError {
    /// An explicit collection is already active on this thread.
    NestedCollection,
    /// The monotone generation counter cannot be represented.
    GenerationOverflow,
}
impl std::fmt::Display for CollectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for CollectError {}
struct CollectionGuard(u64);
impl Drop for CollectionGuard {
    fn drop(&mut self) {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            if s.generation == self.0 {
                s.active = false;
                s.open = None;
            }
        });
    }
}
/// Collect one serial callback with fixed inline state and no profiler arrays.
///
/// Failed callbacks keep all completed regions. Panic unwinding disables the
/// collector so later calls recover. A returned live span invalidates the report.
/// Spans may not overlap; kernel instrumentation uses a separate collector.
pub fn collect<T>(f: impl FnOnce() -> T) -> Result<(T, Report), CollectError> {
    let generation = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.active {
            return Err(CollectError::NestedCollection);
        }
        let generation = s
            .generation
            .checked_add(1)
            .ok_or(CollectError::GenerationOverflow)?;
        s.generation = generation;
        s.active = true;
        s.valid = true;
        s.open = None;
        s.regions.fill(EMPTY);
        Ok(generation)
    })?;
    let guard = CollectionGuard(generation);
    let start = Instant::now();
    let value = f();
    let elapsed_ns = start.elapsed().as_nanos();
    let report = STATE.with(|s| {
        let s = s.borrow();
        let sum = s
            .regions
            .iter()
            .try_fold(0u128, |a, r| a.checked_add(r.elapsed_ns));
        let unattributed = sum.and_then(|sum| elapsed_ns.checked_sub(sum));
        Report {
            valid: s.valid && s.open.is_none() && unattributed.is_some(),
            elapsed_ns,
            unattributed_ns: unattributed.unwrap_or(0),
            regions: s.regions,
        }
    });
    drop(guard);
    Ok((value, report))
}
/// A current-thread region guard. Inactive spans do not read the clock.
/// ```compile_fail
/// use multiway_mg::automatic_profiling::{span, Phase};
/// let guard = span(Phase::LocalSolve);
/// std::thread::spawn(move || drop(guard));
/// ```
#[must_use = "keep the span alive through the measured region"]
pub struct Span {
    phase: Phase,
    generation: u64,
    start: Option<Instant>,
    _thread: PhantomData<Rc<()>>,
}
/// Begin a disjoint region. Overlap invalidates the collection; no nested sum is
/// silently counted twice. The outer span still completes for diagnostic context.
pub fn span(phase: Phase) -> Span {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let mut result = Span {
            phase,
            generation: s.generation,
            start: None,
            _thread: PhantomData,
        };
        if s.active {
            if s.open.is_some() {
                s.valid = false;
            } else {
                s.open = Some(phase);
                result.start = Some(Instant::now());
            }
        }
        result
    })
}
impl Drop for Span {
    fn drop(&mut self) {
        let Some(start) = self.start else {
            return;
        };
        let elapsed = start.elapsed().as_nanos();
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            if !s.active || s.generation != self.generation {
                return;
            }
            if s.open != Some(self.phase) {
                s.valid = false;
                return;
            }
            s.open = None;
            let r = &mut s.regions[self.phase as usize];
            match (r.calls.checked_add(1), r.elapsed_ns.checked_add(elapsed)) {
                (Some(calls), Some(elapsed_ns)) => *r = Region { calls, elapsed_ns },
                _ => s.valid = false,
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn accounted(r: Report) {
        assert!(r.valid);
        assert_eq!(
            r.elapsed_ns,
            r.unattributed_ns + r.regions.iter().map(|r| r.elapsed_ns).sum::<u128>()
        );
    }
    #[test]
    fn disjoint_regions_errors_and_inactive_spans() {
        let inactive = span(Phase::GlobalSolve);
        let (result, r) = collect(|| -> Result<(), ()> {
            {
                let _s = span(Phase::Partition);
            }
            {
                let _s = span(Phase::Partition);
            }
            let _s = span(Phase::LocalSolve);
            Err(())
        })
        .unwrap();
        assert!(result.is_err());
        accounted(r);
        assert_eq!(r.regions[Phase::Partition as usize].calls, 2);
        assert_eq!(r.regions[Phase::LocalSolve as usize].calls, 1);
        drop(inactive);
        let (_, r) = collect(|| ()).unwrap();
        accounted(r);
        assert!(r.regions.iter().all(|r| *r == EMPTY));
    }
    #[test]
    fn overlap_and_open_spans_fail_closed_and_stale_guards_are_ignored() {
        let (_, r) = collect(|| {
            let _a = span(Phase::Screening);
            let _b = span(Phase::Screening);
        })
        .unwrap();
        assert!(!r.valid);
        let (stale, r) = collect(|| span(Phase::LocalSolve)).unwrap();
        assert!(!r.valid);
        let (_, r) = collect(|| {
            let _s = span(Phase::LocalSolve);
            drop(stale);
        })
        .unwrap();
        accounted(r);
        assert_eq!(r.regions[Phase::LocalSolve as usize].calls, 1);
    }
    #[test]
    fn nested_collection_and_unwind_preserve_reuse() {
        let (_, r) = collect(|| {
            assert!(matches!(
                collect(|| panic!("must not run")),
                Err(CollectError::NestedCollection)
            ));
            let _s = span(Phase::DirectSolve);
        })
        .unwrap();
        accounted(r);
        let result = std::panic::catch_unwind(|| {
            collect(|| {
                let _s = span(Phase::DirectSolve);
                panic!("diagnostic unwind");
            })
        });
        assert!(result.is_err());
        let (_, r) = collect(|| {
            let _s = span(Phase::DirectSolve);
        })
        .unwrap();
        accounted(r);
        assert_eq!(r.regions[Phase::DirectSolve as usize].calls, 1);
    }
    #[test]
    fn overflow_is_reported_and_threads_are_isolated() {
        let (_, r) = collect(|| {
            STATE.with(|s| s.borrow_mut().regions[Phase::Partition as usize].calls = u64::MAX);
            let _s = span(Phase::Partition);
        })
        .unwrap();
        assert!(!r.valid);
        let (_, r) = collect(|| {
            STATE
                .with(|s| s.borrow_mut().regions[Phase::Partition as usize].elapsed_ns = u128::MAX);
            let _s = span(Phase::Partition);
        })
        .unwrap();
        assert!(!r.valid);
        let (_, r) = collect(|| {
            std::thread::spawn(|| {
                let _s = span(Phase::Partition);
            })
            .join()
            .unwrap()
        })
        .unwrap();
        accounted(r);
        assert!(r.regions.iter().all(|r| *r == EMPTY));
        let generation = STATE.with(|s| {
            let mut s = s.borrow_mut();
            let old = s.generation;
            s.generation = u64::MAX;
            old
        });
        assert!(matches!(
            collect(|| panic!("must not run")),
            Err(CollectError::GenerationOverflow)
        ));
        STATE.with(|s| s.borrow_mut().generation = generation);
        accounted(collect(|| ()).unwrap().1);
    }
}
