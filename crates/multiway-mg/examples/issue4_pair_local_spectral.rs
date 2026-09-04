//! Deterministic identical-domain pair-local spectral matrix for issue #4.

#[path = "support/issue4_pair_fixtures.rs"]
mod fixtures;

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use fixtures::{DynError, PairCase, PairSuite, small_cases};
use multiway_mg::{
    PairExactOptions, PairExactPseudoinverse, PairLocalAnalysisOptions, PairLocalAnalysisReport,
    PairLocalCmgOptions, PairLocalCmgPreconditioner, PairLocalWithinPreconditioner,
    WithinApproxCholOptions, analyze_pair_local,
};

const CMG_DIRECT_THRESHOLD: usize = 2;
const DENSE_MAXIMUM_DIMENSION: usize = 256;
const RANK_TOLERANCE: f64 = 1.0e-12;
const STRUCTURE_TOLERANCE: f64 = 1.0e-9;

fn main() -> Result<(), DynError> {
    let mut arguments = env::args_os().skip(1);
    let output_directory = arguments
        .next()
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let suite = match arguments.next().and_then(|value| value.into_string().ok()) {
        None => PairSuite::Calibration,
        Some(value) if value == "calibration" => PairSuite::Calibration,
        Some(value) if value == "holdout" => PairSuite::Holdout,
        Some(value) => return Err(format!("unknown suite {value:?}").into()),
    };
    if arguments.next().is_some() {
        return Err(
            "usage: issue4_pair_local_spectral [output-directory] [calibration|holdout]".into(),
        );
    }
    fs::create_dir_all(&output_directory)?;
    run_suite(&output_directory, suite)
}

fn run_suite(output_directory: &Path, suite: PairSuite) -> Result<(), DynError> {
    let stem = format!("issue4-pair-local-{}", suite.label());
    let matrix_path = output_directory.join(format!("{stem}-spectral.tsv"));
    let eigenvalue_path = output_directory.join(format!("{stem}-eigenvalues.tsv"));
    let manifest_path = output_directory.join(format!("{stem}-manifest.tsv"));
    let mut matrix = BufWriter::new(File::create(matrix_path)?);
    let mut eigenvalues = BufWriter::new(File::create(eigenvalue_path)?);
    let mut manifest = BufWriter::new(File::create(manifest_path)?);

    writeln!(
        matrix,
        "suite\tcase\tfamily\tmethod\tdimension\trank\tnullity\tgramian_condition\tlinearity_defect\tfull_symmetry_defect\tquotient_symmetry_defect\trange_leakage\trelative_inverse_frobenius_error\tminimum_action_eigenvalue\tmaximum_action_eigenvalue\tpositive_action_defect\tminimum_preconditioned_eigenvalue\tmaximum_preconditioned_eigenvalue\tpreconditioned_condition\tunit_inverse_energy_error\tnumerically_linear\tnumerically_symmetric\tpreserves_range\tpositive_on_range\tknown_retained_bytes\topaque_retained_bytes\thierarchy_levels\twarnings\tfallback_workspace_allocations"
    )?;
    writeln!(
        eigenvalues,
        "suite\tcase\tfamily\tmethod\teigen_index\tpreconditioned_eigenvalue"
    )?;
    writeln!(
        manifest,
        "suite\tcase\tfamily\tinterpretation\tleft_count\tright_count\tdimension\tedges\tcycle_excess\tminimum_degree\tmaximum_degree\tweight_dynamic_range\tdomain_retained_bytes\tcmg_direct_threshold\twithin_dense_threshold\trank_tolerance\tstructure_tolerance"
    )?;

    for case in small_cases(suite)? {
        record_manifest(&case, suite, &mut manifest)?;
        let analysis_options = PairLocalAnalysisOptions {
            relative_rank_tolerance: RANK_TOLERANCE,
            relative_structure_tolerance: STRUCTURE_TOLERANCE,
            maximum_dimension: DENSE_MAXIMUM_DIMENSION,
        };
        let exact = PairExactPseudoinverse::build(
            case.domain.clone(),
            PairExactOptions {
                relative_rank_tolerance: RANK_TOLERANCE,
                maximum_dimension: DENSE_MAXIMUM_DIMENSION,
            },
        )?;
        let exact_report = analyze_pair_local(&case.domain, &exact, analysis_options)?;
        record_method(
            &case,
            suite,
            "exact-pseudoinverse",
            &exact_report,
            exact.retained_bytes_estimate(),
            false,
            0,
            0,
            0,
            &mut matrix,
            &mut eigenvalues,
        )?;

        for cycles in [1usize, 2] {
            let cmg = PairLocalCmgPreconditioner::build(
                case.domain.clone(),
                PairLocalCmgOptions {
                    cmg: cmg::CmgOptions {
                        direct_threshold: CMG_DIRECT_THRESHOLD,
                        ..cmg::CmgOptions::default()
                    },
                    fixed_cycles: cycles,
                },
            )?;
            let report = analyze_pair_local(&case.domain, &cmg, analysis_options)?;
            record_method(
                &case,
                suite,
                &format!("cmg-{cycles}-fixed"),
                &report,
                cmg.memory_report().total_retained_bytes_estimate(),
                false,
                cmg.hierarchy_levels(),
                0,
                cmg.fallback_workspace_allocations(),
                &mut matrix,
                &mut eigenvalues,
            )?;
        }

        let mut within_options = WithinApproxCholOptions::default();
        within_options.local_solver.dense_threshold = 0;
        let within = PairLocalWithinPreconditioner::build(case.domain.clone(), within_options)?;
        let report = analyze_pair_local(&case.domain, &within, analysis_options)?;
        record_method(
            &case,
            suite,
            "within-approx-cholesky",
            &report,
            within.memory_report().known_retained_bytes_estimate(),
            within.memory_report().within_retained_bytes().is_none(),
            0,
            within.warnings().len(),
            within.fallback_workspace_allocations(),
            &mut matrix,
            &mut eigenvalues,
        )?;
    }
    Ok(())
}

