//! Explicit smoother, coarse-only, and two-grid matrix for issue #2.

#[path = "support/issue2_fixtures.rs"]
mod fixtures;

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use fixtures::{DynError, OracleCase, deterministic_rhs, one_level_cases};
use multiway_mg::{
    DensePairOptions, DensePairSchwarzPreconditioner, DenseRangeDecomposition,
    DiagonalPreconditioner, ExactCoarseCorrection, PairCmgOptions, PairSubsetCmgPreconditioner,
    PcgTraceOptions, Preconditioner, SpectralAnalysisOptions, SymmetricMapPreconditioner,
    SymmetricTwoGridPreconditioner, WeightedSumPreconditioner, analyze_stationary_error,
    estimate_problem_bytes, solve_projected_pcg_traced,
};

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let mut summary = writer(&output_directory.join("issue2-two-grid-matrix.tsv"))?;
    let mut traces = writer(&output_directory.join("issue2-pcg-traces.tsv"))?;
    writeln!(
        summary,
        "case\tfamily\tdimension\ttuples\tcomponents\tcoarse_dimension\tcoarse_tuples\tmethod\trole\tpreconditioner_symmetry_defect\tquotient_symmetry_defect\trange_leakage\tminimum_preconditioner_energy\tminimum_preconditioned_eigenvalue\tmaximum_preconditioned_eigenvalue\tpreconditioned_condition_number\tone_step_error_spectral_radius\tone_step_error_energy_norm\tpcg_iterations\tpcg_converged\tpcg_final_relative_residual\tgramian_applications\tpreconditioner_applications\tretained_bytes_estimate"
    )?;
    writeln!(traces, "case\tmethod\titeration\trelative_true_residual")?;

    for case in one_level_cases()? {
        run_case(&case, &mut summary, &mut traces)?;
    }
    println!(
        "wrote {} and {}",
        output_directory
            .join("issue2-two-grid-matrix.tsv")
            .display(),
        output_directory.join("issue2-pcg-traces.tsv").display()
    );
    Ok(())
}

