//! Exact dense pairwise Schwarz reference preconditioner.

use std::collections::BTreeMap;

use nalgebra::{DMatrix, linalg::SymmetricEigen};

use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

/// Options for exact dense pairwise reference solves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DensePairOptions {
    /// Relative eigenvalue threshold for each pair pseudoinverse.
    pub relative_tolerance: f64,
    /// Symmetric restriction/prolongation partition weight.
    pub partition_weight: f64,
}

impl Default for DensePairOptions {
    fn default() -> Self {
        Self {
            relative_tolerance: 1.0e-12,
            partition_weight: std::f64::consts::FRAC_1_SQRT_2,
        }
    }
}

impl DensePairOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if !self.relative_tolerance.is_finite() || self.relative_tolerance <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "dense_pair_relative_tolerance",
                message: format!(
                    "must be finite and positive, got {}",
                    self.relative_tolerance
                ),
            });
        }
        if !self.partition_weight.is_finite() || self.partition_weight <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "dense_pair_partition_weight",
                message: format!("must be finite and positive, got {}", self.partition_weight),
            });
        }
        Ok(self)
    }
}

/// Additive Schwarz preconditioner using exact dense pseudoinverses of all
/// three factor-pair marginals.
///
/// This is a small-problem quality ceiling for pair-CMG and other local solvers,
/// not a production-scalable implementation.
#[derive(Debug, Clone)]
pub struct DensePairSchwarzPreconditioner {
    problem: ThreeWayProblem,
    pairs: [DensePairSystem; 3],
    partition_weight: f64,
}

impl DensePairSchwarzPreconditioner {
    /// Build exact dense pair systems.
    pub fn build(
        problem: ThreeWayProblem,
        options: DensePairOptions,
    ) -> Result<Self, MultiwayError> {
        let options = options.validate()?;
        let pairs = [
            DensePairSystem::build(&problem, 0, 1, options.relative_tolerance)?,
            DensePairSystem::build(&problem, 0, 2, options.relative_tolerance)?,
            DensePairSystem::build(&problem, 1, 2, options.relative_tolerance)?,
        ];
        Ok(Self {
            problem,
            pairs,
            partition_weight: options.partition_weight,
        })
    }

    /// Underlying weighted problem.
    #[must_use]
    pub const fn problem(&self) -> &ThreeWayProblem {
        &self.problem
    }
}

impl Preconditioner for DensePairSchwarzPreconditioner {
    fn dimension(&self) -> usize {
        self.problem.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        let dimension = self.dimension();
        if rhs.len() != dimension {
            return Err(crate::error::dimension(
                "DensePairSchwarzPreconditioner::apply rhs",
                dimension,
                rhs.len(),
            ));
        }
        if out.len() != dimension {
            return Err(crate::error::dimension(
                "DensePairSchwarzPreconditioner::apply output",
                dimension,
                out.len(),
            ));
        }
        let mut compatible_rhs = rhs.to_vec();
        self.problem
            .components()
            .project_structural_range(&mut compatible_rhs)?;
        out.fill(0.0);
        for pair in &self.pairs {
            pair.accumulate(&compatible_rhs, out, self.partition_weight)?;
        }
        self.problem.components().project_structural_range(out)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct DensePairSystem {
    first_count: usize,
    second_count: usize,
    first_offset: usize,
    second_offset: usize,
    inverse: DensePairInverse,
}

impl DensePairSystem {
    fn build(
        problem: &ThreeWayProblem,
        first: usize,
        second: usize,
        relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        let counts = problem.topology().level_counts();
        let offsets = problem.topology().offsets();
        let first_count = counts[first];
        let second_count = counts[second];
        let dimension = first_count + second_count;
        let mut matrix = DMatrix::zeros(dimension, dimension);
        for level in 0..first_count {
            matrix[(level, level)] = problem.diagonal()[offsets[first] + level];
        }
        for level in 0..second_count {
            matrix[(first_count + level, first_count + level)] =
                problem.diagonal()[offsets[second] + level];
        }
        let mut marginal: BTreeMap<(u32, u32), f64> = BTreeMap::new();
        for (&tuple, &weight) in problem.topology().tuples().iter().zip(problem.weights()) {
            *marginal.entry((tuple[first], tuple[second])).or_insert(0.0) += weight;
        }
        for ((left, right), weight) in marginal {
            let row = left as usize;
            let column = first_count + right as usize;
            matrix[(row, column)] = weight;
            matrix[(column, row)] = weight;
        }
        Ok(Self {
            first_count,
            second_count,
            first_offset: offsets[first],
            second_offset: offsets[second],
            inverse: DensePairInverse::new(matrix, relative_tolerance)?,
        })
    }

    fn accumulate(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        partition_weight: f64,
    ) -> Result<(), MultiwayError> {
        let dimension = self.first_count + self.second_count;
        let mut local_rhs = vec![0.0; dimension];
        for level in 0..self.first_count {
            local_rhs[level] = partition_weight * rhs[self.first_offset + level];
        }
        for level in 0..self.second_count {
            local_rhs[self.first_count + level] =
                partition_weight * rhs[self.second_offset + level];
        }
        let mut local_solution = vec![0.0; dimension];
        self.inverse.solve_into(&local_rhs, &mut local_solution);
        for level in 0..self.first_count {
            out[self.first_offset + level] += partition_weight * local_solution[level];
        }
        for level in 0..self.second_count {
            out[self.second_offset + level] +=
                partition_weight * local_solution[self.first_count + level];
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct DensePairInverse {
    eigenvectors: DMatrix<f64>,
    inverse_eigenvalues: Vec<f64>,
}

impl DensePairInverse {
    fn new(matrix: DMatrix<f64>, relative_tolerance: f64) -> Result<Self, MultiwayError> {
        let decomposition = SymmetricEigen::new(matrix);
        let scale = decomposition
            .eigenvalues
            .iter()
            .copied()
            .map(f64::abs)
            .fold(0.0, f64::max);
        if !scale.is_finite() || scale <= 0.0 {
            return Err(MultiwayError::SpectralAnalysis {
                message: format!("dense pair spectral scale is {scale}"),
            });
        }
        let threshold = relative_tolerance * scale;
        let mut inverse_eigenvalues = Vec::with_capacity(decomposition.eigenvalues.len());
        for &eigenvalue in decomposition.eigenvalues.iter() {
            if eigenvalue < -threshold {
                return Err(MultiwayError::NegativeEigenvalue {
                    value: eigenvalue,
                    tolerance: threshold,
                });
            }
            inverse_eigenvalues.push(if eigenvalue > threshold {
                1.0 / eigenvalue
            } else {
                0.0
            });
        }
        Ok(Self {
            eigenvectors: decomposition.eigenvectors,
            inverse_eigenvalues,
        })
    }

    fn solve_into(&self, rhs: &[f64], out: &mut [f64]) {
        let dimension = self.inverse_eigenvalues.len();
        debug_assert_eq!(rhs.len(), dimension);
        debug_assert_eq!(out.len(), dimension);
        let mut modal = vec![0.0; dimension];
        for (mode, value) in modal.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (row, &right) in rhs.iter().enumerate() {
                sum = self.eigenvectors[(row, mode)].mul_add(right, sum);
            }
            *value = sum * self.inverse_eigenvalues[mode];
        }
        for (row, output) in out.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (mode, &value) in modal.iter().enumerate() {
                sum = self.eigenvectors[(row, mode)].mul_add(value, sum);
            }
            *output = sum;
        }
    }
}
