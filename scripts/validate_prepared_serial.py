#!/usr/bin/env python3
"""Fail-closed validator for M4 serial development evidence, never M10 qualification."""
import argparse
import json
import math
from pathlib import Path
import re
import statistics
import subprocess

from prepared_serial import (ROOT, POLICY, SOURCE_FILES, THREAD_ENV, PHASES, PAYLOAD, WORK, MEMORY_SCOPES,
                             cases, case_id, generate, sha, parse_output, resources)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def integer(value, label, minimum=0):
    require(type(value) is int and value >= minimum, f'invalid {label}')


def finite(value, label, minimum=0.0):
    require(type(value) in (float, int) and math.isfinite(value) and value >= minimum, f'invalid {label}')


def no_nonfinite(value):
    if isinstance(value, float):
        require(math.isfinite(value), 'nonfinite JSON value')
    elif isinstance(value, dict):
        for v in value.values():
            no_nonfinite(v)
    elif isinstance(value, list):
        for v in value:
            no_nonfinite(v)


def hash_string(value, size=64):
    require(isinstance(value, str) and re.fullmatch('[0-9a-f]{' + str(size) + '}', value), 'invalid hash')


def check_work(work, route, certified=False):
    require(set(work) == set(WORK), 'missing/extra work scopes')
    for name, count in work.items():
        integer(count, f'work {name}')
    unused = ['weighted_incidence', 'weighted_adjoint'] if route == 'pcg' else ['gramian', 'rhs_adjoint', 'projection']
    require(all(work[key] == 0 for key in unused), 'inconsistent route work')
    if certified:
        require(work['certificate_incidence'] == 1 and work['certificate_adjoint'] == 2,
                'missing independent certificate work')


