#!/usr/bin/env python3
"""Explicit diagnostic profiling on bounded development inputs, never performance qualification."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

from prepared_serial import (ROOT, SOURCE_FILES, THREAD_ENV, MEMORY_SCOPES, cases, case_id,
    generate, command, provenance, parse_output, resources, sha, dump)

POLICY_PROFILE='benchmarks/policies/prepared-kernel-profile-v1.json'
PROFILE_FILES=list(dict.fromkeys(SOURCE_FILES+[POLICY_PROFILE,'scripts/prepared_kernel_profile.py',
    'scripts/validate_prepared_kernel_profile.py','crates/multiway-incidence/src/profiling.rs',
    'crates/multiway-incidence/Cargo.toml','crates/multiway-mg/Cargo.toml']))
PHASE_NAMES=['incidence','adjoint','weighted_incidence','weighted_adjoint','gramian','rhs',
    'projection','map_sweep','restriction','prolongation','dense_terminal','cycle',
    'pcg_recurrence','prepared_pcg','prepared_lsmr','certificate']
PROFILE_MEMORY=MEMORY_SCOPES | {
    'profiling_thread_local_state':'fixed current-thread state; inline bytes reported separately; included in process RSS',
    'profiling_inline_record':'whole probe Record and per-report inline bytes reported separately; compiler stack copies/metadata not separately observed',
}


def effective_policy(policy,profile):
    return policy | {'widths':policy['profile_widths'][profile]}


def parse_profile_output(text):
    base=[]
    result={'columns':[],'levels':[]}
    column_count=0
    for line in text.splitlines():
        fields=line.split('\t');tag=fields[0]
        if tag=='column':column_count+=1
        if tag=='profiling_metadata' and len(fields)==5:
            if 'metadata' in result:raise ValueError('duplicate profiling metadata')
            result['metadata']=dict(zip(['schema','thread_local_bytes','record_inline_bytes','report_inline_bytes'],map(int,fields[1:])))
        elif tag=='profile_level' and len(fields)==4:
            index,e,n=map(int,fields[1:])
            if index!=len(result['levels']):raise ValueError('misplaced/duplicate profile level')
            result['levels'].append(dict(index=index,tuples=e,coefficients=n))
        elif tag in ('profile','failed_profile') and len(fields)==5:
            index=int(fields[1])
            if fields[2] not in ('true','false'):raise ValueError('invalid profile validity flag')
            record=dict(index=index,valid=fields[2]=='true',elapsed_ns=int(fields[3]),maximum_depth=int(fields[4]),phases={})
            if tag=='profile':
                if index!=len(result['columns']) or index!=column_count-1 or 'failed' in result:raise ValueError('misplaced/duplicate profile')
                result['columns'].append(record)
            else:
                if index!=column_count or 'failed' in result:raise ValueError('misplaced/duplicate failed profile')
                result['failed']=record
        elif tag in ('profile_phase','failed_profile_phase') and len(fields)==6:
            index=int(fields[1]);phase=fields[2]
            record=result.get('failed') if tag=='failed_profile_phase' else (result['columns'][-1] if result['columns'] else None)
            if record is None or index!=record['index'] or phase not in PHASE_NAMES or phase in record['phases']:raise ValueError('misplaced/duplicate profile phase')
            record['phases'][phase]=dict(zip(['calls','inclusive_ns','exclusive_ns'],map(int,fields[3:])))
        elif tag.startswith(('profile','profiling','failed_profile')):
            raise ValueError('malformed profile record')
        else:base.append(line)
    if 'metadata' not in result:raise ValueError('profiling feature metadata missing')
    return parse_output('\n'.join(base)),result


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
            evidence['probe'], evidence['profiling'] = parse_profile_output(stdout.decode())
            json.dumps(evidence['probe'], allow_nan=False)  # Malformed nonfinite output remains a retained protocol error.
            evidence['resources'] = resources(stderr.decode(), system)
            evidence['resource_status'] = 'measured'
        except (ValueError, UnicodeError) as error:
            evidence.update(status='protocol_error', error=str(error))
    return evidence



def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path)
    parser.add_argument('--profile',choices=['smoke','development','expanded'],default='smoke')
    args=parser.parse_args();out=args.output.resolve()
    if out.exists():raise ValueError('use a fresh directory; no overwrite or retry')
    if command(['git','status','--porcelain']).strip():raise ValueError('commit profiling source/recipe first')
    policy=effective_policy(json.loads((ROOT/POLICY_PROFILE).read_text()),args.profile)
    env=os.environ.copy();env.update(THREAD_ENV,LC_ALL='C',PYTHONDONTWRITEBYTECODE='1')
    build=subprocess.run(policy['build_args'],cwd=ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    if build.returncode:
        sys.stderr.buffer.write(build.stdout);raise ValueError('profiling build failed')
    binary=Path(os.environ.get('CARGO_TARGET_DIR',ROOT/'target')).resolve()/'release/examples/prepared_serial_benchmark'
    meta=provenance(binary,policy)
    for name in PROFILE_FILES:
        data=(ROOT/name).read_bytes()
        if data!=command(['git','show',f"{meta['source_commit']}:{name}"]):raise ValueError('uncommitted profiling source')
        meta['source_hashes'][name]=sha(data)
    out.mkdir(parents=True);(out/'build.log').write_bytes(build.stdout)
    (out/'prepared_serial_benchmark').write_bytes(binary.read_bytes());(out/'prepared_serial_benchmark').chmod(0o755)
    manifest=dict(schema=1,scope=policy['scope'],profile=args.profile,policy=policy,policy_path=POLICY_PROFILE,
        policy_sha256=meta['source_hashes'][POLICY_PROFILE],provenance=meta,build_log_sha256=sha(build.stdout),
        memory_scopes=PROFILE_MEMORY,runs=[])
    for case in cases(policy):
        data,dimensions=generate(policy,args.profile,case)
        for kind,repeats in [('warmup',policy['warmups']),('measured',policy['repetitions'])]:
            for repeat in range(repeats):
                routes=policy['routes'][repeat%len(policy['routes']):]+policy['routes'][:repeat%len(policy['routes'])]
                for position,route in enumerate(routes):
                    name=dict(case_id=case_id(case),case=case,dimensions=dimensions,route=route,kind=kind,repeat=repeat,position=position)
                    manifest['runs'].append(run_one(out/'prepared_serial_benchmark',data,policy,meta['system'],out,name,env))
                    dump(out/'manifest.json',manifest)
        print('profiled '+case_id(case),flush=True)
    from validate_prepared_kernel_profile import validate_profile_directory
    summary=validate_profile_directory(out);dump(out/'summary.json',summary)
    print(json.dumps({k:v for k,v in summary.items() if k not in ['timings','kernel_profiles']},indent=2),flush=True)


if __name__=='__main__':main()
