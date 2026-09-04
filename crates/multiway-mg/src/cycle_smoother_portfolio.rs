//! Selective complete-cycle portfolio across symmetric MAP and pair-CMG.
//!
//! Automatic map construction and hard structural gates are identical across
//! the two smoother tiers. The cheaper symmetric-MAP cycle is screened first.
//! All-pair fixed CMG is constructed and screened only when no MAP candidate
//! passes the declared complete-cycle criteria. When neither tier succeeds the
//! result is an explicit rejection rather than an identity or diagonal fallback.

use std::time::{Duration, Instant};

use crate::{
    BootstrapAggregationOptions, CyclePortfolioBuildTiming, CyclePortfolioCandidateSource,
    CyclePortfolioEvaluation, CycleQualityCriteria, CycleQualityOptions,
    CycleScreenedBootstrapResult, MultiwayError, PairCmgOptions, PairCmgPreconditioner,
    Preconditioner, SymmetricMapPreconditioner, SymmetricTwoGridPreconditioner, ThreeWayProblem,
    build_cycle_screened_bootstrap_aggregation_with_timing,
};

/// Complete-cycle smoother selected by the deterministic portfolio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleSmootherKind {
    /// One fixed symmetric-MAP sweep before and after coarse correction.
    SymmetricMap,
    /// One fixed all-pair CMG correction before and after coarse correction.
    AllPairsCmg,
}

/// Fixed options for MAP-first, pair-CMG-fallback cycle screening.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CycleSmootherPortfolioOptions {
    /// Automatic aggregation, witness, repair, and hard structural policy.
    pub bootstrap: BootstrapAggregationOptions,
    /// Matrix-free complete-cycle probe.
    pub probe: CycleQualityOptions,
    /// Explicit complete-cycle acceptance criteria.
    pub criteria: CycleQualityCriteria,
    /// Number of identical pre- and post-smoothing applications.
    pub smoothing_steps: usize,
    /// Scalar multiplying every smoother correction.
    pub smoother_damping: f64,
    /// Relative rank threshold for the exact coarse pseudoinverse.
    pub terminal_relative_tolerance: f64,
    /// CMG options used only after every MAP candidate has failed.
    pub pair_cmg: PairCmgOptions,
}

impl CycleSmootherPortfolioOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        if self.smoothing_steps == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "cycle_smoother_portfolio_smoothing_steps",
                message: "must be positive".to_owned(),
            });
        }
        if !self.smoother_damping.is_finite() || self.smoother_damping <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "cycle_smoother_portfolio_smoother_damping",
                message: format!("must be finite and positive, got {}", self.smoother_damping),
            });
        }
        if !self.terminal_relative_tolerance.is_finite() || self.terminal_relative_tolerance <= 0.0
        {
            return Err(MultiwayError::InvalidOption {
                name: "cycle_smoother_portfolio_terminal_relative_tolerance",
                message: format!(
                    "must be finite and positive, got {}",
                    self.terminal_relative_tolerance
                ),
            });
        }
        Ok(self)
    }
}

/// Why the smoother portfolio stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleSmootherPortfolioStopReason {
    /// At least one MAP cycle passed and the best MAP candidate was selected.
    AcceptedSymmetricMap,
    /// Every MAP candidate failed, but at least one pair-CMG cycle passed.
    AcceptedAllPairsCmg,
    /// Neither smoother tier produced an admissible complete cycle.
    NoAcceptedCycle,
}

/// Phase-separated descriptive timing for the selective portfolio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleSmootherPortfolioBuildTiming {
    map_pass: CyclePortfolioBuildTiming,
    pair_smoother_setup: Duration,
    pair_pass: Option<CyclePortfolioBuildTiming>,
    total: Duration,
}

impl CycleSmootherPortfolioBuildTiming {
    /// MAP cycle-screening pass, including its automatic aggregation build.
    #[must_use]
    pub const fn map_pass(self) -> CyclePortfolioBuildTiming {
        self.map_pass
    }

    /// One-time setup of the all-pair CMG smoother, zero when MAP succeeded.
    #[must_use]
    pub const fn pair_smoother_setup(self) -> Duration {
        self.pair_smoother_setup
    }

    /// Pair-CMG cycle-screening pass, absent when MAP succeeded.
    #[must_use]
    pub const fn pair_pass(self) -> Option<CyclePortfolioBuildTiming> {
        self.pair_pass
    }

