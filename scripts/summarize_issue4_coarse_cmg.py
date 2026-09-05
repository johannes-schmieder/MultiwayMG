#!/usr/bin/env python3
"""Validate and summarize the issue-4 coarse-only CMG calibration."""

from __future__ import annotations

import csv
import math
import statistics
import sys
from collections import defaultdict
from pathlib import Path

METHODS = ("within-all-levels", "within-fine-cmg-coarse")
SOLVERS = ("mlsmr", "pcg-traced")
PREFIXES = (1, 4, 16, 32)

EXPECTED_CASES = {
    "recursive-latin-depth-2-seed-800": ("recursive-latin", 2),
    "recursive-latin-depth-3-seed-801": ("recursive-latin", 3),
    "recursive-weak-chain-depth-2-seed-802": ("recursive-weak-chain", 2),
    "recursive-weak-chain-depth-3-seed-803": ("recursive-weak-chain", 3),
    "recursive-nearly-nested-depth-2-seed-804": ("recursive-nearly-nested", 2),
    "recursive-nearly-nested-depth-3-seed-805": ("recursive-nearly-nested", 3),
    "recursive-dominant-pair-depth-2-seed-806": ("recursive-dominant-pair", 2),
    "recursive-communities-depth-3-seed-807": ("recursive-communities", 3),
}

COLUMNS = tuple(
    """
    case family requested_depth plan_accepted plan_depth plan_seconds
    dimension_complexity tuple_complexity method repeat solver rhs_count
    fine_dimension fine_tuples level_dimensions level_tuples
    numerical_setup_seconds initialization_seconds setup_plus_solve_seconds
    cumulative_solve_seconds cumulative_iterations cumulative_outer_work work_unit
    cumulative_preconditioner_applications cumulative_certificate_work
    max_true_residual converged certified known_retained_bytes cmg_components
    cmg_max_vertices cmg_max_edges cmg_max_cycle_excess cmg_max_levels
    cmg_multilevel_components cmg_direct_components
    cmg_full_contraction_components cmg_stagnated_vertex_components
    cmg_stagnated_fill_components cmg_maximum_levels_components
    cmg_one_level_iterative_components fallback_allocations warning_count error
    """.split()
)
CMG_FIELDS = tuple(
    """
    cmg_components cmg_max_vertices cmg_max_edges cmg_max_cycle_excess
    cmg_max_levels cmg_multilevel_components cmg_direct_components
    cmg_full_contraction_components cmg_stagnated_vertex_components
    cmg_stagnated_fill_components cmg_maximum_levels_components
    cmg_one_level_iterative_components
    """.split()
)
CMG_TERMINALS = tuple(
    """
    cmg_direct_components cmg_full_contraction_components
    cmg_stagnated_vertex_components cmg_stagnated_fill_components
    cmg_maximum_levels_components
    """.split()
)
CASE_INVARIANTS = tuple(
    """
    family requested_depth plan_accepted plan_depth plan_seconds
    dimension_complexity tuple_complexity fine_dimension fine_tuples
    level_dimensions level_tuples
    """.split()
)
BUILD_INVARIANTS = (
    "numerical_setup_seconds",
    "initialization_seconds",
    "known_retained_bytes",
    *CMG_FIELDS,
    "fallback_allocations",
    "warning_count",
)
CUMULATIVE_FIELDS = tuple(
    """
    cumulative_solve_seconds setup_plus_solve_seconds cumulative_iterations
    cumulative_outer_work cumulative_preconditioner_applications
    cumulative_certificate_work max_true_residual
    """.split()
)


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        if tuple(reader.fieldnames or ()) != COLUMNS:
            raise ValueError(
                f"coarse-CMG TSV schema mismatch: expected {len(COLUMNS)} "
                f"ordered columns, found {reader.fieldnames!r}"
            )
        rows = list(reader)
    if not rows:
        raise ValueError("coarse-CMG evidence is empty")
    if any(None in row or None in row.values() for row in rows):
        raise ValueError("coarse-CMG TSV contains a malformed row")
    return rows


