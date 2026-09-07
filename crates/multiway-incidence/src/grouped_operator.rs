//! Opt-in stable row gathers and caller-scratch tuple-image Gramian.
use crate::{IncidenceError, PreparedTupleGrouping, ThreeWayOperatorView, kernels::validate_len};

/// Borrowed original operator plus an exactly matching weights-free row grouping.
///
/// Construction validates structural identity, then immutable borrows keep that
/// relation valid. Numerical values always come from the submitted current frame.
/// The view owns zero arrays. All public actions validate dimensions before any
/// output/image writes. Arithmetic preserves each coefficient's canonical tuple
/// order and the scalar product/sum association; nonfinite values propagate as in
/// the low-level scalar kernels. Acceptance still requires an independent solve
/// certificate. This is an explicit alternative, not an automatic layout selector.
/// A view cannot outlive the optional grouping owner:
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, PreparedTupleGrouping,
///     ThreeWayWeightFrame, WeightFrameInput};
/// let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
/// let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
/// let view = {
///     let groups = PreparedTupleGrouping::try_new(&t).unwrap();
///     f.operator_view().with_grouping(&groups).unwrap()
/// };
/// view.apply_gramian(&[1.0; 3], &mut [0.0; 3]).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ThreeWayGroupedOperatorView<'groups, 'frame, 'topology> {
    original: ThreeWayOperatorView<'frame, 'topology>,
    grouping: &'groups PreparedTupleGrouping<'topology>,
}
impl<'frame, 'topology> ThreeWayOperatorView<'frame, 'topology> {
    /// Bind optional stable coefficient rows without copying topology or weights.
    pub fn with_grouping<'groups>(
        self,
        grouping: &'groups PreparedTupleGrouping<'topology>,
    ) -> Result<ThreeWayGroupedOperatorView<'groups, 'frame, 'topology>, IncidenceError> {
        grouping.validate_for(self.topology())?;
        Ok(ThreeWayGroupedOperatorView {
            original: self,
            grouping,
        })
    }
}
impl<'groups, 'frame, 'topology> ThreeWayGroupedOperatorView<'groups, 'frame, 'topology> {
    /// The original scalar view, suitable for independent certification.
    #[must_use]
    pub const fn original(&self) -> ThreeWayOperatorView<'frame, 'topology> {
        self.original
    }
    /// Exact borrowed grouping owner; count its arrays once at that owner.
    #[must_use]
    pub const fn grouping(&self) -> &'groups PreparedTupleGrouping<'topology> {
        self.grouping
    }
    /// Coefficient dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.original.dimension()
    }
    /// Canonical tuple count.
    #[must_use]
    pub fn tuple_count(&self) -> usize {
        self.original.tuple_count()
    }
    /// Exclusive retained payload is zero; all data are borrowed.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> usize {
        0
    }

    /// Original canonical tuple pass `out = B x`; no grouping is needed.
    pub fn apply_incidence(&self, x: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        self.original.apply_incidence(x, out)
    }
    /// Original canonical weighted tuple pass `out = sqrt(W) B x`.
    pub fn apply_weighted_incidence(
        &self,
        x: &[f64],
        out: &mut [f64],
    ) -> Result<(), IncidenceError> {
        self.original.apply_weighted_incidence(x, out)
    }
    /// Stable row gather `out = B' y`, with disjoint coefficient output ownership.
    pub fn apply_adjoint(&self, y: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _span = crate::profiling::span(crate::profiling::Phase::Adjoint);
        validate_len("grouped adjoint input", self.tuple_count(), y.len())?;
        validate_len("grouped adjoint output", self.dimension(), out.len())?;
        self.reduce(out, |id| y[id]);
        Ok(())
    }
    /// Stable row gather `out = B' sqrt(W) y` in original addition order.
    pub fn apply_weighted_adjoint(&self, y: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _span = crate::profiling::span(crate::profiling::Phase::WeightedAdjoint);
        validate_len(
            "grouped weighted adjoint input",
            self.tuple_count(),
            y.len(),
        )?;
        validate_len(
            "grouped weighted adjoint output",
            self.dimension(),
            out.len(),
        )?;
        let roots = self.original.frame().square_root_weights();
        self.reduce(out, |id| roots[id] * y[id]);
        Ok(())
    }
    /// Stable row gather `out = B' W targets` using current numerical weights.
    pub fn rhs_from_targets_into(
        &self,
        targets: &[f64],
        out: &mut [f64],
    ) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _span = crate::profiling::span(crate::profiling::Phase::Rhs);
        validate_len("grouped RHS input", self.tuple_count(), targets.len())?;
        validate_len("grouped RHS output", self.dimension(), out.len())?;
        let weights = self.original.frame().weights();
        self.reduce(out, |id| weights[id] * targets[id]);
        Ok(())
    }
    /// Stable row Gramian gather without tuple scratch; each factor rereads images.
    pub fn apply_gramian(&self, x: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _span = crate::profiling::span(crate::profiling::Phase::Gramian);
        validate_len("grouped Gramian input", self.dimension(), x.len())?;
        validate_len("grouped Gramian output", self.dimension(), out.len())?;
        let t = self.original.topology().topology();
        let offsets = t.offsets();
        let weights = self.original.frame().weights();
        self.reduce(out, |id| {
            let tuple = t.tuples()[id];
            weights[id]
                * (x[tuple[0] as usize]
                    + x[offsets[1] + tuple[1] as usize]
                    + x[offsets[2] + tuple[2] as usize])
        });
        Ok(())
    }
    /// Two-pass Gramian: write E weighted tuple images, then stable row reduction.
    ///
    /// The caller owns the entire image array and must charge its 8E-byte capacity.
    /// It is fully overwritten before reading, so prior numerical generations or
    /// poisoned scratch cannot leak into the result. Validate all dimensions before
    /// modifying either image or output. No allocation or hidden image cache occurs.
    pub fn apply_gramian_with_image(
        &self,
        x: &[f64],
        out: &mut [f64],
        image: &mut [f64],
    ) -> Result<(), IncidenceError> {
        #[cfg(feature = "profiling")]
        let _span = crate::profiling::span(crate::profiling::Phase::Gramian);
        validate_len("grouped image Gramian input", self.dimension(), x.len())?;
        validate_len("grouped image Gramian output", self.dimension(), out.len())?;
        validate_len("grouped Gramian image", self.tuple_count(), image.len())?;
        let t = self.original.topology().topology();
        let offsets = t.offsets();
        for ((value, tuple), &weight) in image
            .iter_mut()
            .zip(t.tuples())
            .zip(self.original.frame().weights())
        {
            *value = weight
                * (x[tuple[0] as usize]
                    + x[offsets[1] + tuple[1] as usize]
                    + x[offsets[2] + tuple[2] as usize]);
        }
        self.reduce(out, |id| image[id]);
        Ok(())
    }
    /// Original compensated tuple energy, preserving the scalar arithmetic.
    pub fn energy(&self, x: &[f64]) -> Result<f64, IncidenceError> {
        self.original.energy(x)
    }
    /// Compute `out = rhs - G x` with the scratch-free grouped Gramian.
    pub fn residual_into(
        &self,
        rhs: &[f64],
        x: &[f64],
        out: &mut [f64],
    ) -> Result<(), IncidenceError> {
        validate_len("grouped residual RHS", self.dimension(), rhs.len())?;
        self.apply_gramian(x, out)?;
        for (value, &right) in out.iter_mut().zip(rhs) {
            *value = right - *value;
        }
        Ok(())
    }
    #[inline]
    fn reduce(&self, out: &mut [f64], mut value: impl FnMut(usize) -> f64) {
        let t = self.original.topology().topology();
        let offsets = t.offsets();
        for factor in 0..3 {
            for (level, coefficient) in out[offsets[factor]..offsets[factor + 1]]
                .iter_mut()
                .enumerate()
            {
                let mut sum = 0.0;
                self.grouping
                    .row(factor, level)
                    .expect("validated factor row")
                    .for_each(|id| sum += value(id));
                *coefficient = sum;
            }
        }
    }
}
