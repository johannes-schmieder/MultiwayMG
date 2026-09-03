//! Projected compatible-relaxation diagnostics for hard factor aggregations.
//!
//! A hard aggregation `P` represents one coarse value per aggregate. This
//! module removes the coarse-representable component in the diagonal-energy
//! inner product and measures how rapidly a fixed smoother damps the remaining
//! error on the homogeneous three-way Gramian system.

use crate::{FactorAggregation, MultiwayError, Preconditioner, ThreeWayProblem};

/// Orthogonal projector onto the diagonal-energy complement of `range(P)`.
///
/// For `D = diag(G)`, the coarse projection is
///
/// ```text
/// Pi_D = P (P' D P)^(-1) P' D.
/// ```
///
/// Hard factor aggregates have disjoint indicator columns, so `P' D P` is a
/// positive diagonal matrix. The complementary projection is `I - Pi_D`.
#[derive(Debug, Clone)]
pub struct DiagonalAggregationProjector {
    problem: ThreeWayProblem,
    aggregation: FactorAggregation,
    coarse_offsets: [usize; 4],
    global_parents: Vec<usize>,
    aggregate_diagonal: Vec<f64>,
    component_factor_diagonal: Vec<[f64; 3]>,
}

impl DiagonalAggregationProjector {
    /// Validate an aggregation and construct its diagonal-energy projector.
    pub fn new(
        problem: ThreeWayProblem,
        aggregation: FactorAggregation,
    ) -> Result<Self, MultiwayError> {
        let fine_counts = problem.topology().level_counts();
        if aggregation.fine_counts() != fine_counts {
            return Err(MultiwayError::InvalidAggregation {
                message: format!(
                    "fine counts {:?} do not match problem counts {:?}",
                    aggregation.fine_counts(),
                    fine_counts
                ),
            });
        }
        validate_component_preservation(&problem, &aggregation)?;

        let coarse_counts = aggregation.coarse_counts();
        let first = coarse_counts[0];
        let second = first.checked_add(coarse_counts[1]).ok_or_else(|| {
            MultiwayError::InvalidAggregation {
                message: "coarse offset arithmetic overflowed".to_owned(),
            }
        })?;
        let total = second.checked_add(coarse_counts[2]).ok_or_else(|| {
            MultiwayError::InvalidAggregation {
                message: "coarse dimension arithmetic overflowed".to_owned(),
            }
        })?;
        let coarse_offsets = [0, first, second, total];
        let mut global_parents = vec![0; problem.dimension()];
        let mut aggregate_diagonal = vec![0.0; total];
        let fine_offsets = problem.topology().offsets();

        for factor in 0..3 {
            for level in 0..fine_counts[factor] {
                let fine_index = fine_offsets[factor] + level;
                let coarse_index =
                    coarse_offsets[factor] + aggregation.parents(factor)[level] as usize;
                global_parents[fine_index] = coarse_index;
                aggregate_diagonal[coarse_index] += problem.diagonal()[fine_index];
            }
        }
        if let Some((aggregate, value)) = aggregate_diagonal
            .iter()
            .copied()
            .enumerate()
            .find(|(_, value)| !value.is_finite() || *value <= 0.0)
        {
            return Err(MultiwayError::InvalidAggregation {
                message: format!(
                    "aggregate {aggregate} has nonpositive or nonfinite diagonal mass {value}"
                ),
            });
        }

        let mut component_factor_diagonal = vec![[0.0; 3]; problem.components().count()];
        for factor in 0..3 {
            for level in 0..fine_counts[factor] {
                let fine_index = fine_offsets[factor] + level;
                let component = problem.components().component_of(factor, level);
                component_factor_diagonal[component][factor] += problem.diagonal()[fine_index];
            }
        }

        Ok(Self {
            problem,
            aggregation,
            coarse_offsets,
            global_parents,
            aggregate_diagonal,
            component_factor_diagonal,
        })
    }

