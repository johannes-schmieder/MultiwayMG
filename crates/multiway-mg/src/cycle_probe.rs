//! Matrix-free quality probes for complete fixed preconditioner cycles.
//!
//! Compatible relaxation measures how a smoother damps the complement of a
//! proposed hard coarse space. That is useful for witness generation, but it is
//! not a complete acceptance test: coarse correction and smoothing can interact
//! so that a full two-grid cycle is excellent even when smoother-only compatible
//! contraction is slow.
//!
//! This module probes the actual error operator
//!
//! ```text
//! E = I - omega M^{-1} G
//! ```
//!
//! in the Gramian energy norm. For a symmetric preconditioner `M^{-1}`, `E` is
//! self-adjoint in the `G` inner product. Deterministic range starts followed by
//! normalized power iterations therefore estimate its largest-magnitude error
//! eigenvalue without materializing a dense Gramian or preconditioner.

use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

/// Controls deterministic matrix-free probing of a complete cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CycleQualityOptions {
    /// Number of independent deterministic range starts.
    pub test_vectors: usize,
    /// Number of normalized error-operator power iterations per start.
    pub power_iterations: usize,
    /// Number of final iteration factors included in the asymptotic estimate.
    pub tail_iterations: usize,
    /// Fixed scalar multiplying each preconditioner correction.
    pub correction_damping: f64,
    /// Deterministic SplitMix64 seed.
    pub seed: u64,
    /// Relative threshold below which an error is treated as annihilated.
    pub relative_zero_tolerance: f64,
}

impl Default for CycleQualityOptions {
    fn default() -> Self {
        Self {
            test_vectors: 8,
            power_iterations: 16,
            tail_iterations: 4,
            correction_damping: 1.0,
            seed: 0x4d57_4d47_4359_4331,
            relative_zero_tolerance: 1.0e-13,
        }
    }
}

impl CycleQualityOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if self.test_vectors == 0 {
            return Err(invalid("cycle_quality_test_vectors", "must be positive"));
        }
        if self.power_iterations == 0 {
            return Err(invalid(
                "cycle_quality_power_iterations",
                "must be positive",
            ));
        }
        if self.tail_iterations == 0 || self.tail_iterations > self.power_iterations {
            return Err(invalid(
                "cycle_quality_tail_iterations",
                format!(
                    "must lie in 1..={}, got {}",
                    self.power_iterations, self.tail_iterations
                ),
            ));
        }
        if !self.correction_damping.is_finite() || self.correction_damping <= 0.0 {
            return Err(invalid(
                "cycle_quality_correction_damping",
                format!(
                    "must be finite and positive, got {}",
                    self.correction_damping
                ),
            ));
        }
        if !self.relative_zero_tolerance.is_finite() || self.relative_zero_tolerance <= 0.0 {
            return Err(invalid(
                "cycle_quality_relative_zero_tolerance",
                format!(
                    "must be finite and positive, got {}",
                    self.relative_zero_tolerance
                ),
            ));
        }
        Ok(self)
    }
}

/// Power-iteration history for one deterministic range start.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleQualityVectorReport {
    energy_factor_history: Vec<f64>,
    rayleigh_history: Vec<f64>,
    estimated_energy_factor: f64,
    maximum_observed_energy_factor: f64,
    final_rayleigh: f64,
    annihilated: bool,
    final_error: Vec<f64>,
}

impl CycleQualityVectorReport {
    /// Energy-norm factor measured at every error-operator application.
    #[must_use]
    pub fn energy_factor_history(&self) -> &[f64] {
        &self.energy_factor_history
    }

    /// `G`-inner-product Rayleigh quotient of the error operator at every step.
    #[must_use]
    pub fn rayleigh_history(&self) -> &[f64] {
        &self.rayleigh_history
    }

    /// Geometric mean of the final configured tail factors.
    #[must_use]
    pub const fn estimated_energy_factor(&self) -> f64 {
        self.estimated_energy_factor
    }

    /// Largest one-step factor observed from this start.
    #[must_use]
    pub const fn maximum_observed_energy_factor(&self) -> f64 {
        self.maximum_observed_energy_factor
    }

