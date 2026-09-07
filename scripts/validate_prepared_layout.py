#!/usr/bin/env python3
"""Validate complete five-layout evidence, physical payload deltas and paired order."""
import argparse
import math
import statistics
from functools import lru_cache
import json
from pathlib import Path
import struct
import subprocess
import prepared_serial as base
from prepared_layout import FILES, POLICY, PHASES, PAYLOAD, MEMORY_SCOPES, effective_policy, schedule, parse_layout_output
from validate_prepared_serial import (require, integer, hash_string, load, no_nonfinite,
    check_probe, validate_manifest, numerical_signature)

@lru_cache(maxsize=24)
def inventory(policy_text, profile, family, seed):
    policy=json.loads(policy_text)
    data,dims=base.generate(policy,profile,dict(family=family,weights='unit',seed=seed,width=1))
    tuples=set(struct.iter_unpack('<III',data[36:36+12*dims['tuples']]))
    result=[]
    for level in range(dims['depth']+1):
        result.append((len(tuples),dims['coefficients']//(1<<level)))
        tuples={tuple(v//2 for v in t) for t in tuples}
    return tuple(result)

def check_layout_probe(probe, run, policy, profile, layout):
    passed=check_probe(probe,run,policy,schema=3,phase_names=PHASES,payload_names=PAYLOAD,
        zero_phase_names=('grouping',),zero_payload_names=('grouping',))
    info=probe['layout']; levels=probe['layout_levels']
    require(set(info)=={'name','prefix','usize_bytes','group_descriptor_bytes','image_descriptor_bytes',
        'record_bytes','retained','setup_bound','image_len'},'missing/extra layout metadata')
    require(info['name']==layout,'wrong explicit layout')
    for k,v in info.items():
        if k!='name': integer(v,'layout '+k)
    require(all(info[k]==v for k,v in policy['layout_abi'].items()),'unsupported layout ABI')
    require(4096 <= info['record_bytes'] <= 1048576,'invalid fixed record scope')
    require(isinstance(levels,list),'missing hierarchy inventory')
    for level in levels:
        require(set(level)=={'tuples','coefficients','index_bytes'},'missing/extra level inventory')
        for k,v in level.items():integer(v,'level '+k)
    if probe['status']!='complete':
        # The common boundary retains and charges every failed setup/action.
        # Do not invent complete grouping or hierarchy inventory before setup.
        require(info['setup_bound']<=policy['process_budget_bytes'] or probe['error_stage']=='grouping',
            'unexplained grouping admission failure')
        return passed
    expected=inventory(json.dumps(policy,sort_keys=True),profile,run['case']['family'],run['case']['seed'])
    depth=run['dimensions']['depth']
    prefix=0 if layout=='scalar' else (min(1,depth) if layout.startswith('fine-') else depth)
    require(info['prefix']==prefix and len(levels)==depth+1,'wrong grouped prefix or missing levels')
    require([(v['tuples'],v['coefficients']) for v in levels]==list(expected),'wrong realized hierarchy tuples/dimensions')
    require([v['index_bytes'] for v in levels]==[4 if i<prefix else 0 for i in range(depth+1)],'wrong stored tuple ID widths')
    retained=prefix*info['group_descriptor_bytes']+sum(8*(v+3)+8*e for e,v in expected[:prefix])
    require(info['retained']==retained==probe['payload_bytes']['grouping'],'omitted/invented grouping capacities')
    image=max((e for e,_ in expected[:prefix]),default=0) if layout.endswith('image') else 0
    require(info['image_len']==image,'duplicated or missing shared tuple image')
    e,n,k=(run['dimensions'][q] for q in ('tuples','coefficients','rhs'))
    input_bytes=12*e+8*(e+e*k)
    payload=probe['payload_bytes']
    require(payload['caller_arrays']==input_bytes+8*n*k,'wrong input/output capacities')
    require(payload['total']+info['record_bytes']<=run['resources']['peak_rss_bytes'],'record/arrays exceed process RSS')
    if prefix:
        live=input_bytes+payload['fine_topology']+payload['coarse_topology']+payload['fine_frame']+prefix*info['group_descriptor_bytes']
        peak=live
        for e,v in expected[:prefix]:
            array=8*(v+3)+8*e
            peak=max(peak,live+array+8*(v//3))
            live+=array
        require(info['setup_bound']==max(peak,live),'omitted cursor/live owners or summed dead cursors')
        require(info['setup_bound']<=policy['process_budget_bytes'] and probe['phases_ns']['grouping']>0,'uncharged/inadmissible grouping setup')
    else:
        require(info['setup_bound']==0,'invented scalar grouping construction')
    return passed

def compare_complete(a,b,layout):
    aa=numerical_signature(a); bb=numerical_signature(b)
    pa=aa.pop('payload_bytes'); pb=bb.pop('payload_bytes')
    require(aa==bb,'layout changed numerical/work/fingerprint results')
    image=b['layout']['image_len']
    image_bytes=8*image+(b['layout']['image_descriptor_bytes'] if image else 0)
    require(pb['grouping']==b['layout']['retained'],'grouping owner charge mismatch')
    for key in PAYLOAD:
        delta=pb['grouping'] if key=='grouping' else image_bytes if key=='hierarchy_workspace' else pb['grouping']+image_bytes if key=='total' else 0
        require(pb[key]==pa[key]+delta,'wrong complete layout payload delta: '+layout+'/'+key)

def _validate_manifest_pair(manifest,children,policy,hashes):
    no_nonfinite(manifest)
    require(manifest['schema']==1 and manifest['scope']=='prepared_layout_pairing','wrong layout pairing scope')
    require(manifest['policy']==policy and manifest['source_hashes']==hashes,'changed paired policy/source')
    profile=manifest['profile']; require(profile in policy['profiles'],'unknown profile')
    effective=effective_policy(policy,profile)
    require(set(children)==set(policy['layouts']),'missing/extra explicit layout')
    summaries={}; metadata=None; record_sizes=set()
    for layout,child in children.items():
        require(child['layout']==layout and child['profile']==profile,'wrong child layout/profile')
        require(child['provenance']['source_commit']==manifest['source_commit'],'mixed source revisions')
        if metadata is None:metadata=child['provenance']
        require(child['provenance']==metadata,'different layout binary/compiler/hardware provenance')
        record_sizes.update(r['probe']['layout']['record_bytes'] for r in child['runs'] if r['status']=='returned')
        summaries[layout]=validate_manifest(child,effective,hashes,evidence_scope=effective['scope'],
            policy_paths=(POLICY,),memory_scopes=MEMORY_SCOPES,summary_phase_names=PHASES,
            probe_checker=lambda p,r,pol,layout=layout:check_layout_probe(p,r,pol,profile,layout))
    require(len(record_sizes)<=1,'nonrepeatable fixed benchmark record size')
    expected=list(schedule(effective)); require(len(manifest['trace'])==len(expected),'missing paired attempts')
    end=0
    for trace,(layout,index,*_) in zip(manifest['trace'],expected):
        require(set(trace)=={'layout','index','start_ns','end_ns'},'missing/extra trace field')
        require(trace['layout']==layout and trace['index']==index,'non-rotated actual execution order')
        for key in ['start_ns','end_ns']:integer(trace[key],'trace '+key)
        require(end<=trace['start_ns']<trace['end_ns'],'overlapping or reversed processes')
        require(children[layout]['runs'][index]['process_wall_ns']<=trace['end_ns']-trace['start_ns'],'trace omits process cost')
        end=trace['end_ns']
    integer(manifest['total_ns'],'paired driver elapsed',1)
    require(manifest['total_ns']>=end,'omitted final attempt')
    compared=0; complete=True
    reference=children['scalar']['runs']
    for layout,child in children.items():
        if layout=='scalar':continue
        for a,b in zip(reference,child['runs']):
            if a['status']=='returned' and b['status']=='returned' and a['probe']['status']=='complete' and b['probe']['status']=='complete':
                compare_complete(a['probe'],b['probe'],layout);compared+=1
            else:complete=False
    gate=complete and all(s['eligible_routes_certified'] for s in summaries.values())
    paired=[]; balanced={}
    for layout,child in children.items():
        if layout=='scalar':continue
        route_values={route:[] for route in effective['routes']}
        for case in base.cases(effective):
            cid=base.case_id(case)
            for route in effective['routes']:
                pairs=[(a,b) for a,b in zip(reference,child['runs'])
                    if a['case_id']==cid and a['route']==route and a['kind']=='measured']
                valid=[(a,b) for a,b in pairs if a['status']=='returned' and b['status']=='returned'
                    and a['probe']['status']=='complete' and b['probe']['status']=='complete'
                    and all(c['accepted'] for c in a['probe']['columns']+b['probe']['columns'])
                    and a['resources']['peak_rss_bytes']<=effective['process_budget_bytes']
                    and b['resources']['peak_rss_bytes']<=effective['process_budget_bytes']]
                row=dict(layout=layout,case_id=cid,route=route,complete_certified_pairs=len(valid),
                    attempted_pairs=len(pairs),process_speedup_median=None,inner_speedup_median=None,
                    attempted_scalar_seconds=sum(a['process_wall_ns'] for a,_ in pairs)/1e9,
                    attempted_layout_seconds=sum(b['process_wall_ns'] for _,b in pairs)/1e9)
                if len(valid)==effective['repetitions']:
                    row['process_speedup_median']=statistics.median(a['process_wall_ns']/b['process_wall_ns'] for a,b in valid)
                    row['inner_speedup_median']=statistics.median(a['probe']['total_ns']/b['probe']['total_ns'] for a,b in valid)
                    route_values[route].append(row)
                paired.append(row)
        cells=len(list(base.cases(effective)))
        balanced[layout]={route:dict(qualified_cells=len(rows),expected_cells=cells,
            process_geomean=math.exp(sum(math.log(r['process_speedup_median']) for r in rows)/cells) if len(rows)==cells else None,
            inner_geomean=math.exp(sum(math.log(r['inner_speedup_median']) for r in rows)/cells) if len(rows)==cells else None)
            for route,rows in route_values.items()}
    return dict(scope=policy['scope'],profile=profile,complete_layout_gate_passed=gate,
        processes=len(expected),paired_processes_compared=compared,all_pairs_complete=complete,
        exact_numerical_work_fingerprint_agreement=complete,completed_pairs_exact=True,competitive_qualification=False,
        default_layout_selected=False,layouts=summaries,paired_cells=paired,balanced=balanced)

def validate_manifest_pair(manifest,children,policy,hashes):
    try:
        return _validate_manifest_pair(manifest,children,policy,hashes)
    except (KeyError,TypeError,IndexError) as error:
        raise ValueError(f'malformed layout evidence: {error}') from error


def validate_directory(directory):
    directory=Path(directory); manifest=load(directory/'manifest.json')
    commit=manifest['source_commit'];hash_string(commit,40)
    def git(*args):return subprocess.check_output(['git',*args],cwd=base.ROOT)
    source={name:git('show',f'{commit}:{name}') for name in FILES}
    hashes={name:base.sha(value) for name,value in source.items()}
    require(hashes['scripts/prepared_serial.py']==base.sha((base.ROOT/'scripts/prepared_serial.py').read_bytes()),'use the measured source input generator')
    policy=json.loads(source[POLICY]); require(policy['layouts']==['scalar','fine-row','all-row','fine-image','all-image'],'unknown layout protocol')
    tree=git('rev-parse',f'{commit}^{{tree}}').decode().strip()
    children={layout:load(directory/layout/'manifest.json') for layout in policy['layouts']}
    binary=base.sha((directory/'prepared_serial_benchmark').read_bytes());build=base.sha((directory/'build.log').read_bytes())
    for layout,child in children.items():
        require(child['provenance']['source_tree']==tree and child['provenance']['binary_sha256']==binary,'wrong measured tree/binary')
        require(child['build_log_sha256']==build,'wrong build log')
        for run in child['runs']:
            def raw(name):
                require(isinstance(name,str) and Path(name).name==name and name not in ('.','..'),'unsafe raw path')
                return (directory/layout/name).read_bytes()
            stdout,stderr=raw(run['raw_stdout']),raw(run['raw_stderr'])
            require(base.sha(stdout)==run['raw_stdout_sha256'] and base.sha(stderr)==run['raw_stderr_sha256'],'raw evidence hash mismatch')
            if run['status']=='returned':
                require(parse_layout_output(stdout.decode())==run['probe'],'raw layout record disagreement')
                require(base.resources(stderr.decode(),child['provenance']['system'])==run['resources'],'raw RSS/CPU disagreement')
    return validate_manifest_pair(manifest,children,policy,hashes)

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path)
    parser.add_argument('--require-certification',action='store_true');args=parser.parse_args()
    result=validate_directory(args.directory);print(json.dumps(result,indent=2,allow_nan=False))
    raise SystemExit(int(args.require_certification and not result['complete_layout_gate_passed']))
