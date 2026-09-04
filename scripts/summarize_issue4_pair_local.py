#!/usr/bin/env python3
"""Validate pair-local evidence and report costs without imposing a speed gate."""
from __future__ import annotations

import argparse
import csv
import math
import statistics
from collections import defaultdict
from pathlib import Path

FAMILIES = ("path", "hubs", "weak", "dense", "dynamic")
METHODS = ("jacobi", "exact", "cmg-fixed", "within-default")
RHS = (1, 4, 16, 32)
TIME_FIELDS = (
    "domain_seconds", "setup_seconds", "workspace_seconds", "apply_seconds",
    "solve_seconds", "total_seconds",
)
WORK_FIELDS = ("solver_b", "solver_bt", "preconditioner_calls", "certificate_b", "certificate_bt")


def break_even(cmg_setup: float, cmg_rate: float, within_setup: float, within_rate: float) -> str:
    """Strict integer wins under S+n*T, including reversed/absent crossovers."""
    values = (cmg_setup, cmg_rate, within_setup, within_rate)
    if any(not math.isfinite(x) or x < 0 for x in values):
        raise ValueError("costs must be finite and nonnegative")
    setup_gap = cmg_setup - within_setup
    savings = within_rate - cmg_rate
    if savings > 0:
        first = max(1, math.floor(setup_gap / savings) + 1)
        return f"n >= {first}"
    if savings == 0:
        return "all n >= 1" if setup_gap < 0 else "never"
    # A cheaper setup with a slower solve gives an early window, not amortization.
    last = math.ceil(setup_gap / savings) - 1
    return f"only 1 <= n <= {last}; no long-run win" if last >= 1 else "never"


def load(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream, delimiter="\t"))
    if not rows or any(None in row or None in row.values() for row in rows):
        raise ValueError("empty or malformed TSV")
    return rows


def validate(rows: list[dict[str, str]], profile: str) -> list[str]:
    """A failed or partial run never supplies a performance winner."""
    sizes = (32,) if profile == "smoke" else (64, 256)
    expected = {
        (f"{family}-{size}", repeat, method, count)
        for size in sizes for family in FAMILIES for repeat in range(3)
        for method in METHODS if method != "exact" or size * 2 <= 256
        for count in RHS
    }
    seen = set()
    errors = []
    groups = defaultdict(list)
    try:
        for row in rows:
            key = (row["fixture"], int(row["repeat"]), row["method"], int(row["rhs_count"]))
            if key in seen:
                errors.append(f"duplicate row {key}")
            seen.add(key)
            groups[key[:3]].append(row)
            if row["profile"] != profile:
                errors.append(f"wrong profile {key}")
            if row["certified"] != "true" or row["error"]:
                errors.append(f"uncertified solve {key}: {row['error']}")
            residual = float(row["max_true_residual"])
            if not math.isfinite(residual) or not 0 <= residual <= 1e-8:
                errors.append(f"true residual {key}: {residual}")
            times = {field: float(row[field]) for field in TIME_FIELDS}
            if any(not math.isfinite(x) or x < 0 for x in times.values()):
                errors.append(f"invalid timing {key}")
            total = times["domain_seconds"] + times["setup_seconds"] + times["solve_seconds"]
            if not math.isclose(times["total_seconds"], total, rel_tol=5e-8, abs_tol=1e-12):
                errors.append(f"total excludes charged phases {key}")
            if times["workspace_seconds"] > times["setup_seconds"] * (1 + 1e-8):
                errors.append(f"workspace is not a setup subset {key}")
            work = {field: int(row[field]) for field in WORK_FIELDS}
            if any(x < 0 for x in work.values()):
                errors.append(f"negative work count {key}")
            if row["certified"] == "true" and (
                work["certificate_b"] != key[3] or work["certificate_bt"] != 2 * key[3]
            ):
                errors.append(f"incorrect certificate accounting {key}")
            n = int(row["vertices"])
            if n != 2 * int(row["fixture"].rsplit("-", 1)[1]) or int(row["edges"]) < n - 1:
                errors.append(f"invalid connected pair dimensions {key}")
            if row["method"] == "within-default" and row["principal_solver_bytes"] != "NA":
                errors.append(f"opaque within memory presented as measured {key}")
            if int(row["known_workspace_bytes"]) < 8 * n or int(row["common_graph_bytes"]) <= 0:
                errors.append(f"invalid known memory {key}")
            if row["method"] != "within-default" and int(row["principal_solver_bytes"]) <= 0:
                errors.append(f"missing principal state estimate {key}")
            if n <= 256:
                for field in ("symmetry_defect", "linearity_defect"):
                    value = float(row[field])
                    if not math.isfinite(value) or not 0 <= value <= 1e-8:
                        errors.append(f"{field} {key}: {value}")
                minimum = float(row["minimum_energy_eigenvalue"])
                condition = float(row["range_condition"])
                error = float(row["relative_inverse_error"])
                if not all(map(math.isfinite, (minimum, condition, error))) or minimum <= 0 or condition < 1 - 1e-10 or error < 0:
                    errors.append(f"invalid range spectrum {key}")
                if row["method"] == "exact" and (error > 1e-8 or abs(condition - 1) > 1e-6):
                    errors.append(f"exact reference inconsistency {key}")
            elif any(row[field] != "NA" for field in (
                "symmetry_defect", "linearity_defect", "minimum_energy_eigenvalue",
                "range_condition", "relative_inverse_error",
            )):
                errors.append(f"dense diagnostics above the declared limit {key}")
        if seen != expected:
            errors.append(f"incomplete/extra matrix: missing={len(expected - seen)}, extra={len(seen - expected)}")
        for key, batch in groups.items():
            batch.sort(key=lambda row: int(row["rhs_count"]))
            for old, new in zip(batch, batch[1:]):
                for field in (*WORK_FIELDS, "solve_seconds", "max_true_residual"):
                    if float(new[field]) < float(old[field]):
                        errors.append(f"nonmonotone cumulative {field}: {key}")
                for field in ("setup_seconds", "domain_seconds", "workspace_seconds", "apply_seconds"):
                    if new[field] != old[field]:
                        errors.append(f"setup or apply changed across a reused build: {key}")
        cmg = [row for row in rows if row["method"] == "cmg-fixed"]
        if not cmg or max(int(row["cmg_levels"]) for row in cmg) <= 1:
            errors.append("CMG never exercised a multilevel path")
    except (KeyError, ValueError, OverflowError) as error:
        errors.append(f"invalid schema/value: {error}")
    return errors


