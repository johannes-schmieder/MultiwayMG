#!/usr/bin/env python3
"""Validate and summarize the frozen issue-4 balanced size ladder."""
from __future__ import annotations

import argparse
import csv
import math
import statistics
from collections import defaultdict
from pathlib import Path

FAMILIES = ("planted-clones", "noisy-clones", "latin-square")
LEVELS = (12, 18, 24, 36, 48, 72)
METHODS = ("diagonal", "pair-cmg-schwarz", "within-default")
SOLVERS = ("mlsmr", "pcg-traced")
PREFIXES = (1, 4, 16, 32)
TERMINALS = (
    "direct_pair_components", "full_contraction_components",
    "stagnated_vertex_components", "stagnated_fill_components",
    "maximum_levels_components",
)


def load(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream, delimiter="\t"))
    if not rows or any(None in row or None in row.values() for row in rows):
        raise ValueError("empty or malformed size-ladder TSV")
    return rows


def validate(rows: list[dict[str, str]]) -> list[str]:
    expected = {
        (family, levels, repeat, method, solver, rhs)
        for family in FAMILIES for levels in LEVELS for repeat in range(2)
        for method in METHODS for solver in SOLVERS for rhs in PREFIXES
    }
    seen = set()
    errors: list[str] = []
    builds: dict[tuple[str, int, int, str], list[dict[str, str]]] = defaultdict(list)
    series: dict[tuple[str, int, int, str, str], list[dict[str, str]]] = defaultdict(list)
    try:
        for row in rows:
            key = (
                row["family"], int(row["levels"]), int(row["repeat"]), row["method"],
                row["solver"], int(row["rhs_count"]),
            )
            if key in seen:
                errors.append(f"duplicate row {key}")
            seen.add(key)
            builds[key[:4]].append(row)
            series[key[:5]].append(row)
            if row["certified"] != "true" or row["converged"] != "true" or row["error"]:
                errors.append(f"uncertified row {key}: {row['error']}")
            residual = float(row["max_true_residual"])
            if not math.isfinite(residual) or not 0 <= residual <= 1e-8:
                errors.append(f"invalid residual {key}: {residual}")
            for field in (
                "constructor_seconds", "initialization_seconds", "setup_seconds",
                "cumulative_solve_seconds", "setup_plus_solve_seconds",
            ):
                value = float(row[field])
                if not math.isfinite(value) or value < 0:
                    errors.append(f"invalid {field} {key}: {value}")
            setup = float(row["constructor_seconds"]) + float(row["initialization_seconds"])
            if not math.isclose(float(row["setup_seconds"]), setup, rel_tol=5e-8, abs_tol=1e-12):
                errors.append(f"setup mismatch {key}")
            total = setup + float(row["cumulative_solve_seconds"])
            if not math.isclose(float(row["setup_plus_solve_seconds"]), total, rel_tol=5e-8, abs_tol=1e-12):
                errors.append(f"charged total mismatch {key}")
            if min(int(row[field]) for field in (
                "tuples", "components", "cumulative_iterations", "cumulative_outer_work",
                "cumulative_preconditioner_applications",
            )) <= 0:
                errors.append(f"missing problem/solver work {key}")
            if row["solver"] == "mlsmr":
                if row["work_unit"] != "rectangular-operator" or int(row["cumulative_certificate_work"]) != 3 * key[-1]:
                    errors.append(f"bad LSMR accounting {key}")
            elif row["solver"] == "pcg-traced":
                if row["work_unit"] != "gramian" or int(row["cumulative_certificate_work"]) != 0:
                    errors.append(f"bad PCG accounting {key}")
            else:
                errors.append(f"unknown solver {key}")
            if int(row["fallback_allocations"]) != 0 or int(row["warning_count"]) < 0:
                errors.append(f"fallback/warning accounting {key}")
            if row["method"] == "pair-cmg-schwarz":
                pair_components = int(row["pair_components"])
                terminal_sum = sum(int(row[field]) for field in TERMINALS)
                if pair_components <= 0 or terminal_sum != pair_components:
                    errors.append(f"terminal coverage mismatch {key}")
                if int(row["direct_factor_components"]) != int(row["direct_pair_components"]):
                    errors.append(f"direct factor mismatch {key}")
                if not 0 <= int(row["one_level_iterative_components"]) <= pair_components:
                    errors.append(f"one-level iterative mismatch {key}")
                if not 0 <= int(row["multilevel_pair_components"]) <= pair_components:
                    errors.append(f"multilevel count mismatch {key}")
                if min(int(row[field]) for field in (
                    "max_pair_vertices", "max_pair_edges", "max_pair_levels",
                )) <= 0:
                    errors.append(f"missing CMG structure {key}")
                if row["known_retained_bytes"] == "NA" or int(row["known_retained_bytes"]) <= 0:
                    errors.append(f"missing CMG retained estimate {key}")
            else:
                for field in (
                    "pair_components", "max_pair_vertices", "max_pair_edges",
                    "max_pair_cycle_excess", "max_pair_levels", "multilevel_pair_components",
                    *TERMINALS, "one_level_iterative_components", "direct_factor_components",
                ):
                    if int(row[field]) != 0:
                        errors.append(f"CMG metadata on non-CMG route {key}: {field}")
            if row["method"] == "diagonal" and row["known_retained_bytes"] != "NA":
                errors.append(f"diagonal retained bytes unexpectedly claimed {key}")
            if row["method"] == "within-default" and (row["known_retained_bytes"] == "NA" or int(row["known_retained_bytes"]) <= 0):
                errors.append(f"missing known within wrapper bytes {key}")
        if seen != expected:
            errors.append(f"coverage mismatch: missing={len(expected-seen)} extra={len(seen-expected)}")
        for key, group in builds.items():
            immutable = (
                "tuples", "components", "constructor_seconds", "initialization_seconds",
                "setup_seconds", "known_retained_bytes", "pair_components",
                "max_pair_vertices", "max_pair_edges", "max_pair_cycle_excess", "max_pair_levels",
                "multilevel_pair_components", *TERMINALS, "one_level_iterative_components",
                "direct_factor_components", "warning_count",
            )
            for field in immutable:
                if len({row[field] for row in group}) != 1:
                    errors.append(f"build metadata changed {key}: {field}")
        for key, group in series.items():
            group.sort(key=lambda row: int(row["rhs_count"]))
            for old, new in zip(group, group[1:]):
                for field in (
                    "cumulative_solve_seconds", "setup_plus_solve_seconds",
                    "cumulative_iterations", "cumulative_outer_work",
                    "cumulative_preconditioner_applications", "cumulative_certificate_work",
                    "max_true_residual",
                ):
                    if float(new[field]) < float(old[field]):
                        errors.append(f"nonmonotone {field} {key}")
    except (KeyError, ValueError, OverflowError) as error:
        errors.append(f"invalid schema/value: {error}")
    return errors


