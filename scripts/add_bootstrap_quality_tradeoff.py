"""Add explicit quality-for-size thresholds to bootstrap portfolio selection."""

from __future__ import annotations

import re
from pathlib import Path

BOOTSTRAP = Path("crates/multiway-mg/src/bootstrap.rs")
COMPLETION = Path("crates/multiway-mg/examples/issue3_completion_matrix.rs")
RUST_ROOT = Path("crates/multiway-mg")

FIELDS = """    /// Required multiplicative improvement in the conservative compatible-
    /// relaxation factor before a structurally larger baseline may replace an
    /// accepted bootstrap map. A value below one requires a real quality gain.
    pub structural_baseline_required_factor_ratio: f64,
    /// Largest extra coarse coefficient dimension, divided by fine dimension,
    /// admitted for a materially better structural baseline.
    pub structural_baseline_maximum_dimension_overhead_ratio: f64,
    /// Largest extra coarse tuple count, divided by fine tuple count, admitted
    /// for a materially better structural baseline.
    pub structural_baseline_maximum_tuple_overhead_ratio: f64,
"""

VALUES = """            structural_baseline_required_factor_ratio: 0.97,
            structural_baseline_maximum_dimension_overhead_ratio: 0.05,
            structural_baseline_maximum_tuple_overhead_ratio: 0.10,
"""


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new)


def patch_bootstrap() -> None:
    text = BOOTSTRAP.read_text(encoding="utf-8")
    text = replace_once(
        text,
        """    /// Weight on repeated adjacency in test-vector orderings.
    pub signature_hit_weight: f64,
    /// Compatible-relaxation experiment used to screen each proposed map.
""",
        """    /// Weight on repeated adjacency in test-vector orderings.
    pub signature_hit_weight: f64,
""" + FIELDS + """    /// Compatible-relaxation experiment used to screen each proposed map.
""",
        "option fields",
    )

    text = replace_once(
        text,
        """        validate_unit_interval(
            "maximum_coarse_dimension_ratio",
            self.maximum_coarse_dimension_ratio,
            false,
        )?;
""",
        """        validate_unit_interval(
            "structural_baseline_required_factor_ratio",
            self.structural_baseline_required_factor_ratio,
            false,
        )?;
        validate_unit_interval(
            "structural_baseline_maximum_dimension_overhead_ratio",
            self.structural_baseline_maximum_dimension_overhead_ratio,
            true,
        )?;
        validate_unit_interval(
            "structural_baseline_maximum_tuple_overhead_ratio",
            self.structural_baseline_maximum_tuple_overhead_ratio,
            true,
        )?;
        validate_unit_interval(
            "maximum_coarse_dimension_ratio",
            self.maximum_coarse_dimension_ratio,
            false,
        )?;
""",
        "option validation",
    )

    old_selection = """        let baseline_no_worse = structural_baseline_metrics.coarse_tuple_count
            <= current_metrics.coarse_tuple_count
            && structural_baseline_metrics.coarse_dimension <= current_metrics.coarse_dimension
            && baseline_factor <= current_factor;
        let baseline_strictly_better = structural_baseline_metrics.coarse_tuple_count
            < current_metrics.coarse_tuple_count
            || structural_baseline_metrics.coarse_dimension < current_metrics.coarse_dimension
            || baseline_factor < current_factor;
        let prefer_baseline =
            baseline_accepted && (!accepted || (baseline_no_worse && baseline_strictly_better));
"""
    new_selection = """        let baseline_no_worse = structural_baseline_metrics.coarse_tuple_count
            <= current_metrics.coarse_tuple_count
            && structural_baseline_metrics.coarse_dimension <= current_metrics.coarse_dimension
            && baseline_factor <= current_factor;
        let baseline_strictly_better = structural_baseline_metrics.coarse_tuple_count
            < current_metrics.coarse_tuple_count
            || structural_baseline_metrics.coarse_dimension < current_metrics.coarse_dimension
            || baseline_factor < current_factor;
        let dimension_overhead = structural_baseline_metrics
            .coarse_dimension
            .saturating_sub(current_metrics.coarse_dimension) as f64
            / problem.dimension() as f64;
        let tuple_overhead = structural_baseline_metrics
            .coarse_tuple_count
            .saturating_sub(current_metrics.coarse_tuple_count) as f64
            / problem.tuple_count() as f64;
        let quality_for_size_tradeoff = baseline_factor
            <= current_factor * options.structural_baseline_required_factor_ratio
            && dimension_overhead
                <= options.structural_baseline_maximum_dimension_overhead_ratio
            && tuple_overhead <= options.structural_baseline_maximum_tuple_overhead_ratio;
        let prefer_baseline = baseline_accepted
            && (!accepted
                || (baseline_no_worse && baseline_strictly_better)
                || quality_for_size_tradeoff);
"""
    text = replace_once(text, old_selection, new_selection, "portfolio selection")
    BOOTSTRAP.write_text(text, encoding="utf-8")


def patch_literals() -> None:
    pattern = re.compile(
        r"(?P<indent>^[ \t]+)signature_hit_weight: (?P<value>[^,\n]+),\n"
        r"(?P=indent)compatible_relaxation:",
        re.MULTILINE,
    )
    for path in sorted(RUST_ROOT.rglob("*.rs")):
        text = path.read_text(encoding="utf-8")
        if "BootstrapAggregationOptions" not in text:
            continue
        def replacement(match: re.Match[str]) -> str:
            indent = match.group("indent")
            return (
                f"{indent}signature_hit_weight: {match.group('value')},\n"
                f"{indent}structural_baseline_required_factor_ratio: 0.97,\n"
                f"{indent}structural_baseline_maximum_dimension_overhead_ratio: 0.05,\n"
                f"{indent}structural_baseline_maximum_tuple_overhead_ratio: 0.10,\n"
                f"{indent}compatible_relaxation:"
            )
        updated, count = pattern.subn(replacement, text)
        if path == BOOTSTRAP and count != 1:
            raise RuntimeError(f"bootstrap default literal count was {count}")
        if count:
            path.write_text(updated, encoding="utf-8")


def patch_policy() -> None:
    text = COMPLETION.read_text(encoding="utf-8")
    marker = """        (
            "maximum_bootstrap_witnesses",
            options.maximum_bootstrap_witnesses.to_string(),
        ),
"""
    addition = marker + """        (
            "structural_baseline_required_factor_ratio",
            options.structural_baseline_required_factor_ratio.to_string(),
        ),
        (
            "structural_baseline_maximum_dimension_overhead_ratio",
            options
                .structural_baseline_maximum_dimension_overhead_ratio
                .to_string(),
        ),
        (
            "structural_baseline_maximum_tuple_overhead_ratio",
            options
                .structural_baseline_maximum_tuple_overhead_ratio
                .to_string(),
        ),
"""
    text = replace_once(text, marker, addition, "completion policy fields")
    COMPLETION.write_text(text, encoding="utf-8")


def main() -> None:
    patch_bootstrap()
    patch_literals()
    patch_policy()


if __name__ == "__main__":
    main()
