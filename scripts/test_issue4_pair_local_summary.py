#!/usr/bin/env python3
"""Unit tests for strict crossover arithmetic and fail-closed evidence checks."""
import unittest
from summarize_issue4_pair_local import break_even, render, validate


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


if __name__ == "__main__":
    unittest.main()
