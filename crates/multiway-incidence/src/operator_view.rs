//! Allocation-free matrix-free actions bound to one immutable numerical frame.
use crate::{
    IncidenceError, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameBinding,
    kernels::OperatorData,
};

/// Borrowed original-operator actions for one exact immutable weight generation.
///
/// Construction and all actions allocate nothing. No scratch is required by the
/// serial kernels. Copies borrow the same frame; weights and topology are never
/// cloned. An old view remains valid while its old frame lives, and rejects a
/// different numerical owner even if its values happen to match.
///
/// Dimensions are checked before output writes. Arithmetic is exactly the scalar
/// [`crate::ThreeWayProblem`] protocol, including its dimension-error contexts.
/// These are low-level actions, not finite-value checks or solve certificates:
/// finite vectors can overflow and non-finite values propagate as in the original
/// kernels. Solver drivers must certify their candidates independently.
///
/// A view cannot outlive or permit replacement of its numerical owner:
/// ```compile_fail
/// use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
/// let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
/// let view = {
///     let frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
///     frame.operator_view()
/// };
/// view.apply_gramian(&[1.0; 3], &mut [0.0; 3]).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ThreeWayOperatorView<'frame, 'topology> {
    frame: &'frame ThreeWayWeightFrame<'topology>,
}

impl<'topology> ThreeWayWeightFrame<'topology> {
    /// Borrow this frame's original-operator actions without allocation.
    #[must_use]
    pub const fn operator_view(&self) -> ThreeWayOperatorView<'_, 'topology> {
        ThreeWayOperatorView { frame: self }
    }
}

impl<'frame, 'topology> ThreeWayOperatorView<'frame, 'topology> {
    /// Exact borrowed numerical owner.
    #[must_use]
    pub const fn frame(&self) -> &'frame ThreeWayWeightFrame<'topology> {
        self.frame
    }

    /// Exact borrowed structural owner.
    #[must_use]
    pub const fn topology(&self) -> &'topology PreparedThreeWayTopology {
        self.frame.topology()
    }

    /// Exact immutable numerical identity for downstream cache/workspace checks.
    #[must_use]
    pub const fn binding(&self) -> WeightFrameBinding<'frame, 'topology> {
        self.frame.binding()
    }

    /// Reject an equal or changed foreign numerical generation without mutation.
    pub fn validate_for(&self, frame: &ThreeWayWeightFrame<'_>) -> Result<(), IncidenceError> {
        self.binding().validate_for(frame)
    }

    /// Number of coefficient coordinates.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.topology().topology().total_levels()
    }

    /// Number of canonical unique tuples.
    #[must_use]
    pub fn tuple_count(&self) -> usize {
        self.topology().topology().tuple_count()
    }

    /// Exclusive retained heap payload: zero; the borrowed frame is counted by its owner.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> usize {
        0
    }

    fn data(&self) -> OperatorData<'_> {
        OperatorData {
            topology: self.topology().topology(),
            weights: self.frame.weights(),
            square_root_weights: self.frame.square_root_weights(),
        }
    }

    /// Compute `out = B x` in caller storage.
    pub fn apply_incidence(&self, x: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        self.data().apply_incidence(x, out)
    }

    /// Compute `out = B' y` in caller storage.
    pub fn apply_adjoint(&self, y: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        self.data().apply_adjoint(y, out)
    }

    /// Compute `out = sqrt(W) B x` in caller storage.
    pub fn apply_weighted_incidence(
        &self,
        x: &[f64],
        out: &mut [f64],
    ) -> Result<(), IncidenceError> {
        self.data().apply_weighted_incidence(x, out)
    }

    /// Compute `out = B' sqrt(W) y` in caller storage.
    pub fn apply_weighted_adjoint(&self, y: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        self.data().apply_weighted_adjoint(y, out)
    }

    /// Compute `out = B' W B x` in caller storage.
    pub fn apply_gramian(&self, x: &[f64], out: &mut [f64]) -> Result<(), IncidenceError> {
        self.data().apply_gramian(x, out)
    }

    /// Compute the compensated tuple energy `x' B' W B x`.
    pub fn energy(&self, x: &[f64]) -> Result<f64, IncidenceError> {
        self.data().energy(x)
    }

    /// Compute `rhs = B' W targets` in caller storage.
    pub fn rhs_from_targets_into(
        &self,
        targets: &[f64],
        rhs: &mut [f64],
    ) -> Result<(), IncidenceError> {
        self.data().rhs_from_targets_into(targets, rhs)
    }

    /// Compute `out = rhs - B' W B x` in caller storage.
    pub fn residual_into(
        &self,
        rhs: &[f64],
        x: &[f64],
        out: &mut [f64],
    ) -> Result<(), IncidenceError> {
        self.data().residual_into(rhs, x, out)
    }
}
