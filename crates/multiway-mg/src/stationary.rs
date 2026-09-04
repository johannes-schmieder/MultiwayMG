//! Explicit energy-coordinate stationary error operators.

use nalgebra::{DMatrix, linalg::{SVD, SymmetricEigen}};

use crate::{DenseRangeDecomposition, MultiwayError, Preconditioner, SpectralAnalysisOptions};

/// Explicit stationary-error diagnostics in the Gramian energy norm.
#[derive(Debug, Clone, PartialEq)]
pub struct StationaryErrorReport {
    damping: f64,
    sweeps: usize,
    full_preconditioner_symmetry_defect: f64,
    energy_error_symmetry_defect: f64,
    range_leakage: f64,
    one_sweep_spectral_radius: f64,
    one_sweep_energy_operator_norm: f64,
    repeated_spectral_radius: f64,
    repeated_energy_operator_norm: f64,
    one_sweep_error_eigenvalues: Vec<f64>,
}

impl StationaryErrorReport {
    /// Fixed scalar damping applied to the preconditioner.
    #[must_use]
    pub const fn damping(&self) -> f64 {
        self.damping
    }

    /// Number of repeated stationary sweeps.
    #[must_use]
    pub const fn sweeps(&self) -> usize {
        self.sweeps
    }

    /// Relative Frobenius symmetry defect of the full materialized action.
    #[must_use]
    pub const fn full_preconditioner_symmetry_defect(&self) -> f64 {
        self.full_preconditioner_symmetry_defect
    }

    /// Relative symmetry defect of the one-step energy-coordinate error map.
    #[must_use]
    pub const fn energy_error_symmetry_defect(&self) -> f64 {
        self.energy_error_symmetry_defect
    }

    /// Relative norm of preconditioned range vectors leaking outside the range.
    #[must_use]
    pub const fn range_leakage(&self) -> f64 {
        self.range_leakage
    }

    /// Spectral radius of one energy-coordinate error step.
    #[must_use]
    pub const fn one_sweep_spectral_radius(&self) -> f64 {
        self.one_sweep_spectral_radius
    }

    /// Induced energy norm of one error step.
    #[must_use]
    pub const fn one_sweep_energy_operator_norm(&self) -> f64 {
        self.one_sweep_energy_operator_norm
    }

    /// Spectral radius after the requested number of sweeps.
    #[must_use]
    pub const fn repeated_spectral_radius(&self) -> f64 {
        self.repeated_spectral_radius
    }

    /// Induced energy norm after the requested number of sweeps.
    #[must_use]
    pub const fn repeated_energy_operator_norm(&self) -> f64 {
        self.repeated_energy_operator_norm
    }

    /// One-step energy-error eigenvalues in ascending order.
    #[must_use]
    pub fn one_sweep_error_eigenvalues(&self) -> &[f64] {
        &self.one_sweep_error_eigenvalues
    }

    /// Whether the energy error map satisfies the supplied symmetry tolerance.
    #[must_use]
    pub fn numerically_energy_self_adjoint(&self, tolerance: f64) -> bool {
        self.energy_error_symmetry_defect <= tolerance
    }
}

