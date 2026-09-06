//! Opt-in current-thread diagnostic spans; never authoritative benchmark timing.
use std::{cell::RefCell, marker::PhantomData, rc::Rc, time::Instant};

/// Instrumented operation categories. Nested work belongs to exclusive child time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Phase {
    /// Unweighted incidence, including calls nested inside weighted incidence.
    Incidence,
    /// Unweighted adjoint.
    Adjoint,
    /// Weighted incidence, including its nested unweighted incidence.
    WeightedIncidence,
    /// Weighted adjoint.
    WeightedAdjoint,
    /// Gramian action, including actions inside cycles.
    Gramian,
    /// Original weighted right-hand side or residual gradient.
    Rhs,
    /// Structural projection arithmetic.
    Projection,
    /// Ordered forward/middle/reverse MAP arithmetic.
    MapSweep,
    /// Factor-map restriction.
    Restriction,
    /// Factor-map prolongation.
    Prolongation,
    /// Dense terminal application.
    DenseTerminal,
    /// Complete recursive cycle frame, including children.
    Cycle,
    /// PCG recurrence and nested actions.
    PcgRecurrence,
    /// Complete prepared PCG including RHS construction and final certificate.
    PreparedPcg,
    /// Prepared LSMR, validation and nested actions.
    PreparedLsmr,
    /// Original certificate, including operator applications and checks.
    Certificate,
}
/// Stable complete phase inventory, including zero-count phases.
pub const PHASES: [Phase; 16] = [
    Phase::Incidence,
    Phase::Adjoint,
    Phase::WeightedIncidence,
    Phase::WeightedAdjoint,
    Phase::Gramian,
    Phase::Rhs,
    Phase::Projection,
    Phase::MapSweep,
    Phase::Restriction,
    Phase::Prolongation,
    Phase::DenseTerminal,
    Phase::Cycle,
    Phase::PcgRecurrence,
    Phase::PreparedPcg,
    Phase::PreparedLsmr,
    Phase::Certificate,
];
impl Phase {
    /// Stable machine-readable category name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Incidence => "incidence",
            Self::Adjoint => "adjoint",
            Self::WeightedIncidence => "weighted_incidence",
            Self::WeightedAdjoint => "weighted_adjoint",
            Self::Gramian => "gramian",
            Self::Rhs => "rhs",
            Self::Projection => "projection",
            Self::MapSweep => "map_sweep",
            Self::Restriction => "restriction",
            Self::Prolongation => "prolongation",
            Self::DenseTerminal => "dense_terminal",
            Self::Cycle => "cycle",
            Self::PcgRecurrence => "pcg_recurrence",
            Self::PreparedPcg => "prepared_pcg",
            Self::PreparedLsmr => "prepared_lsmr",
            Self::Certificate => "certificate",
        }
    }
}
/// Aggregate diagnostic time for one category. Do not sum inclusive times.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PhaseProfile {
    /// Number of completed spans, including spans unwound on errors.
    pub calls: u64,
    /// Total span duration including nested instrumented work.
    pub inclusive_ns: u128,
    /// Span duration minus complete nested spans; includes observer overhead.
    pub exclusive_ns: u128,
}
const MAX_DEPTH: usize = 128;
const EMPTY_PROFILE: PhaseProfile = PhaseProfile {
    calls: 0,
    inclusive_ns: 0,
    exclusive_ns: 0,
};
#[derive(Clone, Copy)]
struct Frame {
    phase: Phase,
    start: Option<Instant>,
    children_ns: u128,
}
const EMPTY_FRAME: Frame = Frame {
    phase: Phase::Incidence,
    start: None,
    children_ns: 0,
};
struct State {
    active: bool,
    valid: bool,
    generation: u64,
    depth: usize,
    maximum_depth: usize,
    frames: [Frame; MAX_DEPTH],
    phases: [PhaseProfile; PHASES.len()],
}
impl State {
    const fn new() -> Self {
        Self {
            active: false,
            valid: true,
            generation: 0,
            depth: 0,
            maximum_depth: 0,
            frames: [EMPTY_FRAME; MAX_DEPTH],
            phases: [EMPTY_PROFILE; PHASES.len()],
        }
    }
}
thread_local! { static STATE: RefCell<State> = const { RefCell::new(State::new()) }; }