fn record_manifest(
    case: &PairCase,
    suite: PairSuite,
    writer: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let domain = &case.domain;
    writeln!(
        writer,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.17e}\t{}\t{}\t{}\t{:.17e}\t{:.17e}",
        suite.label(),
        case.name,
        case.family,
        case.interpretation,
        domain.left_count(),
        domain.right_count(),
        domain.dimension(),
        domain.edge_count(),
        domain.cycle_excess(),
        domain.minimum_degree(),
        domain.maximum_degree(),
        domain.weight_dynamic_range(),
        domain.retained_bytes_estimate(),
        CMG_DIRECT_THRESHOLD,
        0,
        RANK_TOLERANCE,
        STRUCTURE_TOLERANCE,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record_method(
    case: &PairCase,
    suite: PairSuite,
    method: &str,
    report: &PairLocalAnalysisReport,
    known_retained_bytes: usize,
    opaque_retained_bytes: bool,
    hierarchy_levels: usize,
    warnings: usize,
    fallback_workspace_allocations: usize,
    matrix: &mut BufWriter<File>,
    eigenvalues: &mut BufWriter<File>,
) -> Result<(), DynError> {
    verify_report(case, method, report)?;
    writeln!(
        matrix,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        suite.label(),
        case.name,
        case.family,
        method,
        report.dimension(),
        report.numerical_rank(),
        report.numerical_nullity(),
        report.gramian_condition_number(),
        report.linearity_defect(),
        report.full_symmetry_defect(),
        report.quotient_symmetry_defect(),
        report.range_leakage(),
        report.relative_inverse_frobenius_error(),
        report.minimum_action_eigenvalue(),
        report.maximum_action_eigenvalue(),
        report.positive_action_defect(),
        report.minimum_preconditioned_eigenvalue(),
        report.maximum_preconditioned_eigenvalue(),
        report.preconditioned_condition_number(),
        report.unit_inverse_energy_error(),
        report.numerically_linear(),
        report.numerically_symmetric(),
        report.preserves_range(),
        report.positive_on_range(),
        known_retained_bytes,
        opaque_retained_bytes,
        hierarchy_levels,
        warnings,
        fallback_workspace_allocations,
    )?;
    for (index, &value) in report.preconditioned_eigenvalues().iter().enumerate() {
        writeln!(
            eigenvalues,
            "{}\t{}\t{}\t{}\t{}\t{:.17e}",
            suite.label(),
            case.name,
            case.family,
            method,
            index,
            value,
        )?;
    }
    Ok(())
}

fn verify_report(
    case: &PairCase,
    method: &str,
    report: &PairLocalAnalysisReport,
) -> Result<(), DynError> {
    if report.numerical_nullity() != 1 {
        return Err(format!(
            "{} {method} has numerical nullity {}, expected one",
            case.name,
            report.numerical_nullity()
        )
        .into());
    }
    if !report.numerically_linear()
        || !report.numerically_symmetric()
        || !report.preserves_range()
        || !report.positive_on_range()
    {
        return Err(format!(
            "{} {method} failed algebraic gates: linear={}, symmetric={}, range={}, positive={}",
            case.name,
            report.numerically_linear(),
            report.numerically_symmetric(),
            report.preserves_range(),
            report.positive_on_range(),
        )
        .into());
    }
    if !report.preconditioned_condition_number().is_finite()
        || report.preconditioned_condition_number() < 1.0
    {
        return Err(format!(
            "{} {method} has invalid preconditioned condition {}",
            case.name,
            report.preconditioned_condition_number()
        )
        .into());
    }
    Ok(())
}
