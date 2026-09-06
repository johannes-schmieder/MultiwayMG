#!/usr/bin/env python3
"""Frozen M4 serial development recipes and cold-process evidence collection.

Run only from a clean committed tree. Inputs are canonical unique tuples, not
observations. Every route receives identical serialized bytes through stdin.
"""
import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import platform
import re
import signal
import struct
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
POLICY = 'benchmarks/policies/prepared-serial-v1.json'
POLICY_GATED = 'benchmarks/policies/prepared-serial-gated-v2.json'
SOURCE_FILES = [POLICY, POLICY_GATED, 'Cargo.lock', 'Cargo.toml', 'scripts/prepared_serial.py',
                'scripts/validate_prepared_serial.py',
                'crates/multiway-mg/examples/prepared_serial_benchmark.rs']
THREAD_ENV = {key: '1' for key in ['RAYON_NUM_THREADS', 'OMP_NUM_THREADS',
              'OPENBLAS_NUM_THREADS', 'MKL_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS',
              'BLIS_NUM_THREADS']}
PHASES = ['decode', 'fine_topology', 'fine_frame', 'maps_topology', 'coarse_frames',
          'terminal', 'workspace_output']
PAYLOAD = ['fine_topology', 'coarse_topology', 'fine_frame', 'coarse_frames',
           'terminal', 'hierarchy_workspace', 'outer_workspace', 'caller_arrays', 'total']
WORK = ['weighted_incidence', 'weighted_adjoint', 'gramian', 'rhs_adjoint',
        'hierarchy', 'projection', 'certificate_incidence', 'certificate_adjoint']
GATE_KEYS = ['candidate_checks', 'candidate_vetoes', 'projection_applications',
             'certificate_incidence', 'certificate_adjoint']
