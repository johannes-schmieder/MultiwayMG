"""Adversarial automatic evidence and deterministic recipe boundaries."""
import copy
import json
from pathlib import Path
import struct
import unittest
from unittest.mock import patch
import tempfile
import time
from automatic_protocol import PHASES,COUNTS,WORK,MEMORY_SCOPES,schedule,parse_output
from automatic_recipes import FAMILIES,generate,case_id,topology
from prepared_serial import ROOT,sha,THREAD_ENV
from validate_prepared_automatic import validate_records

def fixture():
    policy=json.loads((ROOT/'benchmarks/policies/prepared-automatic-v1.json').read_text())
    policy.update(families=['uniform'],shapes=['balanced'],weights=['unit'],widths=[1,2,17])
    meta=dict(source_clean=True,rustc='rustc 1.85.0 test',source_commit='a'*40,source_tree='b'*40,
        binary_sha256='c'*64,source_hashes={},build_args=policy['build_args'],thread_env=THREAD_ENV,
        rustflags='',compiler_overrides={},target='aarch64-apple-darwin',cargo_config_hashes={},system='Darwin',
        uname={'machine':'arm64'},hardware='test CPU',memory=None,affinity={'status':'unavailable','reason':'test Mac'},
        scheduler={},python='test',sdk_environment={})
    manifest=dict(schema=1,status='complete',policy=policy,scope=policy['scope'],memory_scopes=MEMORY_SCOPES,
        profile='smoke',provenance=meta,collection_ns=10**9)
    runs=[]
    for index,spec in enumerate(schedule(policy)):
        data,dims=generate(policy,'smoke',spec['case']);e,v,k=dims['tuples'],dims['coefficients'],dims['rhs']
        counts=dict.fromkeys(COUNTS,0);glob=spec['route'].startswith('global-')
        if not glob:counts.update(components=dims['components'],dense_components=dims['components'],global_projection=k,global_certificate_incidence=k,global_certificate_adjoint=2*k)
        work=dict.fromkeys(WORK,0)
        if glob:work.update(final_certificate_incidence=k,final_certificate_adjoint=2*k)
        caller=12*e+8*(e+e*k+v*k)+32*k;fine=28*e+16*v+56*dims['components']
        p=dict(schema=1,route=spec['route'],profiling=False,record_bytes=16384,parameters=policy['parameters'],
            phases_ns=dict.fromkeys(PHASES,1),counts=counts,dimensions=dims,
            payload_bytes=dict(fine=fine,caller=caller,maximum_requested=caller+fine+1024,maximum_admitted=caller+fine+1024),
            solve_work=work,progress=dict(stage=None if glob else 'Complete',component=None,column=None),
            columns=[dict(index=j,accepted=True,certificate=0.0,initial_certificate=None if glob else 0.0,
                global_fallback=False,fingerprint='0'*16) for j in range(k)],
            native=[dict(index=j,converged=True,stop='ZeroRightHandSide' if j==16 else 'ResidualTolerance',iterations=0 if j==16 else 1,residual=0.0,normal_residual=0.0) for j in range(k)] if glob else [],
            total_ns=100,overhead_ns=94,status='complete')
        runs.append(dict(spec,index=index,case_id=case_id(spec['case']),input_sha256=sha(data),dimensions=dims,
            start_ns=300*index,end_ns=300*index+200,process_wall_ns=200,exit_code=0,
            stdout_sha256='d'*64,stderr_sha256='e'*64,status='returned',resource_status='measured',
            resources=dict(peak_rss_bytes=1<<26,user_seconds=0.0,system_seconds=0.0,method='darwin_time_l',scope='isolated_process_including_startup_teardown'),probe=p))
    return manifest,runs,policy

