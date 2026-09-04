//! Frozen issue #3 v3 selective-cycle holdout.
//!
//! Seeds 900--909 and every threshold are declared in
//! `benchmarks/policies/issue3-cycle-portfolio-v3.tsv`. This executable must not
//! alter them in response to numerical results.

#[path = "support/issue2_fixtures.rs"]
mod issue2_fixtures;
#[path = "support/issue3_cycle_fixtures.rs"]
mod issue3_cycle_fixtures;

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
};

use issue2_fixtures::{DynError, deterministic_rhs};
use issue3_cycle_fixtures::{CycleHoldoutFixture, cycle_holdout_v3_fixtures};
use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, CyclePortfolioCandidateSource, CycleQualityCriteria,
    CycleQualityDecision, CycleQualityOptions, CycleQualityReport, CycleSmootherKind,
    CycleSmootherPortfolioOptions, CycleSmootherPortfolioResult, DenseRangeDecomposition,
    DiagonalAggregationProjector, DiagonalPreconditioner, FactorAggregation, PairCmgOptions,
    PairCmgPreconditioner, PairNeighborhoodAggregationOptions, PcgTraceOptions, Preconditioner,
    SelectedTwoGridCycle, SpectralAnalysisOptions, SymmetricMapPreconditioner,
    SymmetricTwoGridPreconditioner, ThreeWayProblem, analyze_cycle_quality,
    build_cycle_smoother_portfolio, build_pair_neighborhood_aggregation, evaluate_cycle_quality,
    solve_projected_pcg_traced,
};

const MAXIMUM_COARSE_DIMENSION_RATIO: f64 = 0.80;
const MINIMUM_TUPLE_REDUCTION: f64 = 0.05;
const MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY: f64 = 1.95;

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let matrix_path = output_directory.join("issue3-cycle-v3-holdout.tsv");
    let trace_path = output_directory.join("issue3-cycle-v3-traces.tsv");
    let timing_path = output_directory.join("issue3-cycle-v3-timing.tsv");
    let mut matrix = BufWriter::new(File::create(&matrix_path)?);
    let mut traces = BufWriter::new(File::create(&trace_path)?);
    let mut timings = BufWriter::new(File::create(&timing_path)?);

    writeln!(
        matrix,
        "set\tcase\tfamily\trequested_seed\tactual_seed\tstructural_skips\tdimension\ttuples\tcomponents\treference_map_accepted\treference_map_condition\treference_map_probe_factor\treference_pair_accepted\treference_pair_condition\treference_pair_probe_factor\treference_admissible\treference_preferred_smoother\tautomatic_accepted\tautomatic_smoother\tautomatic_source\tcoarse_dimension\tcoarse_tuples\tcoarse_dimension_ratio\ttuple_reduction\ttwo_level_tuple_complexity\tbaseline_condition\treference_same_smoother_condition\tcandidate_condition\tcycle_consistent_recovery\tone_shot_condition\trelative_improvement_vs_one_shot\texact_cycle_error_radius\tprobe_estimated_energy_factor\tprobe_accepted\tprobe_underestimate_vs_dense\tpcg_iterations\tpcg_converged\tpcg_final_relative_residual\tbootstrap_rounds\tbootstrap_witnesses\tsplit_repair_splits\tcandidate_maps_considered\tcycle_builds_attempted\tportfolio_stop_reason"
    )?;
    writeln!(traces, "case\titeration\tresidual_norm\trelative_residual")?;
    writeln!(
        timings,
        "case\tmap_pass_ms\tpair_smoother_setup_ms\tpair_pass_ms\ttotal_portfolio_ms"
    )?;

    for fixture in cycle_holdout_v3_fixtures()? {
        run_fixture(&fixture, &mut matrix, &mut traces, &mut timings)?;
    }
    println!("wrote {}", matrix_path.display());
    println!("wrote {}", trace_path.display());
    println!("wrote {}", timing_path.display());
    Ok(())
}