MASK = (1 << 64) - 1
MEMORY_SCOPES = {'reserved_live_array_capacity': 'measured per direct owner, including caller input/output and workspace', 'logical_live_array_lengths': 'unavailable: capacity reports do not expose all lengths separately', 'construction_peak_live_allocations': 'unavailable: allocator high-water instrumentation is separate from this timing run', 'workspace_pool': 'one serial workspace; hierarchy_workspace plus outer_workspace are measured capacities', 'old_new_generation_overlap': 'not applicable: fresh fixed-weight single generation per process', 'opaque_dependency_memory': 'transient terminal factorization not separately observed; retained dense/workspace arrays included', 'process_peak_rss': 'isolated process including runtime, input decode, setup, solve, output and destruction'}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def dump(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n')


def mix(x):
    """SplitMix64 finalizer with its fixed Weyl increment; integer arithmetic only."""
    x = (x + 0x9e3779b97f4a7c15) & MASK
    x = ((x ^ (x >> 30)) * 0xbf58476d1ce4e5b9) & MASK
    x = ((x ^ (x >> 27)) * 0x94d049bb133111eb) & MASK
    return x ^ (x >> 31)


def case_id(case):
    return f"{case['family']}-{case['weights']}-s{case['seed']}-k{case['width']}"


def cases(policy):
    for f, w, s, k in itertools.product(policy['families'], policy['weights'],
                                       policy['seeds'], policy['widths']):
        yield dict(family=f, weights=w, seed=s, width=k)


def generate(policy, profile, case):
    recipe = policy['profiles'][profile]
    n, draws, depth = recipe['levels'], recipe['draws'], recipe['depth']
    # Scaffold connects every factor level in one exact incidence component.
    tuples = {(i, i, i) for i in range(n)} | {(i - 1, i, i) for i in range(1, n)}
    state = case['seed']
    for _ in range(draws):
        values = []
        for _ in range(3):
            state = mix(state)
            values.append(state)
        a, b, c = values
        if case['family'] == 'uniform':
            key = (a % n, b % n, c % n)
        elif case['family'] == 'communities':
            group = (a >> 32) % 4
            size = n // 4
            key = tuple(group * size + value % size for value in values)
        elif case['family'] == 'chain':
            anchor = a % n
            key = (anchor, (anchor + b % 9 - 4) % n, (anchor + c % 9 - 4) % n)
        else:
            raise ValueError('unknown family')
        tuples.add(key)
    tuples = sorted(tuples)
    e, k = len(tuples), case['width']
    result = bytearray(struct.pack('<8sIIIQII', b'MG3BEN1\0', n, n, n, e, k, depth))
    for key in tuples:
        result.extend(struct.pack('<III', *key))
    tuple_keys = [(a << 24) | (b << 12) | c for a, b, c in tuples]
    for key, packed in zip(tuples, tuple_keys):
        weight = 1.0
        if case['weights'] == 'heterogeneous':
            weight = 2.0 ** (int(mix(packed + case['seed']) % 13) - 6)
            if case['family'] == 'communities' and len({v // (n // 4) for v in key}) > 1:
                weight *= 2.0 ** -10
        elif case['weights'] != 'unit':
            raise ValueError('unknown weights')
        result.extend(struct.pack('<d', weight))
    for j in range(k):
        for key, packed in zip(tuples, tuple_keys):
            if j in (16, 31):
                y = 0.0
            elif j % 4 == 1:
                # Exactly representable manufactured fitted value; no trigonometric recipe.
                y = sum(((mix(case['seed'] + (j + 1) * 0x100000000 + q * 4096 + v) & 65535)
                         - 32768) / 32768.0 for q, v in enumerate(key))
            else:
                y = ((mix(packed + case['seed'] + (j + 1) * 0x100000000) & 65535) - 32768) / 32768.0
            result.extend(struct.pack('<d', y))
    return bytes(result), dict(tuples=e, coefficients=3 * n, rhs=k, depth=depth)


def parse_output(text):
    """Strict TSV framing. Semantic validation is deliberately separate."""
    record = {'phases_ns': {}, 'columns': []}
    seen = set()
    for line in text.splitlines():
        f = line.split('\t')
        tag = f[0]
        if tag not in ('phase', 'column', 'gate'):
            if tag in seen:
                raise ValueError(f'duplicate {tag}')
            seen.add(tag)
        if tag == 'schema' and len(f) == 2:
            record['schema'] = int(f[1])
        elif tag == 'route' and len(f) == 2:
            record['route'] = f[1]
        elif tag == 'config' and len(f) == 8:
            record['config'] = dict(native_tolerance=float(f[1]), certificate_tolerance=float(f[2]),
                max_iterations=int(f[3]), local_window=int(f[4]), pcg_recompute_interval=int(f[5]),
                terminal_relative_tolerance=float(f[6]), process_budget_bytes=int(f[7]))
        elif tag == 'dimensions' and len(f) == 6:
            record['dimensions'] = dict(zip(['tuples', 'coefficients', 'rhs', 'depth', 'terminal_rank'], map(int, f[1:])))
        elif tag == 'phase' and len(f) == 3 and f[1] in PHASES:
            if f[1] in record['phases_ns']:
                raise ValueError('duplicate phase')
            record['phases_ns'][f[1]] = int(f[2])
        elif tag == 'payload' and len(f) == 10:
            record['payload_bytes'] = dict(zip(PAYLOAD, map(int, f[1:])))
        elif tag == 'column' and len(f) == 21:
            if f[4] not in ('true', 'false') or f[5] not in ('true', 'false'):
                raise ValueError('invalid boolean')
            record['columns'].append(dict(index=int(f[1]), elapsed_ns=int(f[2]), prefix_ns=int(f[3]),
                accepted=f[4] == 'true', native_converged=f[5] == 'true', native_stop=f[6],
                iterations=int(f[7]), certificate=float(f[8]), native_residual=float(f[9]),
                native_secondary=float(f[10]), native_projection=None if f[11] == 'NA' else float(f[11]),
                coefficient_fnv1a64=f[12], work=dict(zip(WORK, map(int, f[13:])))))
        elif tag == 'gate' and len(f) == 7:
            index = int(f[1])
            if index != len(record['columns']) - 1 or index < 0 or 'gate' in record['columns'][index]:
                raise ValueError('misplaced or duplicate gate work')
            record['columns'][index]['gate'] = dict(zip(GATE_KEYS, map(int, f[2:])))
        elif tag == 'failed_gate' and len(f) == 6:
            record['failed_gate'] = dict(zip(GATE_KEYS, map(int, f[1:])))
        elif tag == 'failed_work' and len(f) == 10:
            record['failed_action_ns'] = int(f[1])
            record['failed_work'] = dict(zip(WORK, map(int, f[2:])))
        elif tag == 'total' and len(f) == 3:
            record['total_ns'], record['overhead_ns'] = map(int, f[1:])
        elif tag == 'status' and len(f) in (2, 4):
            if (f[1] == 'complete' and len(f) != 2) or (f[1] == 'error' and len(f) != 4):
                raise ValueError('invalid terminal status')
            record['status'] = f[1]
            if len(f) == 4:
                record['error_stage'], record['error'] = f[2:]
        else:
            raise ValueError(f'unknown/malformed TSV record: {tag}')
    if 'status' not in seen:
        raise ValueError('missing terminal status')
    return record


def resources(stderr, system):
    if system == 'Darwin':
        peak = re.search(r'^\s*(\d+)\s+maximum resident set size\s*$', stderr, re.M)
        cpu = re.search(r'([\d.]+)\s+real\s+([\d.]+)\s+user\s+([\d.]+)\s+sys', stderr)
        if not peak or not cpu:
            raise ValueError('missing Darwin time resources')
        return dict(peak_rss_bytes=int(peak[1]), user_seconds=float(cpu[2]), system_seconds=float(cpu[3]),
                    method='darwin_time_l', scope='isolated_process_including_startup_teardown')
    peak = re.search(r'Maximum resident set size \(kbytes\):\s*(\d+)', stderr)
    user = re.search(r'User time \(seconds\):\s*([\d.]+)', stderr)
    kernel = re.search(r'System time \(seconds\):\s*([\d.]+)', stderr)
    if not peak or not user or not kernel:
        raise ValueError('missing GNU time resources')
    return dict(peak_rss_bytes=int(peak[1]) * 1024, user_seconds=float(user[1]), system_seconds=float(kernel[1]),
                method='gnu_time_v', scope='isolated_process_including_startup_teardown')


def command(args, **kwargs):
    return subprocess.check_output(args, cwd=ROOT, **kwargs)


def provenance(binary, policy):
    if command(['git', 'status', '--porcelain']).strip():
        raise ValueError('benchmark requires a clean committed source tree')
    if os.environ.get('RUSTFLAGS') or os.environ.get('CARGO_ENCODED_RUSTFLAGS'):
        raise ValueError('v1 requires unset Rust compiler flag overrides')
    import tomllib
    override_names = [key for key in os.environ if key.startswith(('CARGO_PROFILE_', 'CARGO_BUILD_', 'CARGO_TARGET_'))
                      and key != 'CARGO_TARGET_DIR' and os.environ[key]]
    override_names += [key for key in ['RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'] if os.environ.get(key)]
    if override_names:
        raise ValueError(f'undeclared compiler overrides: {override_names}')
    config_hashes = {}
    config_dirs = [parent / '.cargo' for parent in [ROOT, *ROOT.parents]]
    config_dirs.append(Path(os.environ.get('CARGO_HOME', Path.home() / '.cargo')))
    for folder in dict.fromkeys(config_dirs):
        for filename in ('config', 'config.toml'):
            path = folder / filename
            if path.is_file():
                content = path.read_bytes()
                config = tomllib.loads(content.decode())
                if any(key in config for key in ('build', 'target', 'profile', 'env', 'unstable')):
                    raise ValueError(f'undeclared build configuration: {path}')
                config_hashes[str(path)] = sha(content)
    rust = command(['rustc', '--version', '--verbose']).decode()
    if not rust.startswith('rustc 1.85.0 '):
        raise ValueError('Rust 1.85.0 is required')
    commit = command(['git', 'rev-parse', 'HEAD']).decode().strip()
    hashes = {}
    for name in SOURCE_FILES:
        data = (ROOT / name).read_bytes()
        if data != command(['git', 'show', f'{commit}:{name}']):
            raise ValueError(f'uncommitted source {name}')
        hashes[name] = sha(data)
    system = platform.system()
    if system not in ('Darwin', 'Linux'):
        raise ValueError('resource collection supports Darwin/Linux; Windows has separate correctness CI')
    affinity = (dict(status='measured', cpus=sorted(os.sched_getaffinity(0)), scope='inherited_allowed_cpu_set')
                if hasattr(os, 'sched_getaffinity') else
                dict(status='unavailable', reason='macOS has no sched_getaffinity interface; placement is not pinned'))
    hardware = command(['sysctl', '-n', 'machdep.cpu.brand_string', 'hw.memsize', 'hw.ncpu']).decode() if system == 'Darwin' else command(['lscpu']).decode()
    memory = None if system == 'Darwin' else Path('/proc/meminfo').read_text()
    return dict(source_commit=commit, source_tree=command(['git', 'rev-parse', 'HEAD^{tree}']).decode().strip(),
                source_clean=True, source_hashes=hashes, binary_sha256=sha(binary.read_bytes()),
                rustc=rust, target=next(line[6:] for line in rust.splitlines() if line.startswith('host: ')),
                build_args=policy['build_args'], rustflags='', compiler_overrides={}, cargo_config_hashes=config_hashes, thread_env=THREAD_ENV,
                system=system, uname=platform.uname()._asdict(), hardware=hardware, memory=memory,
                affinity=affinity, scheduler={key: os.environ.get(key) for key in ['JOB_ID', 'NSLOTS', 'SLURM_JOB_ID']},
                python=sys.version)


def run_one(binary, data, policy, system, out, name, env):
    args = ['/usr/bin/time', '-l' if system == 'Darwin' else '-v', str(binary), name['route']]
    start = time.perf_counter_ns()
    proc = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, env=env, start_new_session=True)
    timed_out = False
    try:
        stdout, stderr = proc.communicate(data, timeout=policy['timeout_seconds'])
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(proc.pid, signal.SIGKILL)
        stdout, stderr = proc.communicate()
    elapsed = time.perf_counter_ns() - start
    key = f"{name['case_id']}-{name['route']}-{name['kind']}-{name['repeat']}"
    stdout_path, stderr_path = f'{key}.tsv', f'{key}.resources.txt'
    (out / stdout_path).write_bytes(stdout)
    (out / stderr_path).write_bytes(stderr)
    evidence = dict(**name, input_sha256=sha(data), process_wall_ns=elapsed, exit_code=proc.returncode,
                    raw_stdout=stdout_path, raw_stdout_sha256=sha(stdout),
                    raw_stderr=stderr_path, raw_stderr_sha256=sha(stderr))
    if timed_out:
        evidence.update(status='timeout', resource_status='unavailable',
                        resource_reason='timeout killed the process group before resource wrapper completion')
    else:
        evidence['status'] = 'returned'
        try:
            evidence['probe'] = parse_output(stdout.decode())
            json.dumps(evidence['probe'], allow_nan=False)  # Malformed nonfinite output remains a retained protocol error.
            evidence['resources'] = resources(stderr.decode(), system)
            evidence['resource_status'] = 'measured'
        except (ValueError, UnicodeError) as error:
            evidence.update(status='protocol_error', error=str(error))
    return evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--profile', choices=['smoke', 'development'], default='smoke')
    parser.add_argument('--policy', choices=['native', 'gated'], default='native')
    args = parser.parse_args()
    out = args.output.resolve()
    if out.exists():
        raise ValueError('use a fresh output directory; existing evidence is never overwritten')
    policy_path = POLICY if args.policy == 'native' else POLICY_GATED
    policy = json.loads((ROOT / policy_path).read_text())
    env = os.environ.copy()
    env.update(THREAD_ENV, LC_ALL='C', PYTHONDONTWRITEBYTECODE='1')
    # Build locally in this invocation, retaining its actual log and binary identity.
    if command(['git', 'status', '--porcelain']).strip():
        raise ValueError('commit the recipe and source before running evidence')
    build = subprocess.run(policy['build_args'], cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if build.returncode:
        sys.stderr.buffer.write(build.stdout)
        raise ValueError('release build failed')
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve()
    binary = target / 'release/examples/prepared_serial_benchmark'
    meta = provenance(binary, policy)
    out.mkdir(parents=True)
    (out / 'build.log').write_bytes(build.stdout)
    # Keep the exact executable for later engineering comparisons, outside source Git.
    (out / 'prepared_serial_benchmark').write_bytes(binary.read_bytes())
    (out / 'prepared_serial_benchmark').chmod(0o755)
    manifest = dict(schema=1, scope=policy['scope'], profile=args.profile, policy=policy, policy_path=policy_path,
                    policy_sha256=meta['source_hashes'][policy_path], provenance=meta,
                    build_log_sha256=sha(build.stdout), memory_scopes=MEMORY_SCOPES, runs=[])
    for case in cases(policy):
        data, dimensions = generate(policy, args.profile, case)
        # Warmups are separate processes. No per-process allocator or numerical state is reused.
        for kind, reps in [('warmup', policy['warmups']), ('measured', policy['repetitions'])]:
            for repeat in range(reps):
                order = policy['routes'][repeat % len(policy['routes']):] + policy['routes'][:repeat % len(policy['routes'])]
                for position, route in enumerate(order):
                    name = dict(case_id=case_id(case), case=case, dimensions=dimensions,
                                route=route, kind=kind, repeat=repeat, position=position)
                    evidence = run_one(binary, data, policy, meta['system'], out, name, env)
                    manifest['runs'].append(evidence)
                    dump(out / 'manifest.json', manifest)  # Preserve each attempted cell, including failures.
        print(f"recorded {case_id(case)}: {dimensions['tuples']} tuples", flush=True)
    from validate_prepared_serial import validate_directory
    summary = validate_directory(out)
    dump(out / 'summary.json', summary)
    print(json.dumps({k:v for k,v in summary.items() if k != 'timings'}, indent=2), flush=True)
    # Scientific rejection is retained in coverage, never silently rerun or counted as a win.
    return 0  # Valid negative evidence is preserved; coverage is a separate reported gate.


if __name__ == '__main__':
    sys.exit(main())
