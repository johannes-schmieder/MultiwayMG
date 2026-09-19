#!/usr/bin/env python3
"""Fail-closed accounting and complete paired automatic/control evidence validation."""
import argparse
import json
import math
import statistics
from pathlib import Path
import subprocess
from automatic_protocol import PHASES,COUNTS,WORK,STAGES,MEMORY_SCOPES,parse_output,signature,schedule
from automatic_recipes import generate,case_id
from prepared_serial import ROOT,sha,resources,THREAD_ENV
from validate_prepared_serial import require,integer,finite,no_nonfinite,hash_string

def check_probe(p,run,policy):
    no_nonfinite(p)
    require(type(p['schema']) is int and p['schema']==1 and p['route']==run['route'],'wrong automatic schema/route')
    require(p['profiling'] is False,'instrumented timing binary')
    integer(p['record_bytes'],'inline record',1)
    require(p['record_bytes']<=1<<20,'implausible inline record scope')
    require(p['parameters']==policy['parameters'],'wrong numerical/construction configuration')
    require(all(type(v) is type(policy['parameters'][k]) for k,v in p['parameters'].items()),'wrong parameter types')
    require(set(p['phases_ns'])==set(PHASES),'missing/extra complete phases')
    for name,v in p['phases_ns'].items(): integer(v,'phase '+name)
    integer(p['total_ns'],'inner total',1);integer(p['overhead_ns'],'bookkeeping/drop overhead')
    require(p['total_ns']==sum(p['phases_ns'].values())+p['overhead_ns'],'omitted or overlapping phase cost')
    require(p['total_ns']<=run['process_wall_ns'],'inner time exceeds cold process time')
    dims=p.get('dimensions')
    if dims is not None:
        require(dims==run['dimensions'],'wrong realized input/components')
        for value in dims['counts']+[dims[k] for k in ['tuples','coefficients','rhs','components']]: integer(value,'dimension',1)
    require(set(p['payload_bytes'])=={'fine','caller','maximum_requested','maximum_admitted'},'wrong memory scopes')
    mem=p['payload_bytes']
    if dims is None: require(all(v is None for v in mem.values()),'invented memory after failed decode')
    else:
        for name,v in mem.items(): integer(v,'payload '+name)
        require(mem['maximum_requested']>=mem['maximum_admitted'],'inconsistent payload high-water')
        require(mem['maximum_admitted']<=policy['process_budget_bytes'],'admitted over budget')
        require(mem['maximum_admitted']>=mem['fine']+mem['caller'],'missing live root/caller payload')
    require(set(p['counts'])==set(COUNTS) and set(p['solve_work'])==set(WORK),'missing attempted work')
    for name,v in {**p['counts'],**p['solve_work']}.items(): integer(v,'work '+name)
    c=p['counts'];w=p['solve_work'];k=run['case']['width'];tol=policy['parameters']['certificate_tolerance']
    require(w['candidate_vetoes']<=w['candidate_checks'],'more vetoes than checks')
    columns=p['columns'];indices=[x['index'] for x in columns]
    require(indices==sorted(set(indices)) and all(0<=j<k for j in indices),'invalid/duplicate column indices')
    require(len(columns)<=k,'too many columns')
    for col in columns:
        integer(col['index'],'column index')
        require(set(col)=={'index','accepted','certificate','initial_certificate','global_fallback','fingerprint'},'wrong column fields')
        require(type(col['accepted']) is bool and type(col['global_fallback']) is bool,'invalid column status')
        finite(col['certificate'],'original certificate')
        require(col['accepted']==(col['certificate']<=tol),'uncertified success or inconsistent rejection')
        if col['initial_certificate'] is not None: finite(col['initial_certificate'],'initial certificate')
        hash_string(col['fingerprint'],16)
        if col['index'] in (16,31): require(col['certificate']==0.0 and col['accepted'],'declared zero column not certified')
    native=p['native'];ni=[x['index'] for x in native]
    require(ni==sorted(set(ni)) and set(ni)<=set(indices),'invalid native indices')
    for n in native:
        integer(n['index'],'native index')
        require(type(n['converged']) is bool,'invalid native status')
        require(n['stop'] in ['ZeroRightHandSide','InitialNormalEquationResidualZero','ResidualTolerance',
            'NormalEquationTolerance','WarmStartExact','FalseConvergence','MaximumIterations','Escalated'],'unknown native stop')
        integer(n['iterations'],'native iterations')
        require(n['iterations']<=policy['parameters']['max_iterations'],'iteration cap exceeded')
        finite(n['residual'],'native residual');finite(n['normal_residual'],'native normal residual')
        if n['index'] in (16,31): require(n['iterations']==0,'zero column iterated')
    if run['route'].startswith('global-'):
        require(all(v==0 for v in c.values()),'invented component routing on global baseline')
        require(p['progress']==dict(stage=None,component=None,column=None),'invented component progress')
        require(ni==indices,'missing global native diagnostics')
        require('rejection' not in p and 'quality' not in p,'invented automatic rejection on global baseline')
        require(all(not x['global_fallback'] and x['initial_certificate'] is None for x in columns),'wrong global baseline report')
    else:
        require(not native,'invented per-column native diagnostics for component scheduler')
        progress=p['progress'];require(progress['stage'] in STAGES,'unknown component progress stage')
        require(c['quality_rejections']<=c['hierarchy_rejections']<=c['hierarchy_attempts'],'inconsistent hierarchy rejection accounting')
        require(c['accepted_hierarchies']<=c['hierarchy_attempts']<=c['large_components'],'inconsistent hierarchy attempts')
        require(c['baseline_components']<=c['large_components'],'inconsistent local fallbacks')
        require(c['rejections']>=c['hierarchy_rejections'],'lost rejected attempts')
        require(c['global_fallback_columns']<=k and c['global_fallback_columns']>=sum(x['global_fallback'] for x in columns),'inconsistent global fallback count')
        if c['rejections']:
            require('rejection' in p and p['rejection']['stage'] in STAGES and p['rejection']['error'],'missing typed rejection detail')
        else: require('rejection' not in p,'invented rejected attempt')
        if c['quality_rejections']:
            require('quality' in p and p['quality']['accepted'] is False,'missing rejected quality tail')
            q=p['quality'];require(0<=q['component']<c['components'],'wrong rejected component')
            require(0<=q['annihilated_starts']<=q['completed_starts']<=policy['parameters']['test_vectors'],'invalid screen starts')
            for name in ['maximum_estimated_energy_factor','maximum_observed_energy_factor','maximum_absolute_final_rayleigh','maximum_structural_defect']: finite(q[name],name)
        else: require('quality' not in p,'invented quality rejection')
        if run['route']=='components-map':
            require(all(c[name]==0 for name in ['hierarchy_attempts','accepted_hierarchies','hierarchy_rejections','quality_rejections','structural_attempts','provisional_input_tuples','candidate_tuple_visits','screen_cycle']),'hierarchy-disabled ablation attempted construction')
    if p['status']=='complete':
        require(run['exit_code']==0 and dims is not None,'incomplete successful process')
        require(indices==list(range(k)),'missing successful batch columns')
        require(all(v>0 for v in p['phases_ns'].values()),'uncharged complete phase')
        require('error' not in p and 'error_stage' not in p,'contradictory success')
        e,v=dims['tuples'],dims['coefficients']
        required=12*e+8*(e+e*k+v*k)+policy['parameters']['report_slot_bytes']*k
        require(mem['caller']>=required,'missing decoded/caller capacity')
        require(mem['fine']>=28*e+16*v+56*dims['components'],'missing fine input owner capacity')
        require(mem['fine']+mem['caller']<=run['resources']['peak_rss_bytes'],'retained owners exceed process RSS')
        if not run['route'].startswith('global-'):
            require(p['progress']==dict(stage='Complete',component=None,column=None),'incomplete component execution')
            require(c['components']==dims['components'],'wrong original component count')
            visited=c['singleton_components']+c['dense_components']+c['large_components']
            require(visited<=c['components'],'too many component attempts')
            if not c['global_fallback_columns']: require(visited==c['components'],'missing component without final fallback')
            require(c['global_fallback_columns']==sum(x['global_fallback'] for x in columns),'missing completed fallback record')
            for col in columns:
                if not col['global_fallback']:require(col['accepted'] and col['initial_certificate']==col['certificate'],'lost rejected initial certificate/fallback')
                if col['initial_certificate'] is not None and col['initial_certificate']>tol:require(col['global_fallback'],'omitted rejected-certificate fallback')
    else:
        require(p['status']=='error' and run['exit_code']!=0 and p.get('error'),'missing failure status')
        require(p.get('error_stage') in ['cli',*PHASES],'unknown failed phase')
        if p['error_stage']=='driver': require(p['phases_ns']['driver']>0,'omitted failed driver cost')
    return p['status']=='complete' and len(columns)==k and all(x['accepted'] for x in columns)

