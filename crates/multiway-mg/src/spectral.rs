//! Dense quotient-space diagnostics for small research problems.
//!
//! The routines in this module materialize the submitted Gramian and one fixed
//! preconditioner action. They are intended for oracle-hierarchy experiments,
//! regression tests, and method development—not for production-scale solves.

use nalgebra::{DMatrix, linalg::SymmetricEigen};

use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

/// Configuration for dense quotient-space spectral analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpectralAnalysisOptions {
    /// Relative threshold used to distinguish positive Gramian eigenvalues from
    /// numerical null directions.
    pub relative_rank_tolerance: f64,
    /// Relative tolerance used when classifying symmetry and range leakage.
    pub relative_structure_tolerance: f64,
    /// Largest coefficient dimension that may be materialized.
    pub maximum_dimension: usize,
}

impl Default for SpectralAnalysisOptions {
    fn default() -> Self {
        Self {
            relative_rank_tolerance: 1.0e-11,
            relative_structure_tolerance: 1.0e-10,
            maximum_dimension: 512,
        }
    }
}

impl SpectralAnalysisOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if !self.relative_rank_tolerance.is_finite() || self.relative_rank_tolerance <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "relative_rank_tolerance",
                message: format!(
                    "must be finite and positive, got {}",
                    self.relative_rank_tolerance
                ),
            });
        }
        if !self.relative_structure_tolerance.is_finite()
            || self.relative_structure_tolerance <= 0.0
        {
            return Err(MultiwayError::InvalidOption {
                name: "relative_structure_tolerance",
                message: format!(
                    "must be finite and positive, got {}",
                    self.relative_structure_tolerance
                ),
            });
        }
        if self.maximum_dimension == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "maximum_dimension",
                message: "must be positive".to_owned(),
            });
        }
        Ok(self)
    }
}

/// Complete numerical range decomposition of a small submitted Gramian.
///
/// Columns of [`Self::basis`] are Euclidean-orthonormal eigenvectors associated
/// with positive eigenvalues. This discovers extra null directions beyond the
/// known factor-shift kernel.
#[derive(Debug, Clone)]
pub struct DenseRangeDecomposition {
    dimension: usize,
    basis: DMatrix<f64>,
    positive_eigenvalues: Vec<f64>,
    threshold: f64,
    minimum_positive_eigenvalue: f64,
    maximum_eigenvalue: f64,
}

impl DenseRangeDecomposition {
    /// Materialize and decompose the Gramian of a small problem.
    pub fn from_problem(
        problem: &ThreeWayProblem,
        options: SpectralAnalysisOptions,
    ) -> Result<Self, MultiwayError> {
        let options = options.validate()?;
        let dimension = problem.dimension();
        if dimension > options.maximum_dimension {
            return Err(MultiwayError::SpectralAnalysis {
                message: format!(
                    "dimension {dimension} exceeds dense-analysis limit {}",
                    options.maximum_dimension
                ),
            });
        }

        let gramian = dense_gramian(problem);
        ensure_finite_matrix("Gramian", &gramian)?;
        let decomposition = SymmetricEigen::new(gramian);
        let spectral_scale = decomposition
            .eigenvalues
            .iter()
            .copied()
            .map(f64::abs)
            .fold(0.0, f64::max);
        if !spectral_scale.is_finite() || spectral_scale <= 0.0 {
            return Err(MultiwayError::SpectralAnalysis {
                message: format!("Gramian spectral scale is {spectral_scale}"),
            });
        }
        let threshold = options.relative_rank_tolerance * spectral_scale;
        let mut positive_modes = Vec::new();
        for (index, &eigenvalue) in decomposition.eigenvalues.iter().enumerate() {
            if eigenvalue < -threshold {
                return Err(MultiwayError::NegativeEigenvalue {
                    value: eigenvalue,
                    tolerance: threshold,
                });
            }
            if eigenvalue > threshold {
                positive_modes.push((eigenvalue, index));
            }
        }
        if positive_modes.is_empty() {
            return Err(MultiwayError::SpectralAnalysis {
                message: "Gramian has no positive numerical range".to_owned(),
            });
        }
        positive_modes.sort_by(|left, right| left.0.total_cmp(&right.0));

        let rank = positive_modes.len();
        let mut basis = DMatrix::zeros(dimension, rank);
        let mut positive_eigenvalues = Vec::with_capacity(rank);
        for (column, &(eigenvalue, source_column)) in positive_modes.iter().enumerate() {
            positive_eigenvalues.push(eigenvalue);
            for row in 0..dimension {
                basis[(row, column)] = decomposition.eigenvectors[(row, source_column)];
            }
        }
        let minimum_positive_eigenvalue = positive_eigenvalues[0];
        let maximum_eigenvalue = positive_eigenvalues[rank - 1];
        Ok(Self {
            dimension,
            basis,
            positive_eigenvalues,
            threshold,
            minimum_positive_eigenvalue,
            maximum_eigenvalue,
        })
    }