fn run_fixture(
    fixture: &CycleHoldoutFixture,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
    timings: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let problem = &fixture.problem;
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(problem, spectral_options)?;
    let map_smoother = SymmetricMapPreconditioner::new(problem.clone());
    let pair_smoother = PairCmgPreconditioner::build(problem.clone(), PairCmgOptions::default())?;
    let map_baseline_condition = range
        .analyze(&map_smoother, spectral_options)?
        .preconditioned_condition_number();
    let pair_baseline_condition = range
        .analyze(&pair_smoother, spectral_options)?
        .preconditioned_condition_number();

    let reference_map_cycle = ResearchCycle::map(problem, &fixture.oracle, &map_smoother)?;
    let reference_pair_cycle = ResearchCycle::pair(problem, &fixture.oracle, &pair_smoother)?;
    let reference_map = evaluate_cycle(problem, &range, spectral_options, &reference_map_cycle)?;
    let reference_pair = evaluate_cycle(problem, &range, spectral_options, &reference_pair_cycle)?;
    let reference_admissible = reference_map.accepted || reference_pair.accepted;
    let reference_preferred = if reference_map.accepted {
        "symmetric-map"
    } else if reference_pair.accepted {
        "all-pairs-cmg"
    } else {
        "none"
    };

    let primary_smoother = DiagonalPreconditioner::new(problem, 0.5)?;
    let (portfolio, timing) =
        build_cycle_smoother_portfolio(problem, &primary_smoother, portfolio_options())?;
    writeln!(
        timings,
        "{}\t{:.6}\t{:.6}\t{}\t{:.6}",
        fixture.name,
        duration_ms(timing.map_pass().total()),
        duration_ms(timing.pair_smoother_setup()),
        timing.pair_pass().map_or_else(
            || "NA".to_owned(),
            |pass| format!("{:.6}", duration_ms(pass.total()))
        ),
        duration_ms(timing.total()),
    )?;

    if !portfolio.accepted() {
        write_rejected(
            fixture,
            &portfolio,
            &reference_map,
            &reference_pair,
            reference_admissible,
            reference_preferred,
            matrix,
        )?;
        return Ok(());
    }

    let selected_smoother = portfolio
        .selected_smoother()
        .expect("accepted portfolio has a smoother");
    let selected_source = portfolio
        .selected_source()
        .expect("accepted portfolio has a source");
    let selected_evaluation = portfolio
        .selected_evaluation()
        .expect("accepted portfolio has an evaluation");
    let metrics = selected_evaluation.structural_metrics();
    let selected_cycle = portfolio
        .build_selected_cycle(problem)?
        .expect("accepted portfolio builds a fixed cycle");
    let spectral = range.analyze(&selected_cycle, spectral_options)?;
    let candidate_condition = spectral.preconditioned_condition_number();
    let exact_radius = spectral.unit_step_energy_spectral_radius();
    let probe_report = selected_evaluation
        .cycle_report()
        .expect("accepted evaluation has a cycle report");
    let probe_decision = selected_evaluation
        .cycle_decision()
        .expect("accepted evaluation has a cycle decision");
    let probe_factor = probe_report.maximum_estimated_energy_factor();
    let probe_underestimate = exact_radius - probe_factor;

    let (baseline_condition, reference_same_smoother_condition) = match selected_smoother {
        CycleSmootherKind::SymmetricMap => (map_baseline_condition, reference_map.condition),
        CycleSmootherKind::AllPairsCmg => (pair_baseline_condition, reference_pair.condition),
    };
    let recovery = reference_admissible
        .then(|| {
            recovery_fraction(
                baseline_condition,
                reference_same_smoother_condition,
                candidate_condition,
            )
        })
        .flatten();

    let one_shot = build_pair_neighborhood_aggregation(
        problem,
        PairNeighborhoodAggregationOptions {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: 12,
        },
    )?;
    let one_shot_condition = one_shot_condition(
        problem,
        &range,
        spectral_options,
        &one_shot,
        selected_smoother,
        &map_smoother,
        &pair_smoother,
    )?;
    let relative_improvement = one_shot_condition.map(|condition| {
        (condition - candidate_condition) / condition.abs().max(f64::MIN_POSITIVE)
    });

    let rhs = deterministic_rhs(problem)?;
    let pcg = solve_projected_pcg_traced(
        problem,
        &rhs,
        &selected_cycle,
        PcgTraceOptions {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 0.0,
            max_iterations: 2_000,
        },
    )?;
    for sample in pcg.samples() {
        writeln!(
            traces,
            "{}\t{}\t{:.12e}\t{:.12e}",
            fixture.name,
            sample.iteration(),
            sample.residual_norm(),
            sample.relative_residual(),
        )?;
    }

    let primary = selected_primary(&portfolio);
    let bootstrap_rounds = primary.rounds().len();
    let bootstrap_witnesses = primary
        .rounds()
        .iter()
        .map(|round| round.bootstrap_witnesses())
        .max()
        .unwrap_or(0);
    let split_repair_splits = primary
        .split_repair()
        .map_or(0, |repair| repair.accepted_splits());
    let selected_pass = selected_pass(&portfolio);
    let work = selected_pass.work_report();

    writeln!(
        matrix,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.12e}\t{:.12e}\t{}\t{:.12e}\t{:.12e}\t{}\t{}\ttrue\t{}\t{}\t{}\t{}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{}\t{}\t{}\t{:.12e}\t{:.12e}\t{}\t{:.12e}\t{}\t{}\t{:.12e}\t{}\t{}\t{}\t{}\t{}\t{}",
        fixture.set,
        fixture.name,
        fixture.family,
        fixture.requested_seed,
        fixture.actual_seed,
        fixture.structural_skips,
        problem.dimension(),
        problem.tuple_count(),
        problem.components().count(),
        reference_map.accepted,
        reference_map.condition,
        reference_map.probe_factor,
        reference_pair.accepted,
        reference_pair.condition,
        reference_pair.probe_factor,
        reference_admissible,
        reference_preferred,
        smoother_label(selected_smoother),
        source_label(selected_source),
        metrics.coarse_dimension(),
        metrics.coarse_tuple_count(),
        metrics.coarse_dimension_ratio(),
        metrics.tuple_reduction(),
        metrics.two_level_tuple_complexity(),
        baseline_condition,
        reference_same_smoother_condition,
        candidate_condition,
        optional(recovery),
        optional(one_shot_condition),
        optional(relative_improvement),
        exact_radius,
        probe_factor,
        probe_decision.accepted(),
        probe_underestimate,
        pcg.iterations(),
        pcg.converged(),
        pcg.final_relative_residual(),
        bootstrap_rounds,
        bootstrap_witnesses,
        split_repair_splits,
        work.candidate_maps_considered(),
        work.cycle_builds_attempted(),
        stop_reason_label(portfolio.stop_reason()),
    )?;
    Ok(())
}

