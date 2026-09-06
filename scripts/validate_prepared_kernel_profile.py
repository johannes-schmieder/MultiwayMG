#!/usr/bin/env python3
"""Diagnostic profile validation: provenance, work identity, complete exclusive-time accounting."""
import argparse
import json
from pathlib import Path
import statistics

from prepared_serial import ROOT, sha, command, resources
from prepared_kernel_profile import (POLICY_PROFILE, PROFILE_FILES, PHASE_NAMES, PROFILE_MEMORY,
    effective_policy,parse_profile_output)
from validate_prepared_serial import (load,require,integer,hash_string,validate_manifest)


def check_profile(profile,elapsed):
    require(set(profile)=={'index','valid','elapsed_ns','maximum_depth','phases'},'missing/extra profile scopes')
    integer(profile['index'],'profile index');integer(profile['elapsed_ns'],'profile elapsed')
    integer(profile['maximum_depth'],'maximum profile depth')
    require(type(profile['valid']) is bool,'invalid profile status')
    require(profile['elapsed_ns']<=elapsed and profile['maximum_depth']<=128,'invalid profile elapsed/depth')
    require(set(profile['phases'])==set(PHASE_NAMES),'missing/extra profile phase')
    for record in profile['phases'].values():
        require(set(record)=={'calls','inclusive_ns','exclusive_ns'},'missing/extra phase counters')
        for key,value in record.items():integer(value,'profile '+key)
        if profile['valid']:
            require(record['exclusive_ns']<=record['inclusive_ns'],'invalid exclusive time')
            require(record['calls']>0 or (record['inclusive_ns']==record['exclusive_ns']==0),'time without calls')
    if profile['valid']:
        for phase in PHASE_NAMES[:11]:
            if phase != 'weighted_incidence':
                p=profile['phases'][phase]
                require(p['inclusive_ns']==p['exclusive_ns'],'leaf phase hides nested/unattributed work')
    if profile['valid']:
        require(sum(p['exclusive_ns'] for p in profile['phases'].values())<=profile['elapsed_ns'],'exclusive times exceed callback')
    return profile['valid']


def check_complete_work(profile,column,route,depth):
    calls={key:value['calls'] for key,value in profile['phases'].items()}
    w=column['work'];h=w['hierarchy']
    expected=dict(incidence=w['weighted_incidence']+w['certificate_incidence'],adjoint=0,
        weighted_incidence=w['weighted_incidence'],weighted_adjoint=w['weighted_adjoint'],
        gramian=w['gramian']+2*depth*h,rhs=w['rhs_adjoint']+w['certificate_adjoint'],
        projection=w['projection']+(7*depth+1)*h,map_sweep=2*depth*h,
        restriction=depth*h,prolongation=depth*h,dense_terminal=h,cycle=(depth+1)*h,
        pcg_recurrence=int(route=='pcg'),prepared_pcg=int(route=='pcg'),prepared_lsmr=int(route!='pcg'),
        certificate=w['certificate_incidence'])
    require(calls==expected,'profile actions disagree with complete solver/certificate work')