    /// Final signed error-operator Rayleigh quotient.
    #[must_use]
    pub const fn final_rayleigh(&self) -> f64 {
        self.final_rayleigh
    }

    /// Whether the cycle reduced the test error below the numerical zero gate.
    #[must_use]
    pub const fn annihilated(&self) -> bool {
        self.annihilated
    }

    /// Final normalized power vector, or the final near-zero error when
    /// annihilated. This is a complete-cycle slow witness suitable for later
    /// aggregate attribution.
    #[must_use]
    pub fn final_error(&self) -> &[f64] {
        &self.final_error
    }

    /// Principal retained bytes in owned vector storage.
    #[must_use]
    pub fn retained_bytes_estimate(&self) -> usize {
        core::mem::size_of::<Self>()
            .saturating_add(
                self.energy_factor_history
                    .capacity()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
            .saturating_add(
                self.rayleigh_history
                    .capacity()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
            .saturating_add(
                self.final_error
                    .capacity()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
    }
}

/// Aggregate matrix-free quality report for one complete fixed cycle.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleQualityReport {
    dimension: usize,
    test_vectors: usize,
    configured_power_iterations: usize,
    tail_iterations: usize,
    correction_damping: f64,
    maximum_estimated_energy_factor: f64,
    geometric_mean_estimated_energy_factor: f64,
    maximum_observed_energy_factor: f64,
    maximum_absolute_final_rayleigh: f64,
    maximum_structural_defect: f64,
    gramian_applications: usize,
    preconditioner_applications: usize,
    energy_evaluations: usize,
    vectors: Vec<CycleQualityVectorReport>,
}

impl CycleQualityReport {
    /// Coefficient dimension of the submitted problem.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// Number of deterministic starts.
    #[must_use]
    pub const fn test_vectors(&self) -> usize {
        self.test_vectors
    }

    /// Configured maximum power iterations per start.
    #[must_use]
    pub const fn configured_power_iterations(&self) -> usize {
        self.configured_power_iterations
    }

    /// Tail length used for asymptotic estimates.
    #[must_use]
    pub const fn tail_iterations(&self) -> usize {
        self.tail_iterations
    }

    /// Fixed scalar applied to every cycle correction.
    #[must_use]
    pub const fn correction_damping(&self) -> f64 {
        self.correction_damping
    }

    /// Worst tail-geometric energy factor across starts.
    #[must_use]
    pub const fn maximum_estimated_energy_factor(&self) -> f64 {
        self.maximum_estimated_energy_factor
    }

    /// Geometric mean of the per-start tail estimates.
    #[must_use]
    pub const fn geometric_mean_estimated_energy_factor(&self) -> f64 {
        self.geometric_mean_estimated_energy_factor
    }

    /// Largest one-step energy factor seen anywhere in the probe.
    #[must_use]
    pub const fn maximum_observed_energy_factor(&self) -> f64 {
        self.maximum_observed_energy_factor
    }

    /// Largest absolute final signed Rayleigh quotient across starts.
    #[must_use]
    pub const fn maximum_absolute_final_rayleigh(&self) -> f64 {
        self.maximum_absolute_final_rayleigh
    }

    /// Largest known structural-shift defect after an error step.
    #[must_use]
    pub const fn maximum_structural_defect(&self) -> f64 {
        self.maximum_structural_defect
    }

    /// Gramian applications, including range-start construction.
    #[must_use]
    pub const fn gramian_applications(&self) -> usize {
        self.gramian_applications
    }

    /// Complete preconditioner-cycle applications.
    #[must_use]
    pub const fn preconditioner_applications(&self) -> usize {
        self.preconditioner_applications
    }

    /// Direct Gramian-energy evaluations.
    #[must_use]
    pub const fn energy_evaluations(&self) -> usize {
        self.energy_evaluations
    }

    /// Per-start power histories and final complete-cycle witnesses.
    #[must_use]
    pub fn vectors(&self) -> &[CycleQualityVectorReport] {
        &self.vectors
    }

    /// Index of the start with the largest tail factor, with deterministic
    /// lowest-index tie breaking.
    #[must_use]
    pub fn slowest_vector_index(&self) -> Option<usize> {
        self.vectors
            .iter()
            .enumerate()
            .max_by(|(left_index, left), (right_index, right)| {
                left.estimated_energy_factor()
                    .total_cmp(&right.estimated_energy_factor())
                    .then_with(|| right_index.cmp(left_index))
            })
            .map(|(index, _)| index)
    }

    /// Principal retained bytes in reports and final witnesses.
    #[must_use]
    pub fn retained_bytes_estimate(&self) -> usize {
        core::mem::size_of::<Self>()
            .saturating_add(
                self.vectors
                    .capacity()
                    .saturating_mul(core::mem::size_of::<CycleQualityVectorReport>()),
            )
            .saturating_add(
                self.vectors
                    .iter()
                    .map(CycleQualityVectorReport::retained_bytes_estimate)
                    .sum::<usize>(),
            )
    }
}

/// Explicit acceptance limits for a complete-cycle quality report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CycleQualityCriteria {
    /// Largest accepted worst tail-geometric energy factor.
    pub maximum_estimated_energy_factor: f64,
    /// Optional bound on any observed one-step energy amplification.
    pub maximum_observed_energy_factor: Option<f64>,
    /// Largest accepted known structural-shift defect.
    pub maximum_structural_defect: f64,
}

impl CycleQualityCriteria {
    fn validate(self) -> Result<Self, MultiwayError> {
        validate_positive_finite(
            "maximum_estimated_cycle_energy_factor",
            self.maximum_estimated_energy_factor,
        )?;
        if let Some(limit) = self.maximum_observed_energy_factor {
            validate_positive_finite("maximum_observed_cycle_energy_factor", limit)?;
        }
        if !self.maximum_structural_defect.is_finite() || self.maximum_structural_defect < 0.0 {
            return Err(invalid(
                "maximum_cycle_structural_defect",
                format!(
                    "must be finite and nonnegative, got {}",
                    self.maximum_structural_defect
                ),
            ));
        }
        Ok(self)
    }
}

/// One failed complete-cycle criterion.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CycleQualityRejection {
    /// Estimated asymptotic energy factor exceeded its limit.
    EstimatedEnergyFactor {
        /// Observed worst factor.
        observed: f64,
        /// Maximum accepted factor.
        limit: f64,
    },
    /// A one-step factor exceeded the optional amplification limit.
    ObservedEnergyFactor {
        /// Observed maximum one-step factor.
        observed: f64,
        /// Maximum accepted factor.
        limit: f64,
    },
    /// Known structural-shift drift exceeded its limit.
    StructuralDefect {
        /// Observed maximum defect.
        observed: f64,
        /// Maximum accepted defect.
        limit: f64,
    },
}

