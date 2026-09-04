//! Development matrix for a predeclared MAP-to-pair-CMG cycle fallback.
//!
//! This executable intentionally reuses the already observed issue #3 v2 seeds
//! 700--709. Its output is training evidence only. It must not be interpreted as
//! a new holdout or used to change the frozen v2 verdict.

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
use issue3_cycle_fixtures::{CycleHoldoutFixture, cycle_holdout_fixtures};
use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, CycleQualityCriteria, CycleQualityOptions,
    DenseRangeDecomposition, DiagonalAggregationProjector, DiagonalPreconditioner,
    FactorAggregation, PairCmgOptions, PairCmgPreconditioner, PairNeighborhoodAggregationOptions,
    PcgTraceOptions, Preconditioner, SpectralAnalysisOptions, SymmetricMapPreconditioner,
    SymmetricTwoGridPreconditioner, ThreeWayProblem, analyze_cycle_quality,
    build_cycle_screened_bootstrap_aggregation, build_pair_neighborhood_aggregation,
    evaluate_cycle_quality, solve_projected_pcg_traced,
};

const MAXIMUM_COARSE_DIMENSION_RATIO: f64 = 0.80;
const MINIMUM_TUPLE_REDUCTION: f64 = 0.05;
const MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY: f64 = 1.95;

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let path = output_directory.join("issue3-smoother-fallback-development.tsv");
    let mut output = BufWriter::new(File::create(&path)?);
    writeln!(
        output,
        "case\tfamily\tmap_source\tv2_map_accepted\tsmoother\tstructural_admissible\tcoarse_dimension\tcoarse_tuples\ttwo_level_tuple_complexity\tbaseline_condition\toracle_cycle_condition\tcandidate_condition\toracle_improvement_recovered\texact_cycle_error_radius\tprobe_estimated_energy_factor\tcycle_probe_accepted\tpcg_iterations\tpcg_converged\tpcg_final_relative_residual\tstop_reason"
    )?;

    for fixture in cycle_holdout_fixtures()? {
        run_fixture(&fixture, &mut output)?;
    }
    println!("wrote {}", path.display());
    Ok(())
}

