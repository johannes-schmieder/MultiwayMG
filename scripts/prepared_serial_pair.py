#!/usr/bin/env python3
"""Frozen M5a old/new executable pairing; child evidence retains the M4 v2 contract."""
import argparse
import copy
import json
import os
from pathlib import Path
import subprocess
import time

from prepared_serial import (ROOT, POLICY_GATED, THREAD_ENV, MEMORY_SCOPES, cases, case_id,
                             generate, provenance, command, run_one, dump, sha)
from validate_prepared_serial import load, require, validate_directory

PAIR_POLICY = 'benchmarks/policies/prepared-serial-m5a-paired-v1.json'
PAIR_FILES = [PAIR_POLICY, 'scripts/prepared_serial_pair.py', 'scripts/validate_prepared_serial_pair.py']
BUILD_KEYS = ['source_commit', 'source_tree', 'binary_sha256', 'source_hashes', 'rustc', 'target',
              'build_args', 'rustflags', 'compiler_overrides', 'cargo_config_hashes']


def schedule(policy):
    indices = dict(baseline=0, candidate=0)
    for ci, case in enumerate(cases(policy)):
        for kind, repetitions in [('warmup', policy['warmups']), ('measured', policy['repetitions'])]:
            for repeat in range(repetitions):
                routes = policy['routes'][repeat % len(policy['routes']):] + policy['routes'][:repeat % len(policy['routes'])]
                for position, route in enumerate(routes):
                    roles = ['baseline', 'candidate'] if (ci + repeat + position) % 2 == 0 else ['candidate', 'baseline']
                    for role in roles:
                        yield role, indices[role], case, kind, repeat, position, route
                        indices[role] += 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('baseline', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--profile', choices=['smoke', 'development'], default='smoke')
    args = parser.parse_args()
    out = args.output.resolve()
    require(not out.exists(), 'use a fresh output directory; no overwrites or retries')
    require(not command(['git', 'status', '--porcelain']).strip(), 'commit source and paired recipe first')
    pair_policy = load(ROOT / PAIR_POLICY)
    policy = load(ROOT / POLICY_GATED)
    original_summary = validate_directory(args.baseline)
    original = load(args.baseline / 'manifest.json')
    require(original_summary['eligible_routes_certified'], 'baseline artifact failed candidate coverage')
    require(original['provenance']['source_commit'] == pair_policy['baseline_source'], 'wrong frozen baseline')
    require(original['policy_sha256'] == pair_policy['baseline_policy_sha256'] == sha((ROOT / POLICY_GATED).read_bytes()), 'changed v2 recipe')
    env = os.environ.copy()
    env.update(THREAD_ENV, LC_ALL='C', PYTHONDONTWRITEBYTECODE='1')
    build = subprocess.run(policy['build_args'], cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    require(build.returncode == 0, 'candidate release build failed: ' + build.stdout.decode(errors='replace'))
    binary = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve() / 'release/examples/prepared_serial_benchmark'
    meta = provenance(binary, policy)
    old_meta = original['provenance']
    require(meta['target'] == old_meta['target'] and meta['rustc'] == old_meta['rustc'], 'incompatible baseline compiler/target')
    require(old_meta['binary_sha256'] == pair_policy['baseline_binaries'].get(meta['target']), 'unrecognized frozen executable')
    out.mkdir(parents=True)
    children = {}
    for role in ['baseline', 'candidate']:
        folder = out / role
        folder.mkdir()
        child_meta = copy.deepcopy(meta)
        if role == 'baseline':
            child_meta.update({k: copy.deepcopy(old_meta[k]) for k in BUILD_KEYS})
        executable = (args.baseline / 'prepared_serial_benchmark') if role == 'baseline' else binary
        (folder / 'prepared_serial_benchmark').write_bytes(executable.read_bytes())
        (folder / 'prepared_serial_benchmark').chmod(0o755)
        log = (args.baseline / 'build.log').read_bytes() if role == 'baseline' else build.stdout
        (folder / 'build.log').write_bytes(log)
        children[role] = dict(schema=1, scope=policy['scope'], profile=args.profile, policy=policy,
            policy_path=POLICY_GATED, policy_sha256=sha((ROOT / POLICY_GATED).read_bytes()),
            provenance=child_meta, build_log_sha256=sha(log), memory_scopes=MEMORY_SCOPES, runs=[])
    manifest = dict(schema=1, scope=pair_policy['scope'], policy=pair_policy, profile=args.profile,
        source_commit=meta['source_commit'], source_hashes={name: sha((ROOT/name).read_bytes()) for name in PAIR_FILES},
        build_scope='candidate build log retained; frozen baseline executable reused; compilation outside solve timing',
        trace=[], total_ns=0)
    started = time.perf_counter_ns()
    current_case, data, dimensions = None, None, None
    for role, index, case, kind, repeat, position, route in schedule(policy):
        if case != current_case:
            if current_case is not None: print('paired ' + case_id(current_case), flush=True)
            data, dimensions = generate(policy, args.profile, case)
            current_case = case
        name = dict(case_id=case_id(case), case=case, dimensions=dimensions, route=route,
                    kind=kind, repeat=repeat, position=position)
        begin = time.perf_counter_ns() - started
        result = run_one(out/role/'prepared_serial_benchmark', data, policy, meta['system'], out/role, name, env)
        end = time.perf_counter_ns() - started
        children[role]['runs'].append(result)
        dump(out/role/'manifest.json', children[role])
        manifest['trace'].append(dict(role=role, index=index, begin_ns=begin, end_ns=end,
            run_sha256=sha(json.dumps(result, sort_keys=True, allow_nan=False).encode())))
        manifest['total_ns'] = time.perf_counter_ns() - started
        dump(out/'pair.json', manifest)
    print('paired ' + case_id(current_case), flush=True)
    for role in children:
        dump(out/role/'summary.json', validate_directory(out/role))
    from validate_prepared_serial_pair import validate_pair_directory
    summary = validate_pair_directory(out)
    dump(out/'summary.json', summary)
    print(json.dumps({k:v for k,v in summary.items() if k != 'timings'}, indent=2), flush=True)


if __name__ == '__main__': main()
