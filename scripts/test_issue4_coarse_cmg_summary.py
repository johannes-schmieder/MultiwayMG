#!/usr/bin/env python3
"""Fail-closed tests for issue-4 coarse-CMG evidence semantics."""
from __future__ import annotations

import copy
import unittest

from summarize_issue4_coarse_cmg import (
    COLUMNS,
    EXPECTED_CASES,
    METHODS,
    PREFIXES,
    SOLVERS,
    validate,
)


def accepted_case_rows(case: str) -> list[dict[str, str]]:
    family, depth = EXPECTED_CASES[case]
    terminal_dimension = 24
    terminal_tuples = 32
    dimensions = [
        terminal_dimension * (2 ** (depth - level)) for level in range(depth + 1)
    ]
    tuples = [terminal_tuples * (2 ** (depth - level)) for level in range(depth + 1)]
    rows: list[dict[str, str]] = []
    for repeat in (0, 1):
        for method in METHODS:
            hybrid = method == "within-fine-cmg-coarse"
            components = 3 * (depth - 1) if hybrid else 0
            numerical_setup = 0.020 + 0.002 * int(hybrid) + 0.001 * repeat
            initialization = 0.003 + 0.0005 * int(hybrid) + 0.0002 * repeat
            for solver in SOLVERS:
                solver_factor = 1.05 if solver == "pcg-traced" else 1.0
                method_factor = 1.02 if hybrid else 1.0
                for rhs in PREFIXES:
                    cumulative_solve = rhs * 0.01 * solver_factor * method_factor
                    charged = 0.01 + numerical_setup + initialization + cumulative_solve
                    rows.append(
                        {
                            "case": case,
                            "family": family,
                            "requested_depth": str(depth),
                            "plan_accepted": "true",
                            "plan_depth": str(depth),
                            "plan_seconds": "0.01",
                            "dimension_complexity": "1.75",
                            "tuple_complexity": "1.75",
                            "method": method,
                            "repeat": str(repeat),
                            "solver": solver,
                            "rhs_count": str(rhs),
                            "fine_dimension": str(dimensions[0]),
                            "fine_tuples": str(tuples[0]),
                            "level_dimensions": ";".join(map(str, dimensions)),
                            "level_tuples": ";".join(map(str, tuples)),
                            "numerical_setup_seconds": str(numerical_setup),
                            "initialization_seconds": str(initialization),
                            "setup_plus_solve_seconds": str(charged),
                            "cumulative_solve_seconds": str(cumulative_solve),
                            "cumulative_iterations": str(rhs * 5),
                            "cumulative_outer_work": str(rhs * 10),
                            "work_unit": (
                                "rectangular-operator"
                                if solver == "mlsmr"
                                else "gramian"
                            ),
                            "cumulative_preconditioner_applications": str(rhs * 5),
                            "cumulative_certificate_work": (
                                str(rhs * 3) if solver == "mlsmr" else "0"
                            ),
                            "max_true_residual": "1e-11",
                            "converged": "true",
                            "certified": "true",
                            "known_retained_bytes": str(4096 + 1024 * int(hybrid)),
                            "cmg_components": str(components),
                            "cmg_max_vertices": "64" if hybrid else "0",
                            "cmg_max_edges": "128" if hybrid else "0",
                            "cmg_max_cycle_excess": "32" if hybrid else "0",
                            "cmg_max_levels": "4" if hybrid else "0",
                            "cmg_multilevel_components": str(components),
                            "cmg_direct_components": "0",
                            "cmg_full_contraction_components": "0",
                            "cmg_stagnated_vertex_components": str(components),
                            "cmg_stagnated_fill_components": "0",
                            "cmg_maximum_levels_components": "0",
                            "cmg_one_level_iterative_components": "0",
                            "fallback_allocations": "0",
                            "warning_count": "0",
                            "error": "",
                        }
                    )
    return rows


