"""Return initial cycle-split structural-budget failures as normal results."""

from pathlib import Path


PATH = Path("crates/multiway-mg/src/cycle_repair.rs")


def main() -> None:
    text = PATH.read_text(encoding="utf-8")
    old = """    let mut current_metrics = structural_metrics(problem, &current_aggregation)?;
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
    new = """    let mut current_metrics = structural_metrics(problem, &current_aggregation)?;
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
    if new in text:
        return
    if text.count(old) != 1:
        raise RuntimeError("initial cycle-split validation block was not unique")
    PATH.write_text(text.replace(old, new, 1), encoding="utf-8")


if __name__ == "__main__":
    main()