/// Deterministic decision for one complete-cycle probe.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleQualityDecision {
    rejections: Vec<CycleQualityRejection>,
}

impl CycleQualityDecision {
    /// Whether every declared complete-cycle criterion passed.
    #[must_use]
    pub fn accepted(&self) -> bool {
        self.rejections.is_empty()
    }

    /// Failed criteria in stable evaluation order.
    #[must_use]
    pub fn rejections(&self) -> &[CycleQualityRejection] {
        &self.rejections
    }
}

/// Probe the complete error operator with deterministic normalized power starts.
pub fn analyze_cycle_quality<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    preconditioner: &P,
    options: CycleQualityOptions,
) -> Result<CycleQualityReport, MultiwayError> {
    let options = options.validate()?;
    if preconditioner.dimension() != problem.dimension() {
        return Err(crate::error::dimension(
            "analyze_cycle_quality preconditioner",
            problem.dimension(),
            preconditioner.dimension(),
        ));
    }

    let mut vectors = Vec::with_capacity(options.test_vectors);
    let mut gramian_applications = 0_usize;
    let mut preconditioner_applications = 0_usize;
    let mut energy_evaluations = 0_usize;
    let mut maximum_structural_defect = 0.0_f64;

    for vector_index in 0..options.test_vectors {
        let (mut error, start_gramian_applications, start_energy_evaluations) =
            deterministic_range_start(problem, options, vector_index)?;
        gramian_applications =
            gramian_applications.saturating_add(start_gramian_applications);
        energy_evaluations = energy_evaluations.saturating_add(start_energy_evaluations);
        let mut energy_factor_history = Vec::with_capacity(options.power_iterations);
        let mut rayleigh_history = Vec::with_capacity(options.power_iterations);
        let mut gradient = vec![0.0; problem.dimension()];
        let mut correction = vec![0.0; problem.dimension()];
        let mut next = vec![0.0; problem.dimension()];
        let mut annihilated = false;

        for _ in 0..options.power_iterations {
            problem.apply_gramian(&error, &mut gradient)?;
            gramian_applications = gramian_applications.saturating_add(1);
            preconditioner.apply(&gradient, &mut correction)?;
            preconditioner_applications = preconditioner_applications.saturating_add(1);
            ensure_finite("cycle correction", &correction)?;
            problem
                .components()
                .project_structural_range(&mut correction)?;
            for ((next_value, &error_value), &correction_value) in
                next.iter_mut().zip(&error).zip(&correction)
            {
                *next_value = (-options.correction_damping)
                    .mul_add(correction_value, error_value);
            }
            problem.components().project_structural_range(&mut next)?;
            ensure_finite("cycle error", &next)?;
            maximum_structural_defect = maximum_structural_defect.max(
                problem
                    .components()
                    .maximum_structural_defect(&next)?,
            );
            let rayleigh = dot(&gradient, &next);
            let next_energy = energy_norm(problem, &next)?;
            energy_evaluations = energy_evaluations.saturating_add(1);
            energy_factor_history.push(next_energy);
            rayleigh_history.push(rayleigh);
            if next_energy <= options.relative_zero_tolerance {
                annihilated = true;
                error.copy_from_slice(&next);
                break;
            }
            scale_in_place(&mut next, 1.0 / next_energy);
            core::mem::swap(&mut error, &mut next);
        }

        let estimated_energy_factor = if annihilated {
            0.0
        } else {
            tail_geometric_mean(&energy_factor_history, options.tail_iterations)
        };
        let maximum_observed_energy_factor = energy_factor_history
            .iter()
            .copied()
            .fold(0.0, f64::max);
        let final_rayleigh = rayleigh_history.last().copied().unwrap_or(0.0);
        vectors.push(CycleQualityVectorReport {
            energy_factor_history,
            rayleigh_history,
            estimated_energy_factor,
            maximum_observed_energy_factor,
            final_rayleigh,
            annihilated,
            final_error: error,
        });
    }

    let estimates: Vec<f64> = vectors
        .iter()
        .map(CycleQualityVectorReport::estimated_energy_factor)
        .collect();
    let maximum_estimated_energy_factor = estimates.iter().copied().fold(0.0, f64::max);
    let geometric_mean_estimated_energy_factor = geometric_mean(&estimates);
    let maximum_observed_energy_factor = vectors
        .iter()
        .map(CycleQualityVectorReport::maximum_observed_energy_factor)
        .fold(0.0, f64::max);
    let maximum_absolute_final_rayleigh = vectors
        .iter()
        .map(|report| report.final_rayleigh().abs())
        .fold(0.0, f64::max);

    Ok(CycleQualityReport {
        dimension: problem.dimension(),
        test_vectors: options.test_vectors,
        configured_power_iterations: options.power_iterations,
        tail_iterations: options.tail_iterations,
        correction_damping: options.correction_damping,
        maximum_estimated_energy_factor,
        geometric_mean_estimated_energy_factor,
        maximum_observed_energy_factor,
        maximum_absolute_final_rayleigh,
        maximum_structural_defect,
        gramian_applications,
        preconditioner_applications,
        energy_evaluations,
        vectors,
    })
}

