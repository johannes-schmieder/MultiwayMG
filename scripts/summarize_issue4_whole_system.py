#!/usr/bin/env python3
"""Validate and summarize the issue-4 whole-system Schwarz smoke matrix."""
from __future__ import annotations

import argparse
import csv
import math
import statistics
from collections import defaultdict
from pathlib import Path

CASES = (
    "planted-clones", "noisy-clones", "latin-square", "weak-chain",
    "disconnected-latin", "unbalanced-cycle",
)
METHODS = ("diagonal", "pair-cmg-schwarz", "within-default")
SOLVERS = ("mlsmr", "pcg-traced")
RHS = (1, 2, 3, 4)


def load(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream, delimiter="\t"))
    if not rows or any(None in row or None in row.values() for row in rows):
        raise ValueError("empty or malformed whole-system TSV")
    return rows


def validate(rows: list[dict[str, str]], profile: str = "smoke") -> list[str]:
    expected = {
        (case, repeat, method, solver, rhs)
        for case in CASES
        for repeat in range(2)
        for method in METHODS
        for solver in SOLVERS
        for rhs in RHS
    }
    seen = set()
    errors: list[str] = []
    setup_groups: dict[tuple[str, int, str], list[dict[str, str]]] = defaultdict(list)
    try:
        for row in rows:
            key = (
                row["case"], int(row["repeat"]), row["method"], row["solver"], int(row["rhs"])
            )
            if key in seen:
                errors.append(f"duplicate row {key}")
            seen.add(key)
            setup_groups[key[:3]].append(row)
            if row["profile"] != profile:
                errors.append(f"wrong profile {key}: {row['profile']}")
            if row["certified"] != "true" or row["error"]:
                errors.append(f"uncertified row {key}: {row['error']}")
            residual = float(row["true_residual"])
            if not math.isfinite(residual) or not 0 <= residual <= 1e-8:
                errors.append(f"invalid true residual {key}: {residual}")
            if row["converged"] != "true":
                errors.append(f"outer recurrence did not converge {key}")
            for field in (
                "constructor_seconds", "initialization_seconds", "setup_seconds", "solve_seconds"
            ):
                value = float(row[field])
                if not math.isfinite(value) or value < 0:
                    errors.append(f"invalid {field} {key}: {value}")
            total_setup = float(row["constructor_seconds"]) + float(row["initialization_seconds"])
            if not math.isclose(float(row["setup_seconds"]), total_setup, rel_tol=5e-8, abs_tol=1e-12):
                errors.append(f"setup accounting mismatch {key}")
            if int(row["iterations"]) <= 0 or int(row["outer_work"]) <= 0 or int(row["preconditioner_applications"]) <= 0:
                errors.append(f"missing solver work {key}")
            if row["solver"] == "mlsmr":
                if row["work_unit"] != "rectangular-operator" or int(row["certificate_work"]) != 3:
                    errors.append(f"bad LSMR accounting {key}")
            elif row["solver"] == "pcg-traced":
                if row["work_unit"] != "gramian" or int(row["certificate_work"]) != 0:
                    errors.append(f"bad PCG accounting {key}")
            else:
                errors.append(f"unknown solver {key}")
            if min(int(row[name]) for name in ("factor1", "factor2", "factor3", "tuples", "components")) <= 0:
                errors.append(f"invalid problem dimensions {key}")
            if int(row["fallback_allocations"]) != 0:
                errors.append(f"fallback allocation in sequential run {key}")
            if int(row["warning_count"]) < 0:
                errors.append(f"negative warning count {key}")
            if row["method"] == "pair-cmg-schwarz":
                if min(int(row[name]) for name in ("pair_components", "max_pair_vertices", "max_pair_edges", "max_pair_levels")) <= 0:
                    errors.append(f"missing CMG component metadata {key}")
                if int(row["multilevel_pair_components"]) > int(row["pair_components"]):
                    errors.append(f"invalid multilevel count {key}")
                if row["known_retained_bytes"] == "NA" or int(row["known_retained_bytes"]) <= 0:
                    errors.append(f"missing CMG retained-state estimate {key}")
            else:
                if any(int(row[name]) != 0 for name in (
                    "pair_components", "max_pair_vertices", "max_pair_edges",
                    "max_pair_cycle_excess", "max_pair_levels", "multilevel_pair_components",
                )):
                    errors.append(f"CMG metadata on non-CMG method {key}")
            if row["method"] == "within-default" and (row["known_retained_bytes"] == "NA" or int(row["known_retained_bytes"]) <= 0):
                errors.append(f"missing known within wrapper bytes {key}")
            if row["method"] == "diagonal" and row["known_retained_bytes"] != "NA":
                errors.append(f"diagonal memory should remain unclaimed {key}")
        if seen != expected:
            errors.append(f"matrix coverage mismatch: missing={len(expected-seen)} extra={len(seen-expected)}")
        for key, group in setup_groups.items():
            for field in (
                "constructor_seconds", "initialization_seconds", "setup_seconds",
                "known_retained_bytes", "pair_components", "max_pair_vertices", "max_pair_edges",
                "max_pair_cycle_excess", "max_pair_levels", "multilevel_pair_components",
                "warning_count",
            ):
                if len({row[field] for row in group}) != 1:
                    errors.append(f"build metadata changed across reused solves {key}: {field}")
        if not any(
            row["method"] == "pair-cmg-schwarz" and int(row["multilevel_pair_components"]) > 0
            for row in rows
        ):
            errors.append("CMG never exercised a multilevel pair component")
    except (KeyError, ValueError, OverflowError) as error:
        errors.append(f"invalid schema/value: {error}")
    return errors


