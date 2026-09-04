"""Add a fail-safe pair-neighborhood baseline to bootstrap aggregation."""

from pathlib import Path

PATH = Path("crates/multiway-mg/src/bootstrap.rs")
TESTS = Path("crates/multiway-mg/tests/bootstrap.rs")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new)


def patch_source() -> None:
    text = PATH.read_text(encoding="utf-8")
    text = replace_once(
        text,
        """    DiagonalPreconditioner, FactorAggregation, MultiwayError, Preconditioner, ThreeWayProblem,
    analyze_compatible_relaxation, evaluate_compatible_relaxation, repair_aggregation_by_splitting,
};
""",
        """    DiagonalPreconditioner, FactorAggregation, MultiwayError,
    PairNeighborhoodAggregationOptions, Preconditioner, ThreeWayProblem,
    analyze_compatible_relaxation, build_pair_neighborhood_aggregation,
    evaluate_compatible_relaxation, repair_aggregation_by_splitting,
};
""",
        "imports",
    )
    text = replace_once(
        text,
        """    AcceptedAfterSplitRepair {
        /// Number of appended bootstrap witnesses.
        witnesses: usize,
        /// Number of admitted aggregate splits.
        splits: usize,
    },
""",
        """    AcceptedAfterSplitRepair {
        /// Number of appended bootstrap witnesses.
        witnesses: usize,
        /// Number of admitted aggregate splits.
        splits: usize,
    },
    /// The bounded pair-neighborhood baseline was accepted and dominated the
    /// accepted bootstrap map in the declared structural ordering.
    AcceptedStructuralBaseline,
""",
        "stop reason",
    )
    text = replace_once(
        text,
        """    rounds: Vec<BootstrapAggregationRound>,
    split_repair: Option<AggregationRepairResult>,
    work: BootstrapAggregationWorkReport,
}
""",
        """    rounds: Vec<BootstrapAggregationRound>,
    split_repair: Option<AggregationRepairResult>,
    structural_baseline_selected: bool,
    structural_baseline_report: Option<CompatibleRelaxationReport>,
    structural_baseline_decision: Option<CompatibleRelaxationDecision>,
    work: BootstrapAggregationWorkReport,
}
""",
        "result fields",
    )
    text = replace_once(
        text,
        """    pub const fn split_repair(&self) -> Option<&AggregationRepairResult> {
        self.split_repair.as_ref()
    }

    /// Deterministic structural-work and retained-state report.
""",
        """    pub const fn split_repair(&self) -> Option<&AggregationRepairResult> {
        self.split_repair.as_ref()
    }

    /// Whether the final map came from the protected pair-neighborhood baseline.
    #[must_use]
    pub const fn structural_baseline_selected(&self) -> bool {
        self.structural_baseline_selected
    }

    /// Compatible-relaxation report for the protected structural baseline when
    /// it had an admissible nontrivial complement.
    #[must_use]
    pub const fn structural_baseline_report(&self) -> Option<&CompatibleRelaxationReport> {
        self.structural_baseline_report.as_ref()
    }

    /// Acceptance decision for the protected structural baseline.
    #[must_use]
    pub const fn structural_baseline_decision(&self) -> Option<&CompatibleRelaxationDecision> {
        self.structural_baseline_decision.as_ref()
    }

    /// Deterministic structural-work and retained-state report.
""",
        "result accessors",
    )
    old_end = """    let mut final_aggregation =
        final_aggregation.ok_or_else(|| MultiwayError::CompatibleRelaxation {
            message: "bootstrap builder produced no aggregation".to_owned(),
        })?;
    let mut split_repair = None;
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

    let retained_test_vector_bytes = test_vectors
"""
    new_end = """    let mut final_aggregation =
        final_aggregation.ok_or_else(|| MultiwayError::CompatibleRelaxation {
            message: "bootstrap builder produced no aggregation".to_owned(),
        })?;

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
        let current_factor = rounds
            .last()
            .map(|round| {
                round
                    .compatible_decision
                    .maximum_diagonal_factor_per_sweep()
            })
            .unwrap_or(f64::INFINITY);
        let baseline_factor = decision.maximum_diagonal_factor_per_sweep();
        let prefer_baseline = baseline_accepted
            && (!accepted
                || structural_baseline_metrics.coarse_tuple_count
                    < current_metrics.coarse_tuple_count
                || (structural_baseline_metrics.coarse_tuple_count
                    == current_metrics.coarse_tuple_count
                    && (structural_baseline_metrics.coarse_dimension
                        < current_metrics.coarse_dimension
                        || (structural_baseline_metrics.coarse_dimension
                            == current_metrics.coarse_dimension
                            && baseline_factor < current_factor))));
        if prefer_baseline {
            final_aggregation = structural_baseline.clone();
            accepted = true;
            stop_reason = BootstrapAggregationStopReason::AcceptedStructuralBaseline;
            structural_baseline_selected = true;
        }
        structural_baseline_report = Some(report);
        structural_baseline_decision = Some(decision);
    }

    let mut split_repair = None;
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

    let retained_test_vector_bytes = test_vectors
"""
    text = replace_once(text, old_end, new_end, "bootstrap final selection")
    text = replace_once(
        text,
        """    let retained_round_report_bytes_estimate = rounds
        .iter()
        .map(|round| round.compatible_report.retained_bytes_estimate())
        .sum();
""",
        """    let retained_round_report_bytes_estimate = rounds
        .iter()
        .map(|round| round.compatible_report.retained_bytes_estimate())
        .sum::<usize>()
        .saturating_add(
            structural_baseline_report
                .as_ref()
                .map_or(0, CompatibleRelaxationReport::retained_bytes_estimate),
        );
""",
        "retained report bytes",
    )
    text = replace_once(
        text,
        """        compatible_gramian_applications: rounds
            .iter()
            .map(|round| round.compatible_report.gramian_applications())
            .sum(),
        compatible_smoother_applications: rounds
            .iter()
            .map(|round| round.compatible_report.smoother_applications())
            .sum(),
""",
        """        compatible_gramian_applications: rounds
            .iter()
            .map(|round| round.compatible_report.gramian_applications())
            .sum::<usize>()
            .saturating_add(
                structural_baseline_report
                    .as_ref()
                    .map_or(0, CompatibleRelaxationReport::gramian_applications),
            ),
        compatible_smoother_applications: rounds
            .iter()
            .map(|round| round.compatible_report.smoother_applications())
            .sum::<usize>()
            .saturating_add(
                structural_baseline_report
                    .as_ref()
                    .map_or(0, CompatibleRelaxationReport::smoother_applications),
            ),
""",
        "baseline work counts",
    )
    text = replace_once(
        text,
        """        rounds,
        split_repair,
        work,
    })
""",
        """        rounds,
        split_repair,
        structural_baseline_selected,
        structural_baseline_report,
        structural_baseline_decision,
        work,
    })
""",
        "result construction",
    )
    PATH.write_text(text, encoding="utf-8")


