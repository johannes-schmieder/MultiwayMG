#!/usr/bin/env python3
"""Fail-closed tests for issue-4 coarse-CMG evidence semantics."""
from __future__ import annotations

import copy
import unittest

from summarize_issue4_coarse_cmg import METHODS, PREFIXES, SOLVERS, validate


class CoarseCmgValidatorTests(unittest.TestCase):
    @staticmethod
    def rows(case: str = "fixture-a", depth: int = 3):
        rows = []
        for repeat in (0, 1):
            for method in METHODS:
                for solver in SOLVERS:
                    for rhs in PREFIXES:
                        hybrid = method == "within-fine-cmg-coarse"
                        rows.append(
                            {
                                "case": case,
                                "family": "synthetic",
                                "requested_depth": str(depth),
                                "plan_accepted": "true",
                                "plan_depth": str(depth),
                                "plan_seconds": "0.01",
                                "dimension_complexity": "1.5",
                                "tuple_complexity": "1.5",
                                "method": method,
                                "repeat": str(repeat),
                                "solver": solver,
                                "rhs_count": str(rhs),
                                "fine_dimension": "96",
                                "fine_tuples": "256",
                                "level_dimensions": "96;48;24",
                                "level_tuples": "256;128;64",
                                "numerical_setup_seconds": "0.02",
                                "initialization_seconds": "0.001",
                                "setup_plus_solve_seconds": str(0.031 + rhs * 0.01),
                                "cumulative_solve_seconds": str(rhs * 0.01),
                                "cumulative_iterations": str(rhs * 5),
                                "cumulative_outer_work": str(rhs * 10),
                                "work_unit": "rectangular-operator" if solver == "mlsmr" else "gramian",
                                "cumulative_preconditioner_applications": str(rhs * 5),
                                "cumulative_certificate_work": str(rhs * 2 if solver == "mlsmr" else 0),
                                "max_true_residual": "1e-11",
                                "converged": "true",
                                "certified": "true",
                                "known_retained_bytes": "4096",
                                "cmg_components": "6" if hybrid else "0",
                                "cmg_max_vertices": "64" if hybrid else "0",
                                "cmg_max_edges": "128" if hybrid else "0",
                                "cmg_max_cycle_excess": "32" if hybrid else "0",
                                "cmg_max_levels": "4" if hybrid else "0",
                                "cmg_multilevel_components": "6" if hybrid else "0",
                                "cmg_direct_components": "0",
                                "cmg_full_contraction_components": "0",
                                "cmg_stagnated_vertex_components": "6" if hybrid else "0",
                                "cmg_stagnated_fill_components": "0",
                                "cmg_maximum_levels_components": "0",
                                "cmg_one_level_iterative_components": "0",
                                "fallback_allocations": "0",
                                "warning_count": "0",
                                "error": "",
                            }
                        )
        return rows

    def test_complete_certified_case_is_comparable(self):
        rows = self.rows()
        comparable, rejected, baseline_bad = validate(rows)
        self.assertEqual(len(comparable), len(rows))
        self.assertEqual(rejected, [])
        self.assertEqual(baseline_bad, [])

    def test_baseline_failure_excludes_entire_case_symmetrically(self):
        good = self.rows("good")
        bad = self.rows("baseline-bad")
        baseline = next(row for row in bad if row["method"] == "within-all-levels")
        baseline["converged"] = "false"
        baseline["certified"] = "false"
        baseline["error"] = "preconditioner not positive definite"
        comparable, rejected, baseline_bad = validate(good + bad)
        self.assertEqual({row["case"] for row in comparable}, {"good"})
        self.assertEqual(rejected, [])
        self.assertEqual(baseline_bad, ["baseline-bad"])

    def test_hybrid_only_failure_remains_fail_closed(self):
        rows = self.rows()
        hybrid = next(row for row in rows if row["method"] == "within-fine-cmg-coarse")
        hybrid["converged"] = "false"
        hybrid["certified"] = "false"
        hybrid["error"] = "preconditioner not positive definite"
        with self.assertRaisesRegex(ValueError, "uncertified comparison batch"):
            validate(rows)

    def test_fallback_and_structural_mislabel_remain_hard_failures(self):
        for field, value, message in (
            ("fallback_allocations", "1", "fallback allocation"),
            ("plan_depth", "2", "plan depth mismatch"),
        ):
            with self.subTest(field=field):
                rows = copy.deepcopy(self.rows())
                rows[0][field] = value
                with self.assertRaisesRegex(ValueError, message):
                    validate(rows)


if __name__ == "__main__":
    unittest.main()