    /// Underlying problem.
    #[must_use]
    pub const fn problem(&self) -> &ThreeWayProblem {
        &self.problem
    }

    /// Hard factor aggregation.
    #[must_use]
    pub const fn aggregation(&self) -> &FactorAggregation {
        &self.aggregation
    }

    /// Fine coefficient dimension.
    #[must_use]
    pub fn fine_dimension(&self) -> usize {
        self.problem.dimension()
    }

    /// Number of independent hard aggregate columns.
    #[must_use]
    pub const fn coarse_dimension(&self) -> usize {
        self.coarse_offsets[3]
    }

    /// Dimension of the diagonal-energy compatible complement.
    #[must_use]
    pub fn compatible_dimension(&self) -> usize {
        self.fine_dimension() - self.coarse_dimension()
    }

    /// Diagonal mass of every factor-local aggregate in global coarse order.
    #[must_use]
    pub fn aggregate_diagonal(&self) -> &[f64] {
        &self.aggregate_diagonal
    }

    /// Remove the coarse-representable component in place.
    ///
    /// Returns the diagonal-energy norm of the removed component. The retained
    /// and removed vectors are orthogonal in the `D` inner product.
    pub fn project_complement_in_place(&self, values: &mut [f64]) -> Result<f64, MultiwayError> {
        self.validate_values("DiagonalAggregationProjector::project", values)?;
        let moments = self.coarse_moments(values);
        let mut coarse_values = vec![0.0; self.coarse_dimension()];
        let mut removed_squared = 0.0;
        for aggregate in 0..self.coarse_dimension() {
            let value = moments[aggregate] / self.aggregate_diagonal[aggregate];
            coarse_values[aggregate] = value;
            removed_squared =
                value.mul_add(value * self.aggregate_diagonal[aggregate], removed_squared);
        }
        for (fine_index, value) in values.iter_mut().enumerate() {
            *value -= coarse_values[self.global_parents[fine_index]];
        }
        ensure_finite("compatible projection", values)?;
        Ok(removed_squared.max(0.0).sqrt())
    }

    /// Diagonal-energy norm `sqrt(x' D x)`.
    pub fn diagonal_norm(&self, values: &[f64]) -> Result<f64, MultiwayError> {
        self.validate_values("DiagonalAggregationProjector::diagonal_norm", values)?;
        Ok(weighted_norm(values, self.problem.diagonal()))
    }

