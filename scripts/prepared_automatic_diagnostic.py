#!/usr/bin/env python3
"""Collect committed reference/profile diagnostics; never calculate speedups."""
import argparse,json,os,shutil,subprocess,time
from pathlib import Path
import prepared_serial as base
from prepared_automatic import FILES as AUTOMATIC_FILES
from automatic_recipes import cases,case_id,generate
from automatic_diagnostic_runner import run_one
POLICY='benchmarks/policies/prepared-automatic-diagnostic-v1.json'
FILES=list(dict.fromkeys(AUTOMATIC_FILES+[
 POLICY,'scripts/prepared_automatic_diagnostic.py','scripts/validate_prepared_automatic_diagnostic.py',
 'scripts/automatic_diagnostic_protocol.py','scripts/automatic_diagnostic_runner.py',
 'scripts/check_prepared_automatic_diagnostic.py','scripts/test_prepared_automatic_diagnostic.py',
 'crates/multiway-mg/src/automatic_profiling.rs','crates/multiway-mg/src/prepared_automatic.rs',
 'crates/multiway-mg/src/prepared_automatic/hierarchy.rs','crates/multiway-mg/src/prepared_automatic/layout.rs',
 'crates/multiway-mg/examples/prepared_automatic_diagnostic.rs','docs/ISSUE5_AUTOMATIC_DIAGNOSTICS.md']))
SCOPES=dict(timing='instrumented driver regions plus unattributed callback time; no comparative performance inference',
 memory='requested/admitted actual array scopes and full process RSS; automatic and kernel profiler TLS capacities and inline record sizes separately reported; no event history',
 reference='separate uninstrumented executable with exact numerical/work/layout/payload comparison',
 failure='every attempted process is journaled; failed/incomplete pairs produce no diagnostic shares',
 phases='flat nonoverlapping current-thread regions; kernel profiler is a separate inventory; no worker-thread attribution')

def schedule(policy):
    for ci,case in enumerate(cases(policy)):
        arms=policy['arms'];shift=ci%len(arms)
        for position,arm in enumerate(arms[shift:]+arms[:shift]):
            builds=['reference','instrumented'] if (ci+position)%2==0 else ['instrumented','reference']
            for build in builds:
                yield dict(case=case,kind='diagnostic',repeat=0,position=position,build=build,**arm)

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('output',type=Path);p.add_argument('--profile',choices=['smoke','development'],default='smoke');args=p.parse_args()
    out=args.output.resolve()
    if out.exists() or out.is_relative_to(base.ROOT):raise ValueError('fresh external output directory required')
    if base.command(['git','status','--porcelain']).strip():raise ValueError('commit all source and policy before collection')
    policy=json.loads((base.ROOT/POLICY).read_text());env=os.environ.copy();env.update(base.THREAD_ENV,LC_ALL='C',PYTHONDONTWRITEBYTECODE='1')
    out.mkdir(parents=True);metadata={};binaries={};builds={}
    for build,command in policy['builds'].items():
        process=subprocess.run(command,cwd=base.ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        log=build+'-build.log';(out/log).write_bytes(process.stdout)
        if process.returncode:raise RuntimeError('diagnostic build failed; log preserved: '+log)
        binary=Path(os.environ.get('CARGO_TARGET_DIR',base.ROOT/'target')).resolve()/'release/examples/prepared_automatic_diagnostic'
        meta=base.provenance(binary,dict(build_args=command))
        for name in FILES:
            value=(base.ROOT/name).read_bytes()
            if value!=base.command(['git','show',f"{meta['source_commit']}:{name}"]):raise ValueError('uncommitted diagnostic source')
            meta['source_hashes'][name]=base.sha(value)
        meta['sdk_environment']={k:os.environ.get(k) for k in ['DEVELOPER_DIR','SDKROOT','MACOSX_DEPLOYMENT_TARGET']}
        name=build+'-probe';shutil.copy2(binary,out/name);(out/name).chmod(0o755)
        metadata[build]=meta;binaries[build]=name;builds[build]=dict(path=log,sha256=base.sha(process.stdout))
    manifest=dict(schema=1,status='collecting',scope=policy['scope'],profile=args.profile,policy=policy,policy_path=POLICY,
        policy_sha256=base.sha((base.ROOT/POLICY).read_bytes()),provenance=metadata,binaries=binaries,build_logs=builds,
        scopes=SCOPES,runs_sha256=None,collection_ns=None)
    base.dump(out/'manifest.json',manifest);origin=time.perf_counter_ns();last=None
    with (out/'runs.jsonl').open('x') as journal:
        for index,spec in enumerate(schedule(policy)):
            cid=case_id(spec['case'])
            if cid!=last:
                if last is not None:print('recorded '+last,flush=True)
                data,dims=generate(policy,args.profile,spec['case']);last=cid
            row=run_one(out/binaries[spec['build']],data,policy,metadata[spec['build']]['system'],out,index,spec,origin,env)
            row['dimensions']=dims;journal.write(json.dumps(row,sort_keys=True,allow_nan=False)+'\n');journal.flush()
        print('recorded '+last,flush=True)
    manifest.update(status='complete',runs_sha256=base.sha((out/'runs.jsonl').read_bytes()),collection_ns=time.perf_counter_ns()-origin)
    base.dump(out/'manifest.json',manifest)
    from validate_prepared_automatic_diagnostic import validate_directory
    summary=validate_directory(out);base.dump(out/'summary.json',summary)
    print(json.dumps({k:v for k,v in summary.items() if k!='cells'},indent=2,allow_nan=False))
if __name__=='__main__':main()
