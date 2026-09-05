#!/usr/bin/env python3
"""Validate and summarize the issue-4 coarse-only CMG calibration."""

from __future__ import annotations

import csv
import math
import statistics
import sys
from pathlib import Path

METHODS = ("within-all-levels", "within-fine-cmg-coarse")
SOLVERS = ("mlsmr", "pcg-traced")
PREFIXES = (1, 4, 16, 32)


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle, delimiter="\t"))


def median(values: list[float]) -> float:
    if not values:
        raise ValueError("cannot summarize an empty sample")
    return statistics.median(values)


def row_is_certified(row: dict[str, str]) -> bool:
    if row["converged"] != "true" or row["certified"] != "true" or row["error"]:
        return False
    residual = float(row["max_true_residual"])
    return math.isfinite(residual) and residual <= 1.0e-8


def validate(
    rows: list[dict[str, str]],
) -> tuple[list[dict[str, str]], list[str], list[str]]:
    if not rows:
        raise ValueError("coarse-CMG evidence is empty")
    rejected = [row for row in rows if row["plan_accepted"] != "true"]
    accepted = [row for row in rows if row["plan_accepted"] == "true"]
    if not accepted:
        raise ValueError("no accepted frozen issue-3 hierarchy remains for comparison")

    cases = sorted({row["case"] for row in accepted})
    expected = len(cases) * len(METHODS) * 2 * len(SOLVERS) * len(PREFIXES)
    if len(accepted) != expected:
        raise ValueError(f"expected {expected} accepted rows, found {len(accepted)}")

    seen: set[tuple[str, str, int, str, int]] = set()
    for row in accepted:
        method = row["method"]
        solver = row["solver"]
        repeat = int(row["repeat"])
        rhs = int(row["rhs_count"])
        key = (row["case"], method, repeat, solver, rhs)
        if key in seen:
            raise ValueError(f"duplicate row {key}")
        seen.add(key)
        if (
            method not in METHODS
            or solver not in SOLVERS
            or repeat not in (0, 1)
            or rhs not in PREFIXES
        ):
            raise ValueError(f"unexpected comparison cell {key}")
        if int(row["fallback_allocations"]) != 0:
            raise ValueError(f"sequential fallback allocation in {key}")
        if int(row["plan_depth"]) != int(row["requested_depth"]):
            raise ValueError(f"accepted plan depth mismatch in {key}")
        if method == "within-all-levels" and int(row["cmg_components"]) != 0:
            raise ValueError(f"within-only comparator unexpectedly contains CMG in {key}")
        if (
            method == "within-fine-cmg-coarse"
            and int(row["requested_depth"]) > 1
            and int(row["cmg_components"]) == 0
        ):
            raise ValueError(f"hybrid comparator lacks coarse CMG components in {key}")

    baseline_bad_cases = sorted(
        {
            row["case"]
            for row in accepted
            if row["method"] == "within-all-levels" and not row_is_certified(row)
        }
    )
    comparable = [row for row in accepted if row["case"] not in baseline_bad_cases]
    if not comparable:
        raise ValueError("all accepted hierarchies fail the all-within baseline admission gate")

    for row in comparable:
        key = (
            row["case"],
            row["method"],
            int(row["repeat"]),
            row["solver"],
            int(row["rhs_count"]),
        )
        if not row_is_certified(row):
            raise ValueError(f"uncertified comparison batch {key}: {row['error']}")

    return (
        comparable,
        sorted({row["case"] for row in rejected}),
        baseline_bad_cases,
    )


def ratios(rows: list[dict[str, str]], rhs: int = 32) -> list[dict[str, object]]:
    cells: dict[tuple[str, str, int, str], dict[str, str]] = {}
    for row in rows:
        if int(row["rhs_count"]) == rhs:
            cells[(row["case"], row["solver"], int(row["repeat"]), row["method"])] = row

    output: list[dict[str, object]] = []
    cases = sorted({row["case"] for row in rows})
    for case in cases:
        family = next(row["family"] for row in rows if row["case"] == case)
        depth = int(next(row["requested_depth"] for row in rows if row["case"] == case))
        for solver in SOLVERS:
            work_ratios = []
            time_ratios = []
            solve_time_ratios = []
            for repeat in (0, 1):
                baseline = cells[(case, solver, repeat, "within-all-levels")]
                hybrid = cells[(case, solver, repeat, "within-fine-cmg-coarse")]
                work_ratios.append(
                    float(hybrid["cumulative_outer_work"])
                    / float(baseline["cumulative_outer_work"])
                )
                time_ratios.append(
                    float(hybrid["setup_plus_solve_seconds"])
                    / float(baseline["setup_plus_solve_seconds"])
                )
                solve_time_ratios.append(
                    float(hybrid["cumulative_solve_seconds"])
                    / float(baseline["cumulative_solve_seconds"])
                )
            sample = cells[(case, solver, 0, "within-fine-cmg-coarse")]
            output.append(
                {
                    "case": case,
                    "family": family,
                    "depth": depth,
                    "solver": solver,
                    "work_ratio": median(work_ratios),
                    "charged_time_ratio": median(time_ratios),
                    "solve_time_ratio": median(solve_time_ratios),
                    "cmg_components": int(sample["cmg_components"]),
                    "cmg_max_vertices": int(sample["cmg_max_vertices"]),
                    "cmg_max_levels": int(sample["cmg_max_levels"]),
                }
            )
    return output