fn run_fixture(
    fixture: &CycleHoldoutFixture,
    output: &mut BufWriter<File>,
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

    let primary_smoother = DiagonalPreconditioner::new(problem, 0.5)?;
    let v2_portfolio = build_cycle_screened_bootstrap_aggregation(
        problem,
        &primary_smoother,
        bootstrap_options(),
        cycle_probe_options(),
        cycle_probe_criteria(),
        |aggregation| {
            SymmetricTwoGridPreconditioner::build(
                problem.clone(),
                aggregation.clone(),
                map_smoother.clone(),
                1,
                1.0,
                1.0e-12,
            )
        },
    )?;
    let one_shot = build_pair_neighborhood_aggregation(
        problem,
        PairNeighborhoodAggregationOptions {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: 12,
        },
    )?;
    let maps = [
        CandidateMap {
            source: "oracle",
            aggregation: fixture.oracle.clone(),
            v2_accepted: true,
        },
        CandidateMap {
            source: "one-shot-pair-neighborhood",
            aggregation: one_shot,
            v2_accepted: false,
        },
        CandidateMap {
            source: "primary-bootstrap-final",
            aggregation: v2_portfolio.primary_result().final_aggregation().clone(),
            v2_accepted: v2_portfolio.primary_result().accepted(),
        },
        CandidateMap {
            source: "v2-cycle-portfolio-final",
            aggregation: v2_portfolio.final_aggregation().clone(),
            v2_accepted: v2_portfolio.accepted(),
        },
    ];

    for smoother in [SmootherKind::SymmetricMap, SmootherKind::PairCmg] {
        let baseline_condition = match smoother {
            SmootherKind::SymmetricMap => map_baseline_condition,
            SmootherKind::PairCmg => pair_baseline_condition,
        };
        let oracle_cycle = build_cycle(
            problem,
            &fixture.oracle,
            smoother,
            &map_smoother,
            &pair_smoother,
        )?;
        let oracle_cycle_condition = range
            .analyze(&oracle_cycle, spectral_options)?
            .preconditioned_condition_number();

        for candidate in &maps {
            evaluate_candidate(
                fixture,
                candidate,
                smoother,
                baseline_condition,
                oracle_cycle_condition,
                &range,
                spectral_options,
                &map_smoother,
                &pair_smoother,
                output,
            )?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct CandidateMap {
    source: &'static str,
    aggregation: FactorAggregation,
    v2_accepted: bool,
}

#[derive(Debug, Clone, Copy)]
enum SmootherKind {
    SymmetricMap,
    PairCmg,
}

impl SmootherKind {
    const fn label(self) -> &'static str {
        match self {
            Self::SymmetricMap => "symmetric-map",
            Self::PairCmg => "all-pairs-cmg",
        }
    }
}

#[derive(Debug, Clone)]
enum DevelopmentCycle {
    SymmetricMap(Box<SymmetricTwoGridPreconditioner<SymmetricMapPreconditioner>>),
    PairCmg(Box<SymmetricTwoGridPreconditioner<PairCmgPreconditioner>>),
}

impl Preconditioner for DevelopmentCycle {
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

fn build_cycle(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    smoother: SmootherKind,
    map_smoother: &SymmetricMapPreconditioner,
    pair_smoother: &PairCmgPreconditioner,
) -> Result<DevelopmentCycle, multiway_mg::MultiwayError> {
    match smoother {
        SmootherKind::SymmetricMap => SymmetricTwoGridPreconditioner::build(
            problem.clone(),
            aggregation.clone(),
            map_smoother.clone(),
            1,
            1.0,
            1.0e-12,
        )
        .map(Box::new)
        .map(DevelopmentCycle::SymmetricMap),
        SmootherKind::PairCmg => SymmetricTwoGridPreconditioner::build(
            problem.clone(),
            aggregation.clone(),
            pair_smoother.clone(),
            1,
            1.0,
            1.0e-12,
        )
        .map(Box::new)
        .map(DevelopmentCycle::PairCmg),
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_candidate(
    fixture: &CycleHoldoutFixture,
    candidate: &CandidateMap,
    smoother: SmootherKind,
    baseline_condition: f64,
    oracle_cycle_condition: f64,
    range: &DenseRangeDecomposition,
    spectral_options: SpectralAnalysisOptions,
    map_smoother: &SymmetricMapPreconditioner,
    pair_smoother: &PairCmgPreconditioner,
    output: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let problem = &fixture.problem;
    let metrics = structural_metrics(problem, &candidate.aggregation);
    let Ok(metrics) = metrics else {
        write_rejected_row(
            fixture,
            candidate,
            smoother,
            baseline_condition,
            oracle_cycle_condition,
            "component-preservation-failure",
            output,
        )?;
        return Ok(());
    };
    if !metrics.admissible {
        write_structural_row(
            fixture,
            candidate,
            smoother,
            metrics,
            baseline_condition,
            oracle_cycle_condition,
            "hard-structural-gate",
            output,
        )?;
        return Ok(());
    }

    let cycle = build_cycle(
        problem,
        &candidate.aggregation,
        smoother,
        map_smoother,
        pair_smoother,
    )?;
    let spectral = range.analyze(&cycle, spectral_options)?;
    let candidate_condition = spectral.preconditioned_condition_number();
    let exact_radius = spectral
        .preconditioned_eigenvalues()
        .iter()
        .map(|&eigenvalue| (1.0 - eigenvalue).abs())
        .fold(0.0, f64::max);
    let probe = analyze_cycle_quality(problem, &cycle, cycle_probe_options())?;
    let decision = evaluate_cycle_quality(&probe, cycle_probe_criteria())?;
    let rhs = deterministic_rhs(problem)?;
    let pcg = solve_projected_pcg_traced(
        problem,
        &rhs,
        &cycle,
        PcgTraceOptions {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 0.0,
            max_iterations: 2_000,
        },
    )?;
    let recovery = recovery_fraction(
        baseline_condition,
        oracle_cycle_condition,
        candidate_condition,
    );
    writeln!(
        output,
        "{}\t{}\t{}\t{}\t{}\ttrue\t{}\t{}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{}\t{:.12e}\t{:.12e}\t{}\t{}\t{}\t{:.12e}\t{}",
        fixture.name,
        fixture.family,
        candidate.source,
        candidate.v2_accepted,
        smoother.label(),
        metrics.coarse_dimension,
        metrics.coarse_tuples,
        metrics.two_level_tuple_complexity,
        baseline_condition,
        oracle_cycle_condition,
        candidate_condition,
        optional(recovery),
        exact_radius,
        probe.maximum_estimated_energy_factor(),
        decision.accepted(),
        pcg.iterations(),
        pcg.converged(),
        pcg.final_relative_residual(),
        if decision.accepted() {
            "accepted"
        } else {
            "cycle-rejected"
        },
    )?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct StructuralMetrics {
    admissible: bool,
    coarse_dimension: usize,
    coarse_tuples: usize,
    two_level_tuple_complexity: f64,
}

fn structural_metrics(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<StructuralMetrics, multiway_mg::MultiwayError> {
    DiagonalAggregationProjector::new(problem.clone(), aggregation.clone())?;
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension = coarse.dimension();
    let coarse_tuples = coarse.tuple_count();
    let dimension_ratio = coarse_dimension as f64 / problem.dimension() as f64;
    let tuple_ratio = coarse_tuples as f64 / problem.tuple_count() as f64;
    let tuple_reduction = 1.0 - tuple_ratio;
    let two_level_tuple_complexity = 1.0 + tuple_ratio;
    Ok(StructuralMetrics {
        admissible: coarse_dimension < problem.dimension()
            && dimension_ratio <= MAXIMUM_COARSE_DIMENSION_RATIO
            && tuple_reduction >= MINIMUM_TUPLE_REDUCTION
            && two_level_tuple_complexity <= MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
        coarse_dimension,
        coarse_tuples,
        two_level_tuple_complexity,
    })
}

#[allow(clippy::too_many_arguments)]
fn write_structural_row(
    fixture: &CycleHoldoutFixture,
    candidate: &CandidateMap,
    smoother: SmootherKind,
    metrics: StructuralMetrics,
    baseline_condition: f64,
    oracle_cycle_condition: f64,
    reason: &str,
    output: &mut BufWriter<File>,
) -> Result<(), DynError> {
    writeln!(
        output,
        "{}\t{}\t{}\t{}\t{}\tfalse\t{}\t{}\t{:.12e}\t{:.12e}\t{:.12e}\tNA\tNA\tNA\tNA\tfalse\tNA\tNA\tNA\t{}",
        fixture.name,
        fixture.family,
        candidate.source,
        candidate.v2_accepted,
        smoother.label(),
        metrics.coarse_dimension,
        metrics.coarse_tuples,
        metrics.two_level_tuple_complexity,
        baseline_condition,
        oracle_cycle_condition,
        reason,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_rejected_row(
    fixture: &CycleHoldoutFixture,
    candidate: &CandidateMap,
    smoother: SmootherKind,
    baseline_condition: f64,
    oracle_cycle_condition: f64,
    reason: &str,
    output: &mut BufWriter<File>,
) -> Result<(), DynError> {
    writeln!(
        output,
        "{}\t{}\t{}\t{}\t{}\tfalse\tNA\tNA\tNA\t{:.12e}\t{:.12e}\tNA\tNA\tNA\tNA\tfalse\tNA\tNA\tNA\t{}",
        fixture.name,
        fixture.family,
        candidate.source,
        candidate.v2_accepted,
        smoother.label(),
        baseline_condition,
        oracle_cycle_condition,
        reason,
    )?;
    Ok(())
}

fn recovery_fraction(baseline: f64, oracle: f64, candidate: f64) -> Option<f64> {
    let denominator = baseline - oracle;
    (denominator > 1.0e-12 * baseline.abs().max(1.0))
        .then_some((baseline - candidate) / denominator)
}

fn optional(value: Option<f64>) -> String {
    value.map_or_else(|| "NA".to_owned(), |number| format!("{number:.12e}"))
}

fn bootstrap_options() -> BootstrapAggregationOptions {
    BootstrapAggregationOptions {
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
        compatible_relaxation: CompatibleRelaxationOptions {
            test_vectors: 16,
            sweeps: 12,
            relaxation_damping: 1.0,
            seed: 0x4d57_4d47_4352_3031,
            relative_zero_tolerance: 1.0e-13,
        },
        compatible_criteria: CompatibleRelaxationCriteria {
            maximum_diagonal_factor_per_sweep: 0.85,
            maximum_energy_factor_per_sweep: Some(0.85),
            maximum_final_coarse_defect: 1.0e-10,
            maximum_final_structural_defect: 1.0e-10,
        },
        maximum_bootstrap_witnesses: 6,
        maximum_coarse_dimension_ratio: MAXIMUM_COARSE_DIMENSION_RATIO,
        minimum_tuple_reduction: MINIMUM_TUPLE_REDUCTION,
        maximum_two_level_tuple_complexity: MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
        split_repair: Some(AggregationRepairOptions {
            relaxation: CompatibleRelaxationOptions {
                test_vectors: 16,
                sweeps: 12,
                relaxation_damping: 1.0,
                seed: 0x4d57_4d47_4352_3031,
                relative_zero_tolerance: 1.0e-13,
            },
            criteria: CompatibleRelaxationCriteria {
                maximum_diagonal_factor_per_sweep: 0.85,
                maximum_energy_factor_per_sweep: Some(0.85),
                maximum_final_coarse_defect: 1.0e-10,
                maximum_final_structural_defect: 1.0e-10,
            },
            maximum_rounds: 18,
            maximum_coarse_dimension_ratio: MAXIMUM_COARSE_DIMENSION_RATIO,
            minimum_tuple_reduction: MINIMUM_TUPLE_REDUCTION,
            maximum_two_level_tuple_complexity: MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY,
            minimum_split_score_fraction: 0.001,
        }),
        seed: 0x4d57_4d47_434f_5645,
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
