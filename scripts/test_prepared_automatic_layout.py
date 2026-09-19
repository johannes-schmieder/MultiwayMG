"""Adversarial complete-cost pairing, observer exclusion and failed-work tests."""
import copy,json,tempfile,time,unittest,sys
from pathlib import Path
from test_prepared_automatic_diagnostic import collection_fixture
from automatic_layout_protocol import SCOPES,schedule
from automatic_diagnostic_runner import run_one
from prepared_serial import ROOT
from automatic_recipes import generate
from validate_prepared_automatic_layout import validate_records

def fixture():
    m,old,_=collection_fixture()
    policy=json.loads((ROOT/'benchmarks/policies/prepared-automatic-layout-v1.json').read_text())
    policy.update(families=['uniform'],shapes=['balanced'],weights=['unit'],hard_families=['uniform'])
    policy['profiles']['smoke']['widths']=[1]
    manifest=dict(schema=1,status='complete',scope=policy['scope'],policy=policy,profile='smoke',scopes=SCOPES,
        provenance=dict(m['provenance']['reference'],build_args=policy['build_args']),collection_ns=10**9)
    rows=[]
    for i,spec in enumerate(schedule(policy,'smoke')):
        row=copy.deepcopy(next(r for r in old if r['route']==spec['route'] and r['layout']==spec['layout'] and r['build']=='reference'))
        row.pop('build');row.update(spec,index=i,start_ns=i*300,end_ns=i*300+200,process_wall_ns=200)
        rows.append(row)
    return manifest,rows,policy

def failed(row,status='launch_error'):
    row.pop('probe');row.pop('resources')
    row.update(status=status,error='preserved failure',exit_code=None if status=='launch_error' else 1,
        resource_status={'launch_error':'unavailable_after_launch_error','protocol_error':'unavailable_or_invalid_protocol','timeout':'unavailable_after_timeout'}[status])

