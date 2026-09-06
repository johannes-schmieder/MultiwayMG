//! Shared fail-closed original-operator normal-equation certification.
use crate::MultiwayError;
use multiway_incidence::{
    ThreeWayOperatorView, ThreeWayProblem, ThreeWayWeightFrame, WeightFrameBinding,
};

/// Actual original-operator applications attempted by the last admitted certificate.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CertificateWorkReport {
    /// Unweighted incidence applications to the candidate coefficients.
    pub incidence_applications: usize,
    /// Weighted adjoint applications for residual gradient and reference RHS.
    pub adjoint_applications: usize,
}

/// Caller-owned certificate arrays bound to one exact immutable numerical frame.
///
/// Targets are in canonical tuple order, not raw observation order. The workspace
/// owns one tuple residual and two coefficient vectors, with no numerical operator
/// copies. A certificate is `||B'W(y-Bx)|| / ||B'Wy||`; native solver flags play no
/// role. Zero/zero is zero. Unrepresentable products, norms or ratios fail closed.
#[derive(Debug)]
pub struct PreparedCertificateWorkspace<'frame, 'topology> {
    binding: WeightFrameBinding<'frame, 'topology>,
    scratch: CertificateScratch,
}
impl<'frame, 'topology> PreparedCertificateWorkspace<'frame, 'topology> {
    /// Reserve all certificate arrays fallibly at an explicit setup boundary.
    pub fn try_new(frame: &'frame ThreeWayWeightFrame<'topology>) -> Result<Self, MultiwayError> {
        Ok(Self {
            binding: frame.binding(),
            scratch: CertificateScratch::try_new(frame.weights().len(), frame.diagonal().len())?,
        })
    }
    /// Requested exclusive scratch payload; excludes borrowed immutable owners.
    pub fn required_payload_bytes(frame: &ThreeWayWeightFrame<'_>) -> Result<usize, MultiwayError> {
        CertificateScratch::required_payload_bytes(frame.weights().len(), frame.diagonal().len())
    }
    /// Actual exclusive array capacities, excluding inline state and allocator metadata.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        self.scratch.retained_payload_bytes()
    }
    /// Exact current-frame validation before any mutation.
    pub fn validate_for(&self, frame: &ThreeWayWeightFrame<'_>) -> Result<(), MultiwayError> {
        self.binding.validate_for(frame)?;
        Ok(())
    }
    /// Counts for the last certificate that passed static validation, including failure.
    #[must_use]
    pub const fn last_work(&self) -> CertificateWorkReport {
        self.scratch.work
    }
}

/// Independently certify the submitted candidate without allocation.
///
/// Both dimensions, exact frame and finite inputs are checked before scratch or
/// work counters are changed. Invalid arithmetic returns a typed error; a later
/// valid certificate reuses the same storage. No candidate or tolerance is trusted
/// from the iterative solver. The caller must compare the finite result to its
/// declared positive tolerance; this function makes no rank or minimum-norm claim.
pub fn certify_prepared_normal_equations(
    operator: ThreeWayOperatorView<'_, '_>,
    targets: &[f64],
    coefficients: &[f64],
    workspace: &mut PreparedCertificateWorkspace<'_, '_>,
) -> Result<f64, MultiwayError> {
    workspace.validate_for(operator.frame())?;
    certify(&operator, targets, coefficients, &mut workspace.scratch)
}

#[derive(Debug)]
pub(crate) struct CertificateScratch {
    residual: Vec<f64>,
    gradient: Vec<f64>,
    reference: Vec<f64>,
    work: CertificateWorkReport,
}
impl CertificateScratch {
    pub(crate) fn try_new(rows: usize, cols: usize) -> Result<Self, MultiwayError> {
        Self::required_payload_bytes(rows, cols)?;
        Ok(Self {
            residual: vector(rows)?,
            gradient: vector(cols)?,
            reference: vector(cols)?,
            work: CertificateWorkReport::default(),
        })
    }
    fn required_payload_bytes(rows: usize, cols: usize) -> Result<usize, MultiwayError> {
        bytes(rows)?
            .checked_add(bytes(cols)?.checked_mul(2).ok_or_else(overflow)?)
            .ok_or_else(overflow)
    }
    fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        [&self.residual, &self.gradient, &self.reference]
            .iter()
            .try_fold(0usize, |total, v| {
                total.checked_add(bytes(v.capacity())?).ok_or_else(overflow)
            })
    }
}

