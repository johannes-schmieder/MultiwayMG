//! Resolution and hierarchy-depth matrix for issue #2.

#[path = "support/issue2_fixtures.rs"]
mod fixtures;

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use fixtures::{DynError, OracleCase, deterministic_rhs, resolution_cases};
use multiway_mg::{
    DenseRangeDecomposition, DiagonalPreconditioner, OracleLevelSmootherSpec, PairCmgOptions,
    PairSubsetCmgPreconditioner, PcgTraceOptions, Preconditioner, ScheduledOracleHierarchy,
    ScheduledOracleHierarchyOptions, SpectralAnalysisOptions, SymmetricMapPreconditioner,
    analyze_stationary_error, estimate_problem_bytes, solve_projected_pcg_traced,
};

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let mut summary = writer(&output_directory.join("issue2-resolution-matrix.tsv"))?;
    let mut traces = writer(&output_directory.join("issue2-resolution-traces.tsv"))?;
    writeln!(
        summary,
        "case\tfamily\tdepth\tdimension\ttuples\tcomponents\tlevel_dimensions\tlevel_tuples\tterminal_rank\tmethod\tpair_levels\tdimension_complexity\ttuple_complexity\tpreconditioned_condition_number\toptimal_energy_spectral_radius\tone_cycle_error_spectral_radius\tone_cycle_energy_operator_norm\tpcg_iterations\tpcg_final_relative_residual\tgramian_applications\tpreconditioner_applications\tretained_bytes_estimate\tapply_scratch_bytes_estimate"
    )?;
    writeln!(traces, "case\tmethod\titeration\trelative_true_residual")?;

    for case in resolution_cases()? {
        run_case(&case, &mut summary, &mut traces)?;
    }
    println!(
        "wrote {} and {}",
        output_directory.join("issue2-resolution-matrix.tsv").display(),
        output_directory.join("issue2-resolution-traces.tsv").display()
    );
    Ok(())
}

