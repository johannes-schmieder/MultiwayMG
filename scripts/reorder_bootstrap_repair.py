"""Run split repair before comparing the protected structural baseline."""

from pathlib import Path

PATH = Path("crates/multiway-mg/src/bootstrap.rs")


def main() -> None:
    text = PATH.read_text(encoding="utf-8")
    start_marker = """    let structural_baseline = build_pair_neighborhood_aggregation(
"""
    end_marker = """    let retained_test_vector_bytes = test_vectors
"""
    start = text.index(start_marker)
    end = text.index(end_marker, start)
    replacement = """    let mut split_repair = None;
    if !accepted {
        if let Some(repair_options) = options.split_repair {
            let repair = repair_aggregation_by_splitting(
                problem,
                &final_aggregation,
                screen_smoother,
                repair_options,
            )?;
            if repair.accepted() {
                final_aggregation = repair.final_aggregation().clone();
                accepted = true;
                stop_reason = BootstrapAggregationStopReason::AcceptedAfterSplitRepair {
                    witnesses: test_vectors.len() - initial_vector_count,
                    splits: repair.accepted_splits(),
                };
            } else {
                stop_reason = BootstrapAggregationStopReason::SplitRepairRejected;
            }
            split_repair = Some(repair);
        }
    }

    let structural_baseline = build_pair_neighborhood_aggregation(
        problem,
        PairNeighborhoodAggregationOptions {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: options.maximum_neighbor_degree,
        },
    )?;
    let structural_baseline_metrics = structural_metrics(problem, &structural_baseline)?;
    let mut structural_baseline_selected = false;
    let mut structural_baseline_report = None;
    let mut structural_baseline_decision = None;
    if structural_rejection(problem, structural_baseline_metrics, options).is_none()
        && structural_baseline_metrics.coarse_dimension < problem.dimension()
    {
        let report = analyze_compatible_relaxation(
            problem,
            &structural_baseline,
            screen_smoother,
            options.compatible_relaxation,
        )?;
        let decision = evaluate_compatible_relaxation(&report, options.compatible_criteria)?;
        let baseline_accepted = decision.accepted();
        let current_metrics = structural_metrics(problem, &final_aggregation)?;
        let current_factor = split_repair
            .as_ref()
            .filter(|repair| repair.accepted())
            .and_then(|repair| repair.rounds().last())
            .map(|round| round.decision().maximum_diagonal_factor_per_sweep())
            .or_else(|| {
                rounds.last().map(|round| {
                    round
                        .compatible_decision
                        .maximum_diagonal_factor_per_sweep()
                })
            })
            .unwrap_or(f64::INFINITY);
        let baseline_factor = decision.maximum_diagonal_factor_per_sweep();
        let baseline_no_worse = structural_baseline_metrics.coarse_tuple_count
            <= current_metrics.coarse_tuple_count
            && structural_baseline_metrics.coarse_dimension <= current_metrics.coarse_dimension
            && baseline_factor <= current_factor;
        let baseline_strictly_better = structural_baseline_metrics.coarse_tuple_count
            < current_metrics.coarse_tuple_count
            || structural_baseline_metrics.coarse_dimension < current_metrics.coarse_dimension
            || baseline_factor < current_factor;
        let prefer_baseline =
            baseline_accepted && (!accepted || (baseline_no_worse && baseline_strictly_better));
        if prefer_baseline {
            final_aggregation = structural_baseline.clone();
            accepted = true;
            stop_reason = BootstrapAggregationStopReason::AcceptedStructuralBaseline;
            structural_baseline_selected = true;
        }
        structural_baseline_report = Some(report);
        structural_baseline_decision = Some(decision);
    }

"""
    text = text[:start] + replacement + text[end:]

    old_bytes = """    let retained_round_report_bytes_estimate = rounds
        .iter()
        .map(|round| round.compatible_report.retained_bytes_estimate())
        .sum::<usize>()
        .saturating_add(
            structural_baseline_report
                .as_ref()
                .map_or(0, CompatibleRelaxationReport::retained_bytes_estimate),
        );
"""
    new_bytes = """    let retained_round_report_bytes_estimate = rounds
        .iter()
        .map(|round| round.compatible_report.retained_bytes_estimate())
        .sum::<usize>()
        .saturating_add(
            split_repair
                .as_ref()
                .map(|repair| {
                    repair
                        .rounds()
                        .iter()
                        .map(|round| round.report().retained_bytes_estimate())
                        .sum::<usize>()
                })
                .unwrap_or(0),
        )
        .saturating_add(
            structural_baseline_report
                .as_ref()
                .map_or(0, CompatibleRelaxationReport::retained_bytes_estimate),
        );
"""
    if text.count(old_bytes) != 1:
        raise RuntimeError("retained-report block not found exactly once")
    text = text.replace(old_bytes, new_bytes)

    old_gramian = """        compatible_gramian_applications: rounds
            .iter()
            .map(|round| round.compatible_report.gramian_applications())
            .sum::<usize>()
            .saturating_add(
                structural_baseline_report
                    .as_ref()
                    .map_or(0, CompatibleRelaxationReport::gramian_applications),
            ),
"""
    new_gramian = """        compatible_gramian_applications: rounds
            .iter()
            .map(|round| round.compatible_report.gramian_applications())
            .sum::<usize>()
            .saturating_add(
                split_repair
                    .as_ref()
                    .map(|repair| {
                        repair
                            .rounds()
                            .iter()
                            .map(|round| round.report().gramian_applications())
                            .sum::<usize>()
                    })
                    .unwrap_or(0),
            )
            .saturating_add(
                structural_baseline_report
                    .as_ref()
                    .map_or(0, CompatibleRelaxationReport::gramian_applications),
            ),
"""
    if text.count(old_gramian) != 1:
        raise RuntimeError("compatible Gramian block not found exactly once")
    text = text.replace(old_gramian, new_gramian)

    old_smoother = """        compatible_smoother_applications: rounds
            .iter()
            .map(|round| round.compatible_report.smoother_applications())
            .sum::<usize>()
            .saturating_add(
                structural_baseline_report
                    .as_ref()
                    .map_or(0, CompatibleRelaxationReport::smoother_applications),
            ),
"""
    new_smoother = """        compatible_smoother_applications: rounds
            .iter()
            .map(|round| round.compatible_report.smoother_applications())
            .sum::<usize>()
            .saturating_add(
                split_repair
                    .as_ref()
                    .map(|repair| {
                        repair
                            .rounds()
                            .iter()
                            .map(|round| round.report().smoother_applications())
                            .sum::<usize>()
                    })
                    .unwrap_or(0),
            )
            .saturating_add(
                structural_baseline_report
                    .as_ref()
                    .map_or(0, CompatibleRelaxationReport::smoother_applications),
            ),
"""
    if text.count(old_smoother) != 1:
        raise RuntimeError("compatible smoother block not found exactly once")
    text = text.replace(old_smoother, new_smoother)
    PATH.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
