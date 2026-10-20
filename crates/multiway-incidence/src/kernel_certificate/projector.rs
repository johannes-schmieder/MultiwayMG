//! Allocation-free orthogonal projection onto the certified incidence range.

use super::{ComponentKernelKind, IncidenceKernelCertificate, IncidenceKernelLimits, Ledger};
use crate::{
    IncidenceError, PreparedThreeWayTopology, PreparedTopologyBinding,
    construction::{array_bytes, sum_bytes},
};

/// Largest integer magnitude converted to `f64` without rounding.
const EXACT_FLOAT_INTEGER: u64 = 1 << 53;

/// Construction diagnostics of one range projector.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IncidenceRangeProjectorReport {
    /// Largest `|q_i . q_j - delta_ij|` over every component basis.
    pub maximum_orthogonality_defect: f64,
    /// Largest `|q_a + q_b + q_c|` of a unit basis vector on any incidence row.
    pub maximum_incidence_residual: f64,
    /// Peak live requested payload, including the consumed certificate's arrays.
    pub peak_requested_payload_bytes: usize,
}

/// Orthogonal projector onto the complete exact range of every positive-weight Gramian.
///
/// Holds the consumed exact certificate plus a Euclidean-orthonormal floating
/// basis of each component kernel: the closed-form factor shifts of structural
/// components and the orthonormalized verified integer basis of extended ones.
/// Projection is topology-only, deterministic and allocation-free, so it is
/// shared by every numerical weight frame on the same prepared owner.
#[derive(Debug)]
pub struct IncidenceRangeProjector<'a> {
    certificate: IncidenceKernelCertificate<'a>,
    basis_offsets: Vec<usize>,
    basis: Vec<f64>,
    report: IncidenceRangeProjectorReport,
}

impl<'a> IncidenceKernelCertificate<'a> {
    /// Orthonormalize the certified kernel and keep both for projection.
    ///
    /// `owner` must be the certified prepared topology. Each component basis is
    /// orthonormalized twice by modified Gram--Schmidt in fixed order and checked
    /// against `tolerance` for orthogonality and incidence residual.
    pub fn try_into_projector(
        self,
        owner: &PreparedThreeWayTopology,
        limits: IncidenceKernelLimits,
        tolerance: f64,
    ) -> Result<IncidenceRangeProjector<'a>, IncidenceError> {
        IncidenceRangeProjector::build_with(self, owner, limits, tolerance, &mut |_| Ok(()))
    }
}

