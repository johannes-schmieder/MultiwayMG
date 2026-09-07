#!/usr/bin/env python3
"""Frozen paired explicit-layout collection; no profiling or automatic selector."""
import argparse
import copy
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import prepared_serial as base

ROOT = base.ROOT
POLICY_V1 = 'benchmarks/policies/prepared-layout-v1.json'
POLICY = 'benchmarks/policies/prepared-layout-v2.json'
def source_files(policy_path):
    return base.SOURCE_FILES + [policy_path, 'scripts/prepared_layout.py',
    'scripts/validate_prepared_layout.py',
    'crates/multiway-mg/examples/support/prepared_layout.rs']
FILES = source_files(POLICY)

def policy_path_for(policy):
    paths={1:POLICY_V1,2:POLICY}
    if type(policy['revision']) is not int or policy['revision'] not in paths:
        raise ValueError('unknown layout policy revision')
    return paths[policy['revision']]
PHASES = base.PHASES[:4] + ['grouping'] + base.PHASES[4:]
PAYLOAD = base.PAYLOAD[:-1] + ['grouping', 'total']
MEMORY_SCOPES_V1 = dict(base.MEMORY_SCOPES,
    grouping='separate structural owner; requested setup peak includes all live input/fine-frame/structural arrays and one current cursor',
    tuple_image='one maximum-E vector and descriptor per explicit image workspace; no image for scalar/rows',
    benchmark_record='fixed inline record size is reported; stack copies, alignment and allocator metadata remain unmeasured')
MEMORY_SCOPES = dict(MEMORY_SCOPES_V1,tuple_image='one maximum-E slice inside the shared result/traversal/image arena; no separate descriptor or allocation')

def memory_scopes(policy):
    return MEMORY_SCOPES_V1 if policy_path_for(policy)==POLICY_V1 else MEMORY_SCOPES

def effective_policy(policy, profile):
    result = copy.deepcopy(policy)
    result['widths'] = policy['profiles'][profile].get('widths', policy['widths'])
    return result

def schedule(policy):
    indices = {layout: 0 for layout in policy['layouts']}
    for ci, case in enumerate(base.cases(policy)):
        for kind, repetitions in [('warmup', policy['warmups']), ('measured', policy['repetitions'])]:
            for repeat in range(repetitions):
                routes = policy['routes'][repeat % len(policy['routes']):] + policy['routes'][:repeat % len(policy['routes'])]
                for position, route in enumerate(routes):
                    shift = (ci + repeat + policy['routes'].index(route)) % len(policy['layouts'])
                    layouts = policy['layouts'][shift:] + policy['layouts'][:shift]
                    for layout in layouts:
                        yield layout, indices[layout], case, kind, repeat, position, route
                        indices[layout] += 1

def parse_layout_output(text):
    kept, levels, info, payload, nanos = [], [], None, None, None
    for line in text.splitlines():
        fields = line.split('\t')
        if fields[0] == 'layout':
            if len(fields) != 7 or info is not None:
                raise ValueError('malformed/duplicate layout')
            info = dict(name=fields[1], **dict(zip(['prefix','usize_bytes','group_descriptor_bytes',
                'image_descriptor_bytes','record_bytes'], map(int,fields[2:]))))
        elif fields[0] == 'layout_payload':
            if len(fields) != 4 or payload is not None:
                raise ValueError('malformed/duplicate layout payload')
            payload = dict(zip(['retained','setup_bound','image_len'],map(int,fields[1:])))
        elif fields[0] == 'layout_level':
            if len(fields) != 5 or int(fields[1]) != len(levels):
                raise ValueError('malformed/missing/duplicate layout level')
            levels.append(dict(zip(['tuples','coefficients','index_bytes'],map(int,fields[2:]))))
        elif fields[:2] == ['phase','grouping']:
            if len(fields) != 3 or nanos is not None:
                raise ValueError('malformed/duplicate grouping phase')
            nanos = int(fields[2])
        else:
            kept.append(line)
    if info is None or payload is None or nanos is None:
        raise ValueError('missing layout boundary')
    result = base.parse_output('\n'.join(kept))
    result['layout'] = info | payload
    result['layout_levels'] = levels
    result['phases_ns']['grouping'] = nanos
    if 'payload_bytes' in result:
        result['payload_bytes']['grouping'] = payload['retained']
    return result