    /// Original coefficient dimension.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// Complete numerical rank of the submitted Gramian.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.positive_eigenvalues.len()
    }

    /// Complete numerical nullity of the submitted Gramian.
    #[must_use]
    pub fn nullity(&self) -> usize {
        self.dimension - self.rank()
    }

    /// Euclidean-orthonormal basis for the complete numerical range.
    #[must_use]
    pub const fn basis(&self) -> &DMatrix<f64> {
        &self.basis
    }

    /// Positive Gramian eigenvalues in ascending order.
    #[must_use]
    pub fn positive_eigenvalues(&self) -> &[f64] {
        &self.positive_eigenvalues
    }

    /// Absolute rank threshold.
    #[must_use]
    pub const fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Smallest retained positive Gramian eigenvalue.
    #[must_use]
    pub const fn minimum_positive_eigenvalue(&self) -> f64 {
        self.minimum_positive_eigenvalue
    }

    /// Largest Gramian eigenvalue.
    #[must_use]
    pub const fn maximum_eigenvalue(&self) -> f64 {
        self.maximum_eigenvalue
    }

    /// Condition number of the Gramian on its complete numerical range.
    #[must_use]
    pub fn condition_number(&self) -> f64 {
        self.maximum_eigenvalue / self.minimum_positive_eigenvalue
    }

    /// Analyze one fixed preconditioner against this range decomposition.
    pub fn analyze<P: Preconditioner + ?Sized>(
        &self,
        preconditioner: &P,
        options: SpectralAnalysisOptions,
    ) -> Result<SpectralAnalysisReport, MultiwayError> {
        let options = options.validate()?;
        if preconditioner.dimension() != self.dimension {
            return Err(crate::error::dimension(
                "DenseRangeDecomposition::analyze preconditioner",
                self.dimension,
                preconditioner.dimension(),
            ));
        }

        let preconditioner_matrix = materialize_preconditioner(preconditioner)?;
        let preconditioner_symmetry_defect = symmetry_defect(&preconditioner_matrix);
        let applied_range = multiply(&preconditioner_matrix, &self.basis);
        let quotient = left_transpose_multiply(&self.basis, &applied_range);
        let quotient_symmetry_defect = symmetry_defect(&quotient);
        let quotient_symmetric = symmetric_part(&quotient);

        let reconstructed_range = multiply(&self.basis, &quotient);
        let mut leaked = applied_range.clone();
        subtract_assign(&mut leaked, &reconstructed_range);
        let range_leakage =
            frobenius_norm(&leaked) / frobenius_norm(&applied_range).max(f64::MIN_POSITIVE);

        let preconditioner_energy_decomposition = SymmetricEigen::new(quotient_symmetric.clone());
        let mut preconditioner_energy_eigenvalues: Vec<f64> = preconditioner_energy_decomposition
            .eigenvalues
            .iter()
            .copied()
            .collect();
        preconditioner_energy_eigenvalues.sort_by(f64::total_cmp);
        ensure_finite_slice(
            "quotient preconditioner energy eigenvalues",
            &preconditioner_energy_eigenvalues,
        )?;
        let minimum_preconditioner_energy = preconditioner_energy_eigenvalues[0];
        let maximum_preconditioner_energy =
            preconditioner_energy_eigenvalues[preconditioner_energy_eigenvalues.len() - 1];
        let energy_scale = maximum_preconditioner_energy
            .abs()
            .max(minimum_preconditioner_energy.abs());
        let energy_tolerance = options.relative_rank_tolerance * energy_scale;
        let negative_preconditioner_directions = preconditioner_energy_eigenvalues
            .iter()
            .filter(|&&value| value < -energy_tolerance)
            .count();
        let near_zero_preconditioner_directions = preconditioner_energy_eigenvalues
            .iter()
            .filter(|&&value| value.abs() <= energy_tolerance)
            .count();

        let rank = self.rank();
        let mut energy_preconditioned = DMatrix::zeros(rank, rank);
        for row in 0..rank {
            let left_scale = self.positive_eigenvalues[row].sqrt();
            for column in 0..rank {
                energy_preconditioned[(row, column)] = left_scale
                    * quotient_symmetric[(row, column)]
                    * self.positive_eigenvalues[column].sqrt();
            }
        }
        ensure_finite_matrix("energy-preconditioned Gramian", &energy_preconditioned)?;
        let preconditioned_decomposition = SymmetricEigen::new(energy_preconditioned);
        let mut preconditioned_eigenvalues: Vec<f64> = preconditioned_decomposition
            .eigenvalues
            .iter()
            .copied()
            .collect();
        preconditioned_eigenvalues.sort_by(f64::total_cmp);
        ensure_finite_slice("preconditioned eigenvalues", &preconditioned_eigenvalues)?;
        let minimum_preconditioned_eigenvalue = preconditioned_eigenvalues[0];
        let maximum_preconditioned_eigenvalue =
            preconditioned_eigenvalues[preconditioned_eigenvalues.len() - 1];
        let preconditioned_condition_number = if minimum_preconditioned_eigenvalue > 0.0 {
            maximum_preconditioned_eigenvalue / minimum_preconditioned_eigenvalue
        } else {
            f64::INFINITY
        };
        let unit_step_energy_spectral_radius = preconditioned_eigenvalues
            .iter()
            .map(|&value| (1.0 - value).abs())
            .fold(0.0, f64::max);
        let (optimal_richardson_damping, optimal_energy_spectral_radius) =
            if minimum_preconditioned_eigenvalue > 0.0 && maximum_preconditioned_eigenvalue > 0.0 {
                let damping =
                    2.0 / (minimum_preconditioned_eigenvalue + maximum_preconditioned_eigenvalue);
                let radius = preconditioned_eigenvalues
                    .iter()
                    .map(|&value| (1.0 - damping * value).abs())
                    .fold(0.0, f64::max);
                (damping, radius)
            } else {
                (f64::NAN, f64::INFINITY)
            };

        Ok(SpectralAnalysisReport {
            dimension: self.dimension,
            numerical_rank: self.rank(),
            numerical_nullity: self.nullity(),
            gramian_threshold: self.threshold,
            minimum_positive_gramian_eigenvalue: self.minimum_positive_eigenvalue,
            maximum_gramian_eigenvalue: self.maximum_eigenvalue,
            gramian_condition_number: self.condition_number(),
            preconditioner_symmetry_defect,
            quotient_symmetry_defect,
            range_leakage,
            minimum_preconditioner_energy,
            maximum_preconditioner_energy,
            negative_preconditioner_directions,
            near_zero_preconditioner_directions,
            minimum_preconditioned_eigenvalue,
            maximum_preconditioned_eigenvalue,
            preconditioned_condition_number,
            unit_step_energy_spectral_radius,
            optimal_richardson_damping,
            optimal_energy_spectral_radius,
            numerically_symmetric: preconditioner_symmetry_defect
                <= options.relative_structure_tolerance
                && quotient_symmetry_defect <= options.relative_structure_tolerance,
            preserves_range: range_leakage <= options.relative_structure_tolerance,
            positive_definite_on_range: minimum_preconditioner_energy > energy_tolerance,
            preconditioned_eigenvalues,
        })
    }
}