/// Fixed profiler thread-local inline state bytes; allocator metadata/RSS are separate.
#[must_use]
pub const fn thread_local_payload_bytes() -> usize {
    core::mem::size_of::<RefCell<State>>()
}

/// A completed current-thread collection; no per-event history is retained.
#[derive(Debug, Clone, Copy)]
pub struct ProfileReport {
    /// False after counter/stack overflow, out-of-order guards or an open final span.
    pub valid: bool,
    /// Entire callback elapsed time; excludes reset and report extraction.
    pub elapsed_ns: u128,
    /// Maximum simultaneously active span count.
    pub maximum_depth: usize,
    /// Fixed complete inventory in [`PHASES`] order.
    pub phases: [PhaseProfile; PHASES.len()],
}
/// Failure to start an explicit diagnostic collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    /// A collection is already active on this thread; it is left unchanged.
    NestedCollection,
    /// The scope generation counter cannot be represented.
    GenerationOverflow,
}
impl core::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ProfileError {}

struct CollectionGuard {
    generation: u64,
}
impl Drop for CollectionGuard {
    fn drop(&mut self) {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            if s.generation == self.generation {
                s.active = false;
                s.depth = 0;
            }
        });
    }
}
/// Collect one serial callback without allocating profiler arrays.
///
/// Collection does not propagate to spawned threads. Timers perturb execution;
/// retain uninstrumented complete-cost measurements separately. Nested collection
/// rejects before calling its callback. Panic unwinding disables the collection,
/// so a later call recovers. A returned still-live span makes the report invalid.
pub fn collect<T>(f: impl FnOnce() -> T) -> Result<(T, ProfileReport), ProfileError> {
    let generation = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.active {
            return Err(ProfileError::NestedCollection);
        }
        let generation = s
            .generation
            .checked_add(1)
            .ok_or(ProfileError::GenerationOverflow)?;
        s.generation = generation;
        s.depth = 0;
        s.maximum_depth = 0;
        s.valid = true;
        s.phases.fill(EMPTY_PROFILE);
        s.active = true;
        Ok(generation)
    })?;
    let guard = CollectionGuard { generation };
    let started = Instant::now();
    let value = f();
    let elapsed_ns = started.elapsed().as_nanos();
    let report = STATE.with(|s| {
        let s = s.borrow();
        ProfileReport {
            valid: s.valid && s.depth == 0,
            elapsed_ns,
            maximum_depth: s.maximum_depth,
            phases: s.phases,
        }
    });
    drop(guard);
    Ok((value, report))
}

