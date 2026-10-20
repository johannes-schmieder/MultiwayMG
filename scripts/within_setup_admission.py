#!/usr/bin/env python3
"""Collect checked `within` setup admission on the frozen automatic recipes.

Development-only evidence on the frozen automatic recipes (seed 10001, width 1).
No timing gate, routing decision, calibration or holdout input is involved.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

import prepared_serial as base
from automatic_recipes import generate
from range_admission import cases

POLICY = 'benchmarks/policies/prepared-automatic-v1.json'
EXAMPLE = 'within_setup_admission'
BUILD = ['cargo', 'build', '--locked', '--release', '-p', 'multiway-mg', '--features',
         'within-comparator', '--example', EXAMPLE]
FILES = [POLICY, 'Cargo.lock', 'Cargo.toml', 'scripts/within_setup_admission.py',
         'scripts/range_admission.py', 'scripts/automatic_recipes.py',
         'scripts/test_within_setup_admission.py', 'crates/multiway-mg/Cargo.toml',
         f'crates/multiway-mg/examples/{EXAMPLE}.rs',
         'crates/multiway-mg/src/within_comparator.rs']
INTEGERS = ['dimension', 'tuples', 'unchecked_build_ns', 'checked_build_ns', 'setup_peak_bytes',
            'retained_bytes', 'checked_factor_bytes', 'unchecked_factor_bytes']
BOOLEANS = ['admitted_at_peak', 'refused_one_under', 'serial_actions_identical']
FLOATS = ['wrapper_action_difference']


def parse_output(text):
    """Strict framing: every key exactly once, nothing unknown."""
    record = {}
    for line in text.splitlines():
        fields = line.split('\t')
        key = fields[0]
        if key not in INTEGERS + BOOLEANS + FLOATS:
            raise ValueError(f'unknown output key {key!r}')
        if key in record or len(fields) != 2:
            raise ValueError(f'duplicate or malformed {key}')
        value = fields[1]
        if key in INTEGERS:
            record[key] = int(value)
        elif key in BOOLEANS:
            if value not in ('true', 'false'):
                raise ValueError(f'invalid boolean {value!r}')
            record[key] = value == 'true'
        else:
            record[key] = float(value)
    missing = [key for key in INTEGERS + BOOLEANS + FLOATS if key not in record]
    if missing:
        raise ValueError(f'incomplete output: {missing}')
    return record


def summarize(rows):
    """Admission outcomes, payload extremes and the checked/unchecked comparison."""
    summary = dict(runs=len(rows), admitted_at_peak=0, refused_one_under=0,
                   serial_actions_identical=0, maximum_wrapper_action_difference=0.0,
                   maximum_setup_peak_bytes=0, maximum_retained_bytes=0,
                   maximum_peak_to_retained=0.0, minimum_peak_to_retained=None,
                   maximum_retained_to_checked_factor=0.0, checked_factor_not_above_unchecked=0,
                   checked_to_unchecked_build_time_median=None)
    ratios = []
    for row in rows:
        record = row['record']
        for flag in ('admitted_at_peak', 'refused_one_under', 'serial_actions_identical'):
            summary[flag] += record[flag]
        summary['maximum_wrapper_action_difference'] = max(
            summary['maximum_wrapper_action_difference'], record['wrapper_action_difference'])
        summary['maximum_setup_peak_bytes'] = max(summary['maximum_setup_peak_bytes'],
                                                  record['setup_peak_bytes'])
        summary['maximum_retained_bytes'] = max(summary['maximum_retained_bytes'],
                                                record['retained_bytes'])
        peak_ratio = record['setup_peak_bytes'] / record['retained_bytes']
        summary['maximum_peak_to_retained'] = max(summary['maximum_peak_to_retained'], peak_ratio)
        low = summary['minimum_peak_to_retained']
        summary['minimum_peak_to_retained'] = peak_ratio if low is None else min(low, peak_ratio)
        summary['maximum_retained_to_checked_factor'] = max(
            summary['maximum_retained_to_checked_factor'],
            record['retained_bytes'] / record['checked_factor_bytes'])
        summary['checked_factor_not_above_unchecked'] += (record['checked_factor_bytes']
                                                          <= record['unchecked_factor_bytes'])
        ratios.append(record['checked_build_ns'] / record['unchecked_build_ns'])
    if ratios:
        ratios.sort()
        middle = len(ratios) // 2
        summary['checked_to_unchecked_build_time_median'] = (
            ratios[middle] if len(ratios) % 2 else (ratios[middle - 1] + ratios[middle]) / 2)
    return summary


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
        for index, (profile, case) in enumerate(cases()):
            data, dims = generate(policy, profile, case)
            child = subprocess.run([str(binary)], input=data, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, env=env, timeout=600)
            stem = f'{index:03d}'
            (out / f'{stem}.stdout').write_bytes(child.stdout)
            (out / f'{stem}.stderr').write_bytes(child.stderr)
            if child.returncode:
                raise RuntimeError(f'{profile} {case} failed: {child.stderr.decode()}')
            record = parse_output(child.stdout.decode())
            row = dict(index=index, profile=profile, case=case, dimensions=dims,
                       input_sha256=base.sha(data), stdout_sha256=base.sha(child.stdout),
                       record=record)
            rows.append(row)
            journal.write(json.dumps(row, sort_keys=True, allow_nan=False) + '\n')
            journal.flush()
            print(f"recorded {profile} {case['family']} {case['shape']} {case['weights']}",
                  flush=True)
    summary = summarize(rows)
    manifest = dict(schema=1, scope='within_setup_admission_development_only',
                    policy_path=POLICY, provenance=meta,
                    build_log_sha256=base.sha(build.stdout),
                    runs_sha256=base.sha((out / 'runs.jsonl').read_bytes()), python=sys.version)
    base.dump(out / 'manifest.json', manifest)
    base.dump(out / 'summary.json', summary)
    print(json.dumps(summary, indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