def check_probe(probe, run, policy):
    require(probe['schema'] == 1 and probe['route'] == run['route'], 'wrong probe provenance')
    require(probe['status'] in ('complete', 'error'), 'invalid probe status')
    require(probe['config'] == {key: policy[key] for key in ['native_tolerance', 'certificate_tolerance',
        'max_iterations', 'local_window', 'pcg_recompute_interval', 'terminal_relative_tolerance', 'process_budget_bytes']},
        'solver configuration differs from frozen policy')
    require(set(probe['phases_ns']) == set(PHASES), 'missing/extra phase')
    for name, value in probe['phases_ns'].items():
        integer(value, f'phase {name}')
    integer(probe['total_ns'], 'total', 1)
    integer(probe['overhead_ns'], 'overhead')
    require(probe['total_ns'] <= run['process_wall_ns'], 'inner total exceeds complete process cost')
    dims = probe.get('dimensions')
    if dims is not None:
        require(set(dims) == set(run['dimensions']) | {'terminal_rank'}, 'missing dimension')
        require(all(dims[k] == v for k, v in run['dimensions'].items()), 'wrong realized dimensions')
        integer(dims['terminal_rank'], 'terminal rank')
        terminal = run['dimensions']['coefficients'] // (1 << run['dimensions']['depth'])
        require(dims['terminal_rank'] <= terminal, 'impossible terminal rank')
    payload = probe.get('payload_bytes')
    if payload is not None:
        require(set(payload) == set(PAYLOAD), 'missing/extra payload scope')
        for name, value in payload.items():
            integer(value, f'payload {name}', 1)
        require(payload['total'] == sum(payload[name] for name in PAYLOAD[:-1]), 'invalid complete payload sum')
        require(payload['total'] <= policy['process_budget_bytes'], 'payload exceeds declared admission budget')
        require(payload['total'] <= run['resources']['peak_rss_bytes'], 'retained payload exceeds observed process peak')
        e, n, k = (run['dimensions'][key] for key in ('tuples', 'coefficients', 'rhs'))
        require(payload['caller_arrays'] >= 12*e + 8*(e+e*k+n*k), 'missing caller inputs/output capacity')
    columns = probe['columns']
    require(isinstance(columns, list) and len(columns) <= run['case']['width'], 'invalid RHS prefix')
    elapsed = sum(probe['phases_ns'].values())
    last_prefix = elapsed
    stops = (['ZeroRightHandSide', 'Converged', 'MaximumIterations'] if run['route'] == 'pcg' else
             ['ZeroRightHandSide', 'InitialNormalEquationResidualZero', 'ResidualTolerance',
              'NormalEquationTolerance', 'WarmStartExact', 'FalseConvergence', 'MaximumIterations', 'Escalated'])
    for j, column in enumerate(columns):
        require(column['index'] == j, 'missing/duplicate/reordered RHS column')
        integer(column['elapsed_ns'], 'column cost', 1)
        integer(column['prefix_ns'], 'prefix cost', 1)
        integer(column['iterations'], 'iterations')
        require(column['iterations'] <= policy['max_iterations'], 'iteration limit exceeded')
        require(type(column['accepted']) is bool and type(column['native_converged']) is bool, 'invalid status boolean')
        require(column['native_stop'] in stops, 'unknown native stop reason')
        for name in ['certificate', 'native_residual', 'native_secondary']:
            finite(column[name], name)
        if run['route'] == 'pcg':
            finite(column['native_projection'], 'native projection')
        else:
            require(column['native_projection'] is None, 'invented LSMR projection diagnostic')
        require(column['accepted'] == (column['certificate'] <= policy['certificate_tolerance']),
                'uncertified success or inconsistent independent acceptance')
        hash_string(column['coefficient_fnv1a64'], 16)
        check_work(column['work'], run['route'], certified=True)
        elapsed += column['elapsed_ns']
        require(column['prefix_ns'] >= last_prefix + column['elapsed_ns'], 'invalid prefix increment')
        require(elapsed <= column['prefix_ns'] <= probe['total_ns'], 'prefix omits charged setup/solve')
        last_prefix = column['prefix_ns']
        if j in (16, 31):
            require(column['certificate'] == 0.0 and column['iterations'] == 0, 'invalid declared zero RHS result')
    failed = probe.get('failed_action_ns', 0)
    integer(failed, 'failed action cost')
    require(probe['total_ns'] == elapsed + failed + probe['overhead_ns'], 'invalid phase total or omitted failed action cost')
    if probe['status'] == 'complete':
        require(run['exit_code'] == 0 and dims is not None and payload is not None, 'missing completed solve scope')
        require(all(v > 0 for v in probe['phases_ns'].values()), 'uncharged setup phase')
        require(len(columns) == run['case']['width'] and not failed and 'failed_work' not in probe,
                'incomplete or failed successful batch')
        require('error' not in probe and 'error_stage' not in probe, 'contradictory success')
    else:
        require(run['exit_code'] != 0 and probe.get('error'), 'missing failed route diagnostic')
        stage = probe.get('error_stage')
        require(stage in PHASES + ['solve_certificate_output'], 'missing failed route stage')
        if stage == 'solve_certificate_output':
            require(failed > 0 and len(columns) < run['case']['width'], 'uncharged failed RHS action')
            check_work(probe['failed_work'], run['route'])
        else:
            require(not columns and not failed and stage in PHASES and probe['phases_ns'][stage] > 0,
                    'uncharged failed preparation')
            require(all(probe['phases_ns'][p] == 0 for p in PHASES[PHASES.index(stage)+1:]),
                    'work after failed setup')
    return probe['status'] == 'complete' and all(c['accepted'] for c in columns)


def numerical_signature(probe):
    return {key: value for key, value in probe.items() if key in
            ('status', 'dimensions', 'payload_bytes', 'error_stage', 'error', 'failed_work')} | {
        'columns': [{k: v for k, v in column.items() if k not in ('elapsed_ns', 'prefix_ns')}
                    for column in probe['columns']]}


