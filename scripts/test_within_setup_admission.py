"""Strict parsing and aggregation of checked within setup-admission evidence."""
import unittest

from within_setup_admission import BOOLEANS, FLOATS, INTEGERS, parse_output, summarize


def sample(peak=900, retained=600, identical='true', checked_ns=30, unchecked_ns=20):
    values = dict(dimension=40, tuples=120, unchecked_build_ns=unchecked_ns,
                  checked_build_ns=checked_ns, setup_peak_bytes=peak, retained_bytes=retained,
                  checked_factor_bytes=200, unchecked_factor_bytes=250)
    lines = [f'{key}\t{values[key]}' for key in INTEGERS]
    lines += ['admitted_at_peak\ttrue', 'refused_one_under\ttrue',
              f'serial_actions_identical\t{identical}']
    lines += [f'{key}\t1.5e-16' for key in FLOATS]
    return '\n'.join(lines) + '\n'


class ParseTests(unittest.TestCase):
    def test_complete_record_parses(self):
        record = parse_output(sample())
        self.assertEqual(record['setup_peak_bytes'], 900)
        self.assertTrue(record['serial_actions_identical'])
        self.assertEqual(record['wrapper_action_difference'], 1.5e-16)
        self.assertEqual(set(record), set(INTEGERS + BOOLEANS + FLOATS))

    def test_malformed_records_are_rejected(self):
        text = sample()
        for broken in [
            text.replace('retained_bytes', 'retained'),
            text + 'dimension\t5\n',
            text.replace('\ttrue\n', '\tyes\n', 1),
            '\n'.join(line for line in text.splitlines() if not line.startswith('tuples')),
            text.replace('setup_peak_bytes\t900', 'setup_peak_bytes\t9\t0'),
        ]:
            with self.assertRaises(ValueError):
                parse_output(broken)

    def test_summary_tracks_extremes_and_identity(self):
        rows = [dict(record=parse_output(sample(900, 600, 'true', 30, 20))),
                dict(record=parse_output(sample(1200, 400, 'false', 10, 20)))]
        summary = summarize(rows)
        self.assertEqual(summary['runs'], 2)
        self.assertEqual(summary['serial_actions_identical'], 1)
        self.assertEqual(summary['admitted_at_peak'], 2)
        self.assertEqual(summary['maximum_setup_peak_bytes'], 1200)
        self.assertEqual(summary['maximum_peak_to_retained'], 3.0)
        self.assertEqual(summary['minimum_peak_to_retained'], 1.5)
        self.assertEqual(summary['maximum_retained_to_checked_factor'], 3.0)
        self.assertEqual(summary['checked_factor_not_above_unchecked'], 2)
        self.assertEqual(summary['checked_to_unchecked_build_time_median'], 1.0)


if __name__ == '__main__':
    unittest.main()