    /// Factor-block contributions to the diagonal-energy norm.
    pub fn factor_diagonal_norms(&self, values: &[f64]) -> Result<[f64; 3], MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::factor_diagonal_norms",
            values,
        )?;
        let offsets = self.problem.topology().offsets();
        Ok(core::array::from_fn(|factor| {
            weighted_norm(
                &values[offsets[factor]..offsets[factor + 1]],
                &self.problem.diagonal()[offsets[factor]..offsets[factor + 1]],
            )
        }))
    }

    /// Relative maximum normalized coarse moment.
    ///
    /// A value near zero verifies `P' D x = 0` after complementary projection.
    pub fn relative_coarse_defect(&self, values: &[f64]) -> Result<f64, MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::relative_coarse_defect",
            values,
        )?;
        let reference_norm = weighted_norm(values, self.problem.diagonal());
        self.coarse_defect_with_reference(values, reference_norm)
    }

    /// Relative weighted defect against the two structural factor-shift modes
    /// in every incidence component.
    ///
    /// Because those modes lie in `range(P)`, a successful complementary
    /// projection drives this diagnostic to roundoff.
    pub fn relative_structural_defect(&self, values: &[f64]) -> Result<f64, MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::relative_structural_defect",
            values,
        )?;
        let reference_norm = weighted_norm(values, self.problem.diagonal());
        self.structural_defect_with_reference(values, reference_norm)
    }

    fn coarse_defect_with_reference(
        &self,
        values: &[f64],
        reference_norm: f64,
    ) -> Result<f64, MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::coarse_defect_with_reference",
            values,
        )?;
        if !reference_norm.is_finite() || reference_norm < 0.0 {
            return Err(MultiwayError::CompatibleRelaxation {
                message: format!("invalid coarse-defect reference norm {reference_norm}"),
            });
        }
        let moments = self.coarse_moments(values);
        let maximum = moments
            .iter()
            .zip(&self.aggregate_diagonal)
            .map(|(&moment, &weight)| moment.abs() / weight.sqrt())
            .fold(0.0, f64::max);
        Ok(if reference_norm == 0.0 {
            if maximum == 0.0 { 0.0 } else { f64::INFINITY }
        } else {
            maximum / reference_norm
        })
    }

    fn structural_defect_with_reference(
        &self,
        values: &[f64],
        reference_norm: f64,
    ) -> Result<f64, MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::structural_defect_with_reference",
            values,
        )?;
        if !reference_norm.is_finite() || reference_norm < 0.0 {
            return Err(MultiwayError::CompatibleRelaxation {
                message: format!("invalid structural-defect reference norm {reference_norm}"),
            });
        }
        let offsets = self.problem.topology().offsets();
        let counts = self.problem.topology().level_counts();
        let mut sums = vec![[0.0; 3]; self.problem.components().count()];
        for factor in 0..3 {
            for level in 0..counts[factor] {
                let index = offsets[factor] + level;
                let component = self.problem.components().component_of(factor, level);
                sums[component][factor] += self.problem.diagonal()[index] * values[index];
            }
        }
        let mut maximum = 0.0_f64;
        for (component, [first, second, third]) in sums.into_iter().enumerate() {
            let masses = self.component_factor_diagonal[component];
            maximum = maximum.max((first - second).abs() / (masses[0] + masses[1]).sqrt());
            maximum = maximum.max((first - third).abs() / (masses[0] + masses[2]).sqrt());
        }
        Ok(if reference_norm == 0.0 {
            if maximum == 0.0 { 0.0 } else { f64::INFINITY }
        } else {
            maximum / reference_norm
        })
    }

    fn validate_values(&self, context: &'static str, values: &[f64]) -> Result<(), MultiwayError> {
        if values.len() != self.fine_dimension() {
            return Err(crate::error::dimension(
                context,
                self.fine_dimension(),
                values.len(),
            ));
        }
        ensure_finite(context, values)
    }

    fn coarse_moments(&self, values: &[f64]) -> Vec<f64> {
        let mut sums = vec![0.0; self.coarse_dimension()];
        let mut corrections = vec![0.0; self.coarse_dimension()];
        for (fine_index, &value) in values.iter().enumerate() {
            let aggregate = self.global_parents[fine_index];
            neumaier_add(
                &mut sums[aggregate],
                &mut corrections[aggregate],
                self.problem.diagonal()[fine_index] * value,
            );
        }
        for (sum, correction) in sums.iter_mut().zip(corrections) {
            *sum += correction;
        }
        sums
    }
}

/// Controls deterministic projected compatible relaxation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompatibleRelaxationOptions {
    /// Number of deterministic test errors.
    pub test_vectors: usize,
    /// Number of homogeneous smoothing sweeps.
    pub sweeps: usize,
    /// Scalar multiplying each fixed preconditioner correction.
    pub relaxation_damping: f64,
    /// Deterministic seed for the internal SplitMix64 generator.
    pub seed: u64,
    /// Relative threshold used to reject a numerically empty complement or an
    /// energy norm too small for a meaningful ratio.
    pub relative_zero_tolerance: f64,
}

impl Default for CompatibleRelaxationOptions {
    fn default() -> Self {
        Self {
            test_vectors: 8,
            sweeps: 8,
            relaxation_damping: 1.0,
            seed: 0x4d57_4d47_4352_3031,
            relative_zero_tolerance: 1.0e-13,
        }
    }
}