def _validate_manifest(manifest, expected_policy, expected_hashes):
    no_nonfinite(manifest)
    require(manifest['schema'] == 1 and manifest['scope'] == 'prepared_serial_development_only', 'wrong evidence scope')
    require(manifest['memory_scopes'] == MEMORY_SCOPES, 'missing or misleading memory scopes')
    policy = manifest['policy']
    require(policy == expected_policy, 'policy mismatch')
    require(manifest['policy_sha256'] == expected_hashes[POLICY], 'policy hash mismatch')
    meta = manifest['provenance']
    require(meta['source_clean'] is True and meta['source_hashes'] == expected_hashes, 'source provenance mismatch')
    for key in ('source_commit', 'source_tree'):
        hash_string(meta[key], 40)
    hash_string(meta['binary_sha256'])
    for value in expected_hashes.values():
        hash_string(value)
    require(meta['rustc'].startswith('rustc 1.85.0 ') and f"host: {meta['target']}" in meta['rustc'], 'wrong compiler or target')
    require(meta['rustflags'] == '' and meta['build_args'] == policy['build_args'], 'wrong build configuration')
    require(meta['compiler_overrides'] == {} and isinstance(meta['cargo_config_hashes'], dict), 'unrecorded compiler settings')
    require(meta['thread_env'] == THREAD_ENV and policy['workers'] == 1, 'wrong worker limits')
    require(meta['system'] in ('Darwin', 'Linux') and meta['hardware'] and meta['uname'] and meta['scheduler'],
            'missing hardware or scheduler provenance')
    if meta['affinity']['status'] == 'unavailable':
        require(meta['affinity'].get('reason'), 'missing affinity limitation')
    else:
        require(meta['affinity']['status'] == 'measured' and meta['affinity'].get('scope') and meta['affinity'].get('cpus'),
                'missing measured placement scope')
    profile = manifest['profile']
    require(profile in policy['profiles'], 'unknown profile')
    expected, data_hashes = {}, {}
    for case in cases(policy):
        data, dimensions = generate(policy, profile, case)
        data_hashes[case_id(case)] = sha(data)
        for kind, reps in [('warmup', policy['warmups']), ('measured', policy['repetitions'])]:
            for repeat in range(reps):
                routes = policy['routes'][repeat % 2:] + policy['routes'][:repeat % 2]
                for pos, route in enumerate(routes):
                    expected[(case_id(case), route, kind, repeat)] = (case, dimensions, pos)
    seen, signatures, metrics = set(), {}, {}
    certified, total_columns, failed_runs, rejected_columns = 0, 0, 0, 0
    for run in manifest['runs']:
        key = (run['case_id'], run['route'], run['kind'], run['repeat'])
        require(key in expected and key not in seen, 'missing/duplicate/unexpected matrix cell')
        seen.add(key)
        case, dimensions, pos = expected[key]
        require(run['case'] == case and run['dimensions'] == dimensions and run['position'] == pos, 'wrong paired cell/order')
        require(run['input_sha256'] == data_hashes[run['case_id']], 'input hash mismatch')
        integer(run['process_wall_ns'], 'complete process cost', 1)
        require(type(run['exit_code']) is int, 'missing process exit')
        for key_hash in ('raw_stdout_sha256', 'raw_stderr_sha256'):
            hash_string(run[key_hash])
        measured = run['kind'] == 'measured'
        if measured:
            total_columns += case['width']
        if run['status'] == 'timeout':
            require(run['exit_code'] != 0 and run['process_wall_ns'] >= policy['timeout_seconds'] * 1_000_000_000,
                    'uncharged timeout')
            require(run['resource_status'] == 'unavailable' and run.get('resource_reason'), 'missing timeout memory scope')
            require('probe' not in run and 'resources' not in run, 'invented timeout results')
            failed_runs += int(measured)
            continue
        require(run['status'] == 'returned' and run['resource_status'] == 'measured', 'invalid or missing process evidence')
        resource = run['resources']
        integer(resource['peak_rss_bytes'], 'isolated process peak RSS', 1)
        require(resource['scope'] == 'isolated_process_including_startup_teardown', 'wrong process memory scope')
        require(resource['method'] == ('darwin_time_l' if meta['system'] == 'Darwin' else 'gnu_time_v'), 'wrong memory units/method')
        for name in ['user_seconds', 'system_seconds']:
            finite(resource[name], name)
        passed = check_probe(run['probe'], run, policy)
        within_budget = resource['peak_rss_bytes'] <= policy['process_budget_bytes']
        signature_key = (run['case_id'], run['route'])
        sig = numerical_signature(run['probe'])
        if signature_key in signatures:
            require(signatures[signature_key] == sig, 'fixed-configuration numerical/work/payload nonrepeatability')
        signatures[signature_key] = sig
        if measured:
            successful_columns = sum(c['accepted'] for c in run['probe']['columns'])
            certified += successful_columns
            rejected_columns += sum(not c['accepted'] for c in run['probe']['columns'])
            failed_runs += int(not passed or not within_budget)
            metrics.setdefault(signature_key, []).append(run)
    require(seen == set(expected), 'missing matrix cells including warmups or failures')
    require([(r['case_id'], r['route'], r['kind'], r['repeat']) for r in manifest['runs']] == list(expected),
            'actual paired execution order mismatch')
    timing = []
    for (cid, route), runs in sorted(metrics.items()):
        complete = [r for r in runs if r['probe']['status'] == 'complete']
        if complete:
            times = [r['process_wall_ns']/1e9 for r in complete]
            phases = {name: statistics.median(r['probe']['phases_ns'][name]/1e9 for r in complete) for name in PHASES}
            timing.append(dict(case_id=cid, route=route, complete_repetitions=len(complete),
                process_seconds_median=statistics.median(times), process_seconds_min=min(times), process_seconds_max=max(times),
                phase_seconds_median=phases, peak_rss_bytes_max=max(r['resources']['peak_rss_bytes'] for r in complete),
                retained_capacity_bytes=complete[0]['probe']['payload_bytes']['total']))
    return dict(scope=manifest['scope'], profile=profile, expected_runs=len(expected), measured_columns=total_columns,
        certified_columns=certified, rejected_columns=rejected_columns, failed_measured_runs=failed_runs,
        all_measured_columns_certified=certified == total_columns and failed_runs == 0,
        competitive_qualification=False, timings=timing,
        memory_scopes=MEMORY_SCOPES)


