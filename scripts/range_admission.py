#!/usr/bin/env python3
"""Collect exact kernel certificates and fixed-action range diagnostics.

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
from automatic_recipes import FAMILIES, SHAPES, generate

POLICY = 'benchmarks/policies/prepared-automatic-v1.json'
EXAMPLE = 'range_admission_matrix'
BUILD = ['cargo', 'build', '--locked', '--release', '-p', 'multiway-mg', '--features',
         'within-comparator', '--example', EXAMPLE]
FILES = [POLICY, 'Cargo.lock', 'Cargo.toml', 'scripts/range_admission.py',
         'scripts/automatic_recipes.py', 'scripts/test_range_admission.py',
         'crates/multiway-mg/Cargo.toml', f'crates/multiway-mg/examples/{EXAMPLE}.rs',
         'crates/multiway-mg/src/range_projected.rs',
         'crates/multiway-incidence/src/kernel_certificate.rs',
         'crates/multiway-incidence/src/kernel_certificate/modular.rs',
         'crates/multiway-incidence/src/kernel_certificate/projector.rs']
INTEGERS = ['dimension', 'tuples', 'components', 'largest_component', 'exact_rank', 'exact_nullity',
            'additional_nullity', 'extended_components', 'large_extended_components',
            'eliminated_tuples', 'stored_pivot_entries', 'maximum_basis_magnitude',
            'certificate_peak_bytes', 'certificate_retained_bytes', 'certification_ns',
            'projector_peak_bytes', 'projector_retained_bytes', 'projector_ns',
            'oracle_unit_nullity', 'oracle_weighted_nullity']
FLOATS = ['projector_orthogonality', 'projector_residual']
ACTIONS = ['within-output', 'structural-both', 'certified-full-both']
ACTION_FIELDS = ['full_symmetry', 'quotient_symmetry', 'range_leakage', 'minimum_energy',
                 'maximum_energy', 'negative', 'near_zero', 'symmetric', 'preserves_range',
                 'positive_on_range']


def boolean(text):
    if text not in ('true', 'false'):
        raise ValueError(f'invalid boolean {text!r}')
    return text == 'true'


def parse_output(text):
    """Strict framing: every scalar once, header once, each action once in order."""
    record, actions, header = {}, [], False
    for line in text.splitlines():
        fields = line.split('\t')
        key = fields[0]
        if key == 'action_header':
            if header or fields[1:] != ['name'] + ACTION_FIELDS:
                raise ValueError('invalid action header')
            header = True
        elif key == 'action':
            if not header or len(fields) != 2 + len(ACTION_FIELDS):
                raise ValueError('invalid action row')
            values = fields[2:]
            action = dict(name=fields[1])
            for name, value in zip(ACTION_FIELDS, values):
                if name in ('negative', 'near_zero'):
                    action[name] = int(value)
                elif name in ('symmetric', 'preserves_range', 'positive_on_range'):
                    action[name] = boolean(value)
                else:
                    action[name] = float(value)
            actions.append(action)
        elif key in INTEGERS or key in FLOATS:
            if key in record or len(fields) != 2:
                raise ValueError(f'duplicate or malformed {key}')
            record[key] = int(fields[1]) if key in INTEGERS else float(fields[1])
        else:
            raise ValueError(f'unknown output key {key!r}')
    missing = [key for key in INTEGERS + FLOATS if key not in record]
    if missing or [action['name'] for action in actions] != ACTIONS:
        raise ValueError(f'incomplete output: {missing}')
    record['actions'] = {action.pop('name'): action for action in actions}
    return record


def summarize(rows):
    """Aggregate per-action admission outcomes and exact/oracle agreement."""
    summary = dict(runs=len(rows), oracle_unit_agreement=0, oracle_weighted_agreement=0,
                   additional_nullity_cases=0, large_additional_nullity_cases=0,
                   maximum_certificate_peak_bytes=0, maximum_stored_pivot_entries=0,
                   maximum_basis_magnitude=0, maximum_projector_orthogonality=0.0,
                   maximum_projector_residual=0.0, actions={})
    for name in ACTIONS:
        summary['actions'][name] = dict(symmetric=0, preserves_range=0, positive_on_range=0,
                                        admitted=0, maximum_range_leakage=0.0,
                                        maximum_range_leakage_with_additional_nullity=0.0,
                                        maximum_range_leakage_structural_kernel=0.0)
    for row in rows:
        record = row['record']
        summary['oracle_unit_agreement'] += record['oracle_unit_nullity'] == record['exact_nullity']
        summary['oracle_weighted_agreement'] += (record['oracle_weighted_nullity']
                                                 == record['exact_nullity'])
        extended = record['additional_nullity'] > 0
        summary['additional_nullity_cases'] += extended
        summary['large_additional_nullity_cases'] += record['large_extended_components'] > 0
        for key, source in [('maximum_certificate_peak_bytes', 'certificate_peak_bytes'),
                            ('maximum_stored_pivot_entries', 'stored_pivot_entries'),
                            ('maximum_basis_magnitude', 'maximum_basis_magnitude'),
                            ('maximum_projector_orthogonality', 'projector_orthogonality'),
                            ('maximum_projector_residual', 'projector_residual')]:
            summary[key] = max(summary[key], record[source])
        for name, action in record['actions'].items():
            target = summary['actions'][name]
            for flag in ('symmetric', 'preserves_range', 'positive_on_range'):
                target[flag] += action[flag]
            target['admitted'] += (action['symmetric'] and action['preserves_range']
                                   and action['positive_on_range'])
            leak = action['range_leakage']
            target['maximum_range_leakage'] = max(target['maximum_range_leakage'], leak)
            bucket = ('maximum_range_leakage_with_additional_nullity' if extended
                      else 'maximum_range_leakage_structural_kernel')
            target[bucket] = max(target[bucket], leak)
    return summary


def cases():
    for profile in ('smoke', 'development'):
        for family in FAMILIES:
            for shape in SHAPES:
                for weights in ('unit', 'heterogeneous'):
                    yield profile, dict(family=family, shape=shape, weights=weights, seed=10001,
                                        width=1)


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
    manifest = dict(schema=1, scope='range_admission_development_only', policy_path=POLICY,
                    provenance=meta, build_log_sha256=base.sha(build.stdout),
                    runs_sha256=base.sha((out / 'runs.jsonl').read_bytes()), python=sys.version)
    base.dump(out / 'manifest.json', manifest)
    base.dump(out / 'summary.json', summary)
    print(json.dumps(summary, indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