fn run_case(
    case: &OracleCase,
    summary: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let aggregation = case
        .maps
        .first()
        .cloned()
        .ok_or("issue #2 case has no oracle aggregation")?;
    let coarse = aggregation.coarsen(&case.problem)?;
    let rhs = deterministic_rhs(&case.problem)?;
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(&case.problem, spectral_options)?;
    let problem_bytes = estimate_problem_bytes(&case.problem);

    for omega in [0.4_f64, 0.5, 0.6] {
        let diagonal = DiagonalPreconditioner::new(&case.problem, omega)?;
        record_method(
            case,
            &coarse,
            &range,
            &rhs,
            &format!("jacobi-{omega:.1}"),
            "smoother-only",
            &diagonal,
            problem_bytes.saturating_add(case.problem.dimension().saturating_mul(8)),
            summary,
            traces,
        )?;
    }

    let symmetric_map = SymmetricMapPreconditioner::new(case.problem.clone());
    record_method(
        case,
        &coarse,
        &range,
        &rhs,
        "symmetric-map",
        "smoother-only",
        &symmetric_map,
        problem_bytes,
        summary,
        traces,
    )?;

    let exact_pair =
        DensePairSchwarzPreconditioner::build(case.problem.clone(), DensePairOptions::default())?;
    record_method(
        case,
        &coarse,
        &range,
        &rhs,
        "exact-pair-schwarz",
        "smoother-only",
        &exact_pair,
        dense_pair_bytes(&case.problem),
        summary,
        traces,
    )?;

    let pair_cmg =
        PairSubsetCmgPreconditioner::build_all(case.problem.clone(), PairCmgOptions::default())?;
    let pair_memory = pair_cmg.memory_report().total_retained_bytes_estimate();
    record_method(
        case,
        &coarse,
        &range,
        &rhs,
        "pair-cmg-all",
        "smoother-only",
        &pair_cmg,
        pair_memory,
        summary,
        traces,
    )?;

    let selected_diagonal = selected_pair_with_diagonal(case)?;
    let selected_diagonal_bytes = problem_bytes
        .saturating_add(case.problem.dimension().saturating_mul(8))
        .saturating_add(
            selected_diagonal
                .right()
                .memory_report()
                .total_retained_bytes_estimate(),
        );
    record_method(
        case,
        &coarse,
        &range,
        &rhs,
        &format!("selected-{}-plus-jacobi", case.dominant_pair.label()),
        "smoother-only",
        &selected_diagonal,
        selected_diagonal_bytes,
        summary,
        traces,
    )?;

    let selected_map = selected_pair_with_map(case)?;
    let selected_map_bytes = problem_bytes.saturating_add(
        selected_map
            .right()
            .memory_report()
            .total_retained_bytes_estimate(),
    );
    record_method(
        case,
        &coarse,
        &range,
        &rhs,
        &format!("selected-{}-plus-map", case.dominant_pair.label()),
        "smoother-only",
        &selected_map,
        selected_map_bytes,
        summary,
        traces,
    )?;

    let coarse_only =
        ExactCoarseCorrection::build(case.problem.clone(), aggregation.clone(), 1.0e-12)?;
    record_method_without_pcg(
        case,
        &coarse,
        &range,
        "exact-coarse-only",
        "coarse-only",
        &coarse_only,
        coarse_only.retained_bytes_estimate(),
        summary,
    )?;

    let two_grid_jacobi = SymmetricTwoGridPreconditioner::build(
        case.problem.clone(),
        aggregation.clone(),
        DiagonalPreconditioner::new(&case.problem, 0.5)?,
        1,
        1.0,
        1.0e-12,
    )?;
    record_method(
        case,
        &coarse,
        &range,
        &rhs,
        "two-grid-jacobi",
        "two-grid",
        &two_grid_jacobi,
        two_grid_jacobi
            .coarse_correction()
            .retained_bytes_estimate()
            .saturating_add(case.problem.dimension().saturating_mul(8)),
        summary,
        traces,
    )?;

    let two_grid_map = SymmetricTwoGridPreconditioner::build(
        case.problem.clone(),
        aggregation.clone(),
        SymmetricMapPreconditioner::new(case.problem.clone()),
        1,
        1.0,
        1.0e-12,
    )?;
    record_method(
        case,
        &coarse,
        &range,
        &rhs,
        "two-grid-symmetric-map",
        "two-grid",
        &two_grid_map,
        two_grid_map.coarse_correction().retained_bytes_estimate(),
        summary,
        traces,
    )?;

    let two_grid_exact_pair = SymmetricTwoGridPreconditioner::build(
        case.problem.clone(),
        aggregation.clone(),
        DensePairSchwarzPreconditioner::build(case.problem.clone(), DensePairOptions::default())?,
        1,
        1.0,
        1.0e-12,
    )?;
    record_method(
        case,
        &coarse,
        &range,
        &rhs,
        "two-grid-exact-pair",
        "two-grid",
        &two_grid_exact_pair,
        two_grid_exact_pair
            .coarse_correction()
            .retained_bytes_estimate()
            .saturating_add(dense_pair_bytes(&case.problem)),
        summary,
        traces,
    )?;

    let two_grid_pair_cmg = SymmetricTwoGridPreconditioner::build(
        case.problem.clone(),
        aggregation,
        PairSubsetCmgPreconditioner::build_all(case.problem.clone(), PairCmgOptions::default())?,
        1,
        1.0,
        1.0e-12,
    )?;
    let two_grid_pair_bytes = two_grid_pair_cmg
        .coarse_correction()
        .retained_bytes_estimate()
        .saturating_add(
            two_grid_pair_cmg
                .smoother()
                .memory_report()
                .total_retained_bytes_estimate(),
        );
    record_method(
        case,
        &coarse,
        &range,
        &rhs,
        "two-grid-pair-cmg",
        "two-grid",
        &two_grid_pair_cmg,
        two_grid_pair_bytes,
        summary,
        traces,
    )?;
    Ok(())
}

fn selected_pair_with_diagonal(
    case: &OracleCase,
) -> Result<WeightedSumPreconditioner<DiagonalPreconditioner, PairSubsetCmgPreconditioner>, DynError>
{
    Ok(WeightedSumPreconditioner::new(
        DiagonalPreconditioner::new(&case.problem, 0.5)?,
        0.25,
        PairSubsetCmgPreconditioner::build(
            case.problem.clone(),
            &[case.dominant_pair],
            PairCmgOptions::default(),
        )?,
        1.0,
    )?)
}

fn selected_pair_with_map(
    case: &OracleCase,
) -> Result<
    WeightedSumPreconditioner<SymmetricMapPreconditioner, PairSubsetCmgPreconditioner>,
    DynError,
> {
    Ok(WeightedSumPreconditioner::new(
        SymmetricMapPreconditioner::new(case.problem.clone()),
        1.0,
        PairSubsetCmgPreconditioner::build(
            case.problem.clone(),
            &[case.dominant_pair],
            PairCmgOptions::default(),
        )?,
        1.0,
    )?)
}

