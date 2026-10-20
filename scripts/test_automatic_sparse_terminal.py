"""Strict parsing and aggregation of sparse-terminal cost evidence."""
import unittest

from automatic_sparse_terminal import COUNTS, ROUTES, TIMES, parse_output, summarize


def sample(route='automatic-sparse-t0', driver=100, sparse=1, accepted=1, fingerprint='00ab'):
    lines = [f'route\t{route}', 'dimensions\t128\t128\t128\t16572\t1']
    lines += [f'time\t{name}\t{driver if name == "driver_ns" else 10}' for name in TIMES]
    values = dict.fromkeys(COUNTS, 0)
    values.update(sparse_terminals=sparse, accepted_columns=accepted)
    lines += [f'count\t{name}\t{values[name]}' for name in COUNTS]
    lines += ['certificate\t1.5e-9', f'fingerprint\t{fingerprint}']
    return '\n'.join(lines) + '\n'


class ParseTests(unittest.TestCase):
    def test_complete_record_parses(self):
        record = parse_output(sample())
        self.assertEqual(record['route'], 'automatic-sparse-t0')
        self.assertEqual(record['time']['driver_ns'], 100)
        self.assertEqual(record['count']['sparse_terminals'], 1)
        self.assertEqual(record['dimensions'], [128, 128, 128, 16572, 1])

    def test_malformed_records_are_rejected(self):
        text = sample()
        for broken in [
            text.replace('route\tautomatic-sparse-t0', 'route\tsomething'),
            text + 'count\tcomponents\t3\n',
            text.replace('count\tsparse_terminals', 'count\tsparse'),
            '\n'.join(line for line in text.splitlines() if not line.startswith('certificate')),
            text.replace('fingerprint\t00ab', 'fingerprint\tzz'),
            text + 'extra\t1\n',
        ]:
            with self.assertRaises(ValueError):
                parse_output(broken)


class SummaryTests(unittest.TestCase):
    def test_ratios_use_cell_medians_and_flag_engagement(self):
        rows = []
        for route, driver in zip(ROUTES, [400, 100, 400, 300, 200]):
            for repetition, scale in enumerate([1.0, 1.1, 0.9], start=1):
                rows.append(dict(profile='development', index=0, width=1, route=route,
                                 repetition=repetition, wall_ns=driver * 2, maxrss_kib=10,
                                 record=parse_output(sample(route, int(driver * scale),
                                                            int(route.endswith('t0'))))))
        summary = summarize(rows)
        self.assertEqual(summary['processes'], 15)
        t0 = summary['comparisons']['development:automatic-sparse-t0/global-map:driver']
        self.assertAlmostEqual(t0['speedup_geomean'], 2.0)
        self.assertEqual(t0['engaged_cells'], 1)
        self.assertEqual(summary['routes']['automatic-sparse-t0']['sparse_terminal_cells'], 1)
        self.assertEqual(summary['routes']['automatic']['all_columns_accepted'], 1)
        self.assertEqual(summary['routes']['automatic']['deterministic'], 1)

    def test_differing_fingerprints_are_not_deterministic(self):
        rows = [dict(profile='smoke', index=0, width=1, route=route, repetition=r,
                     wall_ns=1, maxrss_kib=1,
                     record=parse_output(sample(route, 5, 0, 1,
                                                f'{r:04x}' if route == 'automatic' else '0')))
                for route in ROUTES for r in (1, 2, 3)]
        summary = summarize(rows)
        self.assertEqual(summary['routes']['automatic']['deterministic'], 0)
        self.assertEqual(summary['routes']['global-map']['deterministic'], 1)
        with self.assertRaises(ValueError):
            summarize([row for row in rows if row['route'] != 'global-map'])


if __name__ == '__main__':
    unittest.main()
