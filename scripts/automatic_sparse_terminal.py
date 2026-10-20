#!/usr/bin/env python3
"""Collect complete cold-process costs of the sparse-terminal policy and its controls.

Development-only evidence on the frozen automatic recipes (seed 10001). Each
(input, width) runs one unmeasured warmup per route, then three measured
repetitions in rotated route order. No timing gate, routing decision,
calibration or holdout input is involved.
"""
import argparse
import json
import math
import os
from pathlib import Path
import statistics
import subprocess
import sys
import time

import prepared_serial as base
from automatic_recipes import generate
from range_admission import cases as recipe_cases

POLICY = 'benchmarks/policies/prepared-automatic-v1.json'
EXAMPLE = 'prepared_automatic_sparse_benchmark'
BUILD = ['cargo', 'build', '--locked', '--release', '-p', 'multiway-mg', '--features',
         'sparse-terminal', '--example', EXAMPLE]
FILES = [POLICY, 'Cargo.lock', 'Cargo.toml', 'scripts/automatic_sparse_terminal.py',
         'scripts/range_admission.py', 'scripts/automatic_recipes.py',
         'scripts/test_automatic_sparse_terminal.py', 'crates/multiway-mg/Cargo.toml',
         f'crates/multiway-mg/examples/{EXAMPLE}.rs',
         'crates/multiway-mg/src/sparse_terminal.rs',
         'crates/multiway-mg/src/prepared_automatic.rs',
         'crates/multiway-mg/src/prepared_automatic/hierarchy.rs',
         'crates/multiway-mg/src/prepared_hierarchy.rs']
ROUTES = ['automatic', 'automatic-sparse-t0', 'automatic-sparse-t1', 'components-map',
          'global-map']
WIDTHS = [1, 32]
REPETITIONS = 3
TIMES = ['decode_ns', 'prepare_ns', 'driver_ns', 'total_ns']
COUNTS = ['components', 'large_components', 'hierarchy_attempts', 'accepted_hierarchies',
          'hierarchy_rejections', 'quality_rejections', 'baseline_components',
          'global_fallback_columns', 'sparse_terminals', 'sparse_terminal_peak_bytes',
          'maximum_admitted_bytes', 'local_incidence_applications',
          'global_incidence_applications', 'accepted_columns']


def parse_output(text):
    """Strict framing: every field exactly once, nothing unknown."""
    record = dict(time={}, count={})
    for line in text.splitlines():
        fields = line.split('\t')
        key = fields[0]
        if key == 'route' and len(fields) == 2 and 'route' not in record:
            if fields[1] not in ROUTES:
                raise ValueError(f'unknown route {fields[1]!r}')
            record['route'] = fields[1]
        elif key == 'dimensions' and len(fields) == 6 and 'dimensions' not in record:
            record['dimensions'] = [int(x) for x in fields[1:]]
        elif key in ('time', 'count') and len(fields) == 3:
            names = TIMES if key == 'time' else COUNTS
            if fields[1] not in names or fields[1] in record[key]:
                raise ValueError(f'unknown or duplicate {key} {fields[1]!r}')
            record[key][fields[1]] = int(fields[2])
        elif key == 'certificate' and len(fields) == 2 and 'certificate' not in record:
            record['certificate'] = float(fields[1])
        elif key == 'fingerprint' and len(fields) == 2 and 'fingerprint' not in record:
            int(fields[1], 16)
            record['fingerprint'] = fields[1]
        else:
            raise ValueError(f'malformed line {line!r}')
    missing = [k for k in ('route', 'dimensions', 'certificate', 'fingerprint')
               if k not in record]
    missing += [n for n in TIMES if n not in record['time']]
    missing += [n for n in COUNTS if n not in record['count']]
    if missing:
        raise ValueError(f'incomplete output: {missing}')
    return record


def geomean(values):
    return math.exp(sum(math.log(v) for v in values) / len(values)) if values else None


def summarize(rows):
    """Median driver/wall time per cell, then geometric-mean ratios between routes."""
    cells = {}
    for row in rows:
        key = (row['profile'], row['index'], row['width'], row['route'])
        cells.setdefault(key, []).append(row)
    medians = {}
    for key, runs in cells.items():
        record = runs[0]['record']
        medians[key] = dict(
            driver=statistics.median(r['record']['time']['driver_ns'] for r in runs),
            total=statistics.median(r['wall_ns'] for r in runs),
            rss=max(r['maxrss_kib'] for r in runs),
            accepted=all(r['record']['count']['accepted_columns'] == key[2] for r in runs),
            sparse=record['count']['sparse_terminals'],
            accepted_hierarchies=record['count']['accepted_hierarchies'],
            quality_rejections=record['count']['quality_rejections'],
            deterministic=len({r['record']['fingerprint'] for r in runs}) == 1,
        )
    inputs = {(p, index, width) for p, index, width, _ in medians}
    for p, index, width in inputs:
        for route in ROUTES:
            if (p, index, width, route) not in medians:
                raise ValueError(f'incomplete cell {p} {index} {width}: no {route}')
    summary = dict(processes=len(rows), cells=len(medians), routes={}, comparisons={})
    for route in ROUTES:
        mine = {k: v for k, v in medians.items() if k[3] == route}
        summary['routes'][route] = dict(
            cells=len(mine),
            all_columns_accepted=sum(v['accepted'] for v in mine.values()),
            deterministic=sum(v['deterministic'] for v in mine.values()),
            sparse_terminal_cells=sum(v['sparse'] > 0 for v in mine.values()),
            accepted_hierarchy_cells=sum(v['accepted_hierarchies'] > 0 for v in mine.values()),
            quality_rejection_cells=sum(v['quality_rejections'] > 0 for v in mine.values()),
            maximum_rss_kib=max((v['rss'] for v in mine.values()), default=0))
    for profile in ('smoke', 'development'):
        for candidate in ('automatic-sparse-t0', 'automatic-sparse-t1', 'automatic'):
            for reference in ('automatic', 'components-map', 'global-map'):
                if candidate == reference:
                    continue
                for metric in ('driver', 'total'):
                    ratios, engaged = [], []
                    for (p, index, width, route), value in medians.items():
                        if p != profile or route != candidate:
                            continue
                        other = medians[(p, index, width, reference)]
                        ratio = other[metric] / value[metric]
                        ratios.append(ratio)
                        if value['sparse'] > 0:
                            engaged.append(ratio)
                    name = f'{profile}:{candidate}/{reference}:{metric}'
                    summary['comparisons'][name] = dict(
                        cells=len(ratios), speedup_geomean=geomean(ratios),
                        engaged_cells=len(engaged), engaged_speedup_geomean=geomean(engaged))
    return summary


