#!/usr/bin/env python3
"""Evaluate the frozen issue #3 v3 selective-cycle holdout."""

from __future__ import annotations

import argparse
import csv
import math
from pathlib import Path
from statistics import median


EXPECTED_SEEDS = set(range(900, 910))
MINIMUM_REFERENCE_ADMISSIBLE = 4
MINIMUM_REFERENCE_INADMISSIBLE = 2
MINIMUM_CONDITIONAL_ACCEPTANCE = 0.80
MINIMUM_MEDIAN_RECOVERY = 0.60
MINIMUM_BOOTSTRAP_IMPROVEMENTS = 2
IMPROVEMENT_THRESHOLD = 0.10
MAXIMUM_BOOTSTRAP_REGRESSION = 0.10
MAXIMUM_RESIDUAL = 1.0e-8
MAXIMUM_PROBE_UNDERESTIMATE = 0.03
MAXIMUM_COMPLEXITY = 1.95


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_directory", type=Path)
    parser.add_argument("output_markdown", type=Path)
    return parser.parse_args()


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    if not rows:
        raise SystemExit(f"empty TSV: {path}")
    return rows


def truth(row: dict[str, str], key: str) -> bool:
    return row[key].strip().lower() == "true"


def number(row: dict[str, str], key: str) -> float | None:
    value = row[key].strip()
    if value in {"", "NA", "NaN", "nan"}:
        return None
    result = float(value)
    return result if math.isfinite(result) else None


def fmt(value: float | None, digits: int = 3) -> str:
    return "—" if value is None else f"{value:.{digits}f}"


def fmt_scientific(value: float | None) -> str:
    return "—" if value is None else f"{value:.3e}"