def median(values: list[float]) -> float:
    if not values:
        raise ValueError("cannot summarize an empty sample")
    return statistics.median(values)


def integer(
    row: dict[str, str],
    field: str,
    context: object,
    minimum: int | None = None,
) -> int:
    try:
        value = int(row[field])
    except (KeyError, TypeError, ValueError) as error:
        raise ValueError(
            f"invalid integer {field} in {context}: {row.get(field)!r}"
        ) from error
    if minimum is not None and value < minimum:
        raise ValueError(f"{field} below {minimum} in {context}: {value}")
    return value


def number(
    row: dict[str, str],
    field: str,
    context: object,
    minimum: float | None = None,
    finite: bool = True,
) -> float:
    try:
        value = float(row[field])
    except (KeyError, TypeError, ValueError) as error:
        raise ValueError(
            f"invalid number {field} in {context}: {row.get(field)!r}"
        ) from error
    if math.isnan(value) or (finite and not math.isfinite(value)):
        raise ValueError(f"nonfinite {field} in {context}: {value}")
    if minimum is not None and value < minimum:
        raise ValueError(f"{field} below {minimum} in {context}: {value}")
    return value


def sequence(row: dict[str, str], field: str, context: object) -> list[int]:
    value = row.get(field)
    if not value or value == "NA":
        raise ValueError(f"missing {field} in {context}")
    try:
        parsed = [int(part) for part in value.split(";")]
    except ValueError as error:
        raise ValueError(f"invalid {field} in {context}: {value!r}") from error
    if any(item <= 0 for item in parsed):
        raise ValueError(f"nonpositive {field} in {context}: {value!r}")
    return parsed


def check_schema(rows: list[dict[str, str]]) -> None:
    expected = set(COLUMNS)
    for index, row in enumerate(rows):
        if set(row) != expected or None in row or None in row.values():
            raise ValueError(
                f"schema mismatch in row {index}: "
                f"missing={sorted(expected - set(row))}, "
                f"extra={sorted(set(row) - expected)}"
            )


def check_identity(
    row: dict[str, str],
    context: object,
    family: str,
    depth: int,
) -> None:
    if row["family"] != family:
        raise ValueError(
            f"family mismatch in {context}: expected {family}, found {row['family']}"
        )
    if integer(row, "requested_depth", context) != depth:
        raise ValueError(
            f"requested depth mismatch in {context}: "
            f"expected {depth}, found {row['requested_depth']}"
        )


def validate_rejected_case(
    case: str,
    rows: list[dict[str, str]],
    family: str,
    depth: int,
) -> None:
    if len(rows) != 1:
        raise ValueError(f"rejected case {case} must have one row, found {len(rows)}")
    row = rows[0]
    context = (case, "rejected-plan")
    check_identity(row, context, family, depth)
    if row["plan_accepted"] != "false":
        raise ValueError(f"invalid rejected-plan status in {case}")
    if (row["method"], row["solver"], row["work_unit"]) != ("NA", "NA", "NA"):
        raise ValueError(f"rejected case {case} contains solver labels")
    if (row["level_dimensions"], row["level_tuples"]) != ("NA", "NA"):
        raise ValueError(f"rejected case {case} claims a built hierarchy")
    if integer(row, "repeat", context) != 0 or integer(row, "rhs_count", context) != 0:
        raise ValueError(f"rejected case {case} contains comparison indices")

    integer(row, "plan_depth", context, 0)
    number(row, "plan_seconds", context, 0.0)
    number(row, "dimension_complexity", context, 1.0)
    number(row, "tuple_complexity", context, 1.0)
    integer(row, "fine_dimension", context, 1)
    integer(row, "fine_tuples", context, 1)

    zero_numbers = (
        "numerical_setup_seconds",
        "initialization_seconds",
        "setup_plus_solve_seconds",
        "cumulative_solve_seconds",
    )
    zero_integers = (
        "cumulative_iterations",
        "cumulative_outer_work",
        "cumulative_preconditioner_applications",
        "cumulative_certificate_work",
        "known_retained_bytes",
        *CMG_FIELDS,
        "fallback_allocations",
        "warning_count",
    )
    for field in zero_numbers:
        if number(row, field, context) != 0.0:
            raise ValueError(f"rejected case {case} has nonzero {field}")
    for field in zero_integers:
        if integer(row, field, context) != 0:
            raise ValueError(f"rejected case {case} has nonzero {field}")

    residual = number(row, "max_true_residual", context, 0.0, finite=False)
    if not math.isinf(residual):
        raise ValueError(f"rejected case {case} must record +inf residual")
    if (row["converged"], row["certified"]) != ("false", "false"):
        raise ValueError(f"rejected case {case} claims solver success")
    if not row["error"].startswith("plan rejected:"):
        raise ValueError(f"rejected case {case} lacks a plan-rejection reason")