def median(rows: list[dict[str, str]], field: str) -> float:
    return statistics.median(float(row[field]) for row in rows)


def render(rows: list[dict[str, str]], profile: str, errors: list[str]) -> str:
    text = [f"# Issue 4 pair-local {profile} evidence", "",
            f"Rows: {len(rows)}. Numerical/accounting gate: {'FAIL' if errors else 'PASS'}.", "",
            "This is an identical-domain, single-pair experiment, not a three-way end-to-end speed claim. "
            "Timing is descriptive; it is never a CI pass criterion.", ""]
    if errors:
        text += ["## Rejected evidence", "", *[f"- {error}" for error in errors], "",
                 "No performance winner or break-even is inferred from failed/partial evidence."]
        return "\n".join(text) + "\n"
    lookup = defaultdict(list)
    for row in rows:
        lookup[(row["fixture"], row["method"], int(row["rhs_count"]))].append(row)
    text += ["## CMG versus within default", "",
             "Speedup is within/CMG (greater than one favors CMG). The range is min/median/max "
             "of three paired repeat ratios, not a confidence interval. Operator-work ratio is "
             "CMG/within for B and B' calls, excluding separately recorded certification calls.", "",
             "| Fixture | RHS | Total speedup min / median / max | Operator-work ratio |",
             "|---|---:|---:|---:|"]
    fixtures = sorted({row["fixture"] for row in rows})
    for fixture in fixtures:
        for count in RHS:
            cmg = sorted(lookup[(fixture, "cmg-fixed", count)], key=lambda row: int(row["repeat"]))
            within = sorted(lookup[(fixture, "within-default", count)], key=lambda row: int(row["repeat"]))
            ratios = [float(w["total_seconds"]) / float(c["total_seconds"]) for c, w in zip(cmg, within)]
            work = [
                (int(c["solver_b"]) + int(c["solver_bt"])) / (int(w["solver_b"]) + int(w["solver_bt"]))
                for c, w in zip(cmg, within)
            ]
            text.append(f"| {fixture} | {count} | {min(ratios):.3f} / {statistics.median(ratios):.3f} / {max(ratios):.3f} | {statistics.median(work):.3f} |")
    text += ["", "## Conditional RHS crossover model", "",
             "S+n*T uses median charged setup and median time per RHS from the 32-RHS prefix. "
             "This assumes future RHS cost resembles that prefix; it is not a measured extrapolation "
             "or a routing rule. Strict integer wins exclude ties. Compare against the observed prefixes above.", "",
             "| Fixture | CMG setup (ms) | Within setup (ms) | CMG / within per RHS (ms) | Modeled CMG-winning RHS counts |",
             "|---|---:|---:|---:|---|"]
    for fixture in fixtures:
        cmg = lookup[(fixture, "cmg-fixed", 32)]
        within = lookup[(fixture, "within-default", 32)]
        cs = median(cmg, "setup_seconds") + median(cmg, "domain_seconds")
        ws = median(within, "setup_seconds") + median(within, "domain_seconds")
        ct = median(cmg, "solve_seconds") / 32
        wt = median(within, "solve_seconds") / 32
        text.append(f"| {fixture} | {cs * 1000:.3f} | {ws * 1000:.3f} | {ct * 1000:.4f} / {wt * 1000:.4f} | {break_even(cs, ct, ws, wt)} |")
    residual = max(float(row["max_true_residual"]) for row in rows)
    warnings = sum(int(row["warning_count"]) for row in rows if row["rhs_count"] == "32")
    recurrence = sum(row["recurrence_converged"] != "true" for row in rows)
    text += ["", "## Numerical and measurement boundaries", "",
             f"Maximum independently recomputed relative normal residual: {residual:.6e}. "
             f"Build warnings across measured builds: {warnings}. "
             f"Certified rows with a false recurrence flag: {recurrence}.", "",
             "Known memory columns are not total process memory. CMG's graph storage can be shared "
             "with the common graph; do not add those columns as disjoint allocations. The pinned "
             "within preconditioner is opaque, so its principal retained bytes are NA. Process peak "
             "RSS, when present, includes all routes and dense diagnostics in that process, not an "
             "attributable solver peak. See docs/ISSUE4_PAIR_LOCAL_PROTOCOL.md.", "",
             "Issue #4 remains open: large domains, fresh holdout, complete memory, multi-threading, "
             "changing weights, whole-system Schwarz and frozen coarse-hierarchy comparisons are not resolved here."]
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
    args.output.write_text(render(rows, args.profile, errors), encoding="utf-8")
    print(f"issue-4 pair-local: {'FAIL' if errors else 'PASS'} ({len(rows)} rows)")
    for error in errors:
        print(error)
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