def main() -> None:
    args = arguments()
    matrix_path = args.input_directory / "issue3-cycle-v3-holdout.tsv"
    traces_path = args.input_directory / "issue3-cycle-v3-traces.tsv"
    timing_path = args.input_directory / "issue3-cycle-v3-timing.tsv"
    rows = read_tsv(matrix_path)
    traces = read_tsv(traces_path)
    timings = read_tsv(timing_path)

    failures: list[str] = []
    if len(rows) != 10:
        failures.append(f"expected 10 cases, found {len(rows)}")
    if len({row["case"] for row in rows}) != len(rows):
        failures.append("case labels are not unique")
    requested_seeds = {int(row["requested_seed"]) for row in rows}
    if requested_seeds != EXPECTED_SEEDS:
        failures.append(
            f"requested seeds differ from frozen 900--909 set: {sorted(requested_seeds)}"
        )
    if any(row["set"] != "cycle-holdout-v3" for row in rows):
        failures.append("one or more rows have the wrong evidence-set label")
    if len(timings) != len(rows):
        failures.append(
            f"expected {len(rows)} descriptive timing rows, found {len(timings)}"
        )

    reference_admissible = [row for row in rows if truth(row, "reference_admissible")]
    reference_inadmissible = [row for row in rows if not truth(row, "reference_admissible")]
    accepted = [row for row in rows if truth(row, "automatic_accepted")]
    accepted_reference = [
        row
        for row in reference_admissible
        if truth(row, "automatic_accepted")
    ]

    if len(reference_admissible) < MINIMUM_REFERENCE_ADMISSIBLE:
        failures.append(
            f"only {len(reference_admissible)} reference-admissible fixtures; "
            f"need at least {MINIMUM_REFERENCE_ADMISSIBLE}"
        )
    if len(reference_inadmissible) < MINIMUM_REFERENCE_INADMISSIBLE:
        failures.append(
            f"only {len(reference_inadmissible)} reference-inadmissible fixtures; "
            f"need at least {MINIMUM_REFERENCE_INADMISSIBLE}"
        )

    conditional_acceptance = (
        len(accepted_reference) / len(reference_admissible)
        if reference_admissible
        else 0.0
    )
    if conditional_acceptance < MINIMUM_CONDITIONAL_ACCEPTANCE:
        failures.append(
            "conditional acceptance rate "
            f"{conditional_acceptance:.3f} is below {MINIMUM_CONDITIONAL_ACCEPTANCE:.3f}"
        )

    recoveries = [
        value
        for row in accepted_reference
        if (value := number(row, "cycle_consistent_recovery")) is not None
    ]
    median_recovery = median(recoveries) if recoveries else None
    if median_recovery is None or median_recovery < MINIMUM_MEDIAN_RECOVERY:
        failures.append(
            "median cycle-consistent recovery is unavailable or below "
            f"{MINIMUM_MEDIAN_RECOVERY:.2f}: {median_recovery}"
        )

    bootstrap_selected = [
        row
        for row in accepted
        if row["automatic_source"] == "bootstrap-final"
    ]
    bootstrap_improvements = [
        row
        for row in bootstrap_selected
        if (value := number(row, "relative_improvement_vs_one_shot")) is not None
        and value >= IMPROVEMENT_THRESHOLD
    ]
    bootstrap_regressions = [
        row
        for row in bootstrap_selected
        if (value := number(row, "relative_improvement_vs_one_shot")) is not None
        and value < -MAXIMUM_BOOTSTRAP_REGRESSION
    ]
    if len(bootstrap_improvements) < MINIMUM_BOOTSTRAP_IMPROVEMENTS:
        failures.append(
            f"only {len(bootstrap_improvements)} accepted bootstrap maps improve one-shot "
            f"by at least {IMPROVEMENT_THRESHOLD:.2f}; need {MINIMUM_BOOTSTRAP_IMPROVEMENTS}"
        )
    if bootstrap_regressions:
        failures.append(
            "accepted bootstrap maps regress more than 10% versus one-shot: "
            + ", ".join(row["case"] for row in bootstrap_regressions)
        )

    if any(not truth(row, "probe_accepted") for row in accepted):
        failures.append("an accepted automatic row has a rejected cycle probe")
    if any(not truth(row, "pcg_converged") for row in accepted):
        failures.append("an accepted automatic row has a nonconverged PCG solve")

    residuals = [
        value
        for row in accepted
        if (value := number(row, "pcg_final_relative_residual")) is not None
    ]
    maximum_residual = max(residuals, default=None)
    if maximum_residual is None or maximum_residual > MAXIMUM_RESIDUAL:
        failures.append(
            f"maximum accepted residual {maximum_residual} exceeds {MAXIMUM_RESIDUAL:.1e}"
        )

    underestimates = [
        value
        for row in accepted
        if (value := number(row, "probe_underestimate_vs_dense")) is not None
    ]
    maximum_underestimate = max(underestimates, default=None)
    if maximum_underestimate is None or maximum_underestimate > MAXIMUM_PROBE_UNDERESTIMATE:
        failures.append(
            "maximum dense-minus-probe underestimate "
            f"{maximum_underestimate} exceeds {MAXIMUM_PROBE_UNDERESTIMATE:.2f}"
        )

    complexities = [
        value
        for row in accepted
        if (value := number(row, "two_level_tuple_complexity")) is not None
    ]
    maximum_complexity = max(complexities, default=None)
    if maximum_complexity is None or maximum_complexity > MAXIMUM_COMPLEXITY:
        failures.append(
            f"maximum accepted tuple complexity {maximum_complexity} exceeds {MAXIMUM_COMPLEXITY:.2f}"
        )

    trace_cases = {row["case"] for row in traces}
    accepted_cases = {row["case"] for row in accepted}
    if trace_cases != accepted_cases:
        failures.append(
            "trace case set does not equal accepted case set: "
            f"traces={sorted(trace_cases)}, accepted={sorted(accepted_cases)}"
        )

    map_count = sum(row["automatic_smoother"] == "symmetric-map" for row in accepted)
    pair_count = sum(row["automatic_smoother"] == "all-pairs-cmg" for row in accepted)
    bootstrap_count = len(bootstrap_selected)
    structural_count = sum(
        row["automatic_source"] == "structural-baseline" for row in accepted
    )
    gate_passed = not failures

    lines = [
        "# Issue #3 selective-cycle holdout v3",
        "",
        "## Verdict",
        "",
        (
            "**PASS.** The frozen v3 selective automatic-coarsening policy satisfies "
            "every predeclared gate."
            if gate_passed
            else "**NEGATIVE RESULT.** The frozen v3 policy fails one or more predeclared gates."
        ),
        "",
        "The fixtures, smoother order, structural limits, complete-cycle thresholds,",
        "and scientific gates were committed before seeds `900`–`909` were evaluated.",
        "Reference admissibility is conditional: the retained generating map is an exact",
        "fiber partition, but is not assumed to be the globally optimal hard map.",
        "",
        "## Case matrix",
        "",
        "| Case | Family | Reference admissible | Automatic | Smoother | Source | κ baseline | κ reference | κ automatic | Recovery | vs one-shot | Probe factor | PCG residual |",
        "|---|---|---:|---:|---|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row in rows:
        lines.append(
            "| {case} | {family} | {reference} | {automatic} | `{smoother}` | `{source}` | {baseline} | {reference_condition} | {candidate} | {recovery} | {one_shot} | {probe} | {residual} |".format(
                case=row["case"],
                family=row["family"],
                reference="Yes" if truth(row, "reference_admissible") else "No",
                automatic="Yes" if truth(row, "automatic_accepted") else "No",
                smoother=row["automatic_smoother"],
                source=row["automatic_source"],
                baseline=fmt(number(row, "baseline_condition")),
                reference_condition=fmt(number(row, "reference_same_smoother_condition")),
                candidate=fmt(number(row, "candidate_condition")),
                recovery=fmt(number(row, "cycle_consistent_recovery")),
                one_shot=fmt(number(row, "relative_improvement_vs_one_shot")),
                probe=fmt(number(row, "probe_estimated_energy_factor")),
                residual=fmt_scientific(number(row, "pcg_final_relative_residual")),
            )
        )

    lines.extend(
        [
            "",
            "## Aggregate gates",
            "",
            f"- Reference-admissible fixtures: **{len(reference_admissible)} of {len(rows)}**; required at least {MINIMUM_REFERENCE_ADMISSIBLE}.",
            f"- Reference-inadmissible fixtures: **{len(reference_inadmissible)} of {len(rows)}**; required at least {MINIMUM_REFERENCE_INADMISSIBLE}.",
            f"- Conditional automatic acceptance: **{len(accepted_reference)} of {len(reference_admissible)}** = `{conditional_acceptance:.3f}`; required `{MINIMUM_CONDITIONAL_ACCEPTANCE:.2f}`.",
            f"- Median cycle-consistent reference recovery: `{fmt(median_recovery)}`; required `{MINIMUM_MEDIAN_RECOVERY:.2f}`.",
            f"- Accepted bootstrap maps improving one-shot by at least 10%: **{len(bootstrap_improvements)}**; required {MINIMUM_BOOTSTRAP_IMPROVEMENTS}.",
            f"- Accepted bootstrap regressions worse than 10%: **{len(bootstrap_regressions)}**; required zero.",
            f"- Maximum accepted true residual: `{fmt_scientific(maximum_residual)}`; limit `{MAXIMUM_RESIDUAL:.1e}`.",
            f"- Maximum probe underestimate versus dense radius: `{fmt(maximum_underestimate)}`; limit `{MAXIMUM_PROBE_UNDERESTIMATE:.2f}`.",
            f"- Maximum accepted two-level tuple complexity: `{fmt(maximum_complexity)}`; limit `{MAXIMUM_COMPLEXITY:.2f}`.",
            f"- Selected smoother counts: MAP `{map_count}`, pair-CMG `{pair_count}`.",
            f"- Selected source counts: bootstrap `{bootstrap_count}`, protected structural baseline `{structural_count}`.",
            "",
            "## Determinism and correctness",
            "",
            "The authoritative workflow executes the holdout twice and byte-compares the",
            "matrix and true-residual traces. Every accepted row must independently pass",
            "the matrix-free complete-cycle probe, dense quotient-space spectral analysis,",
            "hard structural gates, and traced PCG with the original Gramian.",
            "",
        ]
    )
    if failures:
        lines.extend(["## Failed gates", ""])
        lines.extend(f"- {failure}" for failure in failures)
        lines.append("")
    lines.extend(
        [
            "## Interpretation",
            "",
            "A reference-inadmissible rejection is an intended fail-closed outcome, not a",
            "forced regression. Acceptance on such a case is allowed only when an",
            "alternative automatic hard map passes the same independent complete-cycle and",
            "correctness checks. The report does not use those cases in the conditional",
            "reference-recovery median.",
            "",
            "Descriptive setup timings are retained separately and are not byte-compared or",
            "used in any routing or scientific decision.",
            "",
        ]
    )

    args.output_markdown.parent.mkdir(parents=True, exist_ok=True)
    args.output_markdown.write_text("\n".join(lines), encoding="utf-8")
    raise SystemExit(0 if gate_passed else 1)


if __name__ == "__main__":
    main()
