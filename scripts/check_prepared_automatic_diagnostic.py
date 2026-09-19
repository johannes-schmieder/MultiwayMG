#!/usr/bin/env python3
"""Compare actual reference/profile binaries across layouts, failures and framing."""
import argparse,json,os,platform,struct,subprocess,time
from pathlib import Path
from automatic_protocol import ROUTES,parse_output as parse_legacy
from automatic_recipes import FAMILIES,generate
from automatic_diagnostic_protocol import LAYOUTS,check_diagnostic,parse_output,signature,scalar_signature
from automatic_diagnostic_runner import run_one,check_reference
from check_prepared_automatic_probe import packed
from prepared_serial import ROOT,THREAD_ENV,dump,sha
from validate_prepared_serial import require

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['reference','instrumented','legacy','output']:p.add_argument(name,type=Path)
    args=p.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    binaries={key:getattr(args,key).resolve() for key in ['reference','instrumented','legacy']}
    policy=json.loads((ROOT/'benchmarks/policies/prepared-automatic-v1.json').read_text())
    env=os.environ.copy();env.update(THREAD_ENV,LC_ALL='C');origin=time.perf_counter_ns();index=0
    counters=dict(paired=0,legacy=0,layout=0,malformed=0,cli=0)
    with (out/'attempts.jsonl').open('x') as journal:
        def check(data,dims,case,success=True):
            nonlocal index
            for route in ROUTES:
                process=subprocess.run([str(binaries['legacy']),route],input=data,capture_output=True,env=env,timeout=policy['timeout_seconds'])
                legacy=parse_legacy(process.stdout.decode());counters['legacy']+=1
                (out/f'legacy-{counters["legacy"]}.stdout').write_bytes(process.stdout)
                (out/f'legacy-{counters["legacy"]}.stderr').write_bytes(process.stderr)
                scalar=None
                for layout in LAYOUTS:
                    pair=[]
                    for build in ['reference','instrumented']:
                        spec=dict(case=case,kind='protocol',repeat=0,position=0,route=route,layout=layout,build=build)
                        row=run_one(binaries[build],data,policy,platform.system(),out,index,spec,origin,env)
                        index+=1;row['dimensions']=dims;journal.write(json.dumps(row,sort_keys=True,allow_nan=False)+'\n');journal.flush()
                        require(row['status']=='returned',str(row))
                        require(row['resources']['peak_rss_bytes']<=policy['process_budget_bytes'],'RSS over budget')
                        check_diagnostic(row['probe'],layout,build=='instrumented',row['process_wall_ns'])
                        if build=='reference':require(check_reference(row,policy)==success,'wrong reference certification')
                        pair.append(row)
                    a,b=pair;require(a['exit_code']==b['exit_code'] and signature(a['probe'])==signature(b['probe']),'observer changed fixed numerical/work/memory result')
                    counters['paired']+=1
                    common=scalar_signature(a['probe']);common.pop('payload_bytes')
                    if scalar is None:
                        scalar=common
                        require(scalar_signature(a['probe'])==scalar_signature(legacy),'changed frozen scalar result')
                        require(a['exit_code']==process.returncode,'changed legacy exit')
                    else:require(common==scalar,'layout changed numerical/work result');counters['layout']+=1
        for family in FAMILIES:
            for shape in policy['shapes']:
                for weights in policy['weights']:
                    case=dict(family=family,shape=shape,weights=weights,seed=10001,width=1)
                    data,dims=generate(policy,'smoke',case);check(data,dims,case)
        for k in [17,32]:
            case=dict(family='uniform',shape='unbalanced',weights='heterogeneous',seed=10001,width=k)
            data,dims=generate(policy,'smoke',case);check(data,dims,case)
        for family in ['recursive','weak-fallback']:
            if family=='recursive':
                counts=(512,2,2);keys=[(i,j,k) for i in range(512) for j in range(2) for k in range(2)];weights=[1.0]*len(keys)
            else:
                counts=(128,128,128);keys=sorted([(i,i,i) for i in range(128)]+[(i,(i+1)%128,(i+1)%128) for i in range(128)])
                weights=[1.0 if i==j else 2.0**-30 for i,j,k in keys]
            data,dims=packed(counts,keys,weights,[((i*17)%101-50)/64.0 for i in range(len(keys))])
            check(data,dims,dict(family=family,shape='protocol',weights='protocol',seed=10001,width=1))
        keys=[(i,j,k) for i in range(2) for j in range(2) for k in range(2)]
        data,dims=packed((2,2,2),keys,[1e300]*8,[1e20]*8)
        check(data,dims,dict(family='numerical-error',shape='protocol',weights='protocol',seed=10001,width=1),False)
        good,_=packed((2,2,2),keys,[1.0]*8,[0.0]*8)
        bad=[good[:n] for n in [0,8,31,32,47,len(good)-1]]+[good+b'X']
        for offset,value,fmt in [(8,0,'I'),(20,100001,'Q'),(28,0,'I'),(28,33,'I'),(32+12*8,float('nan'),'d'),(32+20*8,float('inf'),'d')]:
            x=bytearray(good);struct.pack_into('<'+fmt,x,offset,value);bad.append(bytes(x))
        for build in ['reference','instrumented']:
            for data in bad:
                process=subprocess.run([str(binaries[build]),'automatic','all-image'],input=data,capture_output=True,env=env,timeout=20)
                require(process.returncode!=0,'malformed input accepted')
                probe=parse_output(process.stdout.decode());require(probe['status']=='error' and not probe['columns'],'invented failed result')
                check_diagnostic(probe,'all-image',build=='instrumented',max(probe['total_ns'],1))
                name=f'malformed-{build}-{counters["malformed"]}';(out/(name+'.stdout')).write_bytes(process.stdout);(out/(name+'.stderr')).write_bytes(process.stderr);counters['malformed']+=1
            for arguments in [[],['automatic'],['automatic','bad'],['automatic','scalar','extra'],['typo','scalar']]:
                process=subprocess.run([str(binaries[build]),*arguments],input=good,capture_output=True,env=env,timeout=20)
                require(process.returncode!=0,'invalid CLI accepted');probe=parse_output(process.stdout.decode())
                require(probe['status']=='error' and not probe['columns'],'invented CLI result')
                name=f'cli-{build}-{counters["cli"]}';(out/(name+'.stdout')).write_bytes(process.stdout);(out/(name+'.stderr')).write_bytes(process.stderr);counters['cli']+=1
    dump(out/'summary.json',dict(complete=True,processes=index,checks=counters,binary_sha256={k:sha(v.read_bytes()) for k,v in binaries.items()},performance_measurement=False))
    print('PASS',index,'reference/profile processes;',counters,'; diagnostic only')
if __name__=='__main__':main()