/// Dense spectral diagnostics for one fixed preconditioner.
#[derive(Debug, Clone, PartialEq)]
pub struct SpectralAnalysisReport {
    dimension: usize,
    numerical_rank: usize,
    numerical_nullity: usize,
    gramian_threshold: f64,
    minimum_positive_gramian_eigenvalue: f64,
    maximum_gramian_eigenvalue: f64,
    gramian_condition_number: f64,
    preconditioner_symmetry_defect: f64,
    quotient_symmetry_defect: f64,
    range_leakage: f64,
    minimum_preconditioner_energy: f64,
    maximum_preconditioner_energy: f64,
    negative_preconditioner_directions: usize,
    near_zero_preconditioner_directions: usize,
    minimum_preconditioned_eigenvalue: f64,
    maximum_preconditioned_eigenvalue: f64,
    preconditioned_condition_number: f64,
    unit_step_energy_spectral_radius: f64,
    optimal_richardson_damping: f64,
    optimal_energy_spectral_radius: f64,
    numerically_symmetric: bool,
    preserves_range: bool,
    positive_definite_on_range: bool,
    preconditioned_eigenvalues: Vec<f64>,
}

impl SpectralAnalysisReport {
    /// Original coefficient dimension.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// Complete numerical rank of the Gramian.
    #[must_use]
    pub const fn numerical_rank(&self) -> usize {
        self.numerical_rank
    }