impl CompatibleRelaxationOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if self.test_vectors == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "compatible_test_vectors",
                message: "must be positive".to_owned(),
            });
        }
        if self.sweeps == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "compatible_sweeps",
                message: "must be positive".to_owned(),
            });
        }
        if !self.relaxation_damping.is_finite() || self.relaxation_damping <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "compatible_relaxation_damping",
                message: format!(
                    "must be finite and positive, got {}",
                    self.relaxation_damping
                ),
            });
        }
        if !self.relative_zero_tolerance.is_finite() || self.relative_zero_tolerance <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "compatible_relative_zero_tolerance",
                message: format!(
                    "must be finite and positive, got {}",
                    self.relative_zero_tolerance
                ),
            });
        }
        Ok(self)
    }
}

/// Per-test-vector compatible-relaxation history.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleRelaxationVectorReport {
    raw_diagonal_norm: f64,
    initially_removed_coarse_norm: f64,
    diagonal_norm_history: Vec<f64>,
    energy_norm_history: Vec<f64>,
    coarse_drift_norm_history: Vec<f64>,
    initial_factor_diagonal_norms: [f64; 3],
    final_factor_diagonal_norms: [f64; 3],
    initial_coarse_defect: f64,
    final_coarse_defect: f64,
    initial_structural_defect: f64,
    final_structural_defect: f64,
    diagonal_contraction: f64,
    energy_contraction: Option<f64>,
}

impl CompatibleRelaxationVectorReport {
    /// Diagonal norm before removing the coarse component.
    #[must_use]
    pub const fn raw_diagonal_norm(&self) -> f64 {
        self.raw_diagonal_norm
    }

    /// Diagonal norm of the initially removed coarse component.
    #[must_use]
    pub const fn initially_removed_coarse_norm(&self) -> f64 {
        self.initially_removed_coarse_norm
    }

    /// Diagonal norm after initial normalization and after every sweep.
    #[must_use]
    pub fn diagonal_norm_history(&self) -> &[f64] {
        &self.diagonal_norm_history
    }

    /// Gramian energy norm after initial normalization and after every sweep.
    #[must_use]
    pub fn energy_norm_history(&self) -> &[f64] {
        &self.energy_norm_history
    }

    /// Diagonal norm of coarse drift removed after each smoothing sweep.
    #[must_use]
    pub fn coarse_drift_norm_history(&self) -> &[f64] {
        &self.coarse_drift_norm_history
    }

    /// Initial factor-block diagonal norms.
    #[must_use]
    pub const fn initial_factor_diagonal_norms(&self) -> [f64; 3] {
        self.initial_factor_diagonal_norms
    }

    /// Final factor-block diagonal norms.
    #[must_use]
    pub const fn final_factor_diagonal_norms(&self) -> [f64; 3] {
        self.final_factor_diagonal_norms
    }

    /// Initial relative `P' D e` defect.
    #[must_use]
    pub const fn initial_coarse_defect(&self) -> f64 {
        self.initial_coarse_defect
    }

    /// Final `P' D e` defect normalized by the initial compatible `D` norm.
    #[must_use]
    pub const fn final_coarse_defect(&self) -> f64 {
        self.final_coarse_defect
    }

    /// Initial weighted structural-shift defect.
    #[must_use]
    pub const fn initial_structural_defect(&self) -> f64 {
        self.initial_structural_defect
    }

    /// Final structural-shift defect normalized by the initial compatible `D` norm.
    #[must_use]
    pub const fn final_structural_defect(&self) -> f64 {
        self.final_structural_defect
    }

    /// Final divided by initial diagonal norm.
    #[must_use]
    pub const fn diagonal_contraction(&self) -> f64 {
        self.diagonal_contraction
    }

    /// Final divided by initial Gramian energy norm, when the initial energy
    /// exceeds the configured relative zero threshold.
    #[must_use]
    pub const fn energy_contraction(&self) -> Option<f64> {
        self.energy_contraction
    }
}