/// Materialize `E = I - damping * G^(1/2) M^-1 G^(1/2)` on the complete
/// numerical range and report one- and multi-sweep factors.
pub fn analyze_stationary_error<P: Preconditioner + ?Sized>(
    range: &DenseRangeDecomposition,
    preconditioner: &P,
    damping: f64,
    sweeps: usize,
    options: SpectralAnalysisOptions,
) -> Result<StationaryErrorReport, MultiwayError> {
    if preconditioner.dimension() != range.dimension() {
        return Err(crate::error::dimension(
            "analyze_stationary_error preconditioner",
            range.dimension(),
            preconditioner.dimension(),
        ));
    }
    if !damping.is_finite() || damping <= 0.0 {
        return Err(MultiwayError::InvalidOption {
            name: "stationary_damping",
            message: format!("must be finite and positive, got {damping}"),
        });
    }
    if sweeps == 0 {
        return Err(MultiwayError::InvalidOption {
            name: "stationary_sweeps",
            message: "must be positive".to_owned(),
        });
    }
    let action = materialize(preconditioner)?;
    let full_preconditioner_symmetry_defect = symmetry_defect(&action);
    let basis = range.basis();
    let applied_range = &action * basis;
    let quotient = basis.transpose() * &applied_range;
    let reconstructed = basis * &quotient;
    let range_leakage = (&applied_range - reconstructed).norm()
        / applied_range.norm().max(f64::MIN_POSITIVE);
    let quotient_symmetric = (&quotient + quotient.transpose()) * 0.5;
    let rank = range.rank();
    let mut energy_preconditioner = DMatrix::zeros(rank, rank);
    for row in 0..rank {
        let left = range.positive_eigenvalues()[row].sqrt();
        for column in 0..rank {
            energy_preconditioner[(row, column)] = left
                * quotient_symmetric[(row, column)]
                * range.positive_eigenvalues()[column].sqrt();
        }
    }
    let mut one_step = DMatrix::identity(rank, rank) - energy_preconditioner * damping;
    ensure_finite("one-step stationary error", one_step.as_slice())?;
    let energy_error_symmetry_defect = symmetry_defect(&one_step);
    let symmetric_one_step = (&one_step + one_step.transpose()) * 0.5;
    let decomposition = SymmetricEigen::new(symmetric_one_step);
    let mut one_sweep_error_eigenvalues: Vec<f64> =
        decomposition.eigenvalues.iter().copied().collect();
    one_sweep_error_eigenvalues.sort_by(f64::total_cmp);
    let one_sweep_spectral_radius = one_sweep_error_eigenvalues
        .iter()
        .copied()
        .map(f64::abs)
        .fold(0.0, f64::max);
    let one_sweep_energy_operator_norm = largest_singular_value(&one_step);
    let repeated = matrix_power(&one_step, sweeps);
    let repeated_symmetric = (&repeated + repeated.transpose()) * 0.5;
    let repeated_eigenvalues = SymmetricEigen::new(repeated_symmetric);
    let repeated_spectral_radius = repeated_eigenvalues
        .eigenvalues
        .iter()
        .copied()
        .map(f64::abs)
        .fold(0.0, f64::max);
    let repeated_energy_operator_norm = largest_singular_value(&repeated);
    let _ = options;
    one_step.fill(0.0);
    Ok(StationaryErrorReport {
        damping,
        sweeps,
        full_preconditioner_symmetry_defect,
        energy_error_symmetry_defect,
        range_leakage,
        one_sweep_spectral_radius,
        one_sweep_energy_operator_norm,
        repeated_spectral_radius,
        repeated_energy_operator_norm,
        one_sweep_error_eigenvalues,
    })
}

fn materialize<P: Preconditioner + ?Sized>(
    preconditioner: &P,
) -> Result<DMatrix<f64>, MultiwayError> {
    let dimension = preconditioner.dimension();
    let mut matrix = DMatrix::zeros(dimension, dimension);
    let mut basis = vec![0.0; dimension];
    let mut output = vec![0.0; dimension];
    for column in 0..dimension {
        basis.fill(0.0);
        output.fill(0.0);
        basis[column] = 1.0;
        preconditioner.apply(&basis, &mut output)?;
        ensure_finite("stationary materialization", &output)?;
        for row in 0..dimension {
            matrix[(row, column)] = output[row];
        }
    }
    Ok(matrix)
}

fn matrix_power(matrix: &DMatrix<f64>, exponent: usize) -> DMatrix<f64> {
    let mut result = DMatrix::identity(matrix.nrows(), matrix.ncols());
    let mut base = matrix.clone();
    let mut remaining = exponent;
    while remaining > 0 {
        if remaining & 1 == 1 {
            result *= &base;
        }
        remaining >>= 1;
        if remaining > 0 {
            base = &base * &base;
        }
    }
    result
}

fn largest_singular_value(matrix: &DMatrix<f64>) -> f64 {
    SVD::new(matrix.clone(), false, false)
        .singular_values
        .iter()
        .copied()
        .fold(0.0, f64::max)
}

fn symmetry_defect(matrix: &DMatrix<f64>) -> f64 {
    (matrix - matrix.transpose()).norm() / matrix.norm().max(f64::MIN_POSITIVE)
}

fn ensure_finite(context: &'static str, values: &[f64]) -> Result<(), MultiwayError> {
    if let Some((index, value)) = values
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(MultiwayError::SpectralAnalysis {
            message: format!("{context} entry {index} is non-finite: {value}"),
        });
    }
    Ok(())
}