def geometric(values): return math.exp(statistics.fmean(math.log(x) for x in values))

def validate_records(manifest,runs,policy):
    require(type(manifest['schema']) is int and manifest['schema']==1 and manifest['status']=='complete','incomplete automatic collection')
    require(manifest['policy']==policy and manifest['scope']==policy['scope'],'policy drift')
    require(manifest['memory_scopes']==MEMORY_SCOPES,'missing or changed memory scopes')
    require(manifest['profile'] in policy['profiles'],'unknown profile')
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
    expected=list(schedule(policy));require(len(runs)==len(expected),'missing/duplicate scheduled process')
    cache={};last_end=0;seen={};cells={};failed=0;warmup_failed=0;failed_ns=0;certified=0;expected_columns=0
    for index,(run,spec) in enumerate(zip(runs,expected)):
        no_nonfinite(run);integer(run['index'],'journal index');require(run['index']==index,'wrong journal order')
        require(run['exit_code'] is None if run['status']=='launch_error' else type(run['exit_code']) is int,'invalid process exit code')
        for key,val in spec.items():require(run[key]==val,'wrong scheduled '+key)
        cid=case_id(spec['case']);require(run['case_id']==cid,'wrong case ID')
        if cid not in cache:
            data,dims=generate(policy,manifest['profile'],spec['case']);cache[cid]=(sha(data),dims)
        ih,dims=cache[cid];require(run['input_sha256']==ih and run['dimensions']==dims,'wrong input hash/dimensions')
        for key in ['start_ns','end_ns','process_wall_ns']:integer(run[key],key)
        require(run['start_ns']>=last_end and run['end_ns']-run['start_ns']==run['process_wall_ns'] and run['process_wall_ns']>0,'overlap or missing process cost')
        last_end=run['end_ns']
        for key in ['stdout_sha256','stderr_sha256']:hash_string(run[key])
        eligible=False
        if run['status']=='returned':
            require(run['resource_status']=='measured','missing returned process resources')
            resource=run['resources'];integer(resource['peak_rss_bytes'],'RSS',1)
            finite(resource['user_seconds'],'user CPU');finite(resource['system_seconds'],'system CPU')
            require(resource['scope']=='isolated_process_including_startup_teardown','wrong RSS scope')
            require(resource['method']==('darwin_time_l' if meta['system']=='Darwin' else 'gnu_time_v'),'wrong RSS units')
            eligible=check_probe(run['probe'],run,policy) and resource['peak_rss_bytes']<=policy['process_budget_bytes']
            key=(cid,run['route']);sig=signature(run['probe'])
            if key in seen:require(seen[key]==sig,'fixed-configuration numerical/work/payload result changed')
            else:seen[key]=sig
        else:
            require(run['status'] in ['timeout','protocol_error','launch_error'],'unknown failed process status')
            require(run.get('error'),'missing failed process detail')
            require('probe' not in run and 'resources' not in run,'invented successful data after protocol failure')
            require(run['resource_status']=={'timeout':'unavailable_after_timeout','protocol_error':'unavailable_or_invalid_protocol','launch_error':'unavailable_after_launch_error'}[run['status']],'wrong failed resource scope')
            if run['status']=='timeout':require(run['process_wall_ns']>=int(policy['timeout_seconds']*1e9),'uncharged timeout')
        if not eligible:failed_ns+=run['process_wall_ns']
        if run['kind']=='warmup':warmup_failed+=not eligible
        if run['kind']=='measured':
            expected_columns+=dims['rhs'];failed+=not eligible
            certified+=sum(c['accepted'] for c in run.get('probe',{}).get('columns',[]))
            cells.setdefault(cid,{}).setdefault(run['route'],[]).append((run,eligible))
    integer(manifest['collection_ns'],'collection time',1);require(manifest['collection_ns']>=last_end,'collection ends before last process')
    summaries=[];comparisons={route:[] for route in policy['routes'] if route!='global-map'}
    for cid,routes in cells.items():
        require(set(routes)==set(policy['routes']),'missing control route')
        first=routes['automatic'][0][0];case=first['case'];row=dict(case=case,case_id=cid,dimensions=first['dimensions'],routes={})
        for route,attempts in routes.items():
            require(len(attempts)==policy['repetitions'],'missing repetition')
            times=[a['process_wall_ns']/1e9 for a,_ in attempts]
            row['routes'][route]=dict(eligible=all(ok for _,ok in attempts),failures=sum(not ok for _,ok in attempts),
                process_seconds=times,process_seconds_median=statistics.median(times),
                peak_rss_bytes_max=max((a.get('resources',{}).get('peak_rss_bytes',0) for a,_ in attempts),default=0) or None,
                driver_seconds_median=statistics.median([a['probe']['phases_ns']['driver']/1e9 for a,_ in attempts]) if all('probe' in a for a,_ in attempts) else None)
            result=row['routes'][route]
            available=all('resources' in a for a,_ in attempts)
            cpu=[a['resources']['user_seconds']+a['resources']['system_seconds'] for a,_ in attempts] if available else None
            result.update(cpu_seconds=cpu,cpu_seconds_median=statistics.median(cpu) if cpu is not None else None,
                phases_seconds_median={name:statistics.median(a['probe']['phases_ns'][name]/1e9 for a,_ in attempts) for name in PHASES} if all('probe' in a for a,_ in attempts) else None)
            # Fixed-configuration counters are invariant, not timing averages.
            returned=next((a['probe'] for a,_ in attempts if 'probe' in a),None)
            result['deterministic_record']=({key:returned[key] for key in ['counts','solve_work','payload_bytes','status','progress']}
                | {key:returned[key] for key in ['rejection','quality'] if key in returned}) if returned else None
            if route!='global-map':
                controls=routes['global-map'];eligible=all(ok for _,ok in attempts+controls)
                ratios=[b['process_wall_ns']/a['process_wall_ns'] for (a,_),(b,_) in zip(attempts,controls)] if eligible else None
                row['routes'][route]['paired_speedup_vs_global_map']=ratios
                if ratios:comparisons[route].append((case,geometric(ratios)))
        summaries.append(row)
    balanced={}
    for route,values in comparisons.items():
        groups={}
        for case,ratio in values:groups.setdefault((case['family'],case['width']),[]).append(ratio)
        expected_groups=len(policy['families'])*len(policy['widths'])
        complete=len(values)==len(cells)
        balanced[route]=dict(qualified_cells=len(values),expected_cells=len(cells),complete_coverage=complete,
            equal_family_width_geomean=geometric([geometric(x) for x in groups.values()]) if complete and len(groups)==expected_groups else None,
            single_rhs_geomean=geometric([geometric(x) for (f,k),x in groups.items() if k==1]) if complete else None,
            repeated_rhs_geomean=geometric([geometric(x) for (f,k),x in groups.items() if k!=1]) if complete and len(policy['widths'])>1 else None)
    return dict(schema=1,scope=policy['scope'],profile=manifest['profile'],processes=len(runs),expected_measured_columns=expected_columns,
        certified_measured_columns=certified,failed_measured_runs=failed,failed_warmup_runs=warmup_failed,
        measured_process_seconds=sum(r['process_wall_ns'] for r in runs if r['kind']=='measured')/1e9,
        warmup_process_seconds=sum(r['process_wall_ns'] for r in runs if r['kind']=='warmup')/1e9,
        failed_process_seconds=failed_ns/1e9,
        cpu_measurement_scope='OS time user+system at printed precision (often 0.01 seconds); zero means below resolution, no CPU speedup inferred',
        complete_certification_gate_passed=failed==0 and warmup_failed==0,
        competitive_qualification=False,default_selected=False,cells=summaries,balanced=balanced)

