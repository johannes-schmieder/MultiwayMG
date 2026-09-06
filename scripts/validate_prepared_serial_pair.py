#!/usr/bin/env python3
"""Validate paired M5a provenance/order, numerical identity and derived payload savings."""
import argparse
import json
import math
from pathlib import Path
import statistics

from prepared_serial import ROOT, POLICY_GATED, sha, command, cases
from prepared_serial_pair import PAIR_POLICY, PAIR_FILES, BUILD_KEYS, schedule
from validate_prepared_serial import (load, require, hash_string, integer, no_nonfinite, validate_directory)


def validate_pair(manifest, children, summaries, policy, hashes):
    no_nonfinite(manifest)
    require(manifest['schema'] == 1 and manifest['scope'] == policy['scope'], 'wrong paired scope')
    require(manifest['policy'] == policy and manifest['source_hashes'] == hashes, 'altered paired policy/source')
    hash_string(manifest['source_commit'], 40)
    require(manifest['source_commit'] == children['candidate']['provenance']['source_commit'], 'wrong candidate source')
    require(children['baseline']['provenance']['source_commit'] == policy['baseline_source'], 'wrong baseline source')
    base, candidate = children['baseline'], children['candidate']
    require(base['policy'] == candidate['policy'] and base['policy_sha256'] == candidate['policy_sha256'] == policy['baseline_policy_sha256'], 'changed input recipe')
    require(base['profile'] == candidate['profile'] == manifest['profile'], 'unpaired profile')
    require(base['provenance']['target'] == candidate['provenance']['target'] and base['provenance']['rustc'] == candidate['provenance']['rustc'], 'different compiler/target')
    require(base['provenance']['binary_sha256'] == policy['baseline_binaries'].get(base['provenance']['target']), 'wrong baseline binary')
    require({k:v for k,v in base['provenance'].items() if k not in BUILD_KEYS} ==
            {k:v for k,v in candidate['provenance'].items() if k not in BUILD_KEYS}, 'different runtime environment')
    integer(manifest['total_ns'], 'complete paired time', 1)
    expected = list(schedule(base['policy']))
    require(len(expected) == len(manifest['trace']), 'missing paired process including failures/warmups')
    last_end, attempted = 0, 0
    for entry, (role, index, case, kind, repeat, position, route) in zip(manifest['trace'], expected):
        require(set(entry) == {'role','index','begin_ns','end_ns','run_sha256'}, 'missing/extra pair trace field')
        require(entry['role'] == role and entry['index'] == index, 'wrong actual pairing order')
        run = children[role]['runs'][index]
        require((run['case'], run['kind'], run['repeat'], run['position'], run['route']) == (case,kind,repeat,position,route), 'trace/child identity mismatch')
        require(entry['run_sha256'] == sha(json.dumps(run, sort_keys=True, allow_nan=False).encode()), 'trace/child hash mismatch')
        integer(entry['begin_ns'], 'pair begin'); integer(entry['end_ns'], 'pair end', 1)
        require(last_end <= entry['begin_ns'] and entry['begin_ns']+run['process_wall_ns'] <= entry['end_ns'] <= manifest['total_ns'], 'uncharged/overlapping paired process')
        last_end = entry['end_ns']; attempted += run['process_wall_ns']
    equivalent, payload_correct, all_completed, timings = True, True, True, {}
    for a, b in zip(base['runs'], candidate['runs']):
        require(a['input_sha256'] == b['input_sha256'], 'unpaired input bytes')
        complete = all(r.get('probe',{}).get('status') == 'complete' for r in [a,b])
        if not complete:
            equivalent = payload_correct = all_completed = False
            continue  # Failure remains in coverage/denominators; no timing win is inferred.
        cols = lambda r: [{k:v for k,v in c.items() if k not in ('elapsed_ns','prefix_ns')} for c in r['probe']['columns']]
        equivalent &= json.dumps(cols(a), sort_keys=True, allow_nan=False) == json.dumps(cols(b), sort_keys=True, allow_nan=False) and a['probe']['dimensions'] == b['probe']['dimensions']
        n,d = a['dimensions']['coefficients'],a['dimensions']['depth']
        reduction = 32*sum(n//(1<<level) for level in range(d)) + 72*d + 24*(d+1)
        pa,pb = a['probe']['payload_bytes'],b['probe']['payload_bytes']
        payload_correct &= all(pa[key]-pb[key] == (reduction if key in ['hierarchy_workspace','total'] else 0) for key in pa)
        if a['kind'] == 'measured':
            key=(a['case_id'],a['route'])
            timings.setdefault(key,[]).append((a,b))
    result_timings=[]
    for (cid,route), pairs in sorted(timings.items()):
        inner=[a['probe']['total_ns']/b['probe']['total_ns'] for a,b in pairs]
        wall=[a['process_wall_ns']/b['process_wall_ns'] for a,b in pairs]
        result_timings.append(dict(case_id=cid,route=route,paired_repetitions=len(pairs),
            inner_speedup_median=statistics.median(inner),inner_speedup_min=min(inner),inner_speedup_max=max(inner),
            process_speedup_median=statistics.median(wall),process_speedup_min=min(wall),process_speedup_max=max(wall)))
    coverage = all(s['eligible_routes_certified'] for s in summaries.values())
    all_pairs = all_completed and len(result_timings) == len(base['policy']['routes'])*len(list(cases(base['policy']))) and all(t['paired_repetitions']==base['policy']['repetitions'] for t in result_timings)
    by_route={}
    for route in base['policy']['eligible_routes']:
        values=[t['inner_speedup_median'] for t in result_timings if t['route']==route]
        by_route[route]=math.exp(statistics.mean(map(math.log,values))) if all_pairs else None
    return dict(scope=policy['scope'],profile=manifest['profile'],attempted_processes=len(expected),
        paired_total_ns=manifest['total_ns'],process_total_ns=attempted,eligible_routes_certified=coverage,
        exact_numerical_and_work_equivalence=equivalent,derived_payload_reduction=payload_correct,
        m5a_gate_passed=coverage and equivalent and payload_correct and all_pairs,
        complete_paired_timing_matrix=all_pairs,eligible_route_inner_geomean_speedup=by_route,
        competitive_qualification=False,timings=result_timings)


def validate_pair_directory(directory):
    directory=Path(directory)
    manifest=load(directory/'pair.json')
    commit=manifest['source_commit'];hash_string(commit,40)
    sources={name:command(['git','show',f'{commit}:{name}']) for name in PAIR_FILES}
    hashes={name:sha(content) for name,content in sources.items()}
    require(all(sha((ROOT/name).read_bytes()) == hashes[name] for name in PAIR_FILES), 'use the measured paired validator source')
    policy=json.loads(sources[PAIR_POLICY])
    require(sha(command(['git','show',f'{commit}:{POLICY_GATED}'])) == policy['baseline_policy_sha256'], 'modified v2 policy')
    children={role:load(directory/role/'manifest.json') for role in ['baseline','candidate']}
    summaries={role:validate_directory(directory/role) for role in children}
    try: return validate_pair(manifest,children,summaries,policy,hashes)
    except (KeyError,TypeError,IndexError) as error:raise ValueError(f'malformed paired evidence: {error}') from error


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory',type=Path)
    parser.add_argument('--require-m5a',action='store_true')
    args=parser.parse_args();result=validate_pair_directory(args.directory)
    print(json.dumps(result,indent=2,allow_nan=False))
    raise SystemExit(1 if args.require_m5a and not result['m5a_gate_passed'] else 0)