    /// Complete numerical nullity of the Gramian.
    #[must_use]
    pub const fn numerical_nullity(&self) -> usize {
        self.numerical_nullity
    }

    /// Absolute Gramian rank threshold.
    #[must_use]
    pub const fn gramian_threshold(&self) -> f64 {
        self.gramian_threshold
    }

    /// Smallest retained positive Gramian eigenvalue.
    #[must_use]
    pub const fn minimum_positive_gramian_eigenvalue(&self) -> f64 {
        self.minimum_positive_gramian_eigenvalue
    }

    /// Largest Gramian eigenvalue.
    #[must_use]
    pub const fn maximum_gramian_eigenvalue(&self) -> f64 {
        self.maximum_gramian_eigenvalue
    }

    /// Gramian condition number on the complete numerical range.
    #[must_use]
    pub const fn gramian_condition_number(&self) -> f64 {
        self.gramian_condition_number
    }

    /// Relative Frobenius symmetry defect of the full materialized preconditioner.
    #[must_use]
    pub const fn preconditioner_symmetry_defect(&self) -> f64 {
        self.preconditioner_symmetry_defect
    }

    /// Relative Frobenius symmetry defect after range restriction.
    #[must_use]
    pub const fn quotient_symmetry_defect(&self) -> f64 {
        self.quotient_symmetry_defect
    }

    /// Relative norm of the preconditioned range component leaking into the null space.
    #[must_use]
    pub const fn range_leakage(&self) -> f64 {
        self.range_leakage
    }

    /// Minimum eigenvalue of the symmetric preconditioner action on the range basis.
    #[must_use]
    pub const fn minimum_preconditioner_energy(&self) -> f64 {
        self.minimum_preconditioner_energy
    }

    /// Maximum eigenvalue of the symmetric preconditioner action on the range basis.
    #[must_use]
    pub const fn maximum_preconditioner_energy(&self) -> f64 {
        self.maximum_preconditioner_energy
    }

    /// Number of materially negative preconditioner-energy directions.
    #[must_use]
    pub const fn negative_preconditioner_directions(&self) -> usize {
        self.negative_preconditioner_directions
    }

    /// Number of numerically zero preconditioner-energy directions.
    #[must_use]
    pub const fn near_zero_preconditioner_directions(&self) -> usize {
        self.near_zero_preconditioner_directions
    }

    /// Smallest eigenvalue of `G^(1/2) M^(-1) G^(1/2)` on the numerical range.
    #[must_use]
    pub const fn minimum_preconditioned_eigenvalue(&self) -> f64 {
        self.minimum_preconditioned_eigenvalue
    }

    /// Largest eigenvalue of `G^(1/2) M^(-1) G^(1/2)` on the numerical range.
    #[must_use]
    pub const fn maximum_preconditioned_eigenvalue(&self) -> f64 {
        self.maximum_preconditioned_eigenvalue
    }

    /// Spectral condition number of the energy-preconditioned operator.
    #[must_use]
    pub const fn preconditioned_condition_number(&self) -> f64 {
        self.preconditioned_condition_number
    }

    /// Energy-norm spectral radius of one undamped correction `I - M^(-1)G`.
    #[must_use]
    pub const fn unit_step_energy_spectral_radius(&self) -> f64 {
        self.unit_step_energy_spectral_radius
    }

    /// Optimal scalar Richardson damping for the reported positive eigenvalue interval.
    #[must_use]
    pub const fn optimal_richardson_damping(&self) -> f64 {
        self.optimal_richardson_damping
    }

    /// Energy-norm spectral radius after optimal scalar Richardson damping.
    #[must_use]
    pub const fn optimal_energy_spectral_radius(&self) -> f64 {
        self.optimal_energy_spectral_radius
    }

    /// Whether full and quotient symmetry defects satisfy the configured tolerance.
    #[must_use]
    pub const fn numerically_symmetric(&self) -> bool {
        self.numerically_symmetric
    }

    /// Whether the preconditioner maps the numerical range back into itself.
    #[must_use]
    pub const fn preserves_range(&self) -> bool {
        self.preserves_range
    }

