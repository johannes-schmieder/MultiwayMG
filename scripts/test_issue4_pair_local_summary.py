#!/usr/bin/env python3
"""Unit tests for strict crossover arithmetic and fail-closed evidence checks."""
import copy
import unittest
from summarize_issue4_pair_local import FAMILIES, METHODS, RHS, break_even, render, validate


class CrossoverTests(unittest.TestCase):
    def test_material_amortization_and_strict_tie(self):
        self.assertEqual(break_even(10, 1, 2, 3), "n >= 5")
        self.assertEqual(break_even(1, 1, 2, 3), "n >= 1")

    def test_equal_slopes(self):
        self.assertEqual(break_even(1, 2, 3, 2), "all n >= 1")
        self.assertEqual(break_even(3, 2, 3, 2), "never")
        self.assertEqual(break_even(4, 2, 3, 2), "never")

    def test_reverse_crossover_is_not_amortization(self):
        self.assertEqual(break_even(1, 3, 10, 1), "only 1 <= n <= 4; no long-run win")
        self.assertEqual(break_even(2, 3, 10, 1), "only 1 <= n <= 3; no long-run win")
        self.assertEqual(break_even(3, 3, 2, 1), "never")

    def test_invalid_costs_rejected(self):
        for bad in (-1.0, float("inf"), float("nan")):
            with self.assertRaises(ValueError):
                break_even(bad, 1, 1, 1)

    def test_partial_or_malformed_data_cannot_win(self):
        for rows in ([], [{"fixture": "path-32"}]):
            errors = validate(rows, "smoke")
            self.assertTrue(errors)
            report = render(rows, "smoke", errors)
            self.assertIn("FAIL", report)
            self.assertNotIn("## Conditional RHS crossover", report)


class MatrixTests(unittest.TestCase):
    @staticmethod
    def rows():
        rows = []
        for family in FAMILIES:
            for method in METHODS:
                for repeat in range(3):
                    for count in RHS:
                        rows.append({
                            "profile": "smoke", "fixture": f"{family}-32", "repeat": str(repeat),
                            "method": method, "vertices": "64", "edges": "96", "rhs_count": str(count),
                            "domain_seconds": "0.001", "setup_seconds": "0.002",
                            "workspace_seconds": "0.0001", "apply_seconds": "0.00001",
                            "solve_seconds": str(count * 0.0003),
                            "total_seconds": str(0.003 + count * 0.0003),
                            "solver_b": str(3 * count), "solver_bt": str(4 * count),
                            "preconditioner_calls": str(3 * count),
                            "certificate_b": str(count), "certificate_bt": str(2 * count),
                            "max_true_residual": "1e-12", "recurrence_converged": "true",
                            "certified": "true", "principal_solver_bytes": "NA" if method == "within-default" else "1024",
                            "known_workspace_bytes": "512", "common_graph_bytes": "4096",
                            "cmg_levels": "3" if method == "cmg-fixed" else "0",
                            "cmg_terminal": "Direct" if method == "cmg-fixed" else "NA",
                            "cmg_direct_factor": "true" if method == "cmg-fixed" else "NA",
                            "symmetry_defect": "1e-14", "linearity_defect": "1e-14",
                            "minimum_energy_eigenvalue": "1", "range_condition": "1",
                            "relative_inverse_error": "1e-14", "warning_count": "0", "error": "",
                        })
        return rows

    def test_complete_matrix_is_admitted(self):
        rows = self.rows()
        self.assertEqual(validate(rows, "smoke"), [])
        self.assertIn("## Conditional RHS crossover", render(rows, "smoke", []))

    def test_missing_and_duplicate_rows_are_rejected(self):
        rows = self.rows()
        self.assertTrue(validate(rows[:-1], "smoke"))
        self.assertTrue(validate(rows + [rows[0]], "smoke"))

    def test_poisoned_numerics_and_accounting_are_rejected(self):
        good = self.rows()
        for field, value in (
            ("certified", "false"), ("max_true_residual", "nan"),
            ("total_seconds", "0"), ("workspace_seconds", "1"),
            ("certificate_b", "0"), ("symmetry_defect", "inf"),
            ("minimum_energy_eigenvalue", "-1"), ("error", "solver failure"),
            ("principal_solver_bytes", "0"), ("warning_count", "-1"),
            ("recurrence_converged", "yes"),
        ):
            with self.subTest(field=field):
                rows = copy.deepcopy(good)
                rows[0][field] = value
                errors = validate(rows, "smoke")
                self.assertTrue(errors)
                self.assertNotIn("## Conditional RHS crossover", render(rows, "smoke", errors))

    def test_true_certificate_not_recurrence_flag_is_authority(self):
        rows = self.rows()
        rows[0]["recurrence_converged"] = "false"
        self.assertEqual(validate(rows, "smoke"), [])

    def test_opaque_memory_cannot_be_reported_as_zero(self):
        rows = self.rows()
        next(row for row in rows if row["method"] == "within-default")["principal_solver_bytes"] = "0"
        self.assertTrue(validate(rows, "smoke"))


if __name__ == "__main__":
    unittest.main()