class AutomaticEvidenceTests(unittest.TestCase):
    def setUp(self):self.args=fixture()
    def validate(self):return validate_records(*self.args)
    def probe(self):return self.args[1][0]['probe']
    def test_complete_balanced_scope_and_rotated_order(self):
        result=self.validate();self.assertTrue(result['complete_certification_gate_passed'])
        self.assertFalse(result['competitive_qualification']);self.assertFalse(result['default_selected'])
        self.assertEqual(result['certified_measured_columns'],500)
        self.assertTrue(all(x['equal_family_width_geomean']==1.0 for x in result['balanced'].values()))
        for route in self.args[2]['routes']:
            positions=[r['position'] for r in self.args[1] if r['case']['width']==1 and r['kind']=='measured' and r['route']==route]
            self.assertEqual(sorted(positions),list(range(5)))
    def test_missing_duplicate_and_reordered_cells_fail(self):
        for mode in ['drop','duplicate','swap']:
            self.args=fixture();runs=self.args[1]
            if mode=='drop':runs.pop()
            elif mode=='duplicate':runs.append(copy.deepcopy(runs[0]))
            else:runs[0],runs[1]=runs[1],runs[0]
            with self.assertRaises(ValueError):self.validate()
    def test_input_hash_and_realized_components_fail_closed(self):
        for field in ['input_sha256','dimensions']:
            self.args=fixture()
            if field=='input_sha256':self.args[1][0][field]='f'*64
            else:self.probe()['dimensions']=dict(self.probe()['dimensions'],components=999)
            with self.assertRaises(ValueError):self.validate()
    def test_omitted_failure_cost_or_false_certificate_is_rejected(self):
        for mode in ['time','zero_phase','native_acceptance','budget','caller','fine','counts']:
            self.args=fixture();p=self.probe()
            if mode=='time':p['total_ns']+=1
            elif mode=='zero_phase':p['phases_ns']['driver']=0;p['overhead_ns']+=1
            elif mode=='native_acceptance':p['columns'][0]['certificate']=1.0
            elif mode=='budget':p['payload_bytes']['maximum_admitted']=1<<31
            elif mode=='caller':p['payload_bytes']['caller']-=32
            elif mode=='fine':p['payload_bytes']['fine']=1
            else:p['counts']['rejections']=1
            with self.assertRaises(ValueError):self.validate()
    def test_nonfinite_wrong_types_and_absent_provenance_fail(self):
        for mode in ['nan','bool','hardware','affinity','parameter']:
            self.args=fixture()
            if mode=='nan':self.probe()['columns'][0]['certificate']=float('nan')
            elif mode=='bool':self.probe()['counts']['dense_components']=True
            elif mode=='hardware':self.args[0]['provenance'].pop('hardware')
            elif mode=='affinity':self.args[0]['provenance']['affinity']={}
            else:self.probe()['parameters']=dict(self.probe()['parameters'],correction_damping=True)
            with self.assertRaises(ValueError):self.validate()
    def test_instrumentation_and_fixed_configuration_drift_fail(self):
        self.probe()['profiling']=True
        with self.assertRaises(ValueError):self.validate()
        self.args=fixture();self.args[1][5]['probe']['columns'][0]['fingerprint']='1'*16
        with self.assertRaises(ValueError):self.validate()
    def test_real_rejections_remain_in_denominator_without_speedup(self):
        for run in self.args[1]:
            if run['route']=='automatic' and run['case']['width']==1:
                run['probe']['columns'][0].update(accepted=False,certificate=1.0,initial_certificate=1.0,global_fallback=True)
                run['probe']['counts'].update(global_fallback_columns=1,rejections=2)
                run['probe']['rejection']=dict(stage='GlobalFallback',component=None,error='final original certificate rejected')
        r=self.validate();self.assertFalse(r['complete_certification_gate_passed'])
        self.assertEqual(r['failed_measured_runs'],5)
        self.assertIsNone(r['balanced']['automatic']['equal_family_width_geomean'])
    def test_resource_overrun_is_ineligible_not_free(self):
        for run in self.args[1]:
            if run['route']=='global-identity':run['resources']['peak_rss_bytes']=2<<30
        r=self.validate();self.assertEqual(r['failed_measured_runs'],15)
        self.assertIsNone(r['balanced']['global-identity']['equal_family_width_geomean'])
    def test_timeout_is_charged_and_warmup_failure_invalidates_gate(self):
        run=self.args[1][-1]
        run.pop('probe');run.pop('resources');run.update(status='timeout',error='timeout',resource_status='unavailable_after_timeout',exit_code=-9)
        run['process_wall_ns']=self.args[2]['timeout_seconds']*10**9
        run['end_ns']=run['start_ns']+run['process_wall_ns'];self.args[0]['collection_ns']=run['end_ns']
        r=self.validate();self.assertEqual(r['failed_measured_runs'],1)
        self.assertEqual(r['failed_process_seconds'],60.0)
        self.assertIsNone(r['balanced'][run['route']]['equal_family_width_geomean'])
        run['process_wall_ns']-=1;run['end_ns']-=1
        with self.assertRaises(ValueError):self.validate()
        self.args=fixture();run=self.args[1][0];run.pop('probe');run.pop('resources')
        run.update(status='protocol_error',error='bad output',resource_status='unavailable_or_invalid_protocol')
        r=self.validate();self.assertEqual(r['failed_warmup_runs'],1)
        self.assertEqual(r['failed_measured_runs'],0);self.assertFalse(r['complete_certification_gate_passed'])
        self.assertEqual(r['failed_process_seconds'],200/1e9)
    def test_rejected_quality_tail_and_work_are_required(self):
        for run in self.args[1]:
            if run['route']=='automatic':
                p=run['probe'];p['counts'].update(dense_components=0,large_components=1,hierarchy_attempts=1,
                    hierarchy_rejections=1,quality_rejections=1,rejections=1,baseline_components=1)
                p['rejection']=dict(stage='Screening',component=0,error='rejected cycle')
                p['quality']=dict(component=0,level=0,dimension=48,completed_starts=2,annihilated_starts=0,
                    maximum_estimated_energy_factor=0.9,maximum_observed_energy_factor=1.1,
                    maximum_absolute_final_rayleigh=0.5,maximum_structural_defect=0.0,accepted=False)
        self.assertTrue(self.validate()['complete_certification_gate_passed'])
        self.probe()['quality']['accepted']=True
        with self.assertRaises(ValueError):self.validate()
    def test_zero_cpu_resolution_is_retained_without_cpu_speedup(self):
        result=self.validate();record=result['cells'][0]['routes']['automatic']
        self.assertEqual(record['cpu_seconds'],[0.0]*5)
        self.assertIn('below resolution',result['cpu_measurement_scope'])
        self.assertEqual(set(record['phases_seconds_median']),set(PHASES))
    def test_launch_failure_preserves_raw_attempt_and_cost(self):
        from prepared_automatic import run_one
        from prepared_serial import sha
        spec=next(schedule(self.args[2]))
        with tempfile.TemporaryDirectory() as directory:
            out=Path(directory)
            with patch('prepared_automatic.subprocess.Popen',side_effect=OSError('fixture launch denied')):
                r=run_one(Path('/missing'),b'fixture',self.args[2],'Darwin',out,0,spec,time.perf_counter_ns(),{})
            self.assertEqual(r['status'],'launch_error');self.assertIsNone(r['exit_code'])
            self.assertGreater(r['process_wall_ns'],0);self.assertNotIn('probe',r)
            for name in ['stdout','stderr']:
                self.assertEqual(sha((out/r[name+'_path']).read_bytes()),r[name+'_sha256'])
        original=self.args[1][0]
        for name in ['probe','resources']:original.pop(name)
        original.update(status='launch_error',error='fixture launch denied',exit_code=None,resource_status='unavailable_after_launch_error')
        result=self.validate();self.assertEqual(result['failed_warmup_runs'],1)
        self.assertFalse(result['complete_certification_gate_passed'])
    def test_missing_raw_boundary_fields_fail(self):
        for text in ['', 'schema\t1\nstatus\tcomplete\n', 'schema\t1\nschema\t1\n', 'unknown\t1\n']:
            with self.assertRaises(ValueError):parse_output(text)