def rejected_case_row(case: str) -> dict[str, str]:
    family, depth = EXPECTED_CASES[case]
    row = {
        "case": case,
        "family": family,
        "requested_depth": str(depth),
        "plan_accepted": "false",
        "plan_depth": "0",
        "plan_seconds": "0.01",
        "dimension_complexity": "1.0",
        "tuple_complexity": "1.0",
        "method": "NA",
        "repeat": "0",
        "solver": "NA",
        "rhs_count": "0",
        "fine_dimension": str(24 * (2**depth)),
        "fine_tuples": str(32 * (2**depth)),
        "level_dimensions": "NA",
        "level_tuples": "NA",
        "numerical_setup_seconds": "0",
        "initialization_seconds": "0",
        "setup_plus_solve_seconds": "0",
        "cumulative_solve_seconds": "0",
        "cumulative_iterations": "0",
        "cumulative_outer_work": "0",
        "work_unit": "NA",
        "cumulative_preconditioner_applications": "0",
        "cumulative_certificate_work": "0",
        "max_true_residual": "inf",
        "converged": "false",
        "certified": "false",
        "known_retained_bytes": "0",
        "cmg_components": "0",
        "cmg_max_vertices": "0",
        "cmg_max_edges": "0",
        "cmg_max_cycle_excess": "0",
        "cmg_max_levels": "0",
        "cmg_multilevel_components": "0",
        "cmg_direct_components": "0",
        "cmg_full_contraction_components": "0",
        "cmg_stagnated_vertex_components": "0",
        "cmg_stagnated_fill_components": "0",
        "cmg_maximum_levels_components": "0",
        "cmg_one_level_iterative_components": "0",
        "fallback_allocations": "0",
        "warning_count": "0",
        "error": "plan rejected: synthetic gate",
    }
    assert tuple(row) == COLUMNS
    return row


def oracle_rows() -> list[dict[str, str]]:
    return [row for case in EXPECTED_CASES for row in accepted_case_rows(case)]


def automatic_rows(
    accepted_case: str = "recursive-nearly-nested-depth-2-seed-804",
) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for case in EXPECTED_CASES:
        if case == accepted_case:
            rows.extend(accepted_case_rows(case))
        else:
            rows.append(rejected_case_row(case))
    return rows


def mark_failed(rows: list[dict[str, str]], predicate) -> None:
    for row in rows:
        if predicate(row):
            row["cumulative_iterations"] = "0"
            row["cumulative_outer_work"] = "0"
            row["cumulative_preconditioner_applications"] = "0"
            row["cumulative_certificate_work"] = "0"
            row["max_true_residual"] = "inf"
            row["converged"] = "false"
            row["certified"] = "false"
            row["error"] = "preconditioner not positive definite"