struct CycleEvaluation {
    accepted: bool,
    condition: f64,
    probe_factor: f64,
}

fn evaluate_cycle(
    problem: &ThreeWayProblem,
    range: &DenseRangeDecomposition,
    spectral_options: SpectralAnalysisOptions,
    cycle: &ResearchCycle,
) -> Result<CycleEvaluation, DynError> {
    let spectral = range.analyze(cycle, spectral_options)?;
    let report = analyze_cycle_quality(problem, cycle, cycle_probe_options())?;
    let decision = evaluate_cycle_quality(&report, cycle_probe_criteria())?;
    Ok(CycleEvaluation {
        accepted: decision.accepted(),
        condition: spectral.preconditioned_condition_number(),
        probe_factor: report.maximum_estimated_energy_factor(),
    })
}

#[derive(Debug, Clone)]
enum ResearchCycle {
    SymmetricMap(Box<SymmetricTwoGridPreconditioner<SymmetricMapPreconditioner>>),
    PairCmg(Box<SymmetricTwoGridPreconditioner<PairCmgPreconditioner>>),
}

impl ResearchCycle {
    fn map(
        problem: &ThreeWayProblem,
        aggregation: &FactorAggregation,
        smoother: &SymmetricMapPreconditioner,
    ) -> Result<Self, multiway_mg::MultiwayError> {
        SymmetricTwoGridPreconditioner::build(
            problem.clone(),
            aggregation.clone(),
            smoother.clone(),
            1,
            1.0,
            1.0e-12,
        )
        .map(Box::new)
        .map(Self::SymmetricMap)
    }

    fn pair(
        problem: &ThreeWayProblem,
        aggregation: &FactorAggregation,
        smoother: &PairCmgPreconditioner,
    ) -> Result<Self, multiway_mg::MultiwayError> {
        SymmetricTwoGridPreconditioner::build(
            problem.clone(),
            aggregation.clone(),
            smoother.clone(),
            1,
            1.0,
            1.0e-12,
        )
        .map(Box::new)
        .map(Self::PairCmg)
    }
}

impl Preconditioner for ResearchCycle {
    fn dimension(&self) -> usize {
        match self {
            Self::SymmetricMap(cycle) => cycle.dimension(),
            Self::PairCmg(cycle) => cycle.dimension(),
        }
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), multiway_mg::MultiwayError> {
        match self {
            Self::SymmetricMap(cycle) => cycle.apply(rhs, out),
            Self::PairCmg(cycle) => cycle.apply(rhs, out),
        }
    }
}

