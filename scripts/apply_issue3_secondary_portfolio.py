"""Integrate the two-stage bootstrap portfolio into the frozen issue #3 matrix."""

from pathlib import Path

COMPLETION = Path("crates/multiway-mg/examples/issue3_completion_matrix.rs")
FIXTURES = Path("crates/multiway-mg/examples/support/issue3_fixtures.rs")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new)


def replace_section(text: str, start: str, end: str, replacement: str, label: str) -> str:
    start_index = text.find(start)
    if start_index < 0:
        raise RuntimeError(f"{label}: start marker not found")
    end_index = text.find(end, start_index)
    if end_index < 0:
        raise RuntimeError(f"{label}: end marker not found")
    return text[:start_index] + replacement + text[end_index:]


def patch_completion() -> None:
    text = COMPLETION.read_text(encoding="utf-8")
    text = replace_once(
        text,
        """use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, BootstrapAggregationResult,
    CompatibleRelaxationCriteria, CompatibleRelaxationOptions, DenseRangeDecomposition,
    DiagonalPreconditioner, FactorAggregation, PairNeighborhoodAggregationOptions, PcgTraceOptions,
    SpectralAnalysisOptions, SymmetricMapPreconditioner, SymmetricTwoGridPreconditioner,
    ThreeWayProblem, analyze_compatible_relaxation, build_bootstrap_aggregation,
    build_pair_neighborhood_aggregation, evaluate_compatible_relaxation,
    solve_projected_pcg_traced,
};
""",
        """use multiway_mg::{
    AggregationRepairOptions, BootstrapAcceptanceScreen, BootstrapAggregationOptions,
    CompatibleRelaxationCriteria, CompatibleRelaxationOptions, DenseRangeDecomposition,
    DiagonalPreconditioner, FactorAggregation, PairNeighborhoodAggregationOptions, PcgTraceOptions,
    ScreenedBootstrapAggregationResult, SpectralAnalysisOptions, SymmetricMapPreconditioner,
    SymmetricTwoGridPreconditioner, ThreeWayProblem, analyze_compatible_relaxation,
    build_pair_neighborhood_aggregation, build_screened_bootstrap_aggregation,
    evaluate_compatible_relaxation, solve_projected_pcg_traced,
};
""",
        "imports",
    )
    text = replace_once(
        text,
        "set\\tcase\\tfamily\\trequested_seed\\tactual_seed\\tstructural_skips\\tmethod\\taccepted\\tstructural_admissible\\tselected_source\\tstructural_baseline_selected\\texact_oracle_partition\\toracle_pair_recall\\twrong_pair_count\\tfine_dimension\\tfine_tuples\\tcoarse_dimension\\tcoarse_tuples\\tcoarse_dimension_ratio\\ttuple_reduction\\ttwo_level_tuple_complexity\\tcompatible_diagonal_factor\\tcompatible_energy_factor\\tbaseline_map_condition\\toracle_two_grid_condition\\tcandidate_condition\\toracle_improvement_recovered\\tpcg_iterations\\tpcg_converged\\tpcg_final_relative_residual\\tpcg_gramian_applications\\tpcg_preconditioner_applications\\tbootstrap_rounds\\tbootstrap_witnesses\\trepair_splits\\tcandidate_pairs_generated\\tcandidate_pairs_retained\\tretained_test_vector_bytes\\tretained_report_bytes_estimate\\tstop_reason",
        "set\\tcase\\tfamily\\trequested_seed\\tactual_seed\\tstructural_skips\\tmethod\\taccepted\\tstructural_admissible\\tselected_source\\tstructural_baseline_selected\\tacceptance_screen\\texact_oracle_partition\\toracle_pair_recall\\twrong_pair_count\\tfine_dimension\\tfine_tuples\\tcoarse_dimension\\tcoarse_tuples\\tcoarse_dimension_ratio\\ttuple_reduction\\ttwo_level_tuple_complexity\\tcompatible_diagonal_factor\\tcompatible_energy_factor\\tconservative_compatible_diagonal_factor\\tconservative_compatible_energy_factor\\tbaseline_map_condition\\toracle_two_grid_condition\\tcandidate_condition\\toracle_improvement_recovered\\tpcg_iterations\\tpcg_converged\\tpcg_final_relative_residual\\tpcg_gramian_applications\\tpcg_preconditioner_applications\\tbootstrap_rounds\\tbootstrap_witnesses\\trepair_splits\\tcandidate_pairs_generated\\tcandidate_pairs_retained\\tretained_test_vector_bytes\\tretained_report_bytes_estimate\\tstop_reason",
        "matrix header",
    )
    text = replace_once(
        text,
        """        (
            "compatible_energy_limit",
            options
                .compatible_criteria
                .maximum_energy_factor_per_sweep
                .expect("completion policy requires energy")
                .to_string(),
        ),
    ] {
""",
        """        (
            "compatible_energy_limit",
            options
                .compatible_criteria
                .maximum_energy_factor_per_sweep
                .expect("completion policy requires energy")
                .to_string(),
        ),
        ("secondary_smoother", "symmetric-map".to_owned()),
        (
            "secondary_compatible_diagonal_limit",
            secondary_criteria()
                .maximum_diagonal_factor_per_sweep
                .to_string(),
        ),
        (
            "secondary_compatible_energy_limit",
            secondary_criteria()
                .maximum_energy_factor_per_sweep
                .expect("completion policy requires secondary energy")
                .to_string(),
        ),
    ] {
""",
        "policy additions",
    )
    text = text.replace(
        "map_acceptance(problem, &one_shot, &screen)?",
        "map_acceptance(problem, &one_shot, &screen, &baseline)?",
    )
    text = replace_once(
        text,
        "let bootstrap = build_bootstrap_aggregation(problem, &screen, bootstrap_options())?;",
        """let bootstrap = build_screened_bootstrap_aggregation(
        problem,
        &screen,
        &baseline,
        bootstrap_options(),
        secondary_criteria(),
    )?;""",
        "screened bootstrap call",
    )
    text = text.replace(
        "bootstrap.initial_aggregation()",
        "bootstrap.primary_result().initial_aggregation()",
    )
    text = text.replace(
        "bootstrap.work_report()",
        "bootstrap.primary_result().work_report()",
    )
    text = text.replace(
        "bootstrap.rounds()",
        "bootstrap.primary_result().rounds()",
    )
    text = text.replace(
        "bootstrap.split_repair()",
        "bootstrap.primary_result().split_repair()",
    )
    text = text.replace(
        "map_acceptance(problem, bootstrap.primary_result().initial_aggregation(), &screen)?",
        "map_acceptance(\n        problem,\n        bootstrap.primary_result().initial_aggregation(),\n        &screen,\n        &baseline,\n    )?",
    )
    text = replace_once(
        text,
        """            structural_baseline_selected: bootstrap.structural_baseline_selected(),
            bootstrap_rounds: bootstrap.primary_result().rounds().len(),
""",
        """            structural_baseline_selected: bootstrap
                .primary_result()
                .structural_baseline_selected()
                || matches!(
                    bootstrap.acceptance_screen(),
                    BootstrapAcceptanceScreen::SecondaryStructuralBaseline
                ),
            bootstrap_rounds: bootstrap.primary_result().rounds().len(),
""",
        "selected structural baseline",
    )
    text = replace_once(
        text,
        "retained_report_bytes_estimate: work.retained_report_bytes_estimate(),",
        """retained_report_bytes_estimate: work
                .retained_report_bytes_estimate()
                .saturating_add(
                    bootstrap
                        .secondary_work_report()
                        .retained_report_bytes_estimate(),
                ),""",
        "secondary retained work",
    )
    text = replace_once(
        text,
        "stop_reason: format!(\"{:?}\", bootstrap.stop_reason()),",
        """stop_reason: format!(
                "screen={:?}; primary={:?}",
                bootstrap.acceptance_screen(),
                bootstrap.primary_result().stop_reason(),
            ),""",
        "portfolio stop reason",
    )
    text = text.replace(
        "        &screen,\n        &range,",
        "        &screen,\n        &baseline,\n        &range,",
    )

    selected_source = r'''fn selected_source(result: &ScreenedBootstrapAggregationResult) -> &'static str {
    match result.acceptance_screen() {
        BootstrapAcceptanceScreen::SecondaryBootstrapFinal => "secondary-map-bootstrap-final",
        BootstrapAcceptanceScreen::SecondaryStructuralBaseline => {
            "secondary-map-protected-structural-baseline"
        }
        BootstrapAcceptanceScreen::Rejected | BootstrapAcceptanceScreen::Primary => {
            let primary = result.primary_result();
            if primary.structural_baseline_selected() {
                "protected-structural-baseline"
            } else if primary
                .split_repair()
                .is_some_and(|repair| repair.accepted())
            {
                "witness-split-repair"
            } else if primary.rounds().len() > 1 {
                "bootstrap-witness-rematching"
            } else {
                "relaxed-signature-initial"
            }
        }
    }
}

'''
    text = replace_section(
        text,
        "fn selected_source(",
        "#[derive(Debug, Clone, Default)]\nstruct WorkFields",
        selected_source,
        "selected source",
    )

    evaluated = r'''#[derive(Debug, Clone, Copy)]
struct MapAcceptance {
    structural_admissible: bool,
    accepted: bool,
    acceptance_screen: &'static str,
    compatible_diagonal_factor: Option<f64>,
    compatible_energy_factor: Option<f64>,
    conservative_compatible_diagonal_factor: Option<f64>,
    conservative_compatible_energy_factor: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct EvaluatedMap {
    condition: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
fn evaluate_map(
    fixture: &Issue3Fixture,
    method: &str,
    selected_source: &str,
    aggregation: &FactorAggregation,
    requested_acceptance: bool,
    work: WorkFields,
    primary_screen: &DiagonalPreconditioner,
    secondary_screen: &SymmetricMapPreconditioner,
    range: &DenseRangeDecomposition,
    range_options: SpectralAnalysisOptions,
    baseline_condition: f64,
    oracle_condition: Option<f64>,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<EvaluatedMap, DynError> {
    let problem = &fixture.problem;
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension: usize = aggregation.coarse_counts().iter().sum();
    let coarse_dimension_ratio = coarse_dimension as f64 / problem.dimension() as f64;
    let coarse_tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    let tuple_reduction = 1.0 - coarse_tuple_ratio;
    let two_level_tuple_complexity = 1.0 + coarse_tuple_ratio;
    let acceptance = map_acceptance(problem, aggregation, primary_screen, secondary_screen)?;
    let accepted = requested_acceptance && acceptance.accepted;
    let partition = partition_metrics(&fixture.oracle, aggregation);

    let (condition, pcg) = if acceptance.structural_admissible {
        let cycle = SymmetricTwoGridPreconditioner::build(
            problem.clone(),
            aggregation.clone(),
            SymmetricMapPreconditioner::new(problem.clone()),
            1,
            1.0,
            1.0e-12,
        )?;
        let condition = range
            .analyze(&cycle, range_options)?
            .preconditioned_condition_number();
        let rhs = deterministic_rhs(problem)?;
        let pcg = solve_projected_pcg_traced(problem, &rhs, &cycle, PcgTraceOptions::default())?;
        for sample in pcg.samples() {
            writeln!(
                traces,
                "{}\t{}\t{}\t{}\t{}\t{:.12e}",
                fixture.set,
                fixture.name,
                fixture.family,
                method,
                sample.iteration(),
                sample.relative_residual(),
            )?;
        }
        (Some(condition), Some(pcg))
    } else {
        (None, None)
    };
    let recovery = oracle_condition.and_then(|oracle| {
        condition.and_then(|candidate| recovery_fraction(baseline_condition, oracle, candidate))
    });

    let row = vec![
        fixture.set.to_owned(),
        fixture.name.clone(),
        fixture.family.to_owned(),
        fixture.requested_seed.to_string(),
        fixture.actual_seed.to_string(),
        fixture.structural_skips.to_string(),
        method.to_owned(),
        accepted.to_string(),
        acceptance.structural_admissible.to_string(),
        selected_source.to_owned(),
        work.structural_baseline_selected.to_string(),
        acceptance.acceptance_screen.to_owned(),
        partition.exact.to_string(),
        format!("{:.12e}", partition.oracle_pair_recall),
        partition.wrong_pair_count.to_string(),
        problem.dimension().to_string(),
        problem.tuple_count().to_string(),
        coarse_dimension.to_string(),
        coarse.tuple_count().to_string(),
        format!("{coarse_dimension_ratio:.12e}"),
        format!("{tuple_reduction:.12e}"),
        format!("{two_level_tuple_complexity:.12e}"),
        optional(acceptance.compatible_diagonal_factor),
        optional(acceptance.compatible_energy_factor),
        optional(acceptance.conservative_compatible_diagonal_factor),
        optional(acceptance.conservative_compatible_energy_factor),
        format!("{baseline_condition:.12e}"),
        optional(oracle_condition),
        optional(condition),
        optional(recovery),
        optional_usize(pcg.as_ref().map(|result| result.iterations())),
        optional_bool(pcg.as_ref().map(|result| result.converged())),
        optional(pcg.as_ref().map(|result| result.final_relative_residual())),
        optional_usize(pcg.as_ref().map(|result| result.gramian_applications())),
        optional_usize(
            pcg.as_ref()
                .map(|result| result.preconditioner_applications()),
        ),
        work.bootstrap_rounds.to_string(),
        work.bootstrap_witnesses.to_string(),
        work.repair_splits.to_string(),
        work.candidate_pairs_generated.to_string(),
        work.candidate_pairs_retained.to_string(),
        work.retained_test_vector_bytes.to_string(),
        work.retained_report_bytes_estimate.to_string(),
        work.stop_reason.replace(['\t', '\n'], " "),
    ];
    writeln!(matrix, "{}", row.join("\t"))?;
    Ok(EvaluatedMap { condition })
}

'''
    text = replace_section(
        text,
        "#[derive(Debug, Clone, Copy)]\nstruct MapAcceptance",
        "fn record_baseline(",
        evaluated,
        "map evaluation",
    )

    baseline = r'''fn record_baseline(
    fixture: &Issue3Fixture,
    baseline: &SymmetricMapPreconditioner,
    baseline_condition: f64,
    oracle_condition: f64,
    matrix: &mut BufWriter<File>,
    traces: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let problem = &fixture.problem;
    let rhs = deterministic_rhs(problem)?;
    let pcg = solve_projected_pcg_traced(problem, &rhs, baseline, PcgTraceOptions::default())?;
    for sample in pcg.samples() {
        writeln!(
            traces,
            "{}\t{}\t{}\tbaseline-symmetric-map\t{}\t{:.12e}",
            fixture.set,
            fixture.name,
            fixture.family,
            sample.iteration(),
            sample.relative_residual(),
        )?;
    }
    let row = vec![
        fixture.set.to_owned(),
        fixture.name.clone(),
        fixture.family.to_owned(),
        fixture.requested_seed.to_string(),
        fixture.actual_seed.to_string(),
        fixture.structural_skips.to_string(),
        "baseline-symmetric-map".to_owned(),
        "true".to_owned(),
        "true".to_owned(),
        "baseline".to_owned(),
        "false".to_owned(),
        "baseline".to_owned(),
        "false".to_owned(),
        "0.000000000000e0".to_owned(),
        "0".to_owned(),
        problem.dimension().to_string(),
        problem.tuple_count().to_string(),
        "0".to_owned(),
        "0".to_owned(),
        "0.000000000000e0".to_owned(),
        "0.000000000000e0".to_owned(),
        "1.000000000000e0".to_owned(),
        "NA".to_owned(),
        "NA".to_owned(),
        "NA".to_owned(),
        "NA".to_owned(),
        format!("{baseline_condition:.12e}"),
        format!("{oracle_condition:.12e}"),
        format!("{baseline_condition:.12e}"),
        "0.000000000000e0".to_owned(),
        pcg.iterations().to_string(),
        pcg.converged().to_string(),
        format!("{:.12e}", pcg.final_relative_residual()),
        pcg.gramian_applications().to_string(),
        pcg.preconditioner_applications().to_string(),
        "0".to_owned(),
        "0".to_owned(),
        "0".to_owned(),
        "0".to_owned(),
        "0".to_owned(),
        "0".to_owned(),
        "0".to_owned(),
        "baseline".to_owned(),
    ];
    writeln!(matrix, "{}", row.join("\t"))?;
    Ok(())
}

fn map_acceptance(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    primary_screen: &DiagonalPreconditioner,
    secondary_screen: &SymmetricMapPreconditioner,
) -> Result<MapAcceptance, DynError> {
    let coarse = aggregation.coarsen(problem)?;
    let coarse_dimension: usize = aggregation.coarse_counts().iter().sum();
    let dimension_ratio = coarse_dimension as f64 / problem.dimension() as f64;
    let tuple_ratio = coarse.tuple_count() as f64 / problem.tuple_count() as f64;
    let tuple_reduction = 1.0 - tuple_ratio;
    let complexity = 1.0 + tuple_ratio;
    let structural_admissible = coarse_dimension < problem.dimension()
        && dimension_ratio <= MAXIMUM_COARSE_DIMENSION_RATIO
        && tuple_reduction >= MINIMUM_TUPLE_REDUCTION
        && complexity <= MAXIMUM_TWO_LEVEL_TUPLE_COMPLEXITY;
    if !structural_admissible {
        return Ok(MapAcceptance {
            structural_admissible,
            accepted: false,
            acceptance_screen: "structural-rejected",
            compatible_diagonal_factor: None,
            compatible_energy_factor: None,
            conservative_compatible_diagonal_factor: None,
            conservative_compatible_energy_factor: None,
        });
    }
    let primary_report = analyze_compatible_relaxation(
        problem,
        aggregation,
        primary_screen,
        compatible_options(),
    )?;
    let primary_decision =
        evaluate_compatible_relaxation(&primary_report, compatible_criteria())?;
    let conservative_diagonal = Some(primary_decision.maximum_diagonal_factor_per_sweep());
    let conservative_energy = primary_decision.maximum_energy_factor_per_sweep();
    if primary_decision.accepted() {
        return Ok(MapAcceptance {
            structural_admissible,
            accepted: true,
            acceptance_screen: "weighted-jacobi",
            compatible_diagonal_factor: conservative_diagonal,
            compatible_energy_factor: conservative_energy,
            conservative_compatible_diagonal_factor: conservative_diagonal,
            conservative_compatible_energy_factor: conservative_energy,
        });
    }

    let secondary_report = analyze_compatible_relaxation(
        problem,
        aggregation,
        secondary_screen,
        compatible_options(),
    )?;
    let secondary_decision =
        evaluate_compatible_relaxation(&secondary_report, secondary_criteria())?;
    Ok(MapAcceptance {
        structural_admissible,
        accepted: secondary_decision.accepted(),
        acceptance_screen: if secondary_decision.accepted() {
            "symmetric-map"
        } else {
            "rejected"
        },
        compatible_diagonal_factor: Some(
            secondary_decision.maximum_diagonal_factor_per_sweep(),
        ),
        compatible_energy_factor: secondary_decision.maximum_energy_factor_per_sweep(),
        conservative_compatible_diagonal_factor: conservative_diagonal,
        conservative_compatible_energy_factor: conservative_energy,
    })
}

'''
    text = replace_section(
        text,
        "fn record_baseline(",
        "fn bootstrap_options()",
        baseline,
        "baseline and acceptance",
    )
    text = replace_once(
        text,
        """fn compatible_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.85,
        maximum_energy_factor_per_sweep: Some(0.85),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    }
}
""",
        """fn compatible_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.85,
        maximum_energy_factor_per_sweep: Some(0.85),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    }
}

fn secondary_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.85,
        maximum_energy_factor_per_sweep: Some(0.85),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    }
}
""",
        "secondary criteria",
    )
    COMPLETION.write_text(text, encoding="utf-8")


