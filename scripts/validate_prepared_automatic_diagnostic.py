#!/usr/bin/env python3
"""Reconstruct and validate cold diagnostic pairs without inferring speedups."""
import argparse,json,subprocess
from pathlib import Path
from prepared_serial import ROOT,THREAD_ENV,sha,resources
from prepared_automatic_diagnostic import FILES,POLICY,SCOPES,schedule
from automatic_recipes import case_id,generate
from automatic_diagnostic_protocol import check_diagnostic,parse_output,signature
from automatic_diagnostic_runner import check_reference,check_common
from validate_prepared_serial import require,integer,finite,no_nonfinite,hash_string

def validate_records(m,runs,policy):
    no_nonfinite(m)
    require(type(policy['repetitions']) is int and policy['repetitions']==1 and type(policy['workers']) is int and policy['workers']==1,'diagnostic policy requires one serial sample')
    require(type(m['schema']) is int and m['schema']==1 and m['status']=='complete','incomplete diagnostic collection')
    require(m['policy']==policy and m['scope']==policy['scope'] and m['scopes']==SCOPES,'diagnostic policy/scope drift')
    require(m['profile'] in policy['profiles'],'unknown diagnostic profile')
    require(set(m['provenance'])==set(m['binaries'])==set(m['build_logs'])=={'reference','instrumented'},'missing build provenance')
    for build,meta in m['provenance'].items():
        require(meta['source_clean'] is True and meta['rustc'].startswith('rustc 1.85.0 '),'wrong source/compiler')
        for key,n in [('source_commit',40),('source_tree',40),('binary_sha256',64)]:hash_string(meta[key],n)
        require(meta['build_args']==policy['builds'][build] and meta['thread_env']==THREAD_ENV,'wrong build/threads')
        require(meta['rustflags']=='' and meta['compiler_overrides']=={},'undeclared compiler override')
        require(all(k in meta for k in ['source_hashes','target','cargo_config_hashes','system','uname','hardware','memory','affinity','scheduler','python','sdk_environment']),'missing provenance')
        require(meta['system'] in ['Darwin','Linux'] and meta['target'] and meta['hardware'] and meta['uname'] and meta['python'],'incomplete hardware provenance')
        a=meta['affinity'];require(a.get('status') in ['measured','unavailable'],'missing affinity')
        if a['status']=='measured':
            cpus=a.get('cpus');require(isinstance(cpus,list) and cpus and cpus==sorted(set(cpus)),'invalid allowed CPUs')
            for cpu in cpus:integer(cpu,'CPU')
        else:require(meta['system']=='Darwin' and a.get('reason'),'missing placement limitation')
    a,b=m['provenance']['reference'],m['provenance']['instrumented']
    for key in ['source_commit','source_tree','source_hashes','rustc','target','system','hardware','affinity','sdk_environment','cargo_config_hashes']:
        require(a[key]==b[key],'reference/profile provenance differs: '+key)
    expected=list(schedule(policy));require(len(runs)==len(expected),'missing/extra diagnostic attempts')
    cache={};pairs={};last_end=0;failed=0;failed_ns=0;columns=0
    for index,(row,spec) in enumerate(zip(runs,expected)):
        no_nonfinite(row);integer(row['index'],'journal index');require(row['index']==index,'wrong attempt order')
        for key,value in spec.items():require(row[key]==value,'wrong scheduled '+key)
        cid=case_id(spec['case']);require(row['case_id']==cid,'wrong case ID')
        if cid not in cache:
            data,dims=generate(policy,m['profile'],spec['case']);cache[cid]=(sha(data),dims)
        ih,dims=cache[cid];require(row['input_sha256']==ih and row['dimensions']==dims,'wrong input/dimensions')
        for key in ['start_ns','end_ns','process_wall_ns']:integer(row[key],key)
        require(row['start_ns']>=last_end and row['end_ns']-row['start_ns']==row['process_wall_ns']>0,'overlap or missing process cost');last_end=row['end_ns']
        require(row['exit_code'] is None if row['status']=='launch_error' else type(row['exit_code']) is int,'invalid exit code')
        for key in ['stdout_sha256','stderr_sha256']:hash_string(row[key])
        eligible=False
        if row['status']=='returned':
            require(row['resource_status']=='measured','missing process resources');resource=row['resources'];system=m['provenance'][row['build']]['system']
            integer(resource['peak_rss_bytes'],'RSS',1);finite(resource['user_seconds'],'user CPU');finite(resource['system_seconds'],'system CPU')
            require(resource['scope']=='isolated_process_including_startup_teardown' and resource['method']==('darwin_time_l' if system=='Darwin' else 'gnu_time_v'),'wrong resource scope/units')
            p=row['probe'];check_diagnostic(p,row['layout'],row['build']=='instrumented',row['process_wall_ns'])
            if row['build']=='reference':eligible=check_reference(row,policy)
            else:eligible=check_common(row,policy)
            eligible=eligible and resource['peak_rss_bytes']<=policy['process_budget_bytes']
            columns+=sum(c['accepted'] for c in p['columns'])
        else:
            require(row['status'] in ['launch_error','timeout','protocol_error'] and row.get('error'),'unknown failed diagnostic')
            require('probe' not in row and 'resources' not in row,'invented data after failed protocol')
            require(row['resource_status']=={'timeout':'unavailable_after_timeout','protocol_error':'unavailable_or_invalid_protocol','launch_error':'unavailable_after_launch_error'}[row['status']],'wrong failure resource scope')
            if row['status']=='timeout':require(row['process_wall_ns']>=int(policy['timeout_seconds']*1e9),'uncharged timeout')
        if not eligible:failed+=1;failed_ns+=row['process_wall_ns']
        key=(cid,row['route'],row['layout']);pairs.setdefault(key,{})[row['build']]=(row,eligible)
    integer(m['collection_ns'],'collection duration',1);require(m['collection_ns']>=last_end,'collection ends before attempts')
    cells=[];exact=0;complete=0
    for (cid,route,layout),pair in pairs.items():
        require(set(pair)=={'reference','instrumented'},'unpaired diagnostic')
        (a,ea),(b,eb)=pair['reference'],pair['instrumented'];same=False
        if a['status']==b['status']=='returned':
            require(a['exit_code']==b['exit_code'] and signature(a['probe'])==signature(b['probe']),'observer changed numerical/work/layout/payload result');same=True;exact+=1
        good=same and ea and eb;complete+=good
        # No timing ratio between binaries: observer overhead is intentionally uncalibrated.
        profile=b.get('probe',{}).get('diagnostic',{}).get('report') if good else None
        cells.append(dict(case=a['case'],case_id=cid,route=route,layout=layout,eligible=good,
            reference_index=a['index'],instrumented_index=b['index'],diagnostic=profile,
            regions=b['probe']['diagnostic']['regions'] if profile is not None else None,
            grouping=b['probe']['grouping'] if same else None,
            deterministic_record=signature(a['probe']) if same else None))
    return dict(schema=1,scope=policy['scope'],profile=m['profile'],processes=len(runs),exact_reference_profile_pairs=exact,
        eligible_pairs=complete,expected_pairs=len(pairs),certified_columns=columns,expected_columns=sum(r['case']['width'] for r in runs),
        failed_processes=failed,failed_process_seconds=failed_ns/1e9,all_process_seconds=sum(r['process_wall_ns'] for r in runs)/1e9,
        complete_certification_gate_passed=failed==0 and complete==len(pairs),performance_measurement=False,competitive_qualification=False,default_selected=False,
        timing_scope='perturbed current-thread diagnostic regions only; no speedups or cross-build CPU comparisons',cells=cells)