fn run_case(
    case: &OracleCase,
    summary: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let rhs = deterministic_rhs(&case.problem)?;
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(&case.problem, spectral_options)?;

    let diagonal = DiagonalPreconditioner::new(&case.problem, 0.5)?;
    record_plain_method(
        case,
        &range,
        &rhs,
        "diagonal",
        &diagonal,
        estimate_problem_bytes(&case.problem)
            .saturating_add(case.problem.dimension().saturating_mul(8)),
        summary,
        traces,
    )?;

    let map = SymmetricMapPreconditioner::new(case.problem.clone());
    record_plain_method(
        case,
        &range,
        &rhs,
        "symmetric-map",
        &map,
        estimate_problem_bytes(&case.problem),
        summary,
        traces,
    )?;

    let pair = PairSubsetCmgPreconditioner::build_all(
        case.problem.clone(),
        PairCmgOptions::default(),
    )?;
    let pair_bytes = pair.memory_report().total_retained_bytes_estimate();
    record_plain_method(
        case,
        &range,
        &rhs,
        "pair-cmg",
        &pair,
        pair_bytes,
        summary,
        traces,
    )?;

    for schedule in [
        Schedule::Jacobi,
        Schedule::PairFinest,
        Schedule::PairFirstTwo,
        Schedule::PairAll,
        Schedule::MapAll,
    ] {
        let hierarchy = build_schedule(case, schedule)?;
        record_hierarchy(
            case,
            &range,
            &rhs,
            schedule.label(),
            schedule.pair_levels(case.depth),
            &hierarchy,
            summary,
            traces,
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum Schedule {
    Jacobi,
    PairFinest,
    PairFirstTwo,
    PairAll,
    MapAll,
}

impl Schedule {
    const fn label(self) -> &'static str {
        match self {
            Self::Jacobi => "oracle-jacobi",
            Self::PairFinest => "oracle-pair-finest",
            Self::PairFirstTwo => "oracle-pair-first-two",
            Self::PairAll => "oracle-pair-all-levels",
            Self::MapAll => "oracle-map-all-levels",
        }
    }

    const fn pair_levels(self, depth: usize) -> usize {
        match self {
            Self::Jacobi | Self::MapAll => 0,
            Self::PairFinest => 1,
            Self::PairFirstTwo => {
                if depth < 2 {
                    depth
                } else {
                    2
                }
            }
            Self::PairAll => depth,
        }
    }
}

fn build_schedule(
    case: &OracleCase,
    schedule: Schedule,
) -> Result<ScheduledOracleHierarchy, DynError> {
    let smoothers = (0..case.depth)
        .map(|level| match schedule {
            Schedule::Jacobi => OracleLevelSmootherSpec::Jacobi { omega: 0.5 },
            Schedule::PairFinest if level == 0 => OracleLevelSmootherSpec::AllPairsCmg {
                options: PairCmgOptions::default(),
            },
            Schedule::PairFirstTwo if level < 2 => OracleLevelSmootherSpec::AllPairsCmg {
                options: PairCmgOptions::default(),
            },
            Schedule::PairAll => OracleLevelSmootherSpec::AllPairsCmg {
                options: PairCmgOptions::default(),
            },
            Schedule::MapAll => OracleLevelSmootherSpec::SymmetricMap,
            Schedule::PairFinest | Schedule::PairFirstTwo => {
                OracleLevelSmootherSpec::Jacobi { omega: 0.5 }
            }
        })
        .collect();
    Ok(ScheduledOracleHierarchy::build(
        case.problem.clone(),
        ScheduledOracleHierarchyOptions {
            aggregations: case.maps.clone(),
            smoothers,
            sweeps: 1,
            terminal_relative_tolerance: 1.0e-12,
        },
    )?)
}

#[allow(clippy::too_many_arguments)]
fn record_plain_method(
    case: &OracleCase,
    range: &DenseRangeDecomposition,
    rhs: &[f64],
    method: &str,
    preconditioner: &dyn Preconditioner,
    retained_bytes_estimate: usize,
    summary: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    record(
        case,
        range,
        rhs,
        method,
        0,
        &[],
        &[],
        0,
        1.0,
        1.0,
        preconditioner,
        retained_bytes_estimate,
        0,
        summary,
        traces,
    )
}

#[allow(clippy::too_many_arguments)]
fn record_hierarchy(
    case: &OracleCase,
    range: &DenseRangeDecomposition,
    rhs: &[f64],
    method: &str,
    pair_levels: usize,
    hierarchy: &ScheduledOracleHierarchy,
    summary: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let memory = hierarchy.memory_report();
    record(
        case,
        range,
        rhs,
        method,
        pair_levels,
        &hierarchy.dimensions(),
        &hierarchy.tuple_counts(),
        hierarchy.terminal_rank(),
        hierarchy.dimension_complexity(),
        hierarchy.tuple_complexity(),
        hierarchy,
        memory.total_retained_bytes_estimate(),
        memory.maximum_apply_scratch_bytes_estimate(),
        summary,
        traces,
    )
}

#[allow(clippy::too_many_arguments)]
fn record(
    case: &OracleCase,
    range: &DenseRangeDecomposition,
    rhs: &[f64],
    method: &str,
    pair_levels: usize,
    dimensions: &[usize],
    tuples: &[usize],
    terminal_rank: usize,
    dimension_complexity: f64,
    tuple_complexity: f64,
    preconditioner: &dyn Preconditioner,
    retained_bytes_estimate: usize,
    apply_scratch_bytes_estimate: usize,
    summary: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let spectral = range.analyze(preconditioner, spectral_options)?;
    if !spectral.positive_definite_on_range() || !spectral.numerically_symmetric() {
        return Err(format!(
            "{method} on {} is not symmetric positive on the complete range",
            case.name
        )
        .into());
    }
    let stationary = analyze_stationary_error(range, preconditioner, 1.0, 1, spectral_options)?;
    let pcg = solve_projected_pcg_traced(
        &case.problem,
        rhs,
        preconditioner,
        PcgTraceOptions {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 0.0,
            max_iterations: 2_000,
        },
    )?;
    if !pcg.converged() {
        return Err(format!("{method} did not converge on {}", case.name).into());
    }
    for sample in pcg.samples() {
        writeln!(
            traces,
            "{}\t{}\t{}\t{:.12e}",
            case.name,
            method,
            sample.iteration(),
            sample.relative_residual()
        )?;
    }
    writeln!(
        summary,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.9}\t{:.9}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{}\t{:.12e}\t{}\t{}\t{}\t{}",
        case.name,
        case.family,
        case.depth,
        case.problem.dimension(),
        case.problem.tuple_count(),
        case.problem.components().count(),
        list(dimensions),
        list(tuples),
        terminal_rank,
        method,
        pair_levels,
        dimension_complexity,
        tuple_complexity,
        spectral.preconditioned_condition_number(),
        spectral.optimal_energy_spectral_radius(),
        stationary.one_sweep_spectral_radius(),
        stationary.one_sweep_energy_operator_norm(),
        pcg.iterations(),
        pcg.final_relative_residual(),
        pcg.gramian_applications(),
        pcg.preconditioner_applications(),
        retained_bytes_estimate,
        apply_scratch_bytes_estimate,
    )?;
    Ok(())
}

fn list(values: &[usize]) -> String {
    if values.is_empty() {
        "NA".to_owned()
    } else {
        values
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn writer(path: &Path) -> Result<BufWriter<File>, DynError> {
    Ok(BufWriter::new(File::create(path)?))
}