def ratios(rows: list[dict[str, str]], case: str, solver: str, numerator: str, denominator: str, field: str) -> list[float]:
    keyed = defaultdict(dict)
    for row in rows:
        if row["case"] == case and row["solver"] == solver:
            keyed[(int(row["repeat"]), int(row["rhs"]))][row["method"]] = row
    result = []
    for methods in keyed.values():
        if numerator in methods and denominator in methods:
            result.append(float(methods[numerator][field]) / float(methods[denominator][field]))
    return result


def charged_four_rhs(rows: list[dict[str, str]], case: str, solver: str, method: str) -> list[float]:
    by_repeat = defaultdict(list)
    for row in rows:
        if row["case"] == case and row["solver"] == solver and row["method"] == method:
            by_repeat[int(row["repeat"])].append(row)
    values = []
    for repeat_rows in by_repeat.values():
        setup = float(repeat_rows[0]["setup_seconds"])
        values.append(setup + sum(float(row["solve_seconds"]) for row in repeat_rows))
    return values


def render(rows: list[dict[str, str]], errors: list[str]) -> str:
    text = [
        "# Issue 4 whole-system Schwarz smoke matrix", "",
        f"Rows: {len(rows)}. Numerical/accounting gate: {'FAIL' if errors else 'PASS'}.", "",
        "This matrix removes the three-way coarse hierarchy and asks only whether the local pair solver changes work on the complete three-way system. Hosted-runner timing is descriptive and never a CI winner gate.", "",
    ]
    if errors:
        text += ["## Rejected evidence", "", *[f"- {error}" for error in errors], "",
                 "No performance conclusion is inferred from failed or partial evidence."]
        return "\n".join(text) + "\n"

    text += [
        "## CMG versus within: outer work", "",
        "Ratios are pair-CMG/within. Values below one favor pair-CMG. LSMR counts weighted B and B' applications; traced PCG counts original-Gramian applications, so ratios are comparable only within a solver.", "",
        "| Case | LSMR work min / median / max | PCG work min / median / max | CMG multilevel pair components |",
        "|---|---:|---:|---:|",
    ]
    for case in CASES:
        lsmr = ratios(rows, case, "mlsmr", "pair-cmg-schwarz", "within-default", "outer_work")
        pcg = ratios(rows, case, "pcg-traced", "pair-cmg-schwarz", "within-default", "outer_work")
        cmg = next(row for row in rows if row["case"] == case and row["method"] == "pair-cmg-schwarz")
        text.append(
            f"| {case} | {min(lsmr):.3f} / {statistics.median(lsmr):.3f} / {max(lsmr):.3f} | "
            f"{min(pcg):.3f} / {statistics.median(pcg):.3f} / {max(pcg):.3f} | "
            f"{cmg['multilevel_pair_components']} / {cmg['pair_components']} |"
        )

    text += [
        "", "## Four-RHS charged timing", "",
        "Each value is setup plus four solves for one outer solver. The range is the two repeat ratios within/CMG; greater than one favors CMG. These tiny hosted-runner timings are diagnostics, not qualification.", "",
        "| Case | LSMR within/CMG min / max | PCG within/CMG min / max |",
        "|---|---:|---:|",
    ]
    for case in CASES:
        lcmg = charged_four_rhs(rows, case, "mlsmr", "pair-cmg-schwarz")
        lwithin = charged_four_rhs(rows, case, "mlsmr", "within-default")
        pcmg = charged_four_rhs(rows, case, "pcg-traced", "pair-cmg-schwarz")
        pwithin = charged_four_rhs(rows, case, "pcg-traced", "within-default")
        lr = [w / c for w, c in zip(lwithin, lcmg)]
        pr = [w / c for w, c in zip(pwithin, pcmg)]
        text.append(f"| {case} | {min(lr):.3f} / {max(lr):.3f} | {min(pr):.3f} / {max(pr):.3f} |")

    maximum = max(float(row["true_residual"]) for row in rows)
    warnings = max(int(row["warning_count"]) for row in rows)
    text += [
        "", "## Boundaries", "",
        f"Maximum true relative residual: {maximum:.6e}. Maximum warnings on a build: {warnings}. Sequential fallback workspace allocations: zero by gate.", "",
        "The within retained-state number is only the wrapper categories exposed by the current API; it excludes the opaque inner preconditioner. Pair-CMG retained bytes are estimates of known retained categories, not process peak RSS.", "",
        "This is still a small deterministic calibration matrix. It is not the broad topology/size calibration, thread study, changing-weight experiment, coarse-hierarchy comparison, or fresh holdout required to close issue #4.",
    ]
    return "\n".join(text) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--profile", choices=("smoke", "calibration"), default="smoke")
    args = parser.parse_args()
    try:
        rows = load(args.input)
        errors = validate(rows, args.profile)
    except (OSError, ValueError) as error:
        rows, errors = [], [str(error)]
    args.output.write_text(render(rows, errors), encoding="utf-8")
    print(f"issue-4 whole-system: {'FAIL' if errors else 'PASS'} ({len(rows)} rows)")
    for error in errors:
        print(error)
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
