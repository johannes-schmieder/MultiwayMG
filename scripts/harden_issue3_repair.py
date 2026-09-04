"""Make current-map structural budgets authoritative before repair acceptance."""

from pathlib import Path

PATH = Path("crates/multiway-mg/src/repair.rs")
TESTS = Path("crates/multiway-mg/tests/repair.rs")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new)


def patch_source() -> None:
    text = PATH.read_text(encoding="utf-8")
    marker = """        let decision = evaluate_compatible_relaxation(&report, options.criteria)?;
        if decision.accepted() {
"""
    replacement = """        let decision = evaluate_compatible_relaxation(&report, options.criteria)?;
        if coarse_dimension > maximum_coarse_dimension {
            rounds.push(AggregationRepairRound {
                index: round_index,
                coarse_dimension,
                coarse_tuple_count,
                coarse_dimension_ratio: metrics.coarse_dimension_ratio,
                tuple_reduction: metrics.tuple_reduction,
                two_level_tuple_complexity: metrics.two_level_tuple_complexity,
                report,
                decision,
                proposed_split: None,
            });
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::CoarseDimensionBudget {
                    attempted_dimension: coarse_dimension,
                    maximum_dimension: maximum_coarse_dimension,
                },
                rounds,
            });
        }
        if metrics.tuple_reduction < options.minimum_tuple_reduction {
            rounds.push(AggregationRepairRound {
                index: round_index,
                coarse_dimension,
                coarse_tuple_count,
                coarse_dimension_ratio: metrics.coarse_dimension_ratio,
                tuple_reduction: metrics.tuple_reduction,
                two_level_tuple_complexity: metrics.two_level_tuple_complexity,
                report,
                decision,
                proposed_split: None,
            });
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::TupleReductionBudget {
                    attempted_reduction: metrics.tuple_reduction,
                    minimum_reduction: options.minimum_tuple_reduction,
                },
                rounds,
            });
        }
        if metrics.two_level_tuple_complexity > options.maximum_two_level_tuple_complexity {
            rounds.push(AggregationRepairRound {
                index: round_index,
                coarse_dimension,
                coarse_tuple_count,
                coarse_dimension_ratio: metrics.coarse_dimension_ratio,
                tuple_reduction: metrics.tuple_reduction,
                two_level_tuple_complexity: metrics.two_level_tuple_complexity,
                report,
                decision,
                proposed_split: None,
            });
            return Ok(AggregationRepairResult {
                initial_aggregation: initial,
                final_aggregation: current,
                accepted: false,
                accepted_splits: round_index,
                stop_reason: AggregationRepairStopReason::TupleComplexityBudget {
                    attempted_complexity: metrics.two_level_tuple_complexity,
                    maximum_complexity: options.maximum_two_level_tuple_complexity,
                },
                rounds,
            });
        }
        if decision.accepted() {
"""
    text = replace_once(text, marker, replacement, "current structural budgets")
    text = text.replace(
        "    /// The proposed split would make the coarse space too large.\n",
        "    /// The current map or proposed split makes the coarse space too large.\n",
    )
    text = text.replace(
        "    /// The proposed split would leave too many unique coarse tuples.\n",
        "    /// The current map or proposed split leaves too many unique coarse tuples.\n",
    )
    text = text.replace(
        "    /// The proposed split would exceed the two-level tuple-work budget.\n",
        "    /// The current map or proposed split exceeds the two-level tuple-work budget.\n",
    )
    PATH.write_text(text, encoding="utf-8")


def patch_tests() -> None:
    text = TESTS.read_text(encoding="utf-8")
    marker = """#[test]
fn coarse_dimension_budget_rejects_a_split_before_mutation() {
"""
    test = """#[test]
fn structurally_overlarge_current_map_is_rejected_even_when_compatible() {
    let (problem, oracle) = refined_weak_chain(8, 2, 0.01);
    let initial_dimension: usize = oracle.coarse_counts().iter().sum();
    let smoother =
        DiagonalPreconditioner::new(&problem, 0.5).expect("diagonal smoother succeeds");
    let result = repair_aggregation_by_splitting(
        &problem,
        &oracle,
        &smoother,
        repair_options(12, 0.49),
    )
    .expect("structural rejection returns a decision");

    assert!(!result.accepted());
    assert_eq!(result.accepted_splits(), 0);
    assert_eq!(result.final_aggregation(), &oracle);
    assert!(result.rounds()[0].decision().accepted());
    assert!(matches!(
        result.stop_reason(),
        AggregationRepairStopReason::CoarseDimensionBudget {
            attempted_dimension,
            maximum_dimension,
        } if *attempted_dimension == initial_dimension
            && *maximum_dimension < initial_dimension
    ));
}

"""
    text = replace_once(text, marker, test + marker, "insert structural current-map test")
    TESTS.write_text(text, encoding="utf-8")


def main() -> None:
    patch_source()
    patch_tests()


if __name__ == "__main__":
    main()