/// Aggregate compatible-relaxation diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleRelaxationReport {
    fine_dimension: usize,
    coarse_dimension: usize,
    compatible_dimension: usize,
    sweeps: usize,
    smoother_applications: usize,
    gramian_applications: usize,
    maximum_diagonal_contraction: f64,
    geometric_mean_diagonal_contraction: f64,
    maximum_energy_contraction: Option<f64>,
    geometric_mean_energy_contraction: Option<f64>,
    maximum_final_coarse_defect: f64,
    maximum_final_structural_defect: f64,
    vectors: Vec<CompatibleRelaxationVectorReport>,
}

impl CompatibleRelaxationReport {
    /// Fine coefficient dimension.
    #[must_use]
    pub const fn fine_dimension(&self) -> usize {
        self.fine_dimension
    }

    /// Coarse aggregate dimension.
    #[must_use]
    pub const fn coarse_dimension(&self) -> usize {
        self.coarse_dimension
    }

    /// Dimension of the compatible complement.
    #[must_use]
    pub const fn compatible_dimension(&self) -> usize {
        self.compatible_dimension
    }

    /// Homogeneous sweeps per test vector.
    #[must_use]
    pub const fn sweeps(&self) -> usize {
        self.sweeps
    }

    /// Total fixed preconditioner applications.
    #[must_use]
    pub const fn smoother_applications(&self) -> usize {
        self.smoother_applications
    }

    /// Total Gramian applications.
    #[must_use]
    pub const fn gramian_applications(&self) -> usize {
        self.gramian_applications
    }

    /// Worst final diagonal-norm contraction across test vectors.
    #[must_use]
    pub const fn maximum_diagonal_contraction(&self) -> f64 {
        self.maximum_diagonal_contraction
    }

    /// Geometric-mean diagonal-norm contraction.
    #[must_use]
    pub const fn geometric_mean_diagonal_contraction(&self) -> f64 {
        self.geometric_mean_diagonal_contraction
    }

    /// Worst Gramian-energy contraction among vectors with meaningful initial energy.
    #[must_use]
    pub const fn maximum_energy_contraction(&self) -> Option<f64> {
        self.maximum_energy_contraction
    }

    /// Geometric-mean Gramian-energy contraction.
    #[must_use]
    pub const fn geometric_mean_energy_contraction(&self) -> Option<f64> {
        self.geometric_mean_energy_contraction
    }

    /// Largest final relative `P' D e` defect.
    #[must_use]
    pub const fn maximum_final_coarse_defect(&self) -> f64 {
        self.maximum_final_coarse_defect
    }

    /// Largest final weighted structural-shift defect.
    #[must_use]
    pub const fn maximum_final_structural_defect(&self) -> f64 {
        self.maximum_final_structural_defect
    }

    /// Per-vector histories and diagnostics.
    #[must_use]
    pub fn vectors(&self) -> &[CompatibleRelaxationVectorReport] {
        &self.vectors
    }
}