class AutomaticRecipeTests(unittest.TestCase):
    def setUp(self):self.policy=json.loads((ROOT/'benchmarks/policies/prepared-automatic-v1.json').read_text())
    def test_every_family_shape_support_components_and_prefixes(self):
        for family in FAMILIES:
            for shape in self.policy['shapes']:
                case=dict(family=family,shape=shape,weights='heterogeneous',seed=10001,width=32)
                full,dims=generate(self.policy,'smoke',case);e=dims['tuples'];counts=dims['counts']
                self.assertEqual(full[:8],b'MG3AUT1\0')
                keys=list(struct.iter_unpack('<III',full[32:32+12*e]))
                self.assertEqual(keys,sorted(set(keys)))
                for q,n in enumerate(counts):self.assertEqual({key[q] for key in keys},set(range(n)))
                weights=struct.unpack('<'+'d'*e,full[32+12*e:32+20*e]);self.assertTrue(all(w>0 for w in weights))
                if family=='nested':self.assertTrue(all(key[1]//(counts[1]//4)==key[2]//(counts[2]//4) for key in keys))
                if family=='tensor':self.assertTrue(all(key[2]==(key[0]+key[1])%counts[2] for key in keys))
                if family=='ragged':self.assertGreaterEqual(dims['components'],3)
                for k in [1,2,4,8,16,17,32]:
                    small,dd=generate(self.policy,'smoke',dict(case,width=k))
                    self.assertEqual(dd,dict(dims,rhs=k));self.assertEqual(small[32:],full[32:len(small)])
                for j in [16,31]:self.assertEqual(set(struct.unpack('<'+'d'*e,full[32+20*e+8*e*j:32+20*e+8*e*(j+1)])),{0.0})
    def test_unknown_family_is_rejected_and_no_holdout_seeds_declared(self):
        with self.assertRaises(ValueError):topology((16,16,16),10,'typo',10001)
        self.assertEqual(self.policy['seeds'],[10001])

if __name__=='__main__':unittest.main()