impl<'a> IncidenceRangeProjector<'a> {
    pub(crate) fn build_with<F>(
        certificate: IncidenceKernelCertificate<'a>,
        owner: &PreparedThreeWayTopology,
        limits: IncidenceKernelLimits,
        tolerance: f64,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        certificate
            .binding
            .validate_for(owner)
            .map_err(|_| IncidenceError::KernelBindingMismatch)?;
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(IncidenceError::KernelProjectorDefect {
                component: 0,
                defect: f64::NAN,
                tolerance,
            });
        }
        let count = certificate.component_count();
        let mut ledger = Ledger::new(limits.maximum_payload_bytes, before);
        ledger.charge(
            "kernel projector certificate",
            certificate.retained_payload_bytes()?,
        )?;
        let mut basis_len = 0usize;
        for component in 0..count {
            basis_len = certificate
                .component_nullity(component)
                .checked_mul(certificate.component_coordinates(component).len())
                .and_then(|size| basis_len.checked_add(size))
                .ok_or(IncidenceError::DimensionOverflow {
                    context: "kernel projector basis",
                })?;
        }
        let offsets_len = count
            .checked_add(1)
            .ok_or(IncidenceError::DimensionOverflow {
                context: "kernel projector offsets",
            })?;
        let mut basis_offsets = ledger.vec::<usize>(offsets_len, "kernel projector offsets")?;
        let mut basis = ledger.vec::<f64>(basis_len, "kernel projector basis")?;
        basis.resize(basis_len, 0.0);
        basis_offsets.push(0);
        let topology = owner.topology();
        let factor_offsets = topology.offsets();
        let mut report = IncidenceRangeProjectorReport::default();
        for component in 0..count {
            let span = certificate.component_coordinates(component);
            let width = span.len();
            let nullity = certificate.component_nullity(component);
            let start = basis_offsets[component];
            let block = &mut basis[start..start + nullity * width];
            match certificate.component_basis(component) {
                None => {
                    // The two factor shifts: (1, -1, 0) and (1, 0, -1).
                    for (local, &coordinate) in span.iter().enumerate() {
                        let factor = (1..3)
                            .filter(|&factor| coordinate >= factor_offsets[factor])
                            .count();
                        block[local] = [1.0, -1.0, 0.0][factor];
                        block[width + local] = [1.0, 0.0, -1.0][factor];
                    }
                }
                Some(integer) => {
                    for vector in 0..nullity {
                        for (slot, &value) in block[vector * width..(vector + 1) * width]
                            .iter_mut()
                            .zip(integer.vector(vector))
                        {
                            if value.unsigned_abs() > EXACT_FLOAT_INTEGER {
                                return Err(IncidenceError::KernelUncertified {
                                    component,
                                    reason: "integer kernel entry is not exactly representable",
                                });
                            }
                            *slot = value as f64;
                        }
                    }
                }
            }
            orthonormalize(component, block, width, tolerance)?;
            let orthogonality = orthogonality_defect(block, width);
            report.maximum_orthogonality_defect =
                report.maximum_orthogonality_defect.max(orthogonality);
            if orthogonality > tolerance {
                return Err(IncidenceError::KernelProjectorDefect {
                    component,
                    defect: orthogonality,
                    tolerance,
                });
            }
            basis_offsets.push(start + nullity * width);
        }
        let labels = owner.component_labels();
        for tuple in topology.tuples() {
            let row = [0, 1, 2].map(|factor| topology.global_index(factor, tuple[factor]));
            let component = labels[row[0]];
            let span = certificate.component_coordinates(component);
            let width = span.len();
            let locals = row.map(|coordinate| {
                span.binary_search(&coordinate)
                    .expect("tuple coordinates belong to their component")
            });
            let block = &basis[basis_offsets[component]..basis_offsets[component + 1]];
            for vector in block.chunks_exact(width) {
                let residual = (vector[locals[0]] + vector[locals[1]] + vector[locals[2]]).abs();
                report.maximum_incidence_residual = report.maximum_incidence_residual.max(residual);
                if residual.is_nan() || residual > tolerance {
                    return Err(IncidenceError::KernelProjectorDefect {
                        component,
                        defect: residual,
                        tolerance,
                    });
                }
            }
        }
        report.peak_requested_payload_bytes = ledger.peak;
        Ok(Self {
            certificate,
            basis_offsets,
            basis,
            report,
        })
    }

    /// The exact certificate this projector was built from.
    #[must_use]
    pub const fn certificate(&self) -> &IncidenceKernelCertificate<'a> {
        &self.certificate
    }

    /// Borrowed identity of the certified prepared owner.
    #[must_use]
    pub const fn binding(&self) -> PreparedTopologyBinding<'a> {
        self.certificate.binding
    }

    /// Coefficient dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.certificate.dimension()
    }

    /// Construction diagnostics.
    #[must_use]
    pub const fn report(&self) -> IncidenceRangeProjectorReport {
        self.report
    }

    /// Whether every component kernel is purely structural.
    #[must_use]
    pub fn is_structural(&self) -> bool {
        (0..self.certificate.component_count()).all(|component| {
            self.certificate.component_kind(component) == ComponentKernelKind::Structural
        })
    }

    /// Remove every certified kernel direction from `values` in place.
    ///
    /// Components and basis vectors are visited in a fixed order; each vector's
    /// dot product is accumulated sequentially. The dimension is checked before
    /// mutation. Nonfinite input propagates into the output unchanged in kind.
    pub fn project(&self, values: &mut [f64]) -> Result<(), IncidenceError> {
        if values.len() != self.dimension() {
            return Err(crate::error::dimension(
                "IncidenceRangeProjector::project values",
                self.dimension(),
                values.len(),
            ));
        }
        for component in 0..self.certificate.component_count() {
            let span = self.certificate.component_coordinates(component);
            let block =
                &self.basis[self.basis_offsets[component]..self.basis_offsets[component + 1]];
            for vector in block.chunks_exact(span.len()) {
                let mut dot = 0.0;
                for (&weight, &coordinate) in vector.iter().zip(span) {
                    dot = weight.mul_add(values[coordinate], dot);
                }
                for (&weight, &coordinate) in vector.iter().zip(span) {
                    values[coordinate] = (-dot).mul_add(weight, values[coordinate]);
                }
            }
        }
        Ok(())
    }

    /// Actual retained capacity, including the consumed certificate.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            self.certificate.retained_payload_bytes()?,
            array_bytes::<usize>(self.basis_offsets.capacity())?,
            array_bytes::<f64>(self.basis.capacity())?,
        ])
    }
}

/// Two passes of modified Gram--Schmidt over contiguous basis vectors.
fn orthonormalize(
    component: usize,
    block: &mut [f64],
    width: usize,
    tolerance: f64,
) -> Result<(), IncidenceError> {
    let count = block.len() / width;
    for current in 0..count {
        let (done, rest) = block.split_at_mut(current * width);
        let vector = &mut rest[..width];
        let original = norm(vector);
        for _ in 0..2 {
            for previous in done.chunks_exact(width) {
                let dot = dot(previous, vector);
                for (value, &weight) in vector.iter_mut().zip(previous) {
                    *value = (-dot).mul_add(weight, *value);
                }
            }
        }
        let remaining = norm(vector);
        // Exact independence was certified; a collapse here is floating cancellation.
        if !(remaining.is_finite() && remaining > tolerance * original) {
            return Err(IncidenceError::KernelProjectorDefect {
                component,
                defect: remaining / original,
                tolerance,
            });
        }
        for value in vector.iter_mut() {
            *value /= remaining;
        }
    }
    Ok(())
}

fn orthogonality_defect(block: &[f64], width: usize) -> f64 {
    let mut maximum: f64 = 0.0;
    for (row, left) in block.chunks_exact(width).enumerate() {
        for (column, right) in block.chunks_exact(width).enumerate() {
            let target = if row == column { 1.0 } else { 0.0 };
            maximum = maximum.max((dot(left, right) - target).abs());
        }
    }
    maximum
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .fold(0.0, |sum, (&a, &b)| a.mul_add(b, sum))
}

fn norm(values: &[f64]) -> f64 {
    dot(values, values).sqrt()
}
