"""Strict parsing and aggregation of range-admission evidence."""
import unittest

from range_admission import ACTIONS, FLOATS, INTEGERS, parse_output, summarize


def sample(leak=1e-14, extended=1):
    lines = [f'{key}\t{index + 1}' for index, key in enumerate(INTEGERS)]
    lines = [line for line in lines if not line.startswith(('additional_nullity',
                                                            'large_extended_components',
                                                            'exact_nullity',
                                                            'oracle_unit_nullity',
                                                            'oracle_weighted_nullity'))]
    lines += [f'additional_nullity\t{extended}', f'large_extended_components\t{extended}',
              'exact_nullity\t9', 'oracle_unit_nullity\t9', 'oracle_weighted_nullity\t8']
    lines += [f'{key}\t1e-15' for key in FLOATS]
    lines.append('action_header\tname\tfull_symmetry\tquotient_symmetry\trange_leakage\t'
                 'minimum_energy\tmaximum_energy\tnegative\tnear_zero\tsymmetric\t'
                 'preserves_range\tpositive_on_range')
    for name in ACTIONS:
        good = name == 'certified-full-both'
        value = leak if good else 0.25
        lines.append(f'action\t{name}\t1e-16\t1e-16\t{value}\t0.5\t2.0\t0\t0\ttrue\t'
                     f"{'true' if good else 'false'}\ttrue")
    return '\n'.join(lines) + '\n'


class ParseTests(unittest.TestCase):
    def test_complete_record_parses(self):
        record = parse_output(sample())
        self.assertEqual(record['exact_nullity'], 9)
        self.assertEqual(list(record['actions']), ACTIONS)
        self.assertTrue(record['actions']['certified-full-both']['preserves_range'])
        self.assertFalse(record['actions']['structural-both']['preserves_range'])

    def test_malformed_records_are_rejected(self):
        text = sample()
        for broken in [
            text.replace('exact_rank', 'exact_rnak'),
            text + 'dimension\t5\n',
            text.replace('\ttrue\n', '\tyes\n', 1),
            '\n'.join(line for line in text.splitlines() if 'structural-both' not in line),
            text.replace('action_header', 'header'),
            '\n'.join(line for line in text.splitlines() if not line.startswith('projector_ns')),
        ]:
            with self.assertRaises(ValueError):
                parse_output(broken)

    def test_summary_separates_kernel_classes(self):
        rows = [dict(record=parse_output(sample(1e-14, 1))),
                dict(record=parse_output(sample(1e-15, 0)))]
        summary = summarize(rows)
        self.assertEqual(summary['runs'], 2)
        self.assertEqual(summary['additional_nullity_cases'], 1)
        self.assertEqual(summary['oracle_unit_agreement'], 2)
        self.assertEqual(summary['oracle_weighted_agreement'], 0)
        full = summary['actions']['certified-full-both']
        self.assertEqual(full['admitted'], 2)
        self.assertEqual(full['maximum_range_leakage_with_additional_nullity'], 1e-14)
        self.assertEqual(full['maximum_range_leakage_structural_kernel'], 1e-15)
        self.assertEqual(summary['actions']['structural-both']['admitted'], 0)


if __name__ == '__main__':
    unittest.main()
