#!/usr/bin/env python3
"""Fail-closed unit tests for the whole-system evidence validator."""
from __future__ import annotations

import copy
import unittest

from summarize_issue4_whole_system import CASES, METHODS, RHS, SOLVERS, render, validate


class ValidatorTests(unittest.TestCase):
    @staticmethod
    def rows():
        rows = []
        for case in CASES:
            for repeat in range(2):
                for method in METHODS:
                    for solver in SOLVERS:
                        for rhs in RHS:
                            is_cmg = method == "pair-cmg-schwarz"
                            rows.append({
                                "profile": "smoke", "case": case, "repeat": str(repeat),
                                "method": method, "solver": solver, "rhs": str(rhs),
                                "factor1": "32", "factor2": "24", "factor3": "8",
                                "tuples": "128", "components": "1",
                                "constructor_seconds": "0.002", "initialization_seconds": "0.001",
                                "setup_seconds": "0.003", "solve_seconds": "0.004",
                                "iterations": "5", "converged": "true", "certified": "true",
                                "true_residual": "1e-12", "outer_work": "12",
                                "work_unit": "rectangular-operator" if solver == "mlsmr" else "gramian",
                                "preconditioner_applications": "6",
                                "certificate_work": "3" if solver == "mlsmr" else "0",
                                "known_retained_bytes": "NA" if method == "diagonal" else "4096",
                                "pair_components": "3" if is_cmg else "0",
                                "max_pair_vertices": "32" if is_cmg else "0",
                                "max_pair_edges": "64" if is_cmg else "0",
                                "max_pair_cycle_excess": "33" if is_cmg else "0",
                                "max_pair_levels": "3" if is_cmg else "0",
                                "multilevel_pair_components": "2" if is_cmg else "0",
                                "fallback_allocations": "0", "warning_count": "0",
                                "stop_reason": "Converged", "error": "",
                            })
        return rows

    def test_complete_matrix_passes(self):
        rows = self.rows()
        self.assertEqual(validate(rows), [])
        self.assertIn("CMG versus within", render(rows, []))

    def test_missing_duplicate_and_failure_rows_reject_evidence(self):
        rows = self.rows()
        self.assertTrue(validate(rows[:-1]))
        self.assertTrue(validate(rows + [copy.deepcopy(rows[0])]))
        for field, value in (
            ("certified", "false"), ("true_residual", "nan"), ("error", "boom"),
            ("fallback_allocations", "1"), ("outer_work", "0"),
            ("setup_seconds", "99"), ("converged", "false"),
        ):
            with self.subTest(field=field):
                bad = copy.deepcopy(rows)
                bad[0][field] = value
                errors = validate(bad)
                self.assertTrue(errors)
                self.assertIn("Rejected evidence", render(bad, errors))

    def test_cmg_metadata_is_required_and_forbidden_on_other_routes(self):
        rows = self.rows()
        cmg = next(row for row in rows if row["method"] == "pair-cmg-schwarz")
        cmg["pair_components"] = "0"
        self.assertTrue(validate(rows))
        rows = self.rows()
        within = next(row for row in rows if row["method"] == "within-default")
        within["max_pair_levels"] = "1"
        self.assertTrue(validate(rows))


if __name__ == "__main__":
    unittest.main()