class CoarseCmgValidatorTests(unittest.TestCase):
    def test_complete_oracle_matrix_is_comparable(self):
        rows = oracle_rows()
        self.assertEqual(len(rows), 256)
        comparable, rejected, baseline_bad = validate(rows, "oracle")
        self.assertEqual(len(comparable), 256)
        self.assertEqual(rejected, [])
        self.assertEqual(baseline_bad, [])

    def test_complete_automatic_mix_preserves_all_fixtures(self):
        rows = automatic_rows()
        self.assertEqual(len(rows), 39)
        comparable, rejected, baseline_bad = validate(rows, "automatic")
        self.assertEqual(len(comparable), 32)
        self.assertEqual(len(rejected), 7)
        self.assertEqual(baseline_bad, [])

    def test_missing_or_unknown_fixture_rejects_evidence(self):
        rows = oracle_rows()
        missing = next(iter(EXPECTED_CASES))
        rows = [row for row in rows if row["case"] != missing]
        with self.assertRaisesRegex(ValueError, "fixture coverage mismatch"):
            validate(rows, "oracle")

        rows = oracle_rows()
        rows[0]["case"] = "unexpected-fixture"
        with self.assertRaisesRegex(ValueError, "unexpected frozen fixture"):
            validate(rows, "oracle")

    def test_duplicate_and_mixed_plan_rows_reject_evidence(self):
        rows = oracle_rows()
        rows.append(copy.deepcopy(rows[0]))
        with self.assertRaisesRegex(ValueError, "must have 32 rows"):
            validate(rows, "oracle")

        rows = automatic_rows()
        rejected_case = next(
            case
            for case in EXPECTED_CASES
            if case != "recursive-nearly-nested-depth-2-seed-804"
        )
        rows.extend(accepted_case_rows(rejected_case))
        with self.assertRaisesRegex(ValueError, "mixes accepted and rejected"):
            validate(rows, "automatic")

    def test_baseline_failure_excludes_entire_case_symmetrically(self):
        rows = oracle_rows()
        bad_case = "recursive-communities-depth-3-seed-807"
        mark_failed(
            rows,
            lambda row: (
                row["case"] == bad_case and row["method"] == "within-all-levels"
            ),
        )
        comparable, rejected, baseline_bad = validate(rows, "oracle")
        self.assertEqual(len(comparable), 224)
        self.assertEqual(rejected, [])
        self.assertEqual(baseline_bad, [bad_case])
        self.assertNotIn(bad_case, {row["case"] for row in comparable})

    def test_hybrid_only_failure_remains_fail_closed(self):
        rows = oracle_rows()
        bad_case = "recursive-latin-depth-2-seed-800"
        mark_failed(
            rows,
            lambda row: (
                row["case"] == bad_case
                and row["method"] == "within-fine-cmg-coarse"
                and row["solver"] == "mlsmr"
                and row["repeat"] == "0"
            ),
        )
        with self.assertRaisesRegex(ValueError, "uncertified comparison batch"):
            validate(rows, "oracle")

    def test_fallback_depth_and_schema_errors_are_hard_failures(self):
        for field, value, message in (
            ("fallback_allocations", "1", "fallback allocation"),
            ("plan_depth", "1", "plan depth mismatch"),
        ):
            with self.subTest(field=field):
                rows = oracle_rows()
                rows[0][field] = value
                with self.assertRaisesRegex(ValueError, message):
                    validate(rows, "oracle")

        rows = oracle_rows()
        del rows[0]["warning_count"]
        with self.assertRaisesRegex(ValueError, "schema mismatch"):
            validate(rows, "oracle")

    def test_charged_total_and_prefix_monotonicity_are_audited(self):
        rows = oracle_rows()
        rows[0]["setup_plus_solve_seconds"] = "99"
        with self.assertRaisesRegex(ValueError, "charged total mismatch"):
            validate(rows, "oracle")

        rows = oracle_rows()
        selected = [
            row
            for row in rows
            if row["case"] == "recursive-latin-depth-2-seed-800"
            and row["method"] == "within-all-levels"
            and row["solver"] == "mlsmr"
            and row["repeat"] == "0"
        ]
        selected.sort(key=lambda row: int(row["rhs_count"]))
        selected[-1]["cumulative_outer_work"] = "1"
        with self.assertRaisesRegex(ValueError, "nonmonotone cumulative_outer_work"):
            validate(rows, "oracle")

    def test_solver_and_cmg_accounting_are_fail_closed(self):
        rows = oracle_rows()
        lsmr = next(row for row in rows if row["solver"] == "mlsmr")
        lsmr["cumulative_certificate_work"] = "0"
        with self.assertRaisesRegex(ValueError, "certificate accounting"):
            validate(rows, "oracle")

        rows = oracle_rows()
        hybrid = next(
            row for row in rows if row["method"] == "within-fine-cmg-coarse"
        )
        hybrid["cmg_stagnated_vertex_components"] = "0"
        with self.assertRaisesRegex(ValueError, "terminal coverage mismatch"):
            validate(rows, "oracle")

    def test_oracle_mode_does_not_admit_rejected_plan_rows(self):
        rows = automatic_rows()
        with self.assertRaisesRegex(ValueError, "oracle-map mode rejected"):
            validate(rows, "oracle")


if __name__ == "__main__":
    unittest.main()
