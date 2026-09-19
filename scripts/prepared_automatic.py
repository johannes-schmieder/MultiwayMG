#!/usr/bin/env python3
"""Collect committed, uninstrumented complete automatic/control development costs."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import prepared_serial as base
from automatic_protocol import MEMORY_SCOPES,parse_output,schedule
from automatic_recipes import generate,case_id

POLICY='benchmarks/policies/prepared-automatic-v1.json'
FILES=list(dict.fromkeys(base.SOURCE_FILES+[POLICY,'scripts/prepared_automatic.py',
    'scripts/automatic_protocol.py','scripts/automatic_recipes.py','scripts/validate_prepared_automatic.py',
    'scripts/check_prepared_automatic_probe.py','scripts/test_prepared_automatic.py','crates/multiway-mg/Cargo.toml',
    'crates/multiway-mg/examples/prepared_automatic_benchmark.rs']))

def run_one(binary,data,policy,system,out,index,spec,origin,env):
    command=['/usr/bin/time','-l' if system=='Darwin' else '-v',str(binary),spec['route']]
    started=time.perf_counter_ns();error=None
    exit_code=None;failure_status=None
    try:
        child=subprocess.Popen(command,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
            env=env,start_new_session=True)
    except OSError as exc:
        stdout=b'';stderr=str(exc).encode();error=str(exc);failure_status='launch_error'
    else:
        try:stdout,stderr=child.communicate(data,timeout=policy['timeout_seconds'])
        except subprocess.TimeoutExpired:
            try:os.killpg(child.pid,signal.SIGKILL)
            except ProcessLookupError:pass
            stdout,stderr=child.communicate();error='declared isolated process timeout';failure_status='timeout'
        exit_code=child.returncode
    ended=time.perf_counter_ns()
    row=dict(spec,index=index,case_id=case_id(spec['case']),input_sha256=base.sha(data),
        start_ns=started-origin,end_ns=ended-origin,process_wall_ns=ended-started,exit_code=exit_code,
        stdout_path=f'{index:06d}.stdout',stderr_path=f'{index:06d}.stderr',
        stdout_sha256=base.sha(stdout),stderr_sha256=base.sha(stderr))
    (out/row['stdout_path']).write_bytes(stdout);(out/row['stderr_path']).write_bytes(stderr)
    if error:row.update(status=failure_status,error=error,resource_status='unavailable_after_timeout' if failure_status=='timeout' else 'unavailable_after_launch_error')
    else:
        try:
            probe=parse_output(stdout.decode());resource=base.resources(stderr.decode(),system)
            json.dumps([probe,resource],allow_nan=False)
            row.update(status='returned',probe=probe,resources=resource,resource_status='measured')
        except (ValueError,UnicodeError) as exc:
            row.update(status='protocol_error',error=str(exc),resource_status='unavailable_or_invalid_protocol')
    return row

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('output',type=Path)
    parser.add_argument('--profile',choices=['smoke','development'],default='smoke');args=parser.parse_args()
    out=args.output.resolve()
    if out.exists():raise ValueError('fresh output directory required; never overwrite evidence')
    if base.command(['git','status','--porcelain']).strip():raise ValueError('commit source and recipe before measuring')
    policy=json.loads((base.ROOT/POLICY).read_text());env=os.environ.copy();env.update(base.THREAD_ENV,LC_ALL='C',PYTHONDONTWRITEBYTECODE='1')
    build=subprocess.run(policy['build_args'],cwd=base.ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    if build.returncode:raise RuntimeError(build.stdout.decode())
    binary=Path(os.environ.get('CARGO_TARGET_DIR',base.ROOT/'target')).resolve()/'release/examples/prepared_automatic_benchmark'
    meta=base.provenance(binary,policy)
    for name in FILES:
        value=(base.ROOT/name).read_bytes()
        if value!=base.command(['git','show',f"{meta['source_commit']}:{name}"]):raise ValueError('uncommitted benchmark source')
        meta['source_hashes'][name]=base.sha(value)
    meta['sdk_environment']={key:os.environ.get(key) for key in ['DEVELOPER_DIR','SDKROOT','MACOSX_DEPLOYMENT_TARGET']}
    out.mkdir(parents=True);(out/'build.log').write_bytes(build.stdout)
    (out/'prepared_automatic_benchmark').write_bytes(binary.read_bytes());(out/'prepared_automatic_benchmark').chmod(0o755)
    manifest=dict(schema=1,status='collecting',scope=policy['scope'],profile=args.profile,policy=policy,
        policy_path=POLICY,policy_sha256=meta['source_hashes'][POLICY],provenance=meta,memory_scopes=MEMORY_SCOPES,
        build_log_sha256=base.sha(build.stdout),runs_sha256=None,collection_ns=None)
    base.dump(out/'manifest.json',manifest)
    origin=time.perf_counter_ns();last=None
    with (out/'runs.jsonl').open('x') as journal:
        for index,spec in enumerate(schedule(policy)):
            cid=case_id(spec['case'])
            if cid!=last:
                if last is not None:print('recorded '+last,flush=True)
                data,dims=generate(policy,args.profile,spec['case']);last=cid
            row=run_one(binary,data,policy,meta['system'],out,index,spec,origin,env)
            row['dimensions']=dims
            journal.write(json.dumps(row,sort_keys=True,allow_nan=False)+'\n');journal.flush()
        print('recorded '+last,flush=True)
    manifest.update(status='complete',collection_ns=time.perf_counter_ns()-origin,runs_sha256=base.sha((out/'runs.jsonl').read_bytes()))
    base.dump(out/'manifest.json',manifest)
    from validate_prepared_automatic import validate_directory
    result=validate_directory(out);base.dump(out/'summary.json',result)
    print(json.dumps({key:value for key,value in result.items() if key!='cells'},indent=2,allow_nan=False))

if __name__=='__main__':main()
