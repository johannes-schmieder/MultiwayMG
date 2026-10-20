//! Two-sided exact-range projection around one fixed approximate-inverse action.
//!
//! For a symmetric action `M`, `P M P` with the certified range projector `P` is
//! symmetric, maps into the complete exact range of every positive-weight
//! Gramian on the same topology and annihilates its complete kernel. It is
//! positive on that range exactly when `M` is. Structural factor-shift
//! projection alone has the same properties only on components whose exact
//! kernel is structural.

use multiway_incidence::IncidenceRangeProjector;

use crate::{MultiwayError, Preconditioner};

/// Apply `out = P M P rhs` with caller-owned `scratch` and no allocation.
///
/// All three dimensions are checked before any output or scratch mutation.
/// On an inner failure `scratch` holds the projected input and `out` is
/// unspecified; callers must not consume it.
pub fn apply_range_projected<P: Preconditioner + ?Sized>(
    projector: &IncidenceRangeProjector<'_>,
    action: &P,
    rhs: &[f64],
    scratch: &mut [f64],
    out: &mut [f64],
) -> Result<(), MultiwayError> {
    let dimension = projector.dimension();
    for (context, actual) in [
        ("range-projected action", action.dimension()),
        ("range-projected rhs", rhs.len()),
        ("range-projected scratch", scratch.len()),
        ("range-projected output", out.len()),
    ] {
        if actual != dimension {
            return Err(crate::error::dimension(context, dimension, actual));
        }
    }
    scratch.copy_from_slice(rhs);
    projector.project(scratch)?;
    action.apply(scratch, out)?;
    projector.project(out)?;
    Ok(())
}

/// Diagnostic [`Preconditioner`] over [`apply_range_projected`].
///
/// This convenience wrapper allocates one coefficient vector per application;
/// repeated prepared solves should call the workspace function directly.
#[derive(Debug)]
pub struct RangeProjectedAction<'p, 'a, P> {
    projector: &'p IncidenceRangeProjector<'a>,
    action: P,
}

impl<'p, 'a, P: Preconditioner> RangeProjectedAction<'p, 'a, P> {
    /// Wrap one fixed action whose dimension must match the projector.
    pub fn new(
        projector: &'p IncidenceRangeProjector<'a>,
        action: P,
    ) -> Result<Self, MultiwayError> {
        if action.dimension() != projector.dimension() {
            return Err(crate::error::dimension(
                "RangeProjectedAction::new action",
                projector.dimension(),
                action.dimension(),
            ));
        }
        Ok(Self { projector, action })
    }

    /// The wrapped fixed action.
    pub const fn action(&self) -> &P {
        &self.action
    }
}

impl<P: Preconditioner> Preconditioner for RangeProjectedAction<'_, '_, P> {
    fn dimension(&self) -> usize {
        self.projector.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        let mut scratch = vec![0.0; self.projector.dimension()];
        apply_range_projected(self.projector, &self.action, rhs, &mut scratch, out)
    }
}
