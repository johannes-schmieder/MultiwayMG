#!/usr/bin/env python3
"""Validate complete uninstrumented layout costs, failed work and declared pairing."""
import argparse,json,statistics,subprocess
from pathlib import Path
from automatic_layout_protocol import SCOPES,schedule,widths,validate_policy
from automatic_diagnostic_protocol import parse_output,signature,scalar_signature
from automatic_diagnostic_runner import check_reference
from automatic_recipes import case_id,generate
from automatic_protocol import PHASES
from prepared_serial import ROOT,sha,resources,THREAD_ENV
from validate_prepared_serial import require,integer,finite,no_nonfinite,hash_string
from validate_prepared_automatic import geometric

def validate_records(manifest,runs,policy):
    no_nonfinite(manifest);validate_policy(policy,manifest['profile'])
    require(type(manifest['schema']) is int and manifest['schema']==1 and manifest['status']=='complete','incomplete layout collection')
    require(manifest['policy']==policy and manifest['scope']==policy['scope'] and manifest['scopes']==SCOPES,'layout policy/scope drift')
    meta=manifest['provenance']
    require(all(k in meta for k in ['source_hashes','target','cargo_config_hashes','system','uname','hardware','memory','affinity','scheduler','python','sdk_environment']),'missing provenance fields')
    require(meta['system'] in ['Darwin','Linux'] and meta['target'] and meta['hardware'] and meta['uname'] and meta['python'],'missing platform provenance')
    require(meta['affinity'].get('status') in ['measured','unavailable'],'missing affinity scope')
    if meta['affinity']['status']=='measured':
        cpus=meta['affinity'].get('cpus');require(isinstance(cpus,list) and cpus==sorted(set(cpus)) and cpus,'invalid measured affinity')
        for cpu in cpus:integer(cpu,'affinity CPU')
    else: require(meta['system']=='Darwin' and meta['affinity'].get('reason'),'missing placement limitation')
    require(meta['source_clean'] is True and meta['rustc'].startswith('rustc 1.85.0 '),'wrong compiler/source state')
    for key,size in [('source_commit',40),('source_tree',40),('binary_sha256',64)]:hash_string(meta[key],size)
    require(meta['build_args']==policy['build_args'] and meta['thread_env']==THREAD_ENV,'wrong build/threads')
    require(meta['rustflags']=='' and meta['compiler_overrides']=={},'undeclared compiler overrides')

    expected=list(schedule(policy,manifest['profile']));require(len(runs)==len(expected),'missing/extra paired attempt')
    cache={};seen={};math_seen={};cells={};last_end=0;failed_ns=0;failed=0;warm_failed=0;certified=0;expected_columns=0;layout_comparisons=0
    for index,(row,spec) in enumerate(zip(runs,expected)):
        no_nonfinite(row);integer(row['index'],'attempt index');require(row['index']==index,'wrong attempt order')
        for key,value in spec.items():require(row[key]==value,'wrong scheduled '+key)
        require(row['exit_code'] is None if row['status']=='launch_error' else type(row['exit_code']) is int,'invalid exit code')
        cid=case_id(spec['case']);require(row['case_id']==cid,'wrong case ID')
        if cid not in cache:
            data,dims=generate(policy,manifest['profile'],spec['case']);cache[cid]=(sha(data),dims)
        ih,dims=cache[cid];require(row['input_sha256']==ih and row['dimensions']==dims,'wrong paired input/dimensions')
        for key in ['start_ns','end_ns','process_wall_ns']:integer(row[key],key)
        require(row['start_ns']>=last_end and row['end_ns']-row['start_ns']==row['process_wall_ns']>0,'overlap or omitted process cost');last_end=row['end_ns']
        for key in ['stdout_sha256','stderr_sha256']:hash_string(row[key])
        eligible=False
        if row['status']=='returned':
            require(row['resource_status']=='measured','missing process resources');resource=row['resources']
            integer(resource['peak_rss_bytes'],'RSS',1);finite(resource['user_seconds'],'user CPU');finite(resource['system_seconds'],'system CPU')
            require(resource['scope']=='isolated_process_including_startup_teardown' and resource['method']==('darwin_time_l' if meta['system']=='Darwin' else 'gnu_time_v'),'wrong resource scope')
            # This validates the real profiling flag, both zero TLS capacities,
            # absent regions and all layout fields before any schema projection.
            eligible=check_reference(row,policy) and resource['peak_rss_bytes']<=policy['process_budget_bytes']
            key=(cid,row['arm']);sig=signature(row['probe'])
            if key in seen:require(sig==seen[key],'changed fixed-configuration math/work/payload/layout')
            else:seen[key]=sig
            if eligible:
                key=(cid,row['route']);sig=scalar_signature(row['probe']);sig.pop('payload_bytes')
                if key in math_seen:require(sig==math_seen[key],'layout changed certified math/work');layout_comparisons+=1
                else:math_seen[key]=sig
        else:
            require(row['status'] in ['launch_error','timeout','protocol_error'] and row.get('error'),'unknown failed process')
            require('probe' not in row and 'resources' not in row,'invented data after failed protocol')
            require(row['resource_status']=={'launch_error':'unavailable_after_launch_error','timeout':'unavailable_after_timeout','protocol_error':'unavailable_or_invalid_protocol'}[row['status']],'wrong failed resource scope')
            if row['status']=='timeout':require(row['process_wall_ns']>=int(policy['timeout_seconds']*1e9),'uncharged timeout')
        if not eligible:failed_ns+=row['process_wall_ns']
        if row['kind']=='warmup':warm_failed+=not eligible
        else:
            failed+=not eligible;expected_columns+=dims['rhs']
            certified+=sum(c['accepted'] for c in row.get('probe',{}).get('columns',[]))
        cells.setdefault(cid,{}).setdefault(row['arm'],{}).setdefault(row['kind'],[]).append((row,eligible))
    integer(manifest['collection_ns'],'collection duration',1);require(manifest['collection_ns']>=last_end,'collection ended before last attempt')
    summaries=[];comparisons={(c['candidate'],c['control']):[] for c in policy['comparisons']}
    for cid,arms in cells.items():
        require(set(arms)=={a['name'] for a in policy['arms']},'missing paired arm')
        first=next(iter(arms.values()))['measured'][0][0]
        cell=dict(case=first['case'],case_id=cid,dimensions=first['dimensions'],arms={},comparisons=[])
        for arm,kinds in arms.items():
            require(set(kinds)=={'warmup','measured'},'missing warmup/measured partition')
            warm,attempts=kinds['warmup'],kinds['measured'];require(len(warm)==policy['warmups'] and len(attempts)==policy['repetitions'],'missing repetitions')
            times=[a['process_wall_ns']/1e9 for a,_ in attempts]
            cpu=[a['resources']['user_seconds']+a['resources']['system_seconds'] for a,_ in attempts] if all('resources' in a for a,_ in attempts) else None
            result=dict(eligible=all(ok for _,ok in warm+attempts),measured_failures=sum(not ok for _,ok in attempts),warmup_failures=sum(not ok for _,ok in warm),
                process_seconds=times,process_seconds_median=statistics.median(times),
                warmup_process_seconds=sum(a['process_wall_ns'] for a,_ in warm)/1e9,
                peak_rss_bytes_max=max((a.get('resources',{}).get('peak_rss_bytes',0) for a,_ in warm+attempts),default=0) or None,
                cpu_seconds=cpu,cpu_seconds_median=statistics.median(cpu) if cpu is not None else None,
                inner_seconds_median=statistics.median(a['probe']['total_ns']/1e9 for a,_ in attempts) if all('probe' in a for a,_ in attempts) else None,
                phases_seconds_median={k:statistics.median(a['probe']['phases_ns'][k]/1e9 for a,_ in attempts) for k in PHASES} if all('probe' in a for a,_ in attempts) else None)
            returned=next((a['probe'] for a,_ in warm+attempts if 'probe' in a),None)
            result['deterministic_record']=({k:returned[k] for k in ['counts','solve_work','payload_bytes','status','progress','grouping','layout','record_bytes']} |
                {k:returned[k] for k in ['rejection','quality','grouping_rejection','error_stage','error'] if k in returned}) if returned else None
            cell['arms'][arm]=result
        for candidate,control in comparisons:
            a,b=cell['arms'][candidate],cell['arms'][control];eligible=a['eligible'] and b['eligible']
            ratios=[y/x for x,y in zip(a['process_seconds'],b['process_seconds'])] if eligible else None
            item=dict(candidate=candidate,control=control,eligible=eligible,paired_process_speedups=ratios,
                paired_process_geomean=geometric(ratios) if ratios else None,
                peak_rss_ratio=a['peak_rss_bytes_max']/b['peak_rss_bytes_max'] if eligible else None)
            cell['comparisons'].append(item)
            if eligible:comparisons[candidate,control].append((cell['case'],item['paired_process_geomean']))
        summaries.append(cell)
    balanced=[]
    for (candidate,control),values in comparisons.items():
        subsets={}
        for name,families in [('all',policy['families']),('hard',policy['hard_families'])]:
            selected=[(c,v) for c,v in values if c['family'] in families];groups={}
            for c,v in selected:groups.setdefault((c['family'],c['width']),[]).append(v)
            expected_cells=len(families)*len(widths(policy,manifest['profile']))*len(policy['shapes'])*len(policy['weights'])*len(policy['seeds'])
            complete=len(selected)==expected_cells and bool(families)
            require(not complete or len(groups)==len(families)*len(widths(policy,manifest['profile'])),'missing balanced groups')
            subsets[name]=dict(qualified_cells=len(selected),expected_cells=expected_cells,complete_coverage=complete,
                equal_family_width_geomean=geometric([geometric(v) for v in groups.values()]) if complete else None,
                single_rhs_geomean=geometric([geometric(v) for (f,k),v in groups.items() if k==1]) if complete and 1 in widths(policy,manifest['profile']) else None,
                repeated_rhs_geomean=geometric([geometric(v) for (f,k),v in groups.items() if k!=1]) if complete and any(k!=1 for k in widths(policy,manifest['profile'])) else None)
        balanced.append(dict(candidate=candidate,control=control,subsets=subsets))
    return dict(schema=1,scope=policy['scope'],profile=manifest['profile'],processes=len(runs),certified_measured_columns=certified,
        expected_measured_columns=expected_columns,failed_measured_runs=failed,failed_warmup_runs=warm_failed,failed_process_seconds=failed_ns/1e9,
        measured_process_seconds=sum(r['process_wall_ns'] for r in runs if r['kind']=='measured')/1e9,
        warmup_process_seconds=sum(r['process_wall_ns'] for r in runs if r['kind']=='warmup')/1e9,
        fixed_configuration_records=len(seen),exact_route_math_work_checks=layout_comparisons,
        complete_certification_gate_passed=failed==0 and warm_failed==0,
        cpu_measurement_scope='OS printed user+system, often 0.01-second resolution; zero means below resolution; no CPU speedup inferred',
        performance_measurement=True,competitive_qualification=False,default_selected=False,cells=summaries,balanced=balanced)