def validate_hierarchy_shape(
    row: dict[str, str],
    context: object,
    depth: int,
) -> None:
    dimensions = sequence(row, "level_dimensions", context)
    tuples = sequence(row, "level_tuples", context)
    if len(dimensions) != depth + 1 or len(tuples) != depth + 1:
        raise ValueError(
            f"hierarchy depth mismatch in {context}: "
            f"dimensions={dimensions}, tuples={tuples}"
        )
    if (
        dimensions[0] != integer(row, "fine_dimension", context, 1)
        or tuples[0] != integer(row, "fine_tuples", context, 1)
    ):
        raise ValueError(f"finest hierarchy metadata mismatch in {context}")
    if any(right >= left for left, right in zip(dimensions, dimensions[1:])):
        raise ValueError(f"coarse dimensions do not strictly decrease in {context}")
    if any(right >= left for left, right in zip(tuples, tuples[1:])):
        raise ValueError(f"coarse tuple counts do not strictly decrease in {context}")


def validate_cmg_metadata(
    row: dict[str, str],
    context: object,
    method: str,
) -> None:
    values = {field: integer(row, field, context, 0) for field in CMG_FIELDS}
    integer(row, "known_retained_bytes", context, 1)
    if method == "within-all-levels":
        nonzero = {field: value for field, value in values.items() if value}
        if nonzero:
            raise ValueError(
                f"within-only comparator contains CMG metadata in {context}: {nonzero}"
            )
        return

    components = values["cmg_components"]
    if components <= 0:
        raise ValueError(f"hybrid comparator lacks coarse CMG components in {context}")
    for field in ("cmg_max_vertices", "cmg_max_edges", "cmg_max_levels"):
        if values[field] <= 0:
            raise ValueError(f"hybrid comparator lacks {field} in {context}")
    terminal_sum = sum(values[field] for field in CMG_TERMINALS)
    if terminal_sum != components:
        raise ValueError(
            f"CMG terminal coverage mismatch in {context}: "
            f"{terminal_sum} != {components}"
        )
    multilevel = values["cmg_multilevel_components"]
    one_level = values["cmg_one_level_iterative_components"]
    if not 0 <= multilevel <= components or not 0 <= one_level <= components:
        raise ValueError(f"invalid CMG level counts in {context}")
    if multilevel + one_level > components:
        raise ValueError(f"overlapping CMG level accounting in {context}")