    /// Whether the symmetric preconditioner action is positive definite on the range.
    #[must_use]
    pub const fn positive_definite_on_range(&self) -> bool {
        self.positive_definite_on_range
    }

    /// Preconditioned eigenvalues in ascending order.
    #[must_use]
    pub fn preconditioned_eigenvalues(&self) -> &[f64] {
        &self.preconditioned_eigenvalues
    }
}

/// Convenience entry point that constructs the complete numerical range first.
pub fn analyze_preconditioner<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    preconditioner: &P,
    options: SpectralAnalysisOptions,
) -> Result<SpectralAnalysisReport, MultiwayError> {
    DenseRangeDecomposition::from_problem(problem, options)?.analyze(preconditioner, options)
}

fn dense_gramian(problem: &ThreeWayProblem) -> DMatrix<f64> {
    let dimension = problem.dimension();
    let flat: Vec<f64> = problem.dense_gramian().into_iter().flatten().collect();
    DMatrix::from_row_slice(dimension, dimension, &flat)
}

fn materialize_preconditioner<P: Preconditioner + ?Sized>(
    preconditioner: &P,
) -> Result<DMatrix<f64>, MultiwayError> {
    let dimension = preconditioner.dimension();
    let mut matrix = DMatrix::zeros(dimension, dimension);
    let mut basis = vec![0.0; dimension];
    let mut output = vec![0.0; dimension];
    for column in 0..dimension {
        basis.fill(0.0);
        basis[column] = 1.0;
        output.fill(0.0);
        preconditioner.apply(&basis, &mut output)?;
        ensure_finite_slice("materialized preconditioner column", &output)?;
        for row in 0..dimension {
            matrix[(row, column)] = output[row];
        }
    }
    Ok(matrix)
}

fn multiply(left: &DMatrix<f64>, right: &DMatrix<f64>) -> DMatrix<f64> {
    debug_assert_eq!(left.ncols(), right.nrows());
    let mut product = DMatrix::zeros(left.nrows(), right.ncols());
    for row in 0..left.nrows() {
        for column in 0..right.ncols() {
            let mut value = 0.0;
            for inner in 0..left.ncols() {
                value = left[(row, inner)].mul_add(right[(inner, column)], value);
            }
            product[(row, column)] = value;
        }
    }
    product
}

fn left_transpose_multiply(left: &DMatrix<f64>, right: &DMatrix<f64>) -> DMatrix<f64> {
    debug_assert_eq!(left.nrows(), right.nrows());
    let mut product = DMatrix::zeros(left.ncols(), right.ncols());
    for row in 0..left.ncols() {
        for column in 0..right.ncols() {
            let mut value = 0.0;
            for inner in 0..left.nrows() {
                value = left[(inner, row)].mul_add(right[(inner, column)], value);
            }
            product[(row, column)] = value;
        }
    }
    product
}

fn symmetric_part(matrix: &DMatrix<f64>) -> DMatrix<f64> {
    DMatrix::from_fn(matrix.nrows(), matrix.ncols(), |row, column| {
        0.5 * (matrix[(row, column)] + matrix[(column, row)])
    })
}

fn symmetry_defect(matrix: &DMatrix<f64>) -> f64 {
    let mut squared_defect = 0.0;
    let mut squared_norm = 0.0;
    for row in 0..matrix.nrows() {
        for column in 0..matrix.ncols() {
            let value = matrix[(row, column)];
            let defect = value - matrix[(column, row)];
            squared_norm = value.mul_add(value, squared_norm);
            squared_defect = defect.mul_add(defect, squared_defect);
        }
    }
    squared_defect.sqrt() / squared_norm.sqrt().max(f64::MIN_POSITIVE)
}

fn frobenius_norm(matrix: &DMatrix<f64>) -> f64 {
    matrix.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn subtract_assign(left: &mut DMatrix<f64>, right: &DMatrix<f64>) {
    debug_assert_eq!(left.shape(), right.shape());
    for (left_value, right_value) in left.iter_mut().zip(right.iter()) {
        *left_value -= right_value;
    }
}

fn ensure_finite_matrix(context: &'static str, matrix: &DMatrix<f64>) -> Result<(), MultiwayError> {
    ensure_finite_slice(context, matrix.as_slice())
}

fn ensure_finite_slice(context: &'static str, values: &[f64]) -> Result<(), MultiwayError> {
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