def patch_tests() -> None:
    text = TESTS.read_text(encoding="utf-8")
    marker = """#[test]
fn structural_dimension_budget_rejects_before_compatible_acceptance() {
"""
    test = """#[test]
fn accepted_structural_baseline_is_not_replaced_by_a_more_expensive_map() {
    let (problem, oracle) = refined_weak_chain(8, 2, 0.01, true);
    let smoother = DiagonalPreconditioner::new(&problem, 0.5).expect("smoother succeeds");
    let result = build_bootstrap_aggregation(&problem, &smoother, bootstrap_options())
        .expect("bootstrap build succeeds");
    let baseline = result
        .structural_baseline_report()
        .expect("nontrivial structural baseline was evaluated");
    let decision = result
        .structural_baseline_decision()
        .expect("structural baseline decision exists");

    assert!(decision.accepted());
    assert!(baseline.maximum_final_coarse_defect() < 1.0e-10);
    let final_coarse = result
        .final_aggregation()
        .coarsen(&problem)
        .expect("final coarsening succeeds");
    let oracle_coarse = oracle.coarsen(&problem).expect("oracle coarsening succeeds");
    assert!(final_coarse.tuple_count() <= oracle_coarse.tuple_count());
}

"""
    text = replace_once(text, marker, test + marker, "portfolio test")
    TESTS.write_text(text, encoding="utf-8")


def main() -> None:
    patch_source()
    patch_tests()


if __name__ == "__main__":
    main()