def run_process(binary, route, data, env, scratch):
    """Run one cold process; return status, stdout, stderr, wall ns and its own max RSS."""
    stdin_path = scratch / 'stdin.bin'
    stderr_path = scratch / 'stderr.txt'
    stdin_path.write_bytes(data)
    with stdin_path.open('rb') as stdin, stderr_path.open('wb') as stderr:
        started = time.perf_counter_ns()
        child = subprocess.Popen([str(binary), route], stdin=stdin, stdout=subprocess.PIPE,
                                 stderr=stderr, env=env)
        stdout = child.stdout.read()
        child.stdout.close()
        _, status, usage = os.wait4(child.pid, 0)
        wall = time.perf_counter_ns() - started
    # Linux reports ru_maxrss in KiB; this collector runs only there.
    return os.waitstatus_to_exitcode(status), stdout, stderr_path.read_bytes(), wall, \
        usage.ru_maxrss


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    if out.exists():
        raise ValueError('fresh output directory required; never overwrite evidence')
    if base.command(['git', 'status', '--porcelain']).strip():
        raise ValueError('commit source before collecting evidence')
    policy = json.loads((base.ROOT / POLICY).read_text())
    env = os.environ.copy()
    env.update(base.THREAD_ENV, LC_ALL='C', PYTHONDONTWRITEBYTECODE='1')
    build = subprocess.run(BUILD, cwd=base.ROOT, env=env, stdout=subprocess.PIPE,
                           stderr=subprocess.STDOUT)
    if build.returncode:
        raise RuntimeError(build.stdout.decode())
    target = Path(os.environ.get('CARGO_TARGET_DIR', base.ROOT / 'target')).resolve()
    binary = target / f'release/examples/{EXAMPLE}'
    meta = base.provenance(binary, dict(build_args=BUILD))
    for name in FILES:
        value = (base.ROOT / name).read_bytes()
        if value != base.command(['git', 'show', f"{meta['source_commit']}:{name}"]):
            raise ValueError(f'uncommitted source {name}')
        meta['source_hashes'][name] = base.sha(value)
    out.mkdir(parents=True)
    (out / 'build.log').write_bytes(build.stdout)
    rows = []
    with (out / 'runs.jsonl').open('x') as journal:
        for index, (profile, case) in enumerate(recipe_cases()):
            for width in WIDTHS:
                data, dims = generate(policy, profile, dict(case, width=width))
                order = ROUTES[index % len(ROUTES):] + ROUTES[:index % len(ROUTES)]
                for repetition in range(REPETITIONS + 1):
                    rotated = order[repetition % len(order):] + order[:repetition % len(order)]
                    for route in rotated:
                        status, stdout, stderr, wall, rss = run_process(
                            binary, route, data, env, out)
                        if status:
                            raise RuntimeError(f'{profile} {case} {route}: {stderr.decode()}')
                        record = parse_output(stdout.decode())
                        if record['route'] != route:
                            raise ValueError('route mismatch')
                        if repetition == 0:
                            continue
                        row = dict(index=index, profile=profile, case=case, width=width,
                                   route=route, repetition=repetition, dimensions=dims,
                                   input_sha256=base.sha(data), wall_ns=wall, maxrss_kib=rss,
                                   record=record)
                        rows.append(row)
                        journal.write(json.dumps(row, sort_keys=True, allow_nan=False) + '\n')
                journal.flush()
            print(f"recorded {profile} {case['family']} {case['shape']} {case['weights']}",
                  flush=True)
    (out / 'stdin.bin').unlink()
    (out / 'stderr.txt').unlink()
    summary = summarize(rows)
    manifest = dict(schema=1, scope='automatic_sparse_terminal_development_only',
                    policy_path=POLICY, routes=ROUTES, widths=WIDTHS,
                    repetitions=REPETITIONS, warmups=1, provenance=meta,
                    build_log_sha256=base.sha(build.stdout),
                    runs_sha256=base.sha((out / 'runs.jsonl').read_bytes()), python=sys.version)
    base.dump(out / 'manifest.json', manifest)
    base.dump(out / 'summary.json', summary)
    print(json.dumps(summary, indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