/// Evaluate a complete-cycle probe against explicit criteria.
pub fn evaluate_cycle_quality(
    report: &CycleQualityReport,
    criteria: CycleQualityCriteria,
) -> Result<CycleQualityDecision, MultiwayError> {
    let criteria = criteria.validate()?;
    let mut rejections = Vec::new();
    if report.maximum_estimated_energy_factor() > criteria.maximum_estimated_energy_factor {
        rejections.push(CycleQualityRejection::EstimatedEnergyFactor {
            observed: report.maximum_estimated_energy_factor(),
            limit: criteria.maximum_estimated_energy_factor,
        });
    }
    if let Some(limit) = criteria.maximum_observed_energy_factor {
        if report.maximum_observed_energy_factor() > limit {
            rejections.push(CycleQualityRejection::ObservedEnergyFactor {
                observed: report.maximum_observed_energy_factor(),
                limit,
            });
        }
    }
    if report.maximum_structural_defect() > criteria.maximum_structural_defect {
        rejections.push(CycleQualityRejection::StructuralDefect {
            observed: report.maximum_structural_defect(),
            limit: criteria.maximum_structural_defect,
        });
    }
    Ok(CycleQualityDecision { rejections })
}

fn deterministic_range_start(
    problem: &ThreeWayProblem,
    options: CycleQualityOptions,
    vector_index: usize,
) -> Result<(Vec<f64>, usize, usize), MultiwayError> {
    let mut random = vec![0.0; problem.dimension()];
    let mut error = vec![0.0; problem.dimension()];
    for attempt in 0..16_u64 {
        fill_deterministic(
            &mut random,
            options.seed
                ^ (vector_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                ^ attempt.wrapping_mul(0xbf58_476d_1ce4_e5b9),
        );
        problem.apply_gramian(&random, &mut error)?;
        problem.components().project_structural_range(&mut error)?;
        let energy = energy_norm(problem, &error)?;
        if energy > options.relative_zero_tolerance {
            scale_in_place(&mut error, 1.0 / energy);
            orient_deterministically(&mut error);
            return Ok((error, attempt as usize + 1, attempt as usize + 1));
        }
    }
    Err(MultiwayError::CycleQuality {
        message: format!(
            "unable to generate nonzero range start for test vector {vector_index}"
        ),
    })
}

fn energy_norm(problem: &ThreeWayProblem, values: &[f64]) -> Result<f64, MultiwayError> {
    let energy = problem.energy(values)?;
    if !energy.is_finite() || energy < -64.0 * f64::EPSILON {
        return Err(MultiwayError::CycleQuality {
            message: format!("invalid Gramian energy {energy}"),
        });
    }
    Ok(energy.max(0.0).sqrt())
}

fn tail_geometric_mean(values: &[f64], tail: usize) -> f64 {
    let start = values.len().saturating_sub(tail);
    geometric_mean(&values[start..])
}

fn geometric_mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    if values.iter().any(|&value| value == 0.0) {
        return 0.0;
    }
    (values.iter().map(|value| value.ln()).sum::<f64>() / values.len() as f64).exp()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .fold(0.0, |sum, (&a, &b)| a.mul_add(b, sum))
}

fn scale_in_place(values: &mut [f64], scale: f64) {
    for value in values {
        *value *= scale;
    }
}

fn orient_deterministically(values: &mut [f64]) {
    if let Some(&first) = values.iter().find(|value| value.abs() > 1.0e-15) {
        if first < 0.0 {
            scale_in_place(values, -1.0);
        }
    }
}

fn fill_deterministic(values: &mut [f64], mut state: u64) {
    for value in values {
        state = splitmix64(state);
        let unit = (state >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64));
        *value = 2.0 * unit - 1.0;
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn ensure_finite(context: &'static str, values: &[f64]) -> Result<(), MultiwayError> {
    if let Some((index, value)) = values
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(MultiwayError::CycleQuality {
            message: format!("{context} entry {index} is nonfinite: {value}"),
        });
    }
    Ok(())
}

fn validate_positive_finite(name: &'static str, value: f64) -> Result<(), MultiwayError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(invalid(
            name,
            format!("must be finite and positive, got {value}"),
        ));
    }
    Ok(())
}

fn invalid(name: &'static str, message: impl Into<String>) -> MultiwayError {
    MultiwayError::InvalidOption {
        name,
        message: message.into(),
    }
}