#[allow(clippy::too_many_arguments)]
fn record_method(
    case: &OracleCase,
    coarse: &ThreeWayProblem,
    range: &DenseRangeDecomposition,
    rhs: &[f64],
    method: &str,
    role: &str,
    preconditioner: &dyn Preconditioner,
    retained_bytes_estimate: usize,
    summary: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let spectral = range.analyze(preconditioner, spectral_options)?;
    let error = analyze_stationary_error(range, preconditioner, 1.0, 1, spectral_options)?;
    let pcg = if spectral.positive_definite_on_range() {
        Some(solve_projected_pcg_traced(
            &case.problem,
            rhs,
            preconditioner,
            PcgTraceOptions {
                relative_tolerance: 1.0e-10,
                absolute_tolerance: 0.0,
                max_iterations: 2_000,
            },
        )?)
    } else {
        None
    };
    if let Some(pcg) = &pcg {
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
    }
    write_summary(
        case,
        coarse,
        method,
        role,
        &spectral,
        &error,
        pcg.as_ref(),
        retained_bytes_estimate,
        summary,
    )
}

#[allow(clippy::too_many_arguments)]
fn record_method_without_pcg(
    case: &OracleCase,
    coarse: &ThreeWayProblem,
    range: &DenseRangeDecomposition,
    method: &str,
    role: &str,
    preconditioner: &dyn Preconditioner,
    retained_bytes_estimate: usize,
    summary: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let spectral = range.analyze(preconditioner, spectral_options)?;
    let error = analyze_stationary_error(range, preconditioner, 1.0, 1, spectral_options)?;
    write_summary(
        case,
        coarse,
        method,
        role,
        &spectral,
        &error,
        None,
        retained_bytes_estimate,
        summary,
    )
}

#[allow(clippy::too_many_arguments)]
fn write_summary(
    case: &OracleCase,
    coarse: &ThreeWayProblem,
    method: &str,
    role: &str,
    spectral: &multiway_mg::SpectralAnalysisReport,
    error: &multiway_mg::StationaryErrorReport,
    pcg: Option<&multiway_mg::PcgTraceResult>,
    retained_bytes_estimate: usize,
    writer: &mut BufWriter<File>,
) -> Result<(), DynError> {
    writeln!(
        writer,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{:.12e}\t{}\t{:.12e}\t{:.12e}\t{}\t{}\t{}\t{}\t{}\t{}",
        case.name,
        case.family,
        case.problem.dimension(),
        case.problem.tuple_count(),
        case.problem.components().count(),
        coarse.dimension(),
        coarse.tuple_count(),
        method,
        role,
        spectral.preconditioner_symmetry_defect(),
        spectral.quotient_symmetry_defect(),
        spectral.range_leakage(),
        spectral.minimum_preconditioner_energy(),
        spectral.minimum_preconditioned_eigenvalue(),
        spectral.maximum_preconditioned_eigenvalue(),
        finite_or_infinity(spectral.preconditioned_condition_number()),
        error.one_sweep_spectral_radius(),
        error.one_sweep_energy_operator_norm(),
        pcg.map_or_else(|| "NA".to_owned(), |value| value.iterations().to_string()),
        pcg.map_or_else(|| "NA".to_owned(), |value| value.converged().to_string()),
        pcg.map_or_else(
            || "NA".to_owned(),
            |value| format!("{:.12e}", value.final_relative_residual())
        ),
        pcg.map_or_else(
            || "NA".to_owned(),
            |value| value.gramian_applications().to_string()
        ),
        pcg.map_or_else(
            || "NA".to_owned(),
            |value| value.preconditioner_applications().to_string()
        ),
        retained_bytes_estimate,
    )?;
    Ok(())
}

fn finite_or_infinity(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.12e}")
    } else {
        "inf".to_owned()
    }
}

fn dense_pair_bytes(problem: &ThreeWayProblem) -> usize {
    let counts = problem.topology().level_counts();
    let pair_bytes = [(0, 1), (0, 2), (1, 2)]
        .into_iter()
        .map(|(first, second)| {
            let dimension = counts[first] + counts[second];
            dimension
                .saturating_mul(dimension.saturating_add(1))
                .saturating_mul(8)
        })
        .sum::<usize>();
    estimate_problem_bytes(problem).saturating_add(pair_bytes)
}

fn writer(path: &Path) -> Result<BufWriter<File>, DynError> {
    Ok(BufWriter::new(File::create(path)?))
}

use multiway_mg::ThreeWayProblem;