def validate_directory(path):
    m=json.loads((path/'manifest.json').read_text());no_nonfinite(m)
    require(set(m['provenance'])==set(m['binaries'])==set(m['build_logs'])=={'reference','instrumented'},'missing/unsafe build inventory')
    require(m['policy_path']==POLICY,'wrong policy path');commit=m['provenance']['reference']['source_commit'];hash_string(commit,40)
    read=lambda name:subprocess.check_output(['git','show',f'{commit}:{name}'],cwd=ROOT)
    policy=json.loads(read(POLICY));require(m['policy_sha256']==sha(read(POLICY)),'policy checksum mismatch')
    tree=subprocess.check_output(['git','rev-parse',f'{commit}^{{tree}}'],cwd=ROOT).decode().strip()
    for build,meta in m['provenance'].items():
        require(set(meta['source_hashes'])==set(FILES),'missing source hashes');require(meta['source_commit']==commit and meta['source_tree']==tree,'wrong source tree')
        for name,h in meta['source_hashes'].items():hash_string(h);require(sha(read(name))==h,'source checksum: '+name)
        require(m['binaries'][build]==build+'-probe','unsafe binary path');require(sha((path/m['binaries'][build]).read_bytes())==meta['binary_sha256'],'binary checksum mismatch')
        log=m['build_logs'][build];require(log['path']==build+'-build.log','unsafe build log path');require(sha((path/log['path']).read_bytes())==log['sha256'],'build log checksum mismatch')
    raw=(path/'runs.jsonl').read_bytes();require(sha(raw)==m['runs_sha256'],'journal checksum mismatch');runs=[json.loads(line) for line in raw.splitlines()]
    for index,row in enumerate(runs):
        for stream in ['stdout','stderr']:
            name=f'{index:06d}.{stream}';require(row[stream+'_path']==name,'unsafe stream path');require(sha((path/name).read_bytes())==row[stream+'_sha256'],'raw stream checksum mismatch')
        if row['status']=='returned':
            require(parse_output((path/row['stdout_path']).read_text())==row['probe'],'parsed probe differs from raw')
            require(resources((path/row['stderr_path']).read_text(),m['provenance'][row['build']]['system'])==row['resources'],'parsed resources differ from raw')
    return validate_records(m,runs,policy)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('directory',type=Path);p.add_argument('--require-certification',action='store_true');args=p.parse_args();r=validate_directory(args.directory)
    if args.require_certification:require(r['complete_certification_gate_passed'],'incomplete diagnostic qualification')
    print(json.dumps(r,indent=2,allow_nan=False))