def first_timing_win(rows: list[dict[str, str]], case: str, solver: str) -> str:
    for rhs in PREFIXES:
        cells = {
            (int(row["repeat"]), row["method"]): row
            for row in rows
            if row["case"] == case
            and row["solver"] == solver
            and int(row["rhs_count"]) == rhs
        }
        ratios_at_rhs = []
        for repeat in (0, 1):
            baseline = cells[(repeat, "within-all-levels")]
            hybrid = cells[(repeat, "within-fine-cmg-coarse")]
            ratios_at_rhs.append(
                float(hybrid["setup_plus_solve_seconds"])
                / float(baseline["setup_plus_solve_seconds"])
            )
        if median(ratios_at_rhs) < 1.0:
            return str(rhs)
    return "none-through-32"


def write_summary(
    rows: list[dict[str, str]],
    rejected: list[str],
    baseline_bad: list[str],
    output: Path,
    mode: str,
) -> None:
    summary = ratios(rows)
    maximum_residual = max(float(row["max_true_residual"]) for row in rows)
    if mode == "oracle":
        title = "# Issue 4 coarse-only CMG oracle-map calibration"
        map_statement = (
            "The revealed issue-3 oracle map sequence and the fine `within` smoother "
            "are identical across methods; only non-finest smoothers change."
        )
    else:
        title = "# Issue 4 coarse-only CMG automatic-map calibration"
        map_statement = (
            "The automatic map plan and the fine `within` smoother are identical "
            "across methods; only non-finest smoothers change."
        )
    lines = [
        title,
        "",
        f"Comparable certified rows: {len(rows)}. Numerical/accounting gate: PASS.",
        f"Maximum true relative residual among comparable rows: {maximum_residual:.6e}.",
        "",
        "This is calibration on the already-revealed recursive issue-3 fixtures, not an issue-4 holdout.",
        map_statement,
        "Ratios below one favor coarse CMG.",
        "",
        "| Case | Depth | Solver | Hybrid/within outer work | Hybrid/within solve time | Hybrid/within fully charged time | First measured charged win | Coarse CMG components | Max pair vertices | Max CMG levels |",
        "|---|---:|---|---:|---:|---:|---|---:|---:|---:|",
    ]
    for row in summary:
        lines.append(
            "| {case} | {depth} | {solver} | {work_ratio:.3f} | {solve_time_ratio:.3f} | {charged_time_ratio:.3f} | {win} | {cmg_components} | {cmg_max_vertices} | {cmg_max_levels} |".format(
                **row,
                win=first_timing_win(rows, str(row["case"]), str(row["solver"])),
            )
        )
    if rejected:
        lines.extend(
            [
                "",
                "Rejected automatic plans (not compared): "
                + ", ".join(sorted(set(rejected)))
                + ".",
            ]
        )
    if baseline_bad:
        lines.extend(
            [
                "",
                "Baseline-inadmissible hierarchies (excluded from solver ratios because the all-`within` hierarchy itself failed the outer SPD/certification gate): "
                + ", ".join(sorted(set(baseline_bad)))
                + ".",
            ]
        )
    work_wins = [row for row in summary if float(row["work_ratio"]) <= 0.80]
    charged_wins = [row for row in summary if float(row["charged_time_ratio"]) < 1.0]
    lines.extend(
        [
            "",
            f"Cells with at least 20% outer-work reduction at 32 RHS: {len(work_wins)}/{len(summary)}.",
            f"Cells with a fully charged timing win at 32 RHS: {len(charged_wins)}/{len(summary)}.",
            "",
            "A routing rule must not be selected from these cases and then described as holdout-validated; any such rule requires a fresh preregistered issue-4 holdout.",
            "",
        ]
    )
    output.write_text("\n".join(lines), encoding="utf-8")


def main(argv: list[str]) -> int:
    if len(argv) not in (3, 4):
        raise SystemExit(
            "usage: summarize_issue4_coarse_cmg.py INPUT.tsv SUMMARY.md [automatic|oracle]"
        )
    mode = argv[3] if len(argv) == 4 else "automatic"
    if mode not in ("automatic", "oracle"):
        raise SystemExit(f"unknown calibration mode: {mode}")
    rows = read_rows(Path(argv[1]))
    comparable, rejected, baseline_bad = validate(rows)
    write_summary(comparable, rejected, baseline_bad, Path(argv[2]), mode)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
