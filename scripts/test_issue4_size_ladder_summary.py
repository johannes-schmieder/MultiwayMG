#!/usr/bin/env python3
"""Fail-closed tests for issue-4 size-ladder evidence."""
from __future__ import annotations

import copy
import unittest

from summarize_issue4_size_ladder import (
    FAMILIES, LEVELS, METHODS, PREFIXES, SOLVERS, render, validate,
)


class SizeLadderValidatorTests(unittest.TestCase):
    @staticmethod
    def rows():
        rows = []
        for family in FAMILIES:
            for levels in LEVELS:
                for repeat in range(2):
                    for method in METHODS:
                        for solver in SOLVERS:
                            for rhs in PREFIXES:
                                cmg = method == "pair-cmg-schwarz"
                                rows.append({
                                    "family": family, "levels": str(levels), "repeat": str(repeat),
                                    "method": method, "solver": solver, "rhs_count": str(rhs),
                                    "tuples": str(levels * levels), "components": "1",
                                    "constructor_seconds": "0.002", "initialization_seconds": "0.001",
                                    "setup_seconds": "0.003",
                                    "cumulative_solve_seconds": str(rhs * 0.004),
                                    "setup_plus_solve_seconds": str(0.003 + rhs * 0.004),
                                    "cumulative_iterations": str(rhs * 5),
                                    "cumulative_outer_work": str(rhs * 12),
                                    "work_unit": "rectangular-operator" if solver == "mlsmr" else "gramian",
                                    "cumulative_preconditioner_applications": str(rhs * 6),
                                    "cumulative_certificate_work": str(rhs * 3 if solver == "mlsmr" else 0),
                                    "max_true_residual": "1e-12", "converged": "true", "certified": "true",
                                    "known_retained_bytes": "NA" if method == "diagonal" else "4096",
                                    "pair_components": "3" if cmg else "0",
                                    "max_pair_vertices": str(levels * 2 if cmg else 0),
                                    "max_pair_edges": str(levels * levels if cmg else 0),
                                    "max_pair_cycle_excess": "10" if cmg else "0",
                                    "max_pair_levels": "3" if cmg else "0",
                                    "multilevel_pair_components": "3" if cmg else "0",
                                    "direct_pair_components": "2" if cmg else "0",
                                    "full_contraction_components": "1" if cmg else "0",
                                    "stagnated_vertex_components": "0",
                                    "stagnated_fill_components": "0",
                                    "maximum_levels_components": "0",
                                    "one_level_iterative_components": "0",
                                    "direct_factor_components": "2" if cmg else "0",
                                    "fallback_allocations": "0", "warning_count": "0", "error": "",
                                })
        return rows

    def test_complete_frozen_matrix_passes(self):
        rows = self.rows()
        self.assertEqual(len(rows), 864)
        self.assertEqual(validate(rows), [])
        report = render(rows, [])
        self.assertIn("32-RHS outer-work ladder", report)
        self.assertIn("Observed fully charged economics", report)

    def test_missing_duplicate_and_failed_rows_reject_evidence(self):
        rows = self.rows()
        self.assertTrue(validate(rows[:-1]))
        self.assertTrue(validate(rows + [copy.deepcopy(rows[0])]))
        for field, value in (
            ("certified", "false"), ("converged", "false"),
            ("max_true_residual", "nan"), ("error", "failure"),
            ("fallback_allocations", "1"), ("setup_seconds", "99"),
            ("cumulative_outer_work", "0"),
        ):
            with self.subTest(field=field):
                bad = copy.deepcopy(rows)
                bad[0][field] = value
                errors = validate(bad)
                self.assertTrue(errors)
                self.assertIn("Rejected evidence", render(bad, errors))

    def test_terminal_accounting_is_fail_closed(self):
        rows = self.rows()
        cmg = next(row for row in rows if row["method"] == "pair-cmg-schwarz")
        cmg["direct_pair_components"] = "1"
        self.assertTrue(validate(rows))
        rows = self.rows()
        within = next(row for row in rows if row["method"] == "within-default")
        within["full_contraction_components"] = "1"
        self.assertTrue(validate(rows))

    def test_cumulative_work_must_be_monotone(self):
        rows = self.rows()
        key = ("planted-clones", 12, 0, "pair-cmg-schwarz", "mlsmr")
        selected = [
            row for row in rows
            if (row["family"], int(row["levels"]), int(row["repeat"]), row["method"], row["solver"]) == key
        ]
        selected.sort(key=lambda row: int(row["rhs_count"]))
        selected[-1]["cumulative_outer_work"] = "1"
        self.assertTrue(validate(rows))


if __name__ == "__main__":
    unittest.main()
