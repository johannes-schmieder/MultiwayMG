//! Deterministic acceptance criteria for compatible-relaxation reports.

use crate::{CompatibleRelaxationReport, MultiwayError};

/// Explicit thresholds for accepting one proposed coarse space.
///
/// MultiwayMG deliberately provides no automatic default. Thresholds are
/// performance policy and must be calibrated for a declared smoother, number
/// of sweeps, problem family, and hierarchy-level role.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompatibleRelaxationCriteria {
    /// Largest accepted worst-case diagonal-norm contraction factor per sweep.
    pub maximum_diagonal_factor_per_sweep: f64,
    /// Optional largest accepted worst-case energy contraction factor per
    /// sweep. When present, a report without meaningful energy measurements is
    /// rejected.
    pub maximum_energy_factor_per_sweep: Option<f64>,
    /// Largest accepted final coarse-orthogonality defect.
    pub maximum_final_coarse_defect: f64,
    /// Largest accepted final structural-shift defect.
    pub maximum_final_structural_defect: f64,
}

impl CompatibleRelaxationCriteria {
    fn validate(self) -> Result<Self, MultiwayError> {
        validate_positive_finite(
            "maximum_diagonal_factor_per_sweep",
            self.maximum_diagonal_factor_per_sweep,
        )?;
        if let Some(limit) = self.maximum_energy_factor_per_sweep {
            validate_positive_finite("maximum_energy_factor_per_sweep", limit)?;
        }
        validate_nonnegative_finite(
            "maximum_final_coarse_defect",
            self.maximum_final_coarse_defect,
        )?;
        validate_nonnegative_finite(
            "maximum_final_structural_defect",
            self.maximum_final_structural_defect,
        )?;
        Ok(self)
    }
}

/// One reason a compatible-relaxation report failed declared criteria.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CompatibleRelaxationRejection {
    /// Worst diagonal-norm contraction was too slow.
    DiagonalContraction {
        /// Observed per-sweep factor.
        observed: f64,
        /// Maximum accepted factor.
        limit: f64,
    },
    /// Worst energy contraction was too slow.
    EnergyContraction {
        /// Observed per-sweep factor.
        observed: f64,
        /// Maximum accepted factor.
        limit: f64,
    },
    /// Energy contraction was required but no test vector had a numerically
    /// meaningful initial energy.
    EnergyUnavailable,
    /// The final coarse-orthogonality defect exceeded its limit.
    CoarseDefect {
        /// Observed defect.
        observed: f64,
        /// Maximum accepted defect.
        limit: f64,
    },
    /// The final structural-shift defect exceeded its limit.
    StructuralDefect {
        /// Observed defect.
        observed: f64,
        /// Maximum accepted defect.
        limit: f64,
    },
}

/// Deterministic acceptance result for one report and one explicit policy.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleRelaxationDecision {
    maximum_diagonal_factor_per_sweep: f64,
    maximum_energy_factor_per_sweep: Option<f64>,
    rejections: Vec<CompatibleRelaxationRejection>,
}

impl CompatibleRelaxationDecision {
    /// Whether every declared criterion was satisfied.
    #[must_use]
    pub fn accepted(&self) -> bool {
        self.rejections.is_empty()
    }

    /// Observed worst diagonal contraction factor per sweep.
    #[must_use]
    pub const fn maximum_diagonal_factor_per_sweep(&self) -> f64 {
        self.maximum_diagonal_factor_per_sweep
    }

    /// Observed worst energy contraction factor per sweep when available.
    #[must_use]
    pub const fn maximum_energy_factor_per_sweep(&self) -> Option<f64> {
        self.maximum_energy_factor_per_sweep
    }

    /// All failed criteria in stable evaluation order.
    #[must_use]
    pub fn rejections(&self) -> &[CompatibleRelaxationRejection] {
        &self.rejections
    }
}

/// Evaluate one compatible-relaxation report against explicit criteria.
pub fn evaluate_compatible_relaxation(
    report: &CompatibleRelaxationReport,
    criteria: CompatibleRelaxationCriteria,
) -> Result<CompatibleRelaxationDecision, MultiwayError> {
    let criteria = criteria.validate()?;
    if report.sweeps() == 0 {
        return Err(MultiwayError::CompatibleRelaxation {
            message: "cannot evaluate a report with zero sweeps".to_owned(),
        });
    }
    let maximum_diagonal_factor_per_sweep =
        per_sweep_factor(report.maximum_diagonal_contraction(), report.sweeps())?;
    let maximum_energy_factor_per_sweep = report
        .maximum_energy_contraction()
        .map(|contraction| per_sweep_factor(contraction, report.sweeps()))
        .transpose()?;
    let mut rejections = Vec::new();

    if maximum_diagonal_factor_per_sweep > criteria.maximum_diagonal_factor_per_sweep {
        rejections.push(CompatibleRelaxationRejection::DiagonalContraction {
            observed: maximum_diagonal_factor_per_sweep,
            limit: criteria.maximum_diagonal_factor_per_sweep,
        });
    }
    if let Some(limit) = criteria.maximum_energy_factor_per_sweep {
        match maximum_energy_factor_per_sweep {
            Some(observed) if observed > limit => {
                rejections.push(CompatibleRelaxationRejection::EnergyContraction {
                    observed,
                    limit,
                });
            }
            None => rejections.push(CompatibleRelaxationRejection::EnergyUnavailable),
            Some(_) => {}
        }
    }
    if report.maximum_final_coarse_defect() > criteria.maximum_final_coarse_defect {
        rejections.push(CompatibleRelaxationRejection::CoarseDefect {
            observed: report.maximum_final_coarse_defect(),
            limit: criteria.maximum_final_coarse_defect,
        });
    }
    if report.maximum_final_structural_defect() > criteria.maximum_final_structural_defect {
        rejections.push(CompatibleRelaxationRejection::StructuralDefect {
            observed: report.maximum_final_structural_defect(),
            limit: criteria.maximum_final_structural_defect,
        });
    }

    Ok(CompatibleRelaxationDecision {
        maximum_diagonal_factor_per_sweep,
        maximum_energy_factor_per_sweep,
        rejections,
    })
}

fn per_sweep_factor(contraction: f64, sweeps: usize) -> Result<f64, MultiwayError> {
    if !contraction.is_finite() || contraction < 0.0 {
        return Err(MultiwayError::CompatibleRelaxation {
            message: format!("invalid total contraction {contraction}"),
        });
    }
    Ok(if contraction == 0.0 {
        0.0
    } else {
        (contraction.ln() / sweeps as f64).exp()
    })
}

fn validate_positive_finite(name: &'static str, value: f64) -> Result<(), MultiwayError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(MultiwayError::InvalidOption {
            name,
            message: format!("must be finite and positive, got {value}"),
        });
    }
    Ok(())
}

fn validate_nonnegative_finite(name: &'static str, value: f64) -> Result<(), MultiwayError> {
    if !value.is_finite() || value < 0.0 {
        return Err(MultiwayError::InvalidOption {
            name,
            message: format!("must be finite and nonnegative, got {value}"),
        });
    }
    Ok(())
}
