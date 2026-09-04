"""Apply the initial fail-closed conversion for cycle-split structural errors."""

from pathlib import Path


PATH = Path("crates/multiway-mg/src/cycle_repair.rs")


def main() -> None:
    text = PATH.read_text(encoding="utf-8")
    old = """    validate_metrics(current_metrics, maximum_coarse_dimension, options)?;
"""
    new = """    validate_metrics(current_metrics, maximum_coarse_dimension, options).map_err(|reason| {
        MultiwayError::InvalidAggregation {
            message: format!(
                "initial cycle-split map violates structural budgets: {reason:?}"
            ),
        }
    })?;
"""
    if old not in text:
        if new in text:
            return
        raise RuntimeError("initial cycle-split validation marker was not found")
    PATH.write_text(text.replace(old, new, 1), encoding="utf-8")


if __name__ == "__main__":
    main()