def validate_accepted_case(
    case: str,
    rows: list[dict[str, str]],
    family: str,
    depth: int,
) -> None:
    expected = {
        (method, repeat, solver, rhs)
        for method in METHODS
        for repeat in (0, 1)
        for solver in SOLVERS
        for rhs in PREFIXES
    }
    if len(rows) != len(expected):
        raise ValueError(
            f"accepted case {case} must have {len(expected)} rows, found {len(rows)}"
        )

    seen: set[tuple[str, int, str, int]] = set()
    case_values: dict[str, set[str]] = defaultdict(set)
    build_values: dict[tuple[int, str], dict[str, set[str]]] = defaultdict(
        lambda: defaultdict(set)
    )
    series: dict[tuple[int, str, str], list[dict[str, str]]] = defaultdict(list)

    for row in rows:
        context = (
            case,
            row.get("method"),
            row.get("repeat"),
            row.get("solver"),
            row.get("rhs_count"),
        )
        if row["plan_accepted"] != "true":
            raise ValueError(f"accepted case {case} contains a rejected row")
        check_identity(row, context, family, depth)
        method, solver = row["method"], row["solver"]
        repeat = integer(row, "repeat", context)
        rhs = integer(row, "rhs_count", context)
        key = (method, repeat, solver, rhs)
        if key not in expected:
            raise ValueError(f"unexpected comparison cell {(case, *key)}")
        if key in seen:
            raise ValueError(f"duplicate comparison cell {(case, *key)}")
        seen.add(key)

        if integer(row, "plan_depth", context) != depth:
            raise ValueError(f"accepted plan depth mismatch in {context}")
        plan = number(row, "plan_seconds", context, 0.0)
        number(row, "dimension_complexity", context, 1.0)
        number(row, "tuple_complexity", context, 1.0)
        validate_hierarchy_shape(row, context, depth)

        setup = number(row, "numerical_setup_seconds", context, 0.0)
        initialization = number(row, "initialization_seconds", context, 0.0)
        solve = number(row, "cumulative_solve_seconds", context, 0.0)
        charged = number(row, "setup_plus_solve_seconds", context, 0.0)
        expected_charged = plan + setup + initialization + solve
        if not math.isclose(
            charged, expected_charged, rel_tol=5.0e-8, abs_tol=1.0e-12
        ):
            raise ValueError(
                f"charged total mismatch in {context}: "
                f"{charged} != {expected_charged}"
            )

        for field in (
            "cumulative_iterations",
            "cumulative_outer_work",
            "cumulative_preconditioner_applications",
            "cumulative_certificate_work",
            "fallback_allocations",
            "warning_count",
        ):
            integer(row, field, context, 0)
        if integer(row, "fallback_allocations", context) != 0:
            raise ValueError(f"sequential fallback allocation in {context}")
        number(row, "max_true_residual", context, 0.0, finite=False)
        if row["converged"] not in ("true", "false"):
            raise ValueError(f"invalid converged flag in {context}")
        if row["certified"] not in ("true", "false"):
            raise ValueError(f"invalid certified flag in {context}")

        if solver == "mlsmr":
            if row["work_unit"] != "rectangular-operator":
                raise ValueError(f"bad MLSMR work unit in {context}")
        elif solver == "pcg-traced":
            if row["work_unit"] != "gramian":
                raise ValueError(f"bad PCG work unit in {context}")
            if integer(row, "cumulative_certificate_work", context) != 0:
                raise ValueError(f"unexpected PCG certificate work in {context}")

        validate_cmg_metadata(row, context, method)
        for field in CASE_INVARIANTS:
            case_values[field].add(row[field])
        for field in BUILD_INVARIANTS:
            build_values[(repeat, method)][field].add(row[field])
        series[(repeat, method, solver)].append(row)

    if seen != expected:
        raise ValueError(
            f"comparison coverage mismatch for {case}: "
            f"missing={sorted(expected - seen)}, extra={sorted(seen - expected)}"
        )
    for field, values in case_values.items():
        if len(values) != 1:
            raise ValueError(
                f"case metadata changed for {case}: {field}={sorted(values)}"
            )
    for build, fields in build_values.items():
        for field, values in fields.items():
            if len(values) != 1:
                raise ValueError(
                    f"build metadata changed for {(case, *build)}: "
                    f"{field}={sorted(values)}"
                )

    for series_key, group in series.items():
        group.sort(key=lambda row: int(row["rhs_count"]))
        if [int(row["rhs_count"]) for row in group] != list(PREFIXES):
            raise ValueError(f"prefix coverage mismatch in {(case, *series_key)}")
        for old, new in zip(group, group[1:]):
            for field in CUMULATIVE_FIELDS:
                old_value, new_value = float(old[field]), float(new[field])
                if new_value < old_value:
                    raise ValueError(
                        f"nonmonotone {field} in {(case, *series_key)}: "
                        f"{old_value} -> {new_value}"
                    )


