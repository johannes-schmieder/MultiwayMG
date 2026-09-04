"""Apply fail-closed and strict-Clippy repairs for cycle-split development."""

from pathlib import Path


PATH = Path("crates/multiway-mg/src/cycle_repair.rs")


def replace_once(text: str, old: str, new: str, description: str) -> str:
    if old in text:
        return text.replace(old, new, 1)
    if new in text:
        return text
    raise RuntimeError(f"{description} marker was not found")


def main() -> None:
    text = PATH.read_text(encoding="utf-8")
    text = replace_once(
        text,
        "#[derive(Debug, Clone, Copy, PartialEq)]\npub struct CycleSplitRepairOptions",
        "#[derive(Debug, Clone, Copy)]\npub struct CycleSplitRepairOptions",
        "repair-options derive",
    )
    old_initial = """    let mut current_metrics = structural_metrics(problem, &current_aggregation)?;
    validate_metrics(current_metrics, maximum_coarse_dimension, options).map_err(|reason| {
        MultiwayError::InvalidAggregation {
            message: format!("initial cycle-split map violates structural budgets: {reason:?}"),
        }
    })?;
    let current_cycle = build_cycle(&current_aggregation)?;
    let mut current_report = analyze_cycle_quality(problem, &current_cycle, options.probe)?;
    let mut current_decision = evaluate_cycle_quality(&current_report, options.criteria)?;
    let initial_metrics = current_metrics;
    let initial_report = current_report.clone();
    let initial_decision = current_decision.clone();
    let mut rounds = Vec::with_capacity(options.maximum_rounds);
"""
    new_initial = """    let mut current_metrics = structural_metrics(problem, &current_aggregation)?;
    let current_cycle = build_cycle(&current_aggregation)?;
    let mut current_report = analyze_cycle_quality(problem, &current_cycle, options.probe)?;
    let mut current_decision = evaluate_cycle_quality(&current_report, options.criteria)?;
    let initial_metrics = current_metrics;
    let initial_report = current_report.clone();
    let initial_decision = current_decision.clone();
    let mut rounds = Vec::with_capacity(options.maximum_rounds);
    if let Err(reason) = validate_metrics(current_metrics, maximum_coarse_dimension, options) {
        return Ok(result(
            initial_aggregation,
            current_aggregation,
            initial_metrics,
            current_metrics,
            initial_report,
            initial_decision,
            current_report,
            current_decision,
            rounds,
            reason,
        ));
    }
"""
    text = replace_once(
        text,
        old_initial,
        new_initial,
        "initial structural-budget result",
    )
    text = replace_once(
        text,
        """    CycleSplitRepairResult {
        initial_aggregation,
""",
        """    let accepted_splits = rounds.len();
    CycleSplitRepairResult {
        initial_aggregation,
""",
        "accepted-split count",
    )
    text = replace_once(
        text,
        """        accepted_splits: rounds.len(),
        stop_reason,
        rounds,
""",
        """        accepted_splits,
        stop_reason,
        rounds,
""",
        "moved rounds length",
    )
    text = replace_once(
        text,
        """                    || (score_fraction.to_bits() == current.0.to_bits()
                        && (factor, parent) < (current.1, current.2 as usize))
""",
        """                    || score_fraction.to_bits() == current.0.to_bits()
                        && (factor, parent) < (current.1, current.2 as usize)
""",
        "redundant tie-break parentheses",
    )
    PATH.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