    /// Total portfolio construction and screening time.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

/// MAP-first, pair-CMG-fallback automatic aggregation result.
///
/// The first implementation deliberately reuses the already validated generic
/// cycle portfolio twice. This rebuilds the deterministic bootstrap candidate
/// set when the pair fallback is needed; the duplication is exposed in timing
/// and retained reports rather than hidden. A later prepared-topology refactor
/// can share candidate construction without changing the scientific policy.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleSmootherPortfolioResult {
    map_pass: CycleScreenedBootstrapResult,
    pair_pass: Option<CycleScreenedBootstrapResult>,
    selected_smoother: Option<CycleSmootherKind>,
    stop_reason: CycleSmootherPortfolioStopReason,
    options: CycleSmootherPortfolioOptions,
}

impl CycleSmootherPortfolioResult {
    /// Complete MAP-tier result and candidate diagnostics.
    #[must_use]
    pub const fn map_pass(&self) -> &CycleScreenedBootstrapResult {
        &self.map_pass
    }

    /// Pair-CMG fallback result, present only when no MAP candidate passed.
    #[must_use]
    pub const fn pair_pass(&self) -> Option<&CycleScreenedBootstrapResult> {
        self.pair_pass.as_ref()
    }

    /// Whether either complete-cycle tier accepted a candidate.
    #[must_use]
    pub const fn accepted(&self) -> bool {
        self.selected_smoother.is_some()
    }

    /// Selected cycle smoother, absent after a declared rejection.
    #[must_use]
    pub const fn selected_smoother(&self) -> Option<CycleSmootherKind> {
        self.selected_smoother
    }

    /// Deterministic stop reason.
    #[must_use]
    pub const fn stop_reason(&self) -> CycleSmootherPortfolioStopReason {
        self.stop_reason
    }

    /// Selected aggregation, or the rejected MAP primary map when no tier passed.
    #[must_use]
    pub fn final_aggregation(&self) -> &crate::FactorAggregation {
        match self.selected_smoother {
            Some(CycleSmootherKind::SymmetricMap) => self.map_pass.final_aggregation(),
            Some(CycleSmootherKind::AllPairsCmg) => self
                .pair_pass
                .as_ref()
                .expect("pair smoother selection has a pair pass")
                .final_aggregation(),
            None => self.map_pass.final_aggregation(),
        }
    }

    /// Candidate-map source selected inside the accepted smoother tier.
    #[must_use]
    pub fn selected_source(&self) -> Option<CyclePortfolioCandidateSource> {
        match self.selected_smoother {
            Some(CycleSmootherKind::SymmetricMap) => self.map_pass.selected_source(),
            Some(CycleSmootherKind::AllPairsCmg) => self
                .pair_pass
                .as_ref()
                .and_then(CycleScreenedBootstrapResult::selected_source),
            None => None,
        }
    }

    /// Selected complete-cycle evaluation.
    #[must_use]
    pub fn selected_evaluation(&self) -> Option<&CyclePortfolioEvaluation> {
        match self.selected_smoother {
            Some(CycleSmootherKind::SymmetricMap) => self.map_pass.selected_evaluation(),
            Some(CycleSmootherKind::AllPairsCmg) => self
                .pair_pass
                .as_ref()
                .and_then(CycleScreenedBootstrapResult::selected_evaluation),
            None => None,
        }
    }

    /// Build the fixed selected two-grid cycle for use in a solve.
    ///
    /// Pair-CMG numerical state is rebuilt here because the screening cycle is
    /// intentionally not retained in the diagnostic result. Issue #5 will add
    /// prepared numerical state and reusable workspaces.
    pub fn build_selected_cycle(
        &self,
        problem: &ThreeWayProblem,
    ) -> Result<Option<SelectedTwoGridCycle>, MultiwayError> {
        match self.selected_smoother {
            Some(CycleSmootherKind::SymmetricMap) => SymmetricTwoGridPreconditioner::build(
                problem.clone(),
                self.final_aggregation().clone(),
                SymmetricMapPreconditioner::new(problem.clone()),
                self.options.smoothing_steps,
                self.options.smoother_damping,
                self.options.terminal_relative_tolerance,
            )
            .map(Box::new)
            .map(SelectedTwoGridCycle::SymmetricMap)
            .map(Some),
            Some(CycleSmootherKind::AllPairsCmg) => {
                let smoother =
                    PairCmgPreconditioner::build(problem.clone(), self.options.pair_cmg)?;
                SymmetricTwoGridPreconditioner::build(
                    problem.clone(),
                    self.final_aggregation().clone(),
                    smoother,
                    self.options.smoothing_steps,
                    self.options.smoother_damping,
                    self.options.terminal_relative_tolerance,
                )
                .map(Box::new)
                .map(SelectedTwoGridCycle::AllPairsCmg)
                .map(Some)
            }
            None => Ok(None),
        }
    }
}