def row_is_certified(row: dict[str, str]) -> bool:
    if row["converged"] != "true" or row["certified"] != "true" or row["error"]:
        return False
    try:
        residual = float(row["max_true_residual"])
    except (TypeError, ValueError):
        return False
    return math.isfinite(residual) and 0.0 <= residual <= 1.0e-8


def validate_success_accounting(row: dict[str, str]) -> None:
    context = (
        row["case"],
        row["method"],
        int(row["repeat"]),
        row["solver"],
        int(row["rhs_count"]),
    )
    for field in (
        "cumulative_iterations",
        "cumulative_outer_work",
        "cumulative_preconditioner_applications",
    ):
        if integer(row, field, context) <= 0:
            raise ValueError(f"missing successful solver work {field} in {context}")
    if number(row, "cumulative_solve_seconds", context) <= 0.0:
        raise ValueError(f"missing successful solve time in {context}")
    rhs = int(row["rhs_count"])
    certificate = integer(row, "cumulative_certificate_work", context)
    if row["solver"] == "mlsmr" and certificate != 3 * rhs:
        raise ValueError(
            f"bad MLSMR certificate accounting in {context}: "
            f"{certificate} != {3 * rhs}"
        )
    if row["solver"] == "pcg-traced" and certificate:
        raise ValueError(f"bad PCG certificate accounting in {context}")


def validate(
    rows: list[dict[str, str]],
    mode: str = "automatic",
) -> tuple[list[dict[str, str]], list[str], list[str]]:
    if mode not in ("automatic", "oracle"):
        raise ValueError(f"unknown calibration mode: {mode}")
    if not rows:
        raise ValueError("coarse-CMG evidence is empty")
    check_schema(rows)

    grouped: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        case = row["case"]
        if case not in EXPECTED_CASES:
            raise ValueError(f"unexpected frozen fixture: {case}")
        if row["plan_accepted"] not in ("true", "false"):
            raise ValueError(
                f"invalid plan_accepted flag for {case}: {row['plan_accepted']}"
            )
        grouped[case].append(row)

    expected, observed = set(EXPECTED_CASES), set(grouped)
    if observed != expected:
        raise ValueError(
            "frozen fixture coverage mismatch: "
            f"missing={sorted(expected - observed)}, extra={sorted(observed - expected)}"
        )

    accepted: list[dict[str, str]] = []
    rejected: list[str] = []
    for case, (family, depth) in EXPECTED_CASES.items():
        case_rows = grouped[case]
        statuses = {row["plan_accepted"] for row in case_rows}
        if len(statuses) != 1:
            raise ValueError(f"case {case} mixes accepted and rejected plan rows")
        status = next(iter(statuses))
        if mode == "oracle" and status != "true":
            raise ValueError(f"oracle-map mode rejected frozen fixture {case}")
        if status == "false":
            validate_rejected_case(case, case_rows, family, depth)
            rejected.append(case)
        else:
            validate_accepted_case(case, case_rows, family, depth)
            accepted.extend(case_rows)

    if not accepted:
        raise ValueError("no accepted frozen issue-3 hierarchy remains for comparison")
    for row in accepted:
        if row_is_certified(row):
            validate_success_accounting(row)

    baseline_bad = sorted(
        {
            row["case"]
            for row in accepted
            if row["method"] == "within-all-levels" and not row_is_certified(row)
        }
    )
    comparable = [row for row in accepted if row["case"] not in baseline_bad]
    if not comparable:
        raise ValueError("all accepted hierarchies fail the all-within baseline gate")
    for row in comparable:
        if not row_is_certified(row):
            key = (
                row["case"],
                row["method"],
                int(row["repeat"]),
                row["solver"],
                int(row["rhs_count"]),
            )
            raise ValueError(f"uncertified comparison batch {key}: {row['error']}")

    return comparable, sorted(rejected), baseline_bad


