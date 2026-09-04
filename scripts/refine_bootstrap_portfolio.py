"""Use Pareto dominance rather than lexicographic tuple count in the bootstrap portfolio."""

from pathlib import Path

PATH = Path("crates/multiway-mg/src/bootstrap.rs")


def main() -> None:
    text = PATH.read_text(encoding="utf-8")
    old = """        let prefer_baseline = baseline_accepted
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
"""
    new = """        let baseline_no_worse = structural_baseline_metrics.coarse_tuple_count
            <= current_metrics.coarse_tuple_count
            && structural_baseline_metrics.coarse_dimension <= current_metrics.coarse_dimension
            && baseline_factor <= current_factor;
        let baseline_strictly_better = structural_baseline_metrics.coarse_tuple_count
            < current_metrics.coarse_tuple_count
            || structural_baseline_metrics.coarse_dimension < current_metrics.coarse_dimension
            || baseline_factor < current_factor;
        let prefer_baseline = baseline_accepted
            && (!accepted || (baseline_no_worse && baseline_strictly_better));
"""
    if text.count(old) != 1:
        raise RuntimeError(f"portfolio block count was {text.count(old)}")
    PATH.write_text(text.replace(old, new), encoding="utf-8")


if __name__ == "__main__":
    main()