def validate_manifest(manifest, expected_policy, expected_hashes):
    try:
        return _validate_manifest(manifest, expected_policy, expected_hashes)
    except (KeyError, TypeError, IndexError) as error:
        raise ValueError(f'malformed evidence: {error}') from error



def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def load(path):
    return json.loads(path.read_text(), object_pairs_hook=unique_object)


def validate_directory(directory):
    directory = Path(directory)
    manifest = load(directory / 'manifest.json')
    commit = manifest['provenance']['source_commit']
    hash_string(commit, 40)
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=ROOT)
    expected_tree = git('rev-parse', f'{commit}^{{tree}}').decode().strip()
    require(manifest['provenance']['source_tree'] == expected_tree, 'source tree mismatch')
    source = {name: git('show', f'{commit}:{name}') for name in SOURCE_FILES}
    hashes = {name: sha(value) for name, value in source.items()}
    policy = json.loads(source[POLICY])
    # The validator uses this recipe to regenerate bytes. Refuse to validate a different generator silently.
    require(hashes['scripts/prepared_serial.py'] == sha((ROOT / 'scripts/prepared_serial.py').read_bytes()),
            'use the measured source version of the input generator')
    require(sha((directory / 'prepared_serial_benchmark').read_bytes()) == manifest['provenance']['binary_sha256'], 'executable hash mismatch')
    require(sha((directory / 'build.log').read_bytes()) == manifest['build_log_sha256'], 'build log hash mismatch')
    def raw(name):
        require(isinstance(name, str) and Path(name).name == name and name not in ('.', '..'), 'unsafe artifact path')
        return (directory / name).read_bytes()
    for run in manifest['runs']:
        stdout, stderr = raw(run['raw_stdout']), raw(run['raw_stderr'])
        require(sha(stdout) == run['raw_stdout_sha256'] and sha(stderr) == run['raw_stderr_sha256'], 'raw evidence hash mismatch')
        if run['status'] == 'returned':
            require(parse_output(stdout.decode()) == run['probe'], 'raw probe disagrees with manifest')
            require(resources(stderr.decode(), manifest['provenance']['system']) == run['resources'], 'raw resources disagree with manifest')
    return validate_manifest(manifest, policy, hashes)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--require-certification', action='store_true')
    args = parser.parse_args()
    result = validate_directory(args.directory)
    print(json.dumps(result, indent=2, allow_nan=False))
    raise SystemExit(1 if args.require_certification and not result['all_measured_columns_certified'] else 0)