/// Measure the ability of a fixed smoother to damp error not represented by a
/// proposed hard coarse space.
pub fn analyze_compatible_relaxation<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    smoother: &P,
    options: CompatibleRelaxationOptions,
) -> Result<CompatibleRelaxationReport, MultiwayError> {
    let options = options.validate()?;
    if smoother.dimension() != problem.dimension() {
        return Err(crate::error::dimension(
            "analyze_compatible_relaxation smoother",
            problem.dimension(),
            smoother.dimension(),
        ));
    }
    let projector = DiagonalAggregationProjector::new(problem.clone(), aggregation.clone())?;
    if projector.compatible_dimension() == 0 {
        return Err(MultiwayError::CompatibleRelaxation {
            message: "aggregation leaves no compatible complement".to_owned(),
        });
    }

    let mut vectors = Vec::with_capacity(options.test_vectors);
    for vector_index in 0..options.test_vectors {
        vectors.push(analyze_vector(&projector, smoother, options, vector_index)?);
    }

    let diagonal_contractions: Vec<f64> = vectors
        .iter()
        .map(CompatibleRelaxationVectorReport::diagonal_contraction)
        .collect();
    let energy_contractions: Vec<f64> = vectors
        .iter()
        .filter_map(CompatibleRelaxationVectorReport::energy_contraction)
        .collect();
    let maximum_diagonal_contraction = diagonal_contractions.iter().copied().fold(0.0, f64::max);
    let geometric_mean_diagonal_contraction = geometric_mean(&diagonal_contractions);
    let maximum_energy_contraction = (!energy_contractions.is_empty())
        .then(|| energy_contractions.iter().copied().fold(0.0, f64::max));
    let geometric_mean_energy_contraction =
        (!energy_contractions.is_empty()).then(|| geometric_mean(&energy_contractions));
    let maximum_final_coarse_defect = vectors
        .iter()
        .map(CompatibleRelaxationVectorReport::final_coarse_defect)
        .fold(0.0, f64::max);
    let maximum_final_structural_defect = vectors
        .iter()
        .map(CompatibleRelaxationVectorReport::final_structural_defect)
        .fold(0.0, f64::max);

    Ok(CompatibleRelaxationReport {
        fine_dimension: projector.fine_dimension(),
        coarse_dimension: projector.coarse_dimension(),
        compatible_dimension: projector.compatible_dimension(),
        sweeps: options.sweeps,
        smoother_applications: options.test_vectors * options.sweeps,
        gramian_applications: options.test_vectors * options.sweeps,
        maximum_diagonal_contraction,
        geometric_mean_diagonal_contraction,
        maximum_energy_contraction,
        geometric_mean_energy_contraction,
        maximum_final_coarse_defect,
        maximum_final_structural_defect,
        vectors,
    })
}

