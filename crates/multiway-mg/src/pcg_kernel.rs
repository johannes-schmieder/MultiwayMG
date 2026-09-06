//! One untraced projected-PCG recurrence with caller-owned vector storage.
use crate::certificate::{bytes, vector};
use crate::{MultiwayError, PcgOptions, PcgStopReason};

pub(crate) trait PcgActions {
    fn project(&mut self, values: &mut [f64]) -> Result<f64, MultiwayError>;
    fn precondition(&mut self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError>;
    fn gramian(&mut self, x: &[f64], out: &mut [f64]) -> Result<(), MultiwayError>;
    fn residual(&mut self, rhs: &[f64], x: &[f64], out: &mut [f64]) -> Result<(), MultiwayError>;
}
#[derive(Debug)]
pub(crate) struct PcgStorage {
    pub(crate) projected_rhs: Vec<f64>,
    pub(crate) solution: Vec<f64>,
    pub(crate) residual: Vec<f64>,
    pub(crate) preconditioned: Vec<f64>,
    pub(crate) direction: Vec<f64>,
    pub(crate) applied: Vec<f64>,
}
impl PcgStorage {
    pub(crate) fn try_new(dimension: usize) -> Result<Self, MultiwayError> {
        Self::build_with(dimension, &mut |_| Ok(()))
    }
    fn build_with<F>(dimension: usize, before: &mut F) -> Result<Self, MultiwayError>
    where
        F: FnMut(&'static str) -> Result<(), MultiwayError>,
    {
        Self::required_payload_bytes(dimension)?;
        let mut make = || {
            if dimension > 0 {
                before("PCG vector storage")?;
            }
            vector(dimension)
        };
        Ok(Self {
            projected_rhs: make()?,
            solution: make()?,
            residual: make()?,
            preconditioned: make()?,
            direction: make()?,
            applied: make()?,
        })
    }
    pub(crate) fn required_payload_bytes(dimension: usize) -> Result<usize, MultiwayError> {
        bytes(dimension)?
            .checked_mul(6)
            .ok_or(MultiwayError::WorkspaceSizeOverflow {
                context: "PCG vector storage",
            })
    }
    pub(crate) fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        [
            &self.projected_rhs,
            &self.solution,
            &self.residual,
            &self.preconditioned,
            &self.direction,
            &self.applied,
        ]
        .iter()
        .try_fold(0usize, |total, v| {
            total
                .checked_add(bytes(v.capacity())?)
                .ok_or(MultiwayError::WorkspaceSizeOverflow {
                    context: "PCG vector storage",
                })
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PcgDiagnostics {
    pub(crate) iterations: usize,
    pub(crate) converged: bool,
    pub(crate) residual_norm: f64,
    pub(crate) relative_residual: f64,
    pub(crate) rhs_projection_norm: f64,
    pub(crate) stop_reason: PcgStopReason,
}
// Callers validate options, dimensions and finite RHS before invoking this core.
pub(crate) fn solve<A: PcgActions>(
    actions: &mut A,
    rhs: &[f64],
    options: PcgOptions,
    storage: &mut PcgStorage,
) -> Result<PcgDiagnostics, MultiwayError> {
    #[cfg(feature = "profiling")]
    let _profile_span =
        multiway_incidence::profiling::span(multiway_incidence::profiling::Phase::PcgRecurrence);

    let PcgStorage {
        projected_rhs,
        solution,
        residual,
        preconditioned,
        direction,
        applied,
    } = storage;
    projected_rhs.copy_from_slice(rhs);
    solution.fill(0.0);
    let rhs_projection_norm = actions.project(projected_rhs)?;
    ensure_finite("projected PCG right-hand side", projected_rhs)?;
    ensure_finite("PCG projection norm", &[rhs_projection_norm])?;
    let rhs_norm = checked_norm(projected_rhs)?;
    if rhs_norm == 0.0 {
        return Ok(PcgDiagnostics {
            iterations: 0,
            converged: true,
            residual_norm: 0.0,
            relative_residual: 0.0,
            rhs_projection_norm,
            stop_reason: PcgStopReason::ZeroRightHandSide,
        });
    }
    let tolerance = options
        .absolute_tolerance
        .max(options.relative_tolerance * rhs_norm);

    ensure_finite("PCG tolerance", &[tolerance])?;
    residual.copy_from_slice(projected_rhs);
    preconditioned.fill(0.0);
    actions.precondition(residual, preconditioned)?;
    actions.project(preconditioned)?;
    ensure_finite("initial preconditioned residual", preconditioned)?;
    let mut rho = dot(residual, preconditioned);
    if !rho.is_finite() || rho <= 0.0 {
        return Err(MultiwayError::PcgBreakdown {
            iteration: 0,
            message: format!("initial preconditioned metric is {rho}"),
        });
    }
    direction.copy_from_slice(preconditioned);
    applied.fill(0.0);

    for iteration in 1..=options.max_iterations {
        actions.gramian(direction, applied)?;
        let curvature = dot(direction, applied);
        if !curvature.is_finite() || curvature <= 0.0 {
            return Err(MultiwayError::PcgBreakdown {
                iteration: iteration - 1,
                message: format!("search-direction curvature is {curvature}"),
            });
        }
        let alpha = rho / curvature;
        if !alpha.is_finite() {
            return Err(MultiwayError::PcgBreakdown {
                iteration: iteration - 1,
                message: format!("step length is {alpha}"),
            });
        }
        axpy(alpha, direction, solution);
        axpy(-alpha, applied, residual);
        actions.project(residual)?;

        if iteration % options.residual_recompute_interval == 0
            || checked_norm(residual)? <= tolerance
        {
            actions.residual(projected_rhs, solution, residual)?;
            actions.project(residual)?;
            let residual_norm = checked_norm(residual)?;
            if residual_norm <= tolerance {
                actions.project(solution)?;
                ensure_finite("PCG solution", solution)?;
                return Ok(PcgDiagnostics {
                    iterations: iteration,
                    converged: true,
                    residual_norm,
                    relative_residual: residual_norm / rhs_norm,
                    rhs_projection_norm,
                    stop_reason: PcgStopReason::Converged,
                });
            }
        }

        actions.precondition(residual, preconditioned)?;
        actions.project(preconditioned)?;
        ensure_finite("preconditioned residual", preconditioned)?;
        let new_rho = dot(residual, preconditioned);
        if !new_rho.is_finite() || new_rho <= 0.0 {
            return Err(MultiwayError::PcgBreakdown {
                iteration,
                message: format!("preconditioned metric is {new_rho}"),
            });
        }
        let beta = new_rho / rho;
        ensure_finite("PCG recurrence beta", &[beta])?;
        for (search, &z) in direction.iter_mut().zip(preconditioned.iter()) {
            *search = beta.mul_add(*search, z);
        }
        actions.project(direction)?;
        rho = new_rho;
    }

    actions.residual(projected_rhs, solution, residual)?;
    actions.project(residual)?;
    actions.project(solution)?;
    let residual_norm = checked_norm(residual)?;
    ensure_finite("PCG solution", solution)?;
    Ok(PcgDiagnostics {
        iterations: options.max_iterations,
        converged: false,
        residual_norm,
        relative_residual: residual_norm / rhs_norm,
        rhs_projection_norm,
        stop_reason: PcgStopReason::MaximumIterations,
    })
}

fn axpy(alpha: f64, x: &[f64], y: &mut [f64]) {
    for (destination, &source) in y.iter_mut().zip(x) {
        *destination = alpha.mul_add(source, *destination);
    }
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (&a, &b) in left.iter().zip(right) {
        let value = a * b;
        let updated = sum + value;
        if sum.abs() >= value.abs() {
            correction += (sum - updated) + value;
        } else {
            correction += (value - updated) + sum;
        }
        sum = updated;
    }
    sum + correction
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

pub(crate) fn ensure_finite(context: &'static str, values: &[f64]) -> Result<(), MultiwayError> {
    if let Some((index, value)) = values
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(MultiwayError::PcgBreakdown {
            iteration: 0,
            message: format!("{context} entry {index} is non-finite: {value}"),
        });
    }
    Ok(())
}

fn checked_norm(values: &[f64]) -> Result<f64, MultiwayError> {
    ensure_finite("PCG norm input", values)?;
    let value = norm(values);
    ensure_finite("PCG norm", &[value])?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Diagonal {
        fail: bool,
    }
    impl PcgActions for Diagonal {
        fn project(&mut self, _: &mut [f64]) -> Result<f64, MultiwayError> {
            Ok(0.0)
        }
        fn precondition(&mut self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
            if self.fail {
                return Err(MultiwayError::WorkspaceNotPrepared {
                    context: "injected PCG action",
                });
            }
            out.copy_from_slice(rhs);
            Ok(())
        }
        fn gramian(&mut self, x: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
            out[0] = x[0];
            out[1] = 4.0 * x[1];
            Ok(())
        }
        fn residual(
            &mut self,
            rhs: &[f64],
            x: &[f64],
            out: &mut [f64],
        ) -> Result<(), MultiwayError> {
            self.gramian(x, out)?;
            for (o, r) in out.iter_mut().zip(rhs) {
                *o = *r - *o;
            }
            Ok(())
        }
    }
    #[test]
    fn six_reservations_fail_cleanly_and_poisoned_or_failed_actions_recover() {
        let mut old = PcgStorage::try_new(2).unwrap();
        for fail_at in 0..6 {
            let mut reached = 0;
            assert!(
                PcgStorage::build_with(2, &mut |context| {
                    reached += 1;
                    if reached == fail_at + 1 {
                        Err(MultiwayError::WorkspaceNotPrepared { context })
                    } else {
                        Ok(())
                    }
                })
                .is_err()
            );
            assert_eq!(reached, fail_at + 1);
            for v in [
                &mut old.projected_rhs,
                &mut old.solution,
                &mut old.residual,
                &mut old.preconditioned,
                &mut old.direction,
                &mut old.applied,
            ] {
                v.fill(f64::NAN);
            }
            assert!(
                solve(
                    &mut Diagonal { fail: true },
                    &[1.0, 2.0],
                    PcgOptions::default(),
                    &mut old
                )
                .is_err()
            );
            let result = solve(
                &mut Diagonal { fail: false },
                &[1.0, 2.0],
                PcgOptions::default(),
                &mut old,
            )
            .unwrap();
            assert!(result.converged);
            assert!((old.solution[0] - 1.0).abs() < 1e-12);
            assert!((old.solution[1] - 0.5).abs() < 1e-12);
        }
        assert!(PcgStorage::build_with(usize::MAX, &mut |_| panic!("overflow first")).is_err());
    }
}