class AutomaticLayoutTests(unittest.TestCase):
    def test_complete_pairing_and_balanced_costs(self):
        r=validate_records(*fixture())
        self.assertEqual((r['processes'],r['certified_measured_columns'],r['fixed_configuration_records']),(54,45,9))
        self.assertTrue(r['complete_certification_gate_passed']);self.assertTrue(r['performance_measurement']);self.assertFalse(r['competitive_qualification']);self.assertFalse(r['default_selected'])
        self.assertEqual(len(r['balanced']),17)
        for b in r['balanced']:
            self.assertEqual(b['subsets']['all']['single_rhs_geomean'],1.0)
            self.assertIsNone(b['subsets']['all']['repeated_rhs_geomean'])
    def test_schedule_reconstructs_every_profile_and_rotation(self):
        policy=json.loads((ROOT/'benchmarks/policies/prepared-automatic-layout-v1.json').read_text())
        for profile,n in [('smoke',15120),('development',15120),('expanded',6480)]:
            rows=list(schedule(policy,profile));self.assertEqual(len(rows),n)
            self.assertEqual(rows[0]['arm'],'automatic/scalar');self.assertEqual(rows[18]['arm'],'automatic/fine-row')
        p=copy.deepcopy(policy);p['arms'].append(p['arms'][0])
        with self.assertRaises(ValueError):list(schedule(p,'smoke'))
    def test_missing_duplicate_order_overlap_input_and_wrong_layout_rejected(self):
        for mode in ['drop','extra','order','overlap','input','layout','index']:
            args=fixture();rows=args[1]
            if mode=='drop':rows.pop()
            elif mode=='extra':rows.append(copy.deepcopy(rows[-1]))
            elif mode=='order':rows[0],rows[1]=rows[1],rows[0]
            elif mode=='overlap':rows[1].update(start_ns=0,end_ns=200)
            elif mode=='input':rows[0]['input_sha256']='f'*64
            elif mode=='layout':rows[0]['probe']['layout']='AllRow'
            else:rows[0]['index']=True
            with self.subTest(mode=mode),self.assertRaises(ValueError):validate_records(*args)
    def test_observer_and_false_certification_never_enter_ratios(self):
        for mode in ['flag','tls','kernel','regions','report','certificate','group_storage']:
            args=fixture();p=args[1][0]['probe']
            if mode=='flag':p['profiling']=True
            elif mode=='tls':p['diagnostic']['tls_bytes']=592
            elif mode=='kernel':p['diagnostic']['kernel_tls_bytes']=6960
            elif mode=='regions':p['diagnostic']['regions']={'local_solve':dict(calls=1,elapsed_ns=1)}
            elif mode=='report':p['diagnostic']['report']=dict(valid=True,elapsed_ns=1,unattributed_ns=1)
            elif mode=='certificate':p['columns'][0]['certificate']=1.0
            else:p['grouping'].update(attempts=1,completed=1,levels=1)
            with self.subTest(mode=mode),self.assertRaises(ValueError):validate_records(*args)
    def test_wrong_build_hardware_scope_and_threads_rejected(self):
        for mode in ['compiler','build','hardware','threads','scope']:
            args=fixture();m=args[0];meta=m['provenance']
            if mode=='compiler':meta['rustflags']='-C target-cpu=native'
            elif mode=='build':meta['build_args']=meta['build_args']+['--all-features']
            elif mode=='hardware':meta['hardware']=''
            elif mode=='threads':meta['thread_env']={}
            else:m['scopes']={}
            with self.subTest(mode=mode),self.assertRaises(ValueError):validate_records(*args)
    def test_changed_repeated_and_layout_math_work_rejected(self):
        args=fixture();args[1][9]['probe']['columns'][0]['fingerprint']='f'*16
        with self.assertRaises(ValueError):validate_records(*args)
        args=fixture()
        for row in args[1]:
            if row['arm']=='automatic/all-row':row['probe']['columns'][0]['fingerprint']='f'*16
        with self.assertRaises(ValueError):validate_records(*args)
    def test_failed_warmup_invalidates_only_related_whole_comparisons(self):
        args=fixture();failed(args[1][0]);r=validate_records(*args)
        self.assertEqual((r['failed_warmup_runs'],r['failed_measured_runs'],r['certified_measured_columns']),(1,0,45))
        self.assertEqual(r['failed_process_seconds'],200/1e9)
        for c in r['cells'][0]['comparisons']:
            bad='automatic/scalar' in [c['candidate'],c['control']]
            self.assertEqual(c['eligible'],not bad)
            self.assertEqual(c['paired_process_speedups'],None if bad else [1.0]*5)
        for c in r['balanced']:
            if 'automatic/scalar' in [c['candidate'],c['control']]:
                self.assertIsNone(c['subsets']['all']['equal_family_width_geomean'])
        self.assertFalse(r['complete_certification_gate_passed'])
    def test_failed_measured_and_rss_budget_costs_preserved(self):
        for mode in ['protocol_error','launch_error','rss']:
            args=fixture();row=args[1][9]
            if mode=='rss':row['resources']['peak_rss_bytes']=args[2]['process_budget_bytes']+1
            else:failed(row,mode)
            r=validate_records(*args)
            self.assertEqual(r['failed_measured_runs'],1);self.assertEqual(r['failed_process_seconds'],200/1e9)
            self.assertFalse(r['complete_certification_gate_passed'])
    def test_timeout_cost_is_required(self):
        args=fixture();row=args[1][0];failed(row,'timeout')
        with self.assertRaises(ValueError):validate_records(*args)
        delta=int(args[2]['timeout_seconds']*1e9)
        row['end_ns']+=delta;row['process_wall_ns']+=delta
        for x in args[1][1:]:x['start_ns']+=delta;x['end_ns']+=delta
        args[0]['collection_ns']+=delta
        r=validate_records(*args);self.assertEqual(r['failed_warmup_runs'],1);self.assertGreaterEqual(r['failed_process_seconds'],60)
    def test_real_isolated_timeout_and_protocol_error_preserve_streams(self):
        _,rows,policy=fixture();spec={k:rows[0][k] for k in ['case','kind','repeat','position','arm','route','layout']}
        with tempfile.TemporaryDirectory() as tmp:
            out=Path(tmp);binary=out/'probe';binary.write_text('#!/bin/sh\nsleep 1\n');binary.chmod(0o755)
            p=dict(policy,timeout_seconds=.03);origin=time.perf_counter_ns()
            x=run_one(binary,b'',p,'Darwin' if sys.platform=='darwin' else 'Linux',out,0,spec,origin,{})
            self.assertEqual(x['status'],'timeout');self.assertGreaterEqual(x['process_wall_ns'],30_000_000)
            self.assertNotIn('probe',x);self.assertTrue((out/x['stderr_path']).exists())
            binary.write_text('#!/bin/sh\nprintf malformed\n')
            x=run_one(binary,b'',policy,'Darwin' if sys.platform=='darwin' else 'Linux',out,1,spec,origin,{})
            self.assertEqual(x['status'],'protocol_error');self.assertEqual((out/x['stdout_path']).read_text(),'malformed')
            self.assertNotIn('resources',x)
    def test_expanded_input_bounds_prefixes_and_zero_lanes(self):
        policy=json.loads((ROOT/'benchmarks/policies/prepared-automatic-layout-v1.json').read_text())
        for shape in ['balanced','unbalanced']:
            case=dict(family='uniform',shape=shape,weights='heterogeneous',seed=10001,width=32)
            a,d=generate(policy,'expanded',case);b,e=generate(policy,'expanded',dict(case,width=1))
            self.assertLessEqual(d['tuples'],100000);self.assertLessEqual(max(d['counts']),4096)
            start=32+20*d['tuples'];end=start+8*d['tuples']
            self.assertEqual(a[start:end],b[start:]);self.assertEqual(d['tuples'],e['tuples'])
            for lane in [16,31]:self.assertEqual(a[start+lane*8*d['tuples']:start+(lane+1)*8*d['tuples']],bytes(8*d['tuples']))

if __name__=='__main__':unittest.main()