def validate_directory(path):
    from prepared_automatic import FILES,POLICY
    manifest=json.loads((path/'manifest.json').read_text());no_nonfinite(manifest)
    commit=manifest['provenance']['source_commit'];hash_string(commit,40)
    require(manifest['policy_path']==POLICY,'wrong policy path')
    require(set(manifest['provenance']['source_hashes'])==set(FILES),'missing source hashes')
    for name,expected in manifest['provenance']['source_hashes'].items():
        hash_string(expected);data=subprocess.check_output(['git','show',f'{commit}:{name}'],cwd=ROOT)
        require(sha(data)==expected,'source hash mismatch: '+name)
    tree=subprocess.check_output(['git','rev-parse',f'{commit}^{{tree}}'],cwd=ROOT).decode().strip()
    require(tree==manifest['provenance']['source_tree'],'source tree mismatch')
    policy=json.loads(subprocess.check_output(['git','show',f'{commit}:{POLICY}'],cwd=ROOT))
    require(manifest['policy_sha256']==manifest['provenance']['source_hashes'][POLICY],'policy checksum mismatch')
    require(sha((path/'prepared_automatic_benchmark').read_bytes())==manifest['provenance']['binary_sha256'],'binary hash mismatch')
    require(sha((path/'build.log').read_bytes())==manifest['build_log_sha256'],'build log hash mismatch')
    raw=(path/'runs.jsonl').read_bytes();require(sha(raw)==manifest['runs_sha256'],'journal checksum mismatch')
    runs=[json.loads(line) for line in raw.splitlines()]
    for index,run in enumerate(runs):
        for stream in ['stdout','stderr']:
            name=f'{index:06d}.{stream}';require(run[stream+'_path']==name,'unsafe/incorrect log path')
            require(sha((path/name).read_bytes())==run[stream+'_sha256'],'raw log hash mismatch')
        if run['status']=='returned':
            require(parse_output((path/run['stdout_path']).read_text())==run['probe'],'parsed probe differs from raw output')
            require(resources((path/run['stderr_path']).read_text(),manifest['provenance']['system'])==run['resources'],'parsed resources differ from raw output')
    return validate_records(manifest,runs,policy)

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path);parser.add_argument('--require-certification',action='store_true');args=parser.parse_args()
    result=validate_directory(args.directory)
    if args.require_certification:require(result['complete_certification_gate_passed'],'incomplete measured certification')
    print(json.dumps(result,indent=2,allow_nan=False))