def patch_fixtures() -> None:
    text = FIXTURES.read_text(encoding="utf-8")
    text = text.replace(
        "/// Fixed unseen-seed holdouts. Their seeds were declared after calibration\n/// options and before their numerical results were evaluated.",
        "/// Second frozen unseen-seed holdouts. These seeds were declared after the\n/// first development holdout exposed the need for a separate production-smoother\n/// screen and before the revised portfolio was evaluated.",
    )
    replacements = {
        "requested_seed: 512,": "requested_seed: 600,",
        "requested_seed: 513,": "requested_seed: 601,",
        "requested_seed: 514,": "requested_seed: 602,",
        "requested_seed: 515,": "requested_seed: 603,",
        "requested_seed: 516,": "requested_seed: 604,",
        "requested_seed: 517,": "requested_seed: 605,",
        "requested_seed: 518,": "requested_seed: 606,",
        "requested_seed: 519,": "requested_seed: 607,",
        "requested_seed: 520,": "requested_seed: 608,",
        "requested_seed: 521,": "requested_seed: 609,",
    }
    for old, new in replacements.items():
        text = replace_once(text, old, new, old)
    FIXTURES.write_text(text, encoding="utf-8")


def main() -> None:
    patch_completion()
    patch_fixtures()


if __name__ == "__main__":
    main()