def ratios(rows: list[dict[str, str]], rhs: int = 32) -> list[dict[str, object]]:
    cells = {
        (row["case"], row["solver"], int(row["repeat"]), row["method"]): row
        for row in rows
        if int(row["rhs_count"]) == rhs
    }
    output: list[dict[str, object]] = []
    for case in sorted({row["case"] for row in rows}):
        family = next(row["family"] for row in rows if row["case"] == case)
        depth = int(next(row["requested_depth"] for row in rows if row["case"] == case))
        for solver in SOLVERS:
            work, charged, solve = [], [], []
            for repeat in (0, 1):
                baseline = cells[(case, solver, repeat, "within-all-levels")]
                hybrid = cells[(case, solver, repeat, "within-fine-cmg-coarse")]
                work.append(
                    float(hybrid["cumulative_outer_work"])
                    / float(baseline["cumulative_outer_work"])
                )
                charged.append(
                    float(hybrid["setup_plus_solve_seconds"])
                    / float(baseline["setup_plus_solve_seconds"])
                )
                solve.append(
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
                    "work_ratio": median(work),
                    "charged_time_ratio": median(charged),
                    "solve_time_ratio": median(solve),
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
        observed = [
            float(cells[(repeat, "within-fine-cmg-coarse")]["setup_plus_solve_seconds"])
            / float(cells[(repeat, "within-all-levels")]["setup_plus_solve_seconds"])
            for repeat in (0, 1)
        ]
        if median(observed) < 1.0:
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
            "| {case} | {depth} | {solver} | {work_ratio:.3f} | "
            "{solve_time_ratio:.3f} | {charged_time_ratio:.3f} | {win} | "
            "{cmg_components} | {cmg_max_vertices} | {cmg_max_levels} |".format(
                **row,
                win=first_timing_win(rows, str(row["case"]), str(row["solver"])),
            )
        )
    if rejected:
        lines += [
            "",
            "Rejected automatic plans (not compared): "
            + ", ".join(sorted(set(rejected)))
            + ".",
        ]
    if baseline_bad:
        lines += [
            "",
            "Baseline-inadmissible hierarchies (excluded from solver ratios because the all-`within` hierarchy itself failed the outer SPD/certification gate): "
            + ", ".join(sorted(set(baseline_bad)))
            + ".",
        ]
    work_wins = [row for row in summary if float(row["work_ratio"]) <= 0.80]
    time_wins = [row for row in summary if float(row["charged_time_ratio"]) < 1.0]
    lines += [
        "",
        f"Cells with at least 20% outer-work reduction at 32 RHS: {len(work_wins)}/{len(summary)}.",
        f"Cells with a fully charged timing win at 32 RHS: {len(time_wins)}/{len(summary)}.",
        "",
        "A routing rule must not be selected from these cases and then described as holdout-validated; any such rule requires a fresh preregistered issue-4 holdout.",
        "",
    ]
    output.write_text("\n".join(lines), encoding="utf-8")


def main(argv: list[str]) -> int:
    if len(argv) not in (3, 4):
        raise SystemExit(
            "usage: summarize_issue4_coarse_cmg.py "
            "INPUT.tsv SUMMARY.md [automatic|oracle]"
        )
    mode = argv[3] if len(argv) == 4 else "automatic"
    if mode not in ("automatic", "oracle"):
        raise SystemExit(f"unknown calibration mode: {mode}")
    comparable, rejected, baseline_bad = validate(read_rows(Path(argv[1])), mode)
    write_summary(comparable, rejected, baseline_bad, Path(argv[2]), mode)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