def run_one(binary, data, policy, system, out, name, env, layout):
    # Same isolated-process/time/kill boundary as v2, with an explicit layout
    # argument and parser. Keep the immutable v2 generator/harness source intact.
    args = ['/usr/bin/time', '-l' if system == 'Darwin' else '-v', str(binary), name['route'], layout]
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
    a,b = f'{key}.tsv',f'{key}.resources.txt'
    (out/a).write_bytes(stdout); (out/b).write_bytes(stderr)
    evidence = dict(**name, input_sha256=base.sha(data), process_wall_ns=elapsed, exit_code=proc.returncode,
        raw_stdout=a, raw_stdout_sha256=base.sha(stdout), raw_stderr=b, raw_stderr_sha256=base.sha(stderr))
    if timed_out:
        evidence.update(status='timeout', resource_status='unavailable',
            resource_reason='timeout killed the process group before resource wrapper completion')
    else:
        evidence['status'] = 'returned'
        try:
            evidence['probe'] = parse_layout_output(stdout.decode())
            json.dumps(evidence['probe'],allow_nan=False)
            evidence['resources'] = base.resources(stderr.decode(),system)
            evidence['resource_status'] = 'measured'
        except (ValueError,UnicodeError) as error:
            evidence.update(status='protocol_error',error=str(error))
    return evidence

def main():
    from validate_prepared_layout import validate_directory
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path)
    parser.add_argument('--profile',choices=['smoke','development','expanded'],default='smoke')
    args=parser.parse_args(); out=args.output.resolve()
    if out.exists() or base.command(['git','status','--porcelain']).strip():
        raise ValueError('commit source/policy first and use a fresh output directory; no retries')
    frozen=json.loads((ROOT/POLICY).read_text()); policy=effective_policy(frozen,args.profile)
    env=os.environ.copy(); env.update(base.THREAD_ENV,LC_ALL='C',PYTHONDONTWRITEBYTECODE='1')
    build=subprocess.run(policy['build_args'],cwd=ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    if build.returncode:
        raise ValueError('release build failed: '+build.stdout.decode(errors='replace'))
    binary=Path(os.environ.get('CARGO_TARGET_DIR',ROOT/'target')).resolve()/'release/examples/prepared_serial_benchmark'
    meta=base.provenance(binary,policy)
    meta['source_hashes']={name:base.sha((ROOT/name).read_bytes()) for name in FILES}
    out.mkdir(parents=True)
    (out/'prepared_serial_benchmark').write_bytes(binary.read_bytes()); (out/'prepared_serial_benchmark').chmod(0o755)
    (out/'build.log').write_bytes(build.stdout)
    children={}
    for layout in policy['layouts']:
        (out/layout).mkdir()
        children[layout]=dict(schema=1,scope=policy['scope'],profile=args.profile,policy=policy,
            layout=layout,policy_path=POLICY,policy_sha256=meta['source_hashes'][POLICY],
            provenance=meta,build_log_sha256=base.sha(build.stdout),memory_scopes=MEMORY_SCOPES,runs=[])
    manifest=dict(schema=1,scope='prepared_layout_pairing',policy=frozen,profile=args.profile,
        source_commit=meta['source_commit'],source_hashes=meta['source_hashes'],trace=[],total_ns=0)
    started=time.perf_counter_ns(); previous=None
    for layout,index,case,kind,repeat,position,route in schedule(policy):
        if case != previous:
            if previous is not None: print('paired '+base.case_id(previous),flush=True)
            data,dimensions=base.generate(policy,args.profile,case); previous=case
        name=dict(case_id=base.case_id(case),case=case,dimensions=dimensions,route=route,kind=kind,repeat=repeat,position=position)
        begin=time.perf_counter_ns()-started
        run=run_one(out/'prepared_serial_benchmark',data,policy,meta['system'],out/layout,name,env,layout)
        end=time.perf_counter_ns()-started
        children[layout]['runs'].append(run)
        manifest['trace'].append(dict(layout=layout,index=index,start_ns=begin,end_ns=end))
        manifest['total_ns']=end
        base.dump(out/layout/'manifest.json',children[layout]); base.dump(out/'manifest.json',manifest)
    summary=validate_directory(out)
    base.dump(out/'summary.json',summary)
    print(json.dumps({k:v for k,v in summary.items() if k != 'layouts'},indent=2,allow_nan=False))
    return int(not summary['complete_layout_gate_passed'])

if __name__ == '__main__':
    raise SystemExit(main())