fn one_shot_condition(
    problem: &ThreeWayProblem,
    range: &DenseRangeDecomposition,
    spectral_options: SpectralAnalysisOptions,
    aggregation: &FactorAggregation,
    smoother: CycleSmootherKind,
    map_smoother: &SymmetricMapPreconditioner,
    pair_smoother: &PairCmgPreconditioner,
) -> Result<Option<f64>, DynError> {
    if !structurally_admissible(problem, aggregation)? {
        return Ok(None);
    }
    let cycle = match smoother {
        CycleSmootherKind::SymmetricMap => ResearchCycle::map(problem, aggregation, map_smoother)?,
        CycleSmootherKind::AllPairsCmg => ResearchCycle::pair(problem, aggregation, pair_smoother)?,
    };
    Ok(Some(
        range
            .analyze(&cycle, spectral_options)?
            .preconditioned_condition_number(),
    ))
}

fn structurally_admissible(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<bool, multiway_mg::MultiwayError> {
    DiagonalAggregationProjector::new(problem.clone(), aggregation.clone())?;
    let coarse = aggregation.coarsen(problem)?;
    let dimension_ratio = coarse.dimension() as f64 / problem.dimension() as f64;
    let tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    Ok(coarse.dimension() < problem.dimension()
        && dimension_ratio <= MAXIMUM_COARSE_DIMENSION_RATIO
        && 1.0 - tuple_ratio >= MINIMUM_TUPLE_REDUCTION
        && 1.0 + tuple_ratio <= MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY)
}

fn write_rejected(
    fixture: &CycleHoldoutFixture,
    portfolio: &CycleSmootherPortfolioResult,
    reference_map: &CycleEvaluation,
    reference_pair: &CycleEvaluation,
    reference_admissible: bool,
    reference_preferred: &str,
    matrix: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let problem = &fixture.problem;
    let primary = selected_primary(portfolio);
    let bootstrap_rounds = primary.rounds().len();
    let bootstrap_witnesses = primary
        .rounds()
        .iter()
        .map(|round| round.bootstrap_witnesses())
        .max()
        .unwrap_or(0);
    let split_repair_splits = primary
        .split_repair()
        .map_or(0, |repair| repair.accepted_splits());
    let work = portfolio.map_pass().work_report();
    writeln!(
        matrix,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.12e}\t{:.12e}\t{}\t{:.12e}\t{:.12e}\t{}\t{}\tfalse\tnone\tnone\tNA\tNA\tNA\tNA\tNA\tNA\tNA\tNA\tNA\tNA\tNA\tNA\tNA\tfalse\tNA\tNA\tfalse\tNA\t{}\t{}\t{}\t{}\t{}\t{}",
        fixture.set,
        fixture.name,
        fixture.family,
        fixture.requested_seed,
        fixture.actual_seed,
        fixture.structural_skips,
        problem.dimension(),
        problem.tuple_count(),
        problem.components().count(),
        reference_map.accepted,
        reference_map.condition,
        reference_map.probe_factor,
        reference_pair.accepted,
        reference_pair.condition,
        reference_pair.probe_factor,
        reference_admissible,
        reference_preferred,
        bootstrap_rounds,
        bootstrap_witnesses,
        split_repair_splits,
        work.candidate_maps_considered(),
        work.cycle_builds_attempted(),
        stop_reason_label(portfolio.stop_reason()),
    )?;
    Ok(())
}

fn selected_pass(
    portfolio: &CycleSmootherPortfolioResult,
) -> &multiway_mg::CycleScreenedBootstrapResult {
    match portfolio.selected_smoother() {
        Some(CycleSmootherKind::SymmetricMap) | None => portfolio.map_pass(),
        Some(CycleSmootherKind::AllPairsCmg) => portfolio
            .pair_pass()
            .expect("pair smoother selection has a pair pass"),
    }
}

fn selected_primary(
    portfolio: &CycleSmootherPortfolioResult,
) -> &multiway_mg::BootstrapAggregationResult {
    selected_pass(portfolio).primary_result()
}

fn recovery_fraction(baseline: f64, reference: f64, candidate: f64) -> Option<f64> {
    let denominator = baseline - reference;
    (denominator > 1.0e-12 * baseline.abs().max(1.0))
        .then_some((baseline - candidate) / denominator)
}

