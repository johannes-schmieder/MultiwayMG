#!/usr/bin/env python3
"""Frozen old/stream dense application experiment; never complete-solver economics."""
import argparse,json,math,os,re,shutil,signal,subprocess,time
from pathlib import Path
import prepared_serial as base
from validate_prepared_serial import require,integer,hash_string,no_nonfinite
POLICY='benchmarks/policies/dense-reconstruction-v1.json'
FILES=list(dict.fromkeys(base.SOURCE_FILES+[POLICY,'scripts/dense_reconstruction.py',
 'scripts/test_dense_reconstruction.py','crates/multiway-mg/src/dense.rs',
 'crates/multiway-mg/src/dense/traversal_tests.rs','crates/multiway-mg/tests/operator_workspaces.rs',
 'crates/multiway-mg/tests/support/pre_workspace_dense.rs','docs/ISSUE5_DENSE_RECONSTRUCTION.md']))

def validate_stdout(text,policy):
    require(policy['schema']==1 and policy['arms']==['old','stream'] and policy['warmups']==1 and policy['repetitions']==5 and policy['workers']==1,'fixed microbenchmark policy changed')
    require(policy['dimensions']==[3,6,9,16,31,32,33,63,64,65,127,128,129,255,256,257] and policy['operation_budget']==1<<24,'fixed application work changed')
    rows=[]
    for line in text.splitlines():
        if not line.startswith('dense_reconstruction'):continue
        f=line.split('\t');require(len(f)==10 and f[:2]==['dense_reconstruction','1'],'malformed dense record')
        require(all(re.fullmatch('[0-9]+',f[i]) for i in [2,4,6,7]),'noninteger dense record')
        require(all(re.fullmatch('[0-9a-f]{16}',f[i]) for i in [8,9]),'malformed fingerprints')
        rows.append(dict(dimension=int(f[2]),arm=f[3],repeat=int(f[4]),kind=f[5],iterations=int(f[6]),elapsed_ns=int(f[7]),output=f[8],modal=f[9]))
    expected=[]
    for case,n in enumerate(policy['dimensions']):
        for round in range(policy['warmups']+policy['repetitions']):
            for position in range(2):
                expected.append(dict(dimension=n,arm=policy['arms'][(case+round+position)%2],repeat=max(0,round-1),kind='warmup' if round==0 else 'measure',iterations=policy['operation_budget']//(2*n*n)))
    require(len(rows)==len(expected),'missing/extra application samples')
    signatures={};pairs={}
    for row,spec in zip(rows,expected):
        require(all(row[k]==v for k,v in spec.items()),'wrong application schedule')
        integer(row['elapsed_ns'],'application time',1)
        n=row['dimension'];sig=(row['output'],row['modal'])
        require(signatures.setdefault(n,sig)==sig,'application changed exact math')
        if row['kind']=='measure':pairs.setdefault((n,row['repeat']),{})[row['arm']]=row['elapsed_ns']
    cells=[]
    for n in policy['dimensions']:
        ratios=[pairs[n,i]['old']/pairs[n,i]['stream'] for i in range(policy['repetitions'])]
        cells.append(dict(dimension=n,paired_ratios=ratios,paired_geomean=math.exp(sum(map(math.log,ratios))/len(ratios))))
    return dict(scope=policy['scope'],samples=len(rows),measured_samples=sum(r['kind']=='measure' for r in rows),
        cells=cells,balanced_geomean=math.exp(sum(math.log(c['paired_geomean']) for c in cells)/len(cells)),
        cost_scope=policy['cost_scope'],competitive_qualification=False,default_selected=False,records=rows)

def validate_directory(path):
    m=json.loads((path/'manifest.json').read_text());no_nonfinite(m)
    require(m['status']=='complete' and m['exit_code']==0,'failed application process')
    meta=m['provenance'];commit=meta['source_commit'];hash_string(commit,40)
    read=lambda name:subprocess.check_output(['git','show',f'{commit}:{name}'],cwd=base.ROOT)
    policy=json.loads(read(POLICY));require(m['policy']==policy,'microbenchmark policy drift')
    require(meta['source_tree']==base.command(['git','rev-parse',commit+'^{tree}']).decode().strip(),'wrong source tree')
    require(meta['system'] in ['Darwin','Linux'] and meta['hardware'] and meta['target'] and meta['affinity'],'missing platform provenance')
    require(meta['source_clean'] is True and meta['rustc'].startswith('rustc 1.85.0 ') and meta['build_args']==policy['build_args'],'wrong source/build')
    require(meta['rustflags']=='' and meta['compiler_overrides']=={} and meta['thread_env']==base.THREAD_ENV,'undeclared compiler/worker policy')
    require(set(meta['source_hashes'])==set(FILES),'missing source inventory')
    for name,h in meta['source_hashes'].items():require(base.sha(read(name))==h,'source checksum: '+name)
    require(base.sha((path/'probe').read_bytes())==meta['binary_sha256'],'binary checksum')
    for name in ['build.log','stdout.txt','stderr.txt']:require(base.sha((path/name).read_bytes())==m['raw_sha256'][name],'raw checksum: '+name)
    resources=base.resources((path/'stderr.txt').read_text(),meta['system']);require(resources==m['resources'],'resource mismatch')
    require(0<resources['peak_rss_bytes']<=policy['process_budget_bytes'],'RSS budget')
    result=validate_stdout((path/'stdout.txt').read_text(),policy)
    integer(m['process_wall_ns'],'harness process time',1)
    require(sum(r['elapsed_ns'] for r in result['records'])<=m['process_wall_ns'],'unaccounted process time')
    result.update(process_wall_ns=m['process_wall_ns'],resources=resources)
    return result

def collect(out):
    require(not out.exists() and not out.is_relative_to(base.ROOT),'fresh external output directory required')
    require(not base.command(['git','status','--porcelain']).strip(),'commit source before measurement')
    policy=json.loads((base.ROOT/POLICY).read_text());env=os.environ.copy();env.update(base.THREAD_ENV,LC_ALL='C',PYTHONDONTWRITEBYTECODE='1')
    out.mkdir(parents=True)
    build=subprocess.run(policy['build_args'],cwd=base.ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    (out/'build.log').write_bytes(build.stdout);require(build.returncode==0,'build failed; log preserved')
    candidates=[]
    for line in build.stdout.splitlines():
        try:r=json.loads(line)
        except (ValueError,UnicodeDecodeError):continue
        if r.get('reason')=='compiler-artifact' and r.get('target',{}).get('name')=='multiway_mg' and r.get('profile',{}).get('test') and r.get('executable'):candidates.append(r['executable'])
    require(len(candidates)==1,'missing/ambiguous test executable');binary=Path(candidates[0])
    meta=base.provenance(binary,policy)
    for name in FILES:
        data=(base.ROOT/name).read_bytes();require(data==base.command(['git','show',f"{meta['source_commit']}:{name}"]),'uncommitted source')
        meta['source_hashes'][name]=base.sha(data)
    meta['sdk_environment']={k:os.environ.get(k) for k in ['DEVELOPER_DIR','SDKROOT','MACOSX_DEPLOYMENT_TARGET']}
    shutil.copy2(binary,out/'probe');(out/'probe').chmod(0o755)
    m=dict(status='running',policy=policy,provenance=meta)
    base.dump(out/'manifest.json',m);start=time.perf_counter_ns()
    try:
        proc=subprocess.Popen(['/usr/bin/time','-l' if meta['system']=='Darwin' else '-v',str(out/'probe'),*policy['test_args']],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
        try:stdout,stderr=proc.communicate(timeout=policy['timeout_seconds']);status='complete'
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid,signal.SIGKILL);stdout,stderr=proc.communicate();status='timeout'
        code=proc.returncode
    except OSError as error:stdout=b'';stderr=str(error).encode();status='launch_error';code=None
    m.update(status=status,exit_code=code,process_wall_ns=time.perf_counter_ns()-start)
    (out/'stdout.txt').write_bytes(stdout);(out/'stderr.txt').write_bytes(stderr)
    m['raw_sha256']={n:base.sha((out/n).read_bytes()) for n in ['build.log','stdout.txt','stderr.txt']}
    m['resources']=None
    if status=='complete':
        try:m['resources']=base.resources(stderr.decode(),meta['system'])
        except (ValueError,UnicodeDecodeError) as error:m.update(status='protocol_error',error=str(error))
    base.dump(out/'manifest.json',m)
    summary=validate_directory(out);base.dump(out/'summary.json',summary)
    return summary

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('output',type=Path);p.add_argument('--validate',action='store_true');args=p.parse_args()
    r=validate_directory(args.output.resolve()) if args.validate else collect(args.output.resolve())
    print(json.dumps({k:v for k,v in r.items() if k!='records'},indent=2,allow_nan=False))