/// Non-Send span guard. It records only within an explicitly active collection.
/// ```compile_fail
/// use multiway_incidence::profiling::{span, Phase};
/// let guard = span(Phase::Cycle);
/// std::thread::spawn(move || drop(guard));
/// ```
#[must_use = "the span must remain alive through the instrumented operation"]
pub struct Span {
    index: Option<usize>,
    generation: u64,
    _thread: PhantomData<Rc<()>>,
}
/// Begin an opt-in nested span. Inactive calls read the flag but never read the clock.
pub fn span(phase: Phase) -> Span {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let mut result = Span {
            index: None,
            generation: s.generation,
            _thread: PhantomData,
        };
        if !s.active {
            return result;
        }
        if s.depth == MAX_DEPTH {
            s.valid = false;
            return result;
        }
        let index = s.depth;
        s.frames[index] = Frame {
            phase,
            start: Some(Instant::now()),
            children_ns: 0,
        };
        s.depth += 1;
        s.maximum_depth = s.maximum_depth.max(s.depth);
        result.index = Some(index);
        result
    })
}
impl Drop for Span {
    fn drop(&mut self) {
        let Some(index) = self.index else {
            return;
        };
        let end = Instant::now();
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            if !s.active || self.generation != s.generation {
                return;
            }
            if s.depth != index + 1 {
                s.valid = false;
                s.depth = 0;
                return;
            }
            let frame = s.frames[index];
            s.depth -= 1;
            let elapsed = end
                .duration_since(frame.start.expect("active span timestamp"))
                .as_nanos();
            let Some(exclusive) = elapsed.checked_sub(frame.children_ns) else {
                s.valid = false;
                return;
            };
            let before = s.phases[frame.phase as usize];
            let Some(calls) = before.calls.checked_add(1) else {
                s.valid = false;
                return;
            };
            let Some(inclusive_ns) = before.inclusive_ns.checked_add(elapsed) else {
                s.valid = false;
                return;
            };
            let Some(exclusive_ns) = before.exclusive_ns.checked_add(exclusive) else {
                s.valid = false;
                return;
            };
            s.phases[frame.phase as usize] = PhaseProfile {
                calls,
                inclusive_ns,
                exclusive_ns,
            };
            if index > 0 {
                let Some(total) = s.frames[index - 1].children_ns.checked_add(elapsed) else {
                    s.valid = false;
                    return;
                };
                s.frames[index - 1].children_ns = total;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_accounting_disabled_spans_and_recovery() {
        let inactive = span(Phase::MapSweep);
        drop(inactive);
        let ((), r) = collect(|| {
            let _outer = span(Phase::Cycle);
            {
                let _child = span(Phase::MapSweep);
                std::hint::black_box([1.0; 16]);
            }
            assert!(matches!(
                collect(|| panic!("nested callback must not run")),
                Err(ProfileError::NestedCollection)
            ));
        })
        .unwrap();
        assert!(r.valid);
        assert_eq!(r.maximum_depth, 2);
        assert_eq!(r.phases[Phase::Cycle as usize].calls, 1);
        assert_eq!(r.phases[Phase::MapSweep as usize].calls, 1);
        assert_eq!(
            r.phases[Phase::Cycle as usize].inclusive_ns,
            r.phases[Phase::Cycle as usize].exclusive_ns
                + r.phases[Phase::MapSweep as usize].inclusive_ns
        );
        assert!(r.phases.iter().map(|p| p.exclusive_ns).sum::<u128>() <= r.elapsed_ns);
        assert!(
            std::panic::catch_unwind(|| {
                let _ = collect(|| {
                    let _s = span(Phase::Cycle);
                    panic!("unwind");
                });
            })
            .is_err()
        );
        let ((), next) = collect(|| {}).unwrap();
        assert!(next.valid);
        assert_eq!(next.phases, [EMPTY_PROFILE; PHASES.len()]);
    }
    #[test]
    fn malformed_lifetimes_and_overflows_fail_closed_and_recover() {
        let ((), r) = collect(|| {
            let a = span(Phase::Cycle);
            let b = span(Phase::MapSweep);
            drop(a);
            drop(b);
        })
        .unwrap();
        assert!(!r.valid);
        let (open, r) = collect(|| span(Phase::Cycle)).unwrap();
        assert!(!r.valid);
        drop(open);
        let ((), r) = collect(|| {
            let mut spans: [Option<Span>; MAX_DEPTH + 1] = std::array::from_fn(|_| None);
            for s in &mut spans {
                *s = Some(span(Phase::Cycle));
            }
            for s in spans.iter_mut().rev() {
                drop(s.take());
            }
        })
        .unwrap();
        assert!(!r.valid);
        let ((), r) = collect(|| {
            STATE.with(|s| s.borrow_mut().phases[Phase::Cycle as usize].calls = u64::MAX);
            let _s = span(Phase::Cycle);
        })
        .unwrap();
        assert!(!r.valid);
        assert!(collect(|| {}).unwrap().1.valid);
    }
    #[test]
    fn stale_guards_and_clock_accounting_failures_do_not_poison_later_scopes() {
        let (old, report) = collect(|| span(Phase::Cycle)).unwrap();
        assert!(!report.valid);
        let ((), next) = collect(|| {
            drop(old);
            let _span = span(Phase::MapSweep);
        })
        .unwrap();
        assert!(next.valid);
        assert_eq!(next.phases[Phase::Cycle as usize].calls, 0);
        assert_eq!(next.phases[Phase::MapSweep as usize].calls, 1);
        let ((), bad) = collect(|| {
            let _span = span(Phase::Cycle);
            STATE.with(|s| s.borrow_mut().frames[0].children_ns = u128::MAX);
        })
        .unwrap();
        assert!(!bad.valid);
        let old_generation = STATE.with(|s| {
            let mut s = s.borrow_mut();
            let old = s.generation;
            s.generation = u64::MAX;
            old
        });
        assert!(matches!(
            collect(|| panic!("overflow callback must not run")),
            Err(ProfileError::GenerationOverflow)
        ));
        STATE.with(|s| s.borrow_mut().generation = old_generation);
        assert!(collect(|| {}).unwrap().1.valid);
    }
}