def validate_profiles(manifest,policy,hashes):
    summary=validate_manifest(manifest,policy,hashes,evidence_scope='prepared_serial_kernel_profile_only',
        policy_paths=(POLICY_PROFILE,),memory_scopes=PROFILE_MEMORY)
    valid,invalid_count,profiles,signatures=True,0,{},{}
    for run in manifest['runs']:
        if run['status']=='timeout':valid=False;continue
        p=run['profiling'];base=run['probe']
        require(set(p)<= {'metadata','columns','failed','levels'} and {'metadata','columns','levels'}<=set(p),'missing profile scope')
        meta=p['metadata']
        require(set(meta)=={'schema','thread_local_bytes','record_inline_bytes','report_inline_bytes'} and type(meta['schema']) is int and meta['schema']==1,'invalid profiler memory metadata')
        for key in ['thread_local_bytes','record_inline_bytes','report_inline_bytes']:integer(meta[key],key,1)
        require(meta['record_inline_bytes']>=32*meta['report_inline_bytes'],'missing inline column profiles')
        if 'payload_bytes' in base:
            require(base['payload_bytes']['total']+meta['thread_local_bytes']+meta['record_inline_bytes']<=run['resources']['peak_rss_bytes'],'profile plus solver payload exceeds process RSS')
        inventory=base['status']=='complete' or base.get('error_stage') in ['terminal','workspace_output','solve_certificate_output']
        require(len(p['levels'])==(run['dimensions']['depth']+1 if inventory else 0),'missing or premature hierarchy inventory')
        previous=run['dimensions']['tuples']
        for index,level in enumerate(p['levels']):
            require(set(level)=={'index','tuples','coefficients'} and level['index']==index,'invalid level inventory')
            integer(level['index'],'level index');integer(level['tuples'],'level tuples',1);integer(level['coefficients'],'level coefficients',1)
            require(level['coefficients']==run['dimensions']['coefficients']//(1<<index),'invalid coarse dimensions')
            require(level['tuples']<=previous and (index!=0 or level['tuples']==previous),'invalid coarse tuple inventory')
            previous=level['tuples']
        require(len(p['columns'])==len(base['columns']),'missing completed column profile')
        for index,(profile,column) in enumerate(zip(p['columns'],base['columns'])):
            require(profile['index']==index,'wrong profile column')
            okay=check_profile(profile,column['elapsed_ns']);valid &= okay;invalid_count+=int(not okay)
            if okay:check_complete_work(profile,column,run['route'],run['dimensions']['depth'])
        failed=base['status']=='error' and base['error_stage']=='solve_certificate_output'
        require(('failed' in p)==failed,'missing/contradictory failed profile')
        if failed:
            require(p['failed']['index']==len(base['columns']),'wrong failed profile prefix')
            okay=check_profile(p['failed'],base['failed_action_ns']);valid &= okay;invalid_count+=int(not okay)
        def signature(profile):return {k:v for k,v in profile.items() if k not in ['elapsed_ns','phases']} | {
            'calls':{k:v['calls'] for k,v in profile['phases'].items()}}
        sig={'metadata':meta,'levels':p['levels'],'columns':[signature(c) for c in p['columns']],
             'failed':signature(p['failed']) if failed else None}
        key=(run['case_id'],run['route'])
        if key in signatures:require(sig==signatures[key],'nonrepeatable profiling work or memory')
        signatures[key]=sig
        if run['kind']=='measured' and base['status']=='complete' and all(c['valid'] for c in p['columns']):
            profiles.setdefault(key,[]).append(p['columns'])
    attribution=[]
    for (cid,route),repeats in sorted(profiles.items()):
        phases={}
        for phase in PHASE_NAMES:
            phases[phase]=dict(calls_per_batch=sum(c['phases'][phase]['calls'] for c in repeats[0]),
                inclusive_ns_median=statistics.median(sum(c['phases'][phase]['inclusive_ns'] for c in cols) for cols in repeats),
                exclusive_ns_median=statistics.median(sum(c['phases'][phase]['exclusive_ns'] for c in cols) for cols in repeats))
        attribution.append(dict(case_id=cid,route=route,levels=signatures[(cid,route)]['levels'],measured_repetitions=len(repeats),phases=phases,
            callback_ns_median=statistics.median(sum(c['elapsed_ns'] for c in cols) for cols in repeats),
            unattributed_ns_median=statistics.median(sum(c['elapsed_ns']-sum(p['exclusive_ns'] for p in c['phases'].values()) for c in cols) for cols in repeats)))
    return summary | dict(valid_profiles=valid,invalid_profile_reports=invalid_count,
        diagnostic_gate_passed=valid and summary['eligible_routes_certified'],
        authoritative_performance_measurement=False,kernel_profiles=attribution,
        timing_scope='instrumented callback; exclusive categories partition it with observer overhead; inclusive categories overlap')


def validate_profile_directory(directory):
    directory=Path(directory);manifest=load(directory/'manifest.json')
    commit=manifest['provenance']['source_commit'];hash_string(commit,40)
    require(manifest['provenance']['source_tree']==command(['git','rev-parse',f'{commit}^{{tree}}']).decode().strip(),'wrong source tree')
    source={name:command(['git','show',f'{commit}:{name}']) for name in PROFILE_FILES}
    hashes={name:sha(value) for name,value in source.items()}
    for name in ['scripts/prepared_serial.py','scripts/prepared_kernel_profile.py','scripts/validate_prepared_kernel_profile.py']:
        require(hashes[name]==sha((ROOT/name).read_bytes()),'use measured generator/profile validator')
    policy=effective_policy(json.loads(source[POLICY_PROFILE]),manifest['profile'])
    require(sha((directory/'prepared_serial_benchmark').read_bytes())==manifest['provenance']['binary_sha256'],'wrong profiling executable')
    require(sha((directory/'build.log').read_bytes())==manifest['build_log_sha256'],'wrong build log')
    for run in manifest['runs']:
        data=[]
        for key in ['raw_stdout','raw_stderr']:
            name=run[key];require(Path(name).name==name and name not in ['.','..'],'unsafe artifact path')
            value=(directory/name).read_bytes();require(sha(value)==run[key+'_sha256'],'raw artifact hash mismatch');data.append(value)
        if run['status']=='returned':
            base,profile=parse_profile_output(data[0].decode())
            require(base==run['probe'] and profile==run['profiling'],'raw profiling output mismatch')
            require(resources(data[1].decode(),manifest['provenance']['system'])==run['resources'],'raw resource mismatch')
    try:return validate_profiles(manifest,policy,hashes)
    except (KeyError,TypeError,IndexError) as error:raise ValueError(f'malformed profiling evidence: {error}') from error


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path)
    parser.add_argument('--require-diagnostics',action='store_true');args=parser.parse_args()
    result=validate_profile_directory(args.directory);print(json.dumps(result,indent=2,allow_nan=False))
    raise SystemExit(1 if args.require_diagnostics and not result['diagnostic_gate_passed'] else 0)