pub(crate) trait CertificateOperator {
    fn rows(&self) -> usize;
    fn cols(&self) -> usize;
    fn weights(&self) -> &[f64];
    fn incidence(&self, x: &[f64], out: &mut [f64]) -> Result<(), MultiwayError>;
    fn rhs(&self, targets: &[f64], out: &mut [f64]) -> Result<(), MultiwayError>;
}
impl CertificateOperator for ThreeWayProblem {
    fn rows(&self) -> usize {
        self.tuple_count()
    }
    fn cols(&self) -> usize {
        self.dimension()
    }
    fn weights(&self) -> &[f64] {
        self.weights()
    }
    fn incidence(&self, x: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.apply_incidence(x, out)?;
        Ok(())
    }
    fn rhs(&self, targets: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.rhs_from_targets_into(targets, out)?;
        Ok(())
    }
}
impl CertificateOperator for ThreeWayOperatorView<'_, '_> {
    fn rows(&self) -> usize {
        self.tuple_count()
    }
    fn cols(&self) -> usize {
        self.dimension()
    }
    fn weights(&self) -> &[f64] {
        self.frame().weights()
    }
    fn incidence(&self, x: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.apply_incidence(x, out)?;
        Ok(())
    }
    fn rhs(&self, targets: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.rhs_from_targets_into(targets, out)?;
        Ok(())
    }
}

pub(crate) fn certify<O: CertificateOperator>(
    operator: &O,
    targets: &[f64],
    coefficients: &[f64],
    scratch: &mut CertificateScratch,
) -> Result<f64, MultiwayError> {
    #[cfg(feature = "profiling")]
    let _profile_span =
        multiway_incidence::profiling::span(multiway_incidence::profiling::Phase::Certificate);

    if targets.len() != operator.rows() {
        return Err(crate::error::dimension(
            "certificate targets",
            operator.rows(),
            targets.len(),
        ));
    }
    if coefficients.len() != operator.cols() {
        return Err(crate::error::dimension(
            "certificate coefficients",
            operator.cols(),
            coefficients.len(),
        ));
    }
    ensure_finite(targets, "certificate targets")?;
    ensure_finite(coefficients, "certificate coefficients")?;
    scratch.work = CertificateWorkReport::default();
    scratch.work.incidence_applications += 1;
    operator.incidence(coefficients, &mut scratch.residual)?;
    for (value, &target) in scratch.residual.iter_mut().zip(targets) {
        *value = target - *value;
    }
    ensure_finite(&scratch.residual, "certificate residual")?;
    validate_weighted_products(operator.weights(), &scratch.residual)?;
    validate_weighted_products(operator.weights(), targets)?;
    scratch.work.adjoint_applications += 1;
    operator.rhs(&scratch.residual, &mut scratch.gradient)?;
    scratch.work.adjoint_applications += 1;
    operator.rhs(targets, &mut scratch.reference)?;
    ensure_finite(&scratch.gradient, "certificate gradient")?;
    ensure_finite(&scratch.reference, "certificate reference")?;
    let numerator = norm(&scratch.gradient);
    let denominator = norm(&scratch.reference);
    ensure_finite(&[numerator, denominator], "certificate norms")?;
    let residual = if denominator == 0.0 && numerator == 0.0 {
        0.0
    } else {
        numerator / denominator
    };
    ensure_finite(&[residual], "certificate relative residual")?;
    if numerator != 0.0 && residual == 0.0 {
        return Err(MultiwayError::NumericalFailure {
            context: "certificate ratio underflow",
        });
    }
    Ok(residual)
}

pub(crate) fn ensure_finite(values: &[f64], context: &'static str) -> Result<(), MultiwayError> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(MultiwayError::NumericalFailure { context })
    }
}
fn validate_weighted_products(weights: &[f64], values: &[f64]) -> Result<(), MultiwayError> {
    for (&weight, &value) in weights.iter().zip(values) {
        let product = weight * value;
        if !product.is_finite() || (value != 0.0 && product == 0.0) {
            return Err(MultiwayError::NumericalFailure {
                context: "certificate weighted product",
            });
        }
    }
    Ok(())
}
fn norm(values: &[f64]) -> f64 {
    let scale = values.iter().copied().map(f64::abs).fold(0.0, f64::max);
    if scale == 0.0 {
        return 0.0;
    }
    scale
        * values
            .iter()
            .map(|value| (value / scale) * (value / scale))
            .sum::<f64>()
            .sqrt()
}
pub(crate) fn vector(count: usize) -> Result<Vec<f64>, MultiwayError> {
    bytes(count)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|source| MultiwayError::WorkspaceAllocation {
            context: "prepared solve vector",
            source,
        })?;
    values.resize(count, 0.0);
    Ok(values)
}
pub(crate) fn bytes(count: usize) -> Result<usize, MultiwayError> {
    count
        .checked_mul(8)
        .filter(|&n| n <= isize::MAX as usize)
        .ok_or_else(overflow)
}
fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow {
        context: "certificate workspace",
    }
}
