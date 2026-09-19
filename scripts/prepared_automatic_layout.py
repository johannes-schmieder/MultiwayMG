#!/usr/bin/env python3
"""Collect frozen complete-cost layouts using only the uninstrumented schema2 probe."""
import argparse
import json
import os
from pathlib import Path
import sys
import subprocess
import time
import prepared_serial as base
from automatic_layout_protocol import SCOPES,schedule,validate_policy
from automatic_diagnostic_runner import run_one
from automatic_recipes import generate,case_id
from prepared_automatic_diagnostic import FILES as DIAGNOSTIC_FILES
POLICY='benchmarks/policies/prepared-automatic-layout-v1.json'
FILES=list(dict.fromkeys(DIAGNOSTIC_FILES+[POLICY,'scripts/automatic_layout_protocol.py',
 'scripts/prepared_automatic_layout.py','scripts/validate_prepared_automatic_layout.py',
 'scripts/test_prepared_automatic_layout.py','docs/ISSUE5_AUTOMATIC_LAYOUT_ECONOMICS.md']))

def main():
    if sys.version_info<(3,11):raise ValueError('Python 3.11 or newer is required for build provenance')
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('output',type=Path)
    parser.add_argument('--profile',choices=['smoke','development','expanded'],default='smoke');args=parser.parse_args()
    out=args.output.resolve()
    if out.exists() or out.resolve().is_relative_to(base.ROOT):raise ValueError('fresh external output directory required; never overwrite evidence')
    if base.command(['git','status','--porcelain']).strip():raise ValueError('commit source and recipe before measuring')
    policy=json.loads((base.ROOT/POLICY).read_text());validate_policy(policy,args.profile);env=os.environ.copy();env.update(base.THREAD_ENV,LC_ALL='C',PYTHONDONTWRITEBYTECODE='1')
    out.mkdir(parents=True)
    build=subprocess.run(policy['build_args'],cwd=base.ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    (out/'build.log').write_bytes(build.stdout)
    if build.returncode:raise RuntimeError('build failed; complete log preserved')
    binary=Path(os.environ.get('CARGO_TARGET_DIR',base.ROOT/'target')).resolve()/'release/examples/prepared_automatic_diagnostic'
    meta=base.provenance(binary,policy)
    for name in FILES:
        value=(base.ROOT/name).read_bytes()
        if value!=base.command(['git','show',f"{meta['source_commit']}:{name}"]):raise ValueError('uncommitted benchmark source')
        meta['source_hashes'][name]=base.sha(value)
    meta['sdk_environment']={key:os.environ.get(key) for key in ['DEVELOPER_DIR','SDKROOT','MACOSX_DEPLOYMENT_TARGET']}
    (out/'uninstrumented-probe').write_bytes(binary.read_bytes());(out/'uninstrumented-probe').chmod(0o755)
    manifest=dict(schema=1,status='collecting',scope=policy['scope'],profile=args.profile,policy=policy,
        policy_path=POLICY,policy_sha256=meta['source_hashes'][POLICY],provenance=meta,scopes=SCOPES,
        build_log_sha256=base.sha(build.stdout),runs_sha256=None,collection_ns=None)
    base.dump(out/'manifest.json',manifest)
    origin=time.perf_counter_ns();last=None
    with (out/'runs.jsonl').open('x') as journal:
        for index,spec in enumerate(schedule(policy,args.profile)):
            cid=case_id(spec['case'])
            if cid!=last:
                if last is not None:print('recorded '+last,flush=True)
                data,dims=generate(policy,args.profile,spec['case']);last=cid
            row=run_one(out/'uninstrumented-probe',data,policy,meta['system'],out,index,spec,origin,env)
            row['dimensions']=dims
            journal.write(json.dumps(row,sort_keys=True,allow_nan=False)+'\n');journal.flush()
        print('recorded '+last,flush=True)
    manifest.update(status='complete',collection_ns=time.perf_counter_ns()-origin,runs_sha256=base.sha((out/'runs.jsonl').read_bytes()))
    base.dump(out/'manifest.json',manifest)
    from validate_prepared_automatic_layout import validate_directory
    result=validate_directory(out);base.dump(out/'summary.json',result)
    print(json.dumps({key:value for key,value in result.items() if key!='cells'},indent=2,allow_nan=False))

if __name__=='__main__':main()
