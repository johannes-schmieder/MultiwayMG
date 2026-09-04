#!/usr/bin/env python3
"""Evaluate the frozen issue #3 complete-cycle holdout.

The script uses only the Python standard library. It writes a complete report
before returning a nonzero status when a scientific gate fails, allowing the
negative result to be preserved without silently changing the frozen policy.
"""

from __future__ import annotations

import argparse
import csv
import math
from collections import Counter, defaultdict
from pathlib import Path
from statistics import median


METHODS = (
    "oracle",
    "one-shot-pair-neighborhood",
    "primary-bootstrap-final",
    "cycle-portfolio-final",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_directory", type=Path)
    parser.add_argument("output_markdown", type=Path)
    parser.add_argument("--minimum-accepted", type=int, default=8)
    parser.add_argument("--minimum-median-recovery", type=float, default=0.60)
    parser.add_argument("--minimum-improvement-cases", type=int, default=2)
    parser.add_argument("--minimum-improvement", type=float, default=0.10)
    parser.add_argument("--maximum-regression", type=float, default=0.10)
    parser.add_argument("--maximum-residual", type=float, default=1.0e-8)
    parser.add_argument("--maximum-probe-underestimate", type=float, default=0.03)
    parser.add_argument("--maximum-tuple-complexity", type=float, default=1.95)
    return parser.parse_args()


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    if not rows:
        raise RuntimeError(f"no rows found in {path}")
    required = {
        "set",
        "case",
        "family",
        "method",
        "accepted",
        "structural_admissible",
        "selected_source",
        "coarse_dimension",
        "coarse_tuples",
        "two_level_tuple_complexity",
        "baseline_map_condition",
        "oracle_two_grid_condition",
        "candidate_condition",
        "exact_cycle_error_radius",
        "probe_estimated_energy_factor",
        "probe_underestimate",
        "cycle_probe_accepted",
        "oracle_improvement_recovered",
        "pcg_converged",
        "pcg_final_relative_residual",
        "portfolio_cycle_build_failures",
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
    if not math.isfinite(number):
        return None
    return number


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
    matrix_path = args.input_directory / "issue3-cycle-holdout.tsv"
    rows = read_rows(matrix_path)
    by_case: dict[str, dict[str, dict[str, str]]] = defaultdict(dict)
    for row in rows:
        method = row["method"]
        if method in by_case[row["case"]]:
            raise RuntimeError(f"duplicate {row['case']} {method} row")
        by_case[row["case"]][method] = row

    cases = sorted(by_case)
    if len(cases) != 10:
        raise RuntimeError(f"expected 10 frozen cases, found {len(cases)}")
    for case in cases:
        missing = set(METHODS).difference(by_case[case])
        if missing:
            raise RuntimeError(f"{case} is missing methods {sorted(missing)}")

    oracle_rows = [by_case[case]["oracle"] for case in cases]
    one_shot_rows = [by_case[case]["one-shot-pair-neighborhood"] for case in cases]
    primary_rows = [by_case[case]["primary-bootstrap-final"] for case in cases]
    portfolio_rows = [by_case[case]["cycle-portfolio-final"] for case in cases]

    oracle_valid = all(
        boolean(row, "structural_admissible")
        and boolean(row, "cycle_probe_accepted")
        and boolean(row, "pcg_converged")
        and required_number(row, "pcg_final_relative_residual") <= args.maximum_residual
        for row in oracle_rows
    )
    accepted_portfolio = [row for row in portfolio_rows if boolean(row, "accepted")]
    accepted_count = len(accepted_portfolio)
    accepted_count_gate = accepted_count >= args.minimum_accepted

    accepted_recoveries = [
        required_number(row, "oracle_improvement_recovered")
        for row in accepted_portfolio
    ]
    median_recovery = median(accepted_recoveries) if accepted_recoveries else float("-inf")
    median_recovery_gate = median_recovery >= args.minimum_median_recovery

    improvement_cases = 0
    material_regressions: list[tuple[str, float, float]] = []
    for case in cases:
        one_shot = by_case[case]["one-shot-pair-neighborhood"]
        portfolio = by_case[case]["cycle-portfolio-final"]
        if not (boolean(one_shot, "accepted") and boolean(portfolio, "accepted")):
            continue
        one_recovery = required_number(one_shot, "oracle_improvement_recovered")
        portfolio_recovery = required_number(portfolio, "oracle_improvement_recovered")
        if portfolio_recovery - one_recovery >= args.minimum_improvement:
            improvement_cases += 1
        if one_recovery - portfolio_recovery > args.maximum_regression:
            material_regressions.append((case, one_recovery, portfolio_recovery))
    improvement_gate = improvement_cases >= args.minimum_improvement_cases
    regression_gate = not material_regressions

    active_rows = [row for row in rows if boolean(row, "accepted")]
    residuals = [required_number(row, "pcg_final_relative_residual") for row in active_rows]
    residual_gate = bool(residuals) and max(residuals) <= args.maximum_residual
    convergence_gate = all(boolean(row, "pcg_converged") for row in active_rows)
    structural_gate = all(
        boolean(row, "structural_admissible")
        and required_number(row, "two_level_tuple_complexity")
        <= args.maximum_tuple_complexity
        for row in active_rows
    )

    probe_underestimates = []
    for row in rows:
        exact = optional_number(row, "exact_cycle_error_radius")
        estimate = optional_number(row, "probe_estimated_energy_factor")
        if exact is not None and estimate is not None:
            probe_underestimates.append(max(0.0, exact - estimate))
    maximum_underestimate = max(probe_underestimates, default=float("inf"))
    probe_calibration_gate = maximum_underestimate <= args.maximum_probe_underestimate

    no_build_failures = all(
        int(row["portfolio_cycle_build_failures"] or "0") == 0
        for row in portfolio_rows
    )
    known_sources = all(
        row["selected_source"]
        not in {"cycle-screened-unknown", "rejected-unknown-screen"}
        for row in portfolio_rows
    )

    source_counts = Counter(row["selected_source"] for row in accepted_portfolio)
    exact_partition_count = sum(boolean(row, "exact_oracle_partition") for row in accepted_portfolio)
    maximum_residual = max(residuals, default=float("nan"))
    maximum_complexity = max(
        (required_number(row, "two_level_tuple_complexity") for row in active_rows),
        default=float("nan"),
    )

    gates = [
        ("All ten supplied oracle maps pass structural, cycle, convergence, and residual checks", oracle_valid),
        (
            f"At least {args.minimum_accepted} of 10 portfolio cases are accepted",
            accepted_count_gate,
        ),
        (
            f"Median accepted portfolio oracle-recovery fraction is at least {args.minimum_median_recovery:.2f}",
            median_recovery_gate,
        ),
        (
            f"Portfolio improves one-shot recovery by at least {args.minimum_improvement:.2f} in at least {args.minimum_improvement_cases} cases",
            improvement_gate,
        ),
        (
            f"No accepted portfolio regresses more than {args.maximum_regression:.2f} below an accepted one-shot map",
            regression_gate,
        ),
        ("Every accepted PCG solve converges", convergence_gate),
        (
            f"Every accepted true residual is at most {args.maximum_residual:.1e}",
            residual_gate,
        ),
        (
            f"Every accepted map respects structural and tuple-complexity gates",
            structural_gate,
        ),
        (
            f"Maximum matrix-free probe underestimate is at most {args.maximum_probe_underestimate:.3f}",
            probe_calibration_gate,
        ),
        ("No candidate complete-cycle construction fails", no_build_failures),
        ("All selected-source labels are recognized", known_sources),
    ]
    passed = all(flag for _, flag in gates)

    lines = [
        "# Issue #3 complete-cycle holdout results",
        "",
        "## Verdict",
        "",
        (
            "**The frozen v2 automatic-coarsening gate passes.**"
            if passed
            else "**The frozen v2 automatic-coarsening gate does not pass.**"
        ),
        "",
        "The matrix uses the predeclared four-sheet hypergraph-cover fixtures and",
        "policy in `benchmarks/policies/issue3-cycle-portfolio-v2.tsv`. The primary",
        "bootstrap process still uses conservative Jacobi-compatible witnesses, while",
        "final numerical acceptance measures the actual symmetric-MAP two-grid error",
        "operator with a deterministic matrix-free energy power probe.",
        "",
        "## Case matrix",
        "",
        "| Case | Family | One-shot accepted | One-shot recovery | Primary accepted | Primary recovery | Portfolio accepted | Portfolio source | Portfolio recovery | Oracle κ | Portfolio κ | Probe factor | Exact error radius | Coarse tuples |",
        "|---|---|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|",
    ]
    for case in cases:
        one = by_case[case]["one-shot-pair-neighborhood"]
        primary = by_case[case]["primary-bootstrap-final"]
        portfolio = by_case[case]["cycle-portfolio-final"]
        oracle = by_case[case]["oracle"]
        lines.append(
            "| {case} | {family} | {one_accept} | {one_recovery} | {primary_accept} | {primary_recovery} | {portfolio_accept} | `{source}` | {portfolio_recovery} | {oracle_condition} | {portfolio_condition} | {probe} | {radius} | {tuples} |".format(
                case=case,
                family=portfolio["family"],
                one_accept=boolean(one, "accepted"),
                one_recovery=fmt(optional_number(one, "oracle_improvement_recovered")),
                primary_accept=boolean(primary, "accepted"),
                primary_recovery=fmt(optional_number(primary, "oracle_improvement_recovered")),
                portfolio_accept=boolean(portfolio, "accepted"),
                source=portfolio["selected_source"],
                portfolio_recovery=fmt(optional_number(portfolio, "oracle_improvement_recovered")),
                oracle_condition=fmt(optional_number(oracle, "candidate_condition")),
                portfolio_condition=fmt(optional_number(portfolio, "candidate_condition")),
                probe=fmt(optional_number(portfolio, "probe_estimated_energy_factor")),
                radius=fmt(optional_number(portfolio, "exact_cycle_error_radius")),
                tuples=portfolio["coarse_tuples"],
            )
        )

    lines.extend(
        [
            "",
            "## Aggregate diagnostics",
            "",
            f"- Accepted portfolio cases: **{accepted_count} of {len(cases)}**.",
            f"- Median accepted portfolio oracle-recovery fraction: **{fmt(median_recovery)}**.",
            f"- Cases improving accepted one-shot recovery by at least {args.minimum_improvement:.2f}: **{improvement_cases}**.",
            f"- Exact oracle partitions selected by the portfolio: **{exact_partition_count}**. Exact partition recovery is diagnostic, not required when a different compact partition performs equally well.",
            f"- Maximum accepted true residual: `{maximum_residual:.3e}`.",
            f"- Maximum accepted two-level tuple complexity: `{maximum_complexity:.3f}`.",
            f"- Maximum positive exact-radius minus matrix-free-estimate gap: `{maximum_underestimate:.3e}`.",
            "- Accepted portfolio source counts: "
            + (", ".join(f"`{source}` {count}" for source, count in sorted(source_counts.items())) or "none")
            + ".",
            "",
            "## Scientific gates",
            "",
        ]
    )
    lines.extend(f"- [{pass_text(flag)}] {description}." for description, flag in gates)
    if material_regressions:
        lines.extend(
            [
                "",
                "## Material regressions",
                "",
            ]
        )
        for case, one, portfolio in material_regressions:
            lines.append(
                f"- `{case}`: one-shot recovery `{one:.3f}`, portfolio recovery `{portfolio:.3f}`."
            )

    lines.extend(
        [
            "",
            "## Interpretation",
            "",
            "The matrix-free probe is used only after hard structural admission. It cannot",
            "rescue an identity map, an over-large coarse space, or a map without sufficient",
            "unique-tuple contraction. Conservative compatible relaxation remains valuable",
            "for constructing signatures and repair witnesses, but the full fixed cycle is",
            "the final numerical authority.",
            "",
            "A passing result establishes feasibility on the frozen synthetic holdout, not",
            "production runtime superiority. Large pair-solver comparisons, reusable",
            "allocation-free state, and eventual fereg certification remain separate issues.",
            "",
        ]
    )

    args.output_markdown.parent.mkdir(parents=True, exist_ok=True)
    args.output_markdown.write_text("\n".join(lines), encoding="utf-8")
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
