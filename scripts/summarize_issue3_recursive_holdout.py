#!/usr/bin/env python3
"""Evaluate the frozen recursive issue #3 holdout."""

from __future__ import annotations

import argparse
import csv
import math
from collections import Counter, defaultdict
from pathlib import Path
from statistics import median


METHODS = (
    "oracle-map-hierarchy",
    "recursive-one-shot",
    "cycle-screened-automatic",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_directory", type=Path)
    parser.add_argument("output_markdown", type=Path)
    parser.add_argument("--minimum-accepted", type=int, default=6)
    parser.add_argument("--minimum-median-recovery", type=float, default=0.60)
    parser.add_argument("--minimum-improvement-cases", type=int, default=2)
    parser.add_argument("--minimum-improvement", type=float, default=0.10)
    parser.add_argument("--maximum-regression", type=float, default=0.10)
    parser.add_argument("--maximum-residual", type=float, default=1.0e-8)
    parser.add_argument("--maximum-dimension-complexity", type=float, default=2.25)
    parser.add_argument("--maximum-tuple-complexity", type=float, default=2.25)
    return parser.parse_args()


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    if not rows:
        raise RuntimeError(f"no rows found in {path}")
    required = {
        "case",
        "family",
        "requested_depth",
        "method",
        "accepted",
        "achieved_depth",
        "stop_reason",
        "dimension_complexity",
        "tuple_complexity",
        "baseline_condition",
        "oracle_condition",
        "candidate_condition",
        "oracle_improvement_recovered",
        "pcg_converged",
        "pcg_final_relative_residual",
        "level_sources",
        "level_cycle_factors",
    }
    missing = required.difference(rows[0])
    if missing:
        raise RuntimeError(f"missing columns in {path}: {sorted(missing)}")
    return rows


def optional_number(row: dict[str, str], key: str) -> float | None:
    value = row[key].strip()
    if value in {"", "NA", "NaN", "nan", "None"}:
        return None
    number = float(value)
    return number if math.isfinite(number) else None


def required_number(row: dict[str, str], key: str) -> float:
    number = optional_number(row, key)
    if number is None:
        raise RuntimeError(f"{row['case']} {row['method']} lacks finite {key}")
    return number


def boolean(row: dict[str, str], key: str) -> bool:
    value = row[key].strip().lower()
    if value == "true":
        return True
    if value == "false":
        return False
    raise RuntimeError(f"{row['case']} {row['method']} has invalid {key}={row[key]!r}")


def fmt(value: float | None, digits: int = 3) -> str:
    if value is None:
        return "—"
    if abs(value) >= 1000:
        return f"{value:,.0f}"
    if abs(value) >= 100:
        return f"{value:.1f}"
    if abs(value) >= 10:
        return f"{value:.2f}"
    return f"{value:.{digits}f}"


def pass_text(flag: bool) -> str:
    return "PASS" if flag else "FAIL"


def main() -> None:
    args = parse_args()
    rows = read_rows(args.input_directory / "issue3-recursive-holdout.tsv")
    by_case: dict[str, dict[str, dict[str, str]]] = defaultdict(dict)
    for row in rows:
        if row["method"] in by_case[row["case"]]:
            raise RuntimeError(f"duplicate {row['case']} {row['method']} row")
        by_case[row["case"]][row["method"]] = row
    cases = sorted(by_case)
    if len(cases) != 8:
        raise RuntimeError(f"expected 8 recursive fixtures, found {len(cases)}")
    for case in cases:
        missing = set(METHODS).difference(by_case[case])
        if missing:
            raise RuntimeError(f"{case} is missing methods {sorted(missing)}")

    oracle_rows = [by_case[case]["oracle-map-hierarchy"] for case in cases]
    one_shot_rows = [by_case[case]["recursive-one-shot"] for case in cases]
    automatic_rows = [by_case[case]["cycle-screened-automatic"] for case in cases]

    oracle_gate = all(
        boolean(row, "accepted")
        and int(row["achieved_depth"]) == int(row["requested_depth"])
        and boolean(row, "pcg_converged")
        and required_number(row, "pcg_final_relative_residual") <= args.maximum_residual
        and required_number(row, "dimension_complexity") <= args.maximum_dimension_complexity
        and required_number(row, "tuple_complexity") <= args.maximum_tuple_complexity
        for row in oracle_rows
    )

    accepted_automatic = [row for row in automatic_rows if boolean(row, "accepted")]
    accepted_count = len(accepted_automatic)
    accepted_count_gate = accepted_count >= args.minimum_accepted
    depth_gate = all(
        int(row["achieved_depth"]) == int(row["requested_depth"])
        and "ReachedTerminal" in row["stop_reason"]
        for row in accepted_automatic
    )
    recoveries = [
        required_number(row, "oracle_improvement_recovered")
        for row in accepted_automatic
    ]
    median_recovery = median(recoveries) if recoveries else float("-inf")
    recovery_gate = median_recovery >= args.minimum_median_recovery

    improvement_cases = 0
    regressions: list[tuple[str, float, float]] = []
    for case in cases:
        one = by_case[case]["recursive-one-shot"]
        automatic = by_case[case]["cycle-screened-automatic"]
        if not (boolean(one, "accepted") and boolean(automatic, "accepted")):
            continue
        one_recovery = required_number(one, "oracle_improvement_recovered")
        automatic_recovery = required_number(automatic, "oracle_improvement_recovered")
        if automatic_recovery - one_recovery >= args.minimum_improvement:
            improvement_cases += 1
        if one_recovery - automatic_recovery > args.maximum_regression:
            regressions.append((case, one_recovery, automatic_recovery))
    improvement_gate = improvement_cases >= args.minimum_improvement_cases
    regression_gate = not regressions

    active_rows = [row for row in rows if boolean(row, "accepted")]
    convergence_gate = all(boolean(row, "pcg_converged") for row in active_rows)
    residuals = [required_number(row, "pcg_final_relative_residual") for row in active_rows]
    residual_gate = bool(residuals) and max(residuals) <= args.maximum_residual
    complexity_gate = all(
        required_number(row, "dimension_complexity") <= args.maximum_dimension_complexity
        and required_number(row, "tuple_complexity") <= args.maximum_tuple_complexity
        for row in active_rows
    )
    finite_level_factors = []
    for row in automatic_rows:
        for value in row["level_cycle_factors"].split(";"):
            value = value.strip()
            if value not in {"", "NA", "None"}:
                number = float(value)
                if math.isfinite(number):
                    finite_level_factors.append(number)
    level_factor_gate = all(value <= 0.50 + 1.0e-12 for value in finite_level_factors)

    source_counts = Counter()
    for row in accepted_automatic:
        for source in filter(None, row["level_sources"].split(";")):
            source_counts[source] += 1
    maximum_residual = max(residuals, default=float("nan"))
    maximum_dimension_complexity = max(
        (required_number(row, "dimension_complexity") for row in active_rows),
        default=float("nan"),
    )
    maximum_tuple_complexity = max(
        (required_number(row, "tuple_complexity") for row in active_rows),
        default=float("nan"),
    )

    gates = [
        ("All eight supplied oracle hierarchies reach their requested depth and pass residual/complexity checks", oracle_gate),
        (f"At least {args.minimum_accepted} of 8 automatic hierarchies are accepted", accepted_count_gate),
        ("Every accepted automatic hierarchy reaches the exact requested terminal depth", depth_gate),
        (f"Median accepted automatic oracle-recovery fraction is at least {args.minimum_median_recovery:.2f}", recovery_gate),
        (f"Automatic hierarchy improves recursive one-shot recovery by at least {args.minimum_improvement:.2f} in at least {args.minimum_improvement_cases} cases", improvement_gate),
        (f"No accepted automatic hierarchy regresses more than {args.maximum_regression:.2f} below an accepted recursive one-shot hierarchy", regression_gate),
        ("Every accepted traced PCG solve converges", convergence_gate),
        (f"Every accepted final true residual is at most {args.maximum_residual:.1e}", residual_gate),
        ("Every accepted hierarchy respects cumulative dimension and tuple budgets", complexity_gate),
        ("Every reported accepted level satisfies the frozen 0.50 complete-cycle factor gate", level_factor_gate),
    ]
    passed = all(flag for _, flag in gates)

    lines = [
        "# Issue #3 recursive complete-cycle holdout results",
        "",
        "## Verdict",
        "",
        (
            "**The frozen recursive automatic-coarsening gate passes.**"
            if passed
            else "**The frozen recursive automatic-coarsening gate does not pass.**"
        ),
        "",
        "The holdout uses the predeclared seeded multi-level graph covers and policy",
        "in `benchmarks/policies/issue3-recursive-cycle-v1.tsv`. Each automatic level",
        "is proposed by conservative bootstrap/repair, screened through its actual",
        "symmetric-MAP two-grid cycle, and admitted only after cumulative hierarchy",
        "dimension and tuple budgets remain valid.",
        "",
        "## Case matrix",
        "",
        "| Case | Depth | One-shot accepted | One-shot recovery | Automatic accepted | Automatic depth | Automatic recovery | Oracle κ | Automatic κ | Dimension complexity | Tuple complexity | PCG iterations |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for case in cases:
        oracle = by_case[case]["oracle-map-hierarchy"]
        one = by_case[case]["recursive-one-shot"]
        automatic = by_case[case]["cycle-screened-automatic"]
        lines.append(
            "| {case} | {depth} | {one_accepted} | {one_recovery} | {auto_accepted} | {auto_depth} | {auto_recovery} | {oracle_condition} | {auto_condition} | {dim_complexity} | {tuple_complexity} | {iterations} |".format(
                case=case,
                depth=automatic["requested_depth"],
                one_accepted=boolean(one, "accepted"),
                one_recovery=fmt(optional_number(one, "oracle_improvement_recovered")),
                auto_accepted=boolean(automatic, "accepted"),
                auto_depth=automatic["achieved_depth"],
                auto_recovery=fmt(optional_number(automatic, "oracle_improvement_recovered")),
                oracle_condition=fmt(optional_number(oracle, "candidate_condition")),
                auto_condition=fmt(optional_number(automatic, "candidate_condition")),
                dim_complexity=fmt(optional_number(automatic, "dimension_complexity")),
                tuple_complexity=fmt(optional_number(automatic, "tuple_complexity")),
                iterations=automatic["pcg_iterations"],
            )
        )

    lines.extend(
        [
            "",
            "## Aggregate diagnostics",
            "",
            f"- Accepted automatic hierarchies: **{accepted_count} of {len(cases)}**.",
            f"- Median accepted automatic oracle-recovery fraction: **{fmt(median_recovery)}**.",
            f"- Cases improving recursive one-shot recovery by at least {args.minimum_improvement:.2f}: **{improvement_cases}**.",
            f"- Maximum accepted true residual: `{maximum_residual:.3e}`.",
            f"- Maximum accepted dimension complexity: `{maximum_dimension_complexity:.3f}`.",
            f"- Maximum accepted tuple complexity: `{maximum_tuple_complexity:.3f}`.",
            "- Selected level-source counts: "
            + (", ".join(f"`{source}` {count}" for source, count in sorted(source_counts.items())) or "none")
            + ".",
            "",
            "## Scientific gates",
            "",
        ]
    )
    lines.extend(f"- [{pass_text(flag)}] {description}." for description, flag in gates)
    if regressions:
        lines.extend(["", "## Material regressions", ""])
        for case, one, automatic in regressions:
            lines.append(
                f"- `{case}`: recursive one-shot recovery `{one:.3f}`, automatic recovery `{automatic:.3f}`."
            )
    lines.extend(
        [
            "",
            "## Interpretation",
            "",
            "A passing result demonstrates that the one-level acceptance rule composes into",
            "a bounded recursive hierarchy on unseen synthetic graph covers. It remains a",
            "research feasibility result: production runtime, allocation-free workspaces,",
            "large approximate pair solvers, and fereg's independent observation-space",
            "certificate are separate milestones.",
            "",
        ]
    )

    args.output_markdown.parent.mkdir(parents=True, exist_ok=True)
    args.output_markdown.write_text("\n".join(lines), encoding="utf-8")
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