fn smoother_label(smoother: CycleSmootherKind) -> &'static str {
    match smoother {
        CycleSmootherKind::SymmetricMap => "symmetric-map",
        CycleSmootherKind::AllPairsCmg => "all-pairs-cmg",
    }
}

fn source_label(source: CyclePortfolioCandidateSource) -> &'static str {
    match source {
        CyclePortfolioCandidateSource::BootstrapFinal => "bootstrap-final",
        CyclePortfolioCandidateSource::StructuralBaseline => "structural-baseline",
        _ => "unknown",
    }
}

fn stop_reason_label(reason: multiway_mg::CycleSmootherPortfolioStopReason) -> &'static str {
    match reason {
        multiway_mg::CycleSmootherPortfolioStopReason::AcceptedSymmetricMap => {
            "accepted-symmetric-map"
        }
        multiway_mg::CycleSmootherPortfolioStopReason::AcceptedAllPairsCmg => {
            "accepted-all-pairs-cmg"
        }
        multiway_mg::CycleSmootherPortfolioStopReason::NoAcceptedCycle => "no-accepted-cycle",
    }
}

fn duration_ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn optional(value: Option<f64>) -> String {
    value.map_or_else(|| "NA".to_owned(), |number| format!("{number:.12e}"))
}

fn portfolio_options() -> CycleSmootherPortfolioOptions {
    CycleSmootherPortfolioOptions {
        bootstrap: BootstrapAggregationOptions {
            setup_test_vectors: 5,
            setup_sweeps: 5,
            setup_jacobi_omega: 0.5,
            maximum_neighbor_degree: 12,
            signature_window: 3,
            maximum_candidate_degree: 12,
            minimum_combined_affinity: 0.40,
            algebraic_affinity_weight: 0.75,
            structural_affinity_weight: 0.05,
            degree_affinity_weight: 0.10,
            signature_hit_weight: 0.10,
            structural_baseline_required_factor_ratio: 0.90,
            structural_baseline_maximum_dimension_overhead_ratio: 0.05,
            structural_baseline_maximum_tuple_overhead_ratio: 0.05,
            compatible_relaxation: compatible_options(),
            compatible_criteria: compatible_criteria(),
            maximum_bootstrap_witnesses: 6,
            maximum_coarse_dimension_ratio: MAXIMUM_COARSE_DIMENSION_RATIO,
            minimum_tuple_reduction: MINIMUM_TUPLE_REDUCTION,
            maximum_two_level_tuple_complexity: MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
            split_repair: Some(AggregationRepairOptions {
                relaxation: compatible_options(),
                criteria: compatible_criteria(),
                maximum_rounds: 18,
                maximum_coarse_dimension_ratio: MAXIMUM_COARSE_DIMENSION_RATIO,
                minimum_tuple_reduction: MINIMUM_TUPLE_REDUCTION,
                maximum_two_level_tuple_complexity: MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
                minimum_split_score_fraction: 0.001,
            }),
            seed: 0x4d57_4d47_434f_5645,
        },
        probe: cycle_probe_options(),
        criteria: cycle_probe_criteria(),
        smoothing_steps: 1,
        smoother_damping: 1.0,
        terminal_relative_tolerance: 1.0e-12,
        pair_cmg: PairCmgOptions::default(),
    }
}

fn compatible_options() -> CompatibleRelaxationOptions {
    CompatibleRelaxationOptions {
        test_vectors: 16,
        sweeps: 12,
        relaxation_damping: 1.0,
        seed: 0x4d57_4d47_4352_3031,
        relative_zero_tolerance: 1.0e-13,
    }
}

fn compatible_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.85,
        maximum_energy_factor_per_sweep: Some(0.85),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    }
}

fn cycle_probe_options() -> CycleQualityOptions {
    CycleQualityOptions {
        test_vectors: 12,
        power_iterations: 24,
        tail_iterations: 6,
        correction_damping: 1.0,
        seed: 0x4d57_4d47_4359_4331,
        relative_zero_tolerance: 1.0e-13,
    }
}

fn cycle_probe_criteria() -> CycleQualityCriteria {
    CycleQualityCriteria {
        maximum_estimated_energy_factor: 0.50,
        maximum_observed_energy_factor: Some(1.05),
        maximum_structural_defect: 1.0e-10,
    }
}