def validate_directory(path):
    from prepared_automatic_layout import FILES,POLICY
    m=json.loads((path/'manifest.json').read_text());no_nonfinite(m)
    require(m['policy_path']==POLICY,'wrong policy path');commit=m['provenance']['source_commit'];hash_string(commit,40)
    require(set(m['provenance']['source_hashes'])==set(FILES),'missing source hashes')
    read=lambda name:subprocess.check_output(['git','show',f'{commit}:{name}'],cwd=ROOT)
    for name,h in m['provenance']['source_hashes'].items():hash_string(h);require(sha(read(name))==h,'source checksum: '+name)
    require(subprocess.check_output(['git','rev-parse',f'{commit}^{{tree}}'],cwd=ROOT).decode().strip()==m['provenance']['source_tree'],'wrong source tree')
    policy=json.loads(read(POLICY));require(m['policy_sha256']==sha(read(POLICY)),'policy checksum mismatch')
    require(sha((path/'uninstrumented-probe').read_bytes())==m['provenance']['binary_sha256'],'binary checksum mismatch')
    require(sha((path/'build.log').read_bytes())==m['build_log_sha256'],'build log checksum mismatch')
    raw=(path/'runs.jsonl').read_bytes();require(sha(raw)==m['runs_sha256'],'journal checksum mismatch');runs=[json.loads(x) for x in raw.splitlines()]
    for index,row in enumerate(runs):
        for stream in ['stdout','stderr']:
            name=f'{index:06d}.{stream}';require(row[stream+'_path']==name,'unsafe raw stream path');require(sha((path/name).read_bytes())==row[stream+'_sha256'],'raw stream checksum mismatch')
        if row['status']=='returned':
            require(parse_output((path/row['stdout_path']).read_text())==row['probe'],'parsed probe differs from raw')
            require(resources((path/row['stderr_path']).read_text(),m['provenance']['system'])==row['resources'],'parsed resources differ from raw')
    return validate_records(m,runs,policy)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('directory',type=Path);p.add_argument('--require-certification',action='store_true');a=p.parse_args();r=validate_directory(a.directory)
    if a.require_certification:require(r['complete_certification_gate_passed'],'incomplete layout qualification')
    print(json.dumps(r,indent=2,allow_nan=False))