/// Fixed two-grid cycle selected by [`CycleSmootherPortfolioResult`].
#[derive(Debug, Clone)]
pub enum SelectedTwoGridCycle {
    /// Symmetric-MAP-smoothed two-grid cycle.
    SymmetricMap(Box<SymmetricTwoGridPreconditioner<SymmetricMapPreconditioner>>),
    /// All-pair-CMG-smoothed two-grid cycle.
    AllPairsCmg(Box<SymmetricTwoGridPreconditioner<PairCmgPreconditioner>>),
}

impl Preconditioner for SelectedTwoGridCycle {
    fn dimension(&self) -> usize {
        match self {
            Self::SymmetricMap(cycle) => cycle.dimension(),
            Self::AllPairsCmg(cycle) => cycle.dimension(),
        }
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        match self {
            Self::SymmetricMap(cycle) => cycle.apply(rhs, out),
            Self::AllPairsCmg(cycle) => cycle.apply(rhs, out),
        }
    }
}

/// Build and screen a MAP-first, pair-CMG-fallback automatic hierarchy level.
pub fn build_cycle_smoother_portfolio(
    problem: &ThreeWayProblem,
    primary_smoother: &dyn Preconditioner,
    options: CycleSmootherPortfolioOptions,
) -> Result<
    (
        CycleSmootherPortfolioResult,
        CycleSmootherPortfolioBuildTiming,
    ),
    MultiwayError,
> {
    let options = options.validate()?;
    let total_start = Instant::now();
    let map_smoother = SymmetricMapPreconditioner::new(problem.clone());
    let (map_pass, map_timing) = build_cycle_screened_bootstrap_aggregation_with_timing(
        problem,
        primary_smoother,
        options.bootstrap,
        options.probe,
        options.criteria,
        |aggregation| {
            SymmetricTwoGridPreconditioner::build(
                problem.clone(),
                aggregation.clone(),
                map_smoother.clone(),
                options.smoothing_steps,
                options.smoother_damping,
                options.terminal_relative_tolerance,
            )
        },
    )?;
    if map_pass.accepted() {
        return Ok((
            CycleSmootherPortfolioResult {
                map_pass,
                pair_pass: None,
                selected_smoother: Some(CycleSmootherKind::SymmetricMap),
                stop_reason: CycleSmootherPortfolioStopReason::AcceptedSymmetricMap,
                options,
            },
            CycleSmootherPortfolioBuildTiming {
                map_pass: map_timing,
                pair_smoother_setup: Duration::ZERO,
                pair_pass: None,
                total: total_start.elapsed(),
            },
        ));
    }

    let pair_setup_start = Instant::now();
    let pair_smoother = PairCmgPreconditioner::build(problem.clone(), options.pair_cmg)?;
    let pair_smoother_setup = pair_setup_start.elapsed();
    let (pair_pass, pair_timing) = build_cycle_screened_bootstrap_aggregation_with_timing(
        problem,
        primary_smoother,
        options.bootstrap,
        options.probe,
        options.criteria,
        |aggregation| {
            SymmetricTwoGridPreconditioner::build(
                problem.clone(),
                aggregation.clone(),
                pair_smoother.clone(),
                options.smoothing_steps,
                options.smoother_damping,
                options.terminal_relative_tolerance,
            )
        },
    )?;
    verify_identical_candidate_maps(&map_pass, &pair_pass)?;
    let pair_accepted = pair_pass.accepted();
    Ok((
        CycleSmootherPortfolioResult {
            map_pass,
            pair_pass: Some(pair_pass),
            selected_smoother: pair_accepted.then_some(CycleSmootherKind::AllPairsCmg),
            stop_reason: if pair_accepted {
                CycleSmootherPortfolioStopReason::AcceptedAllPairsCmg
            } else {
                CycleSmootherPortfolioStopReason::NoAcceptedCycle
            },
            options,
        },
        CycleSmootherPortfolioBuildTiming {
            map_pass: map_timing,
            pair_smoother_setup,
            pair_pass: Some(pair_timing),
            total: total_start.elapsed(),
        },
    ))
}

fn verify_identical_candidate_maps(
    map_pass: &CycleScreenedBootstrapResult,
    pair_pass: &CycleScreenedBootstrapResult,
) -> Result<(), MultiwayError> {
    let map_evaluations = map_pass.evaluations();
    let pair_evaluations = pair_pass.evaluations();
    if map_evaluations.len() != pair_evaluations.len()
        || map_evaluations
            .iter()
            .zip(pair_evaluations)
            .any(|(left, right)| {
                left.source() != right.source() || left.aggregation() != right.aggregation()
            })
    {
        return Err(MultiwayError::CycleQuality {
            message: "MAP and pair-CMG passes produced different deterministic map portfolios"
                .to_owned(),
        });
    }
    Ok(())
}