def selected(rows, family, levels, solver, method, rhs):
    return sorted(
        [row for row in rows if row["family"] == family and int(row["levels"]) == levels
         and row["solver"] == solver and row["method"] == method and int(row["rhs_count"]) == rhs],
        key=lambda row: int(row["repeat"]),
    )


def paired_ratio(rows, family, levels, solver, numerator, denominator, rhs, field):
    a = selected(rows, family, levels, solver, numerator, rhs)
    b = selected(rows, family, levels, solver, denominator, rhs)
    return [float(x[field]) / float(y[field]) for x, y in zip(a, b)]


def first_observed_timing_win(rows, family, levels, solver):
    for rhs in PREFIXES:
        ratios = paired_ratio(
            rows, family, levels, solver, "within-default", "pair-cmg-schwarz", rhs,
            "setup_plus_solve_seconds",
        )
        if statistics.median(ratios) > 1:
            return str(rhs)
    return "none through 32"


def render(rows: list[dict[str, str]], errors: list[str]) -> str:
    text = [
        "# Issue 4 balanced size ladder", "",
        f"Rows: {len(rows)}. Numerical/accounting gate: {'FAIL' if errors else 'PASS'}.", "",
        "This is calibration around an observed crossover, not a holdout. Timing is descriptive and never a CI pass criterion.", "",
    ]
    if errors:
        text += ["## Rejected evidence", "", *[f"- {error}" for error in errors], "",
                 "No crossover or routing conclusion is inferred from rejected evidence."]
        return "\n".join(text) + "\n"

    text += [
        "## 32-RHS outer-work ladder", "",
        "Ratios are pair-CMG/within; below one favors pair-CMG. Reported values are paired medians of two independently built repeats. LSMR and PCG work units differ and are compared only within an outer solver.", "",
        "| Family | Levels/factor | Tuples | LSMR work ratio | PCG work ratio | Direct / iterative pair terminals | Max CMG levels |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for family in FAMILIES:
        for levels in LEVELS:
            lsmr = statistics.median(paired_ratio(rows, family, levels, "mlsmr", "pair-cmg-schwarz", "within-default", 32, "cumulative_outer_work"))
            pcg = statistics.median(paired_ratio(rows, family, levels, "pcg-traced", "pair-cmg-schwarz", "within-default", 32, "cumulative_outer_work"))
            cmg = selected(rows, family, levels, "mlsmr", "pair-cmg-schwarz", 32)[0]
            iterative = int(cmg["pair_components"]) - int(cmg["direct_pair_components"])
            text.append(
                f"| {family} | {levels} | {cmg['tuples']} | {lsmr:.3f} | {pcg:.3f} | "
                f"{cmg['direct_pair_components']} / {iterative} | {cmg['max_pair_levels']} |"
            )

    text += [
        "", "## Observed fully charged economics", "",
        "The final column is the first measured RHS prefix where the paired-median within/CMG setup-plus-solve ratio exceeds one. It is an observed prefix, not an interpolated break-even. `none through 32` means within remained faster at every measured prefix.", "",
        "| Family | Levels | LSMR within/CMG at 32 RHS | First observed LSMR CMG win | PCG within/CMG at 32 RHS | First observed PCG CMG win |",
        "|---|---:|---:|---|---:|---|",
    ]
    for family in FAMILIES:
        for levels in LEVELS:
            lr = statistics.median(paired_ratio(rows, family, levels, "mlsmr", "within-default", "pair-cmg-schwarz", 32, "setup_plus_solve_seconds"))
            pr = statistics.median(paired_ratio(rows, family, levels, "pcg-traced", "within-default", "pair-cmg-schwarz", 32, "setup_plus_solve_seconds"))
            text.append(
                f"| {family} | {levels} | {lr:.3f} | {first_observed_timing_win(rows, family, levels, 'mlsmr')} | "
                f"{pr:.3f} | {first_observed_timing_win(rows, family, levels, 'pcg-traced')} |"
            )

    maximum = max(float(row["max_true_residual"]) for row in rows)
    text += [
        "", "## Boundaries", "",
        f"Maximum true relative residual: {maximum:.6e}. Sequential fallback allocations are zero by gate.", "",
        "The ladder was chosen after seeing the earlier smoke/calibration crossover, so it can characterize that crossover but cannot qualify a policy. A routing rule must be frozen only after this analysis and tested on a fresh holdout.",
    ]
    return "\n".join(text) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    try:
        rows = load(args.input)
        errors = validate(rows)
    except (OSError, ValueError) as error:
        rows, errors = [], [str(error)]
    args.output.write_text(render(rows, errors), encoding="utf-8")
    print(f"issue-4 size ladder: {'FAIL' if errors else 'PASS'} ({len(rows)} rows)")
    for error in errors:
        print(error)
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