fn analyze_vector<P: Preconditioner + ?Sized>(
    projector: &DiagonalAggregationProjector,
    smoother: &P,
    options: CompatibleRelaxationOptions,
    vector_index: usize,
) -> Result<CompatibleRelaxationVectorReport, MultiwayError> {
    let dimension = projector.fine_dimension();
    let mut error = vec![0.0; dimension];
    let mut raw_diagonal_norm = 0.0;
    let mut initially_removed_coarse_norm = 0.0;
    let mut projected_norm = 0.0;
    let mut found = false;
    for attempt in 0..16_u64 {
        fill_deterministic(
            &mut error,
            options.seed
                ^ (vector_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                ^ attempt.wrapping_mul(0xbf58_476d_1ce4_e5b9),
        );
        raw_diagonal_norm = projector.diagonal_norm(&error)?;
        initially_removed_coarse_norm = projector.project_complement_in_place(&mut error)?;
        projected_norm = projector.diagonal_norm(&error)?;
        if projected_norm
            > options.relative_zero_tolerance * raw_diagonal_norm.max(f64::MIN_POSITIVE)
        {
            found = true;
            break;
        }
    }
    if !found {
        return Err(MultiwayError::CompatibleRelaxation {
            message: format!("unable to generate nonzero compatible test vector {vector_index}"),
        });
    }
    scale_in_place(&mut error, 1.0 / projected_norm);

    let initial_diagonal_norm = projector.diagonal_norm(&error)?;
    let initial_energy_norm = energy_norm(projector.problem(), &error)?;
    let initial_factor_diagonal_norms = projector.factor_diagonal_norms(&error)?;
    let initial_coarse_defect = projector.relative_coarse_defect(&error)?;
    let initial_structural_defect = projector.relative_structural_defect(&error)?;
    let mut diagonal_norm_history = vec![initial_diagonal_norm];
    let mut energy_norm_history = vec![initial_energy_norm];
    let mut coarse_drift_norm_history = Vec::with_capacity(options.sweeps);
    let mut gradient = vec![0.0; dimension];
    let mut correction = vec![0.0; dimension];

    for _ in 0..options.sweeps {
        projector.problem().apply_gramian(&error, &mut gradient)?;
        smoother.apply(&gradient, &mut correction)?;
        ensure_finite("compatible smoother correction", &correction)?;
        for (value, &step) in error.iter_mut().zip(&correction) {
            *value = (-options.relaxation_damping).mul_add(step, *value);
        }
        let drift = projector.project_complement_in_place(&mut error)?;
        coarse_drift_norm_history.push(drift);
        diagonal_norm_history.push(projector.diagonal_norm(&error)?);
        energy_norm_history.push(energy_norm(projector.problem(), &error)?);
    }

    let final_diagonal_norm = *diagonal_norm_history
        .last()
        .expect("history contains the normalized initial state");
    let final_energy_norm = *energy_norm_history
        .last()
        .expect("history contains the initial energy state");
    let energy_scale = initial_energy_norm.max(initial_diagonal_norm);
    let energy_contraction = (initial_energy_norm
        > options.relative_zero_tolerance * energy_scale.max(f64::MIN_POSITIVE))
    .then_some(final_energy_norm / initial_energy_norm);

    Ok(CompatibleRelaxationVectorReport {
        raw_diagonal_norm,
        initially_removed_coarse_norm,
        diagonal_norm_history,
        energy_norm_history,
        coarse_drift_norm_history,
        initial_factor_diagonal_norms,
        final_factor_diagonal_norms: projector.factor_diagonal_norms(&error)?,
        initial_coarse_defect,
        final_coarse_defect: projector
            .coarse_defect_with_reference(&error, initial_diagonal_norm)?,
        initial_structural_defect,
        final_structural_defect: projector
            .structural_defect_with_reference(&error, initial_diagonal_norm)?,
        diagonal_contraction: final_diagonal_norm / initial_diagonal_norm,
        energy_contraction,
    })
}

fn validate_component_preservation(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<(), MultiwayError> {
    let fine_counts = aggregation.fine_counts();
    let coarse_counts = aggregation.coarse_counts();
    for factor in 0..3 {
        let mut parent_components = vec![None; coarse_counts[factor]];
        for level in 0..fine_counts[factor] {
            let parent = aggregation.parents(factor)[level] as usize;
            let component = problem.components().component_of(factor, level);
            match parent_components[parent] {
                None => parent_components[parent] = Some(component),
                Some(existing) if existing == component => {}
                Some(existing) => {
                    return Err(MultiwayError::InvalidAggregation {
                        message: format!(
                            "factor {factor} aggregate {parent} crosses incidence components {existing} and {component}"
                        ),
                    });
                }
            }
        }
    }
    Ok(())
}

fn energy_norm(problem: &ThreeWayProblem, values: &[f64]) -> Result<f64, MultiwayError> {
    Ok(problem.energy(values)?.max(0.0).sqrt())
}

fn weighted_norm(values: &[f64], weights: &[f64]) -> f64 {
    values
        .iter()
        .zip(weights)
        .fold(0.0, |norm, (&value, &weight)| {
            norm.hypot(value.abs() * weight.sqrt())
        })
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

fn geometric_mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    if values.iter().any(|&value| value == 0.0) {
        return 0.0;
    }
    (values.iter().map(|value| value.ln()).sum::<f64>() / values.len() as f64).exp()
}

fn scale_in_place(values: &mut [f64], scale: f64) {
    for value in values {
        *value *= scale;
    }
}

fn ensure_finite(context: &'static str, values: &[f64]) -> Result<(), MultiwayError> {
    if let Some((index, value)) = values
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(MultiwayError::CompatibleRelaxation {
            message: format!("{context} entry {index} is nonfinite: {value}"),
        });
    }
    Ok(())
}

fn neumaier_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let updated = *sum + value;
    if sum.abs() >= value.abs() {
        *correction += (*sum - updated) + value;
    } else {
        *correction += (value - updated) + *sum;
    }
    *sum = updated;
}
