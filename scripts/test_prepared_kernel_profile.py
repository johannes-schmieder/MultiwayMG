import copy
import json
import unittest

from prepared_serial import ROOT,sha,WORK,GATE_KEYS,PHASES,PAYLOAD
from prepared_kernel_profile import POLICY_PROFILE,PROFILE_FILES,PROFILE_MEMORY,PHASE_NAMES,parse_profile_output
from validate_prepared_kernel_profile import validate_profiles,check_profile,check_complete_work
from validate_prepared_serial import validate_manifest
from test_prepared_serial_evidence import fixture


def profile_fixture():
    manifest,policy,_=fixture(gated=True)
    policy['scope']='prepared_serial_kernel_profile_only'
    policy['build_args']=json.loads((ROOT/POLICY_PROFILE).read_text())['build_args']
    hashes={name:sha(name.encode()) for name in PROFILE_FILES}
    manifest.update(scope=policy['scope'],policy_path=POLICY_PROFILE,policy_sha256=hashes[POLICY_PROFILE],memory_scopes=PROFILE_MEMORY)
    manifest['provenance'].update(source_hashes=hashes,build_args=policy['build_args'])
    for run in manifest['runs']:
        c=run['probe']['columns'][0];w=c['work'];h=w['hierarchy'];d=run['dimensions']['depth'];pcg=run['route']=='pcg'
        calls=dict(incidence=w['weighted_incidence']+w['certificate_incidence'],adjoint=0,
            weighted_incidence=w['weighted_incidence'],weighted_adjoint=w['weighted_adjoint'],
            gramian=w['gramian']+2*d*h,rhs=w['rhs_adjoint']+w['certificate_adjoint'],
            projection=w['projection']+(7*d+1)*h,map_sweep=2*d*h,
            restriction=d*h,prolongation=d*h,dense_terminal=h,cycle=(d+1)*h,
            pcg_recurrence=int(pcg),prepared_pcg=int(pcg),prepared_lsmr=int(not pcg),certificate=w['certificate_incidence'])
        p=dict(index=0,valid=True,elapsed_ns=8,maximum_depth=5,phases={phase:dict(calls=calls[phase],inclusive_ns=0,exclusive_ns=0) for phase in PHASE_NAMES})
        run['profiling']=dict(metadata=dict(schema=1,thread_local_bytes=8000,record_inline_bytes=32768,report_inline_bytes=800),columns=[p],levels=[dict(index=i,tuples=run['dimensions']['tuples']//(1<<i),coefficients=run['dimensions']['coefficients']//(1<<i)) for i in range(d+1)])
    return manifest,policy,hashes


class ProfileTests(unittest.TestCase):
    def setUp(self):self.args=profile_fixture()
    def validate(self):return validate_profiles(*self.args)
    def profile(self):return self.args[0]['runs'][0]['profiling']['columns'][0]
    def test_complete_diagnostic_scope_cannot_become_normal_evidence(self):
        result=self.validate();self.assertTrue(result['diagnostic_gate_passed'])
        self.assertFalse(result['authoritative_performance_measurement']);self.assertFalse(result['competitive_qualification'])
        with self.assertRaises(ValueError):validate_manifest(*self.args)
    def test_missing_scope_phase_or_wrong_work_fails(self):
        for change in [lambda p:p.pop('columns'),lambda p:p['columns'][0]['phases'].pop('gramian'),
                       lambda p:p['columns'][0]['phases']['rhs'].update(calls=0)]:
            self.setUp();change(self.args[0]['runs'][0]['profiling'])
            with self.assertRaises(ValueError):self.validate()
    def test_leaf_or_exclusive_accounting_cannot_omit_work(self):
        self.profile()['phases']['gramian'].update(inclusive_ns=1,exclusive_ns=0)
        with self.assertRaises(ValueError):self.validate()
        self.setUp();self.profile()['phases']['gramian'].update(inclusive_ns=9,exclusive_ns=9)
        with self.assertRaises(ValueError):self.validate()
    def test_time_without_calls_and_callback_overrun_fail(self):
        self.profile()['phases']['adjoint'].update(inclusive_ns=1,exclusive_ns=1)
        with self.assertRaises(ValueError):self.validate()
        self.setUp();self.profile()['elapsed_ns']=11
        with self.assertRaises(ValueError):self.validate()
    def test_invalid_span_state_is_preserved_negative_not_a_diagnostic_win(self):
        for run in self.args[0]['runs']:run['profiling']['columns'][0]['valid']=False
        result=self.validate();self.assertFalse(result['diagnostic_gate_passed']);self.assertEqual(result['invalid_profile_reports'],12)
    def test_profiler_memory_scopes_are_required_and_bounded_by_rss(self):
        self.args[0]['runs'][0]['profiling']['metadata']['record_inline_bytes']=1
        with self.assertRaises(ValueError):self.validate()
        self.setUp();self.args[0]['runs'][0]['profiling']['metadata']['thread_local_bytes']=10**9
        with self.assertRaises(ValueError):self.validate()
    def test_nonrepeatable_counter_or_depth_is_rejected(self):
        self.args[0]['runs'][-1]['profiling']['columns'][0]['maximum_depth']=4
        with self.assertRaises(ValueError):self.validate()
    def test_missing_or_contradictory_failed_profile_is_rejected(self):
        self.args[0]['runs'][0]['profiling']['failed']=copy.deepcopy(self.profile())
        with self.assertRaises(ValueError):self.validate()
    def test_nonfinite_negative_and_boolean_counters_fail(self):
        for value in [float('nan'),-1,True]:
            self.setUp();self.profile()['phases']['gramian']['calls']=value
            with self.assertRaises(ValueError):self.validate()
    def test_complete_profile_wire_roundtrip_including_native_gate_records(self):
        for run in self.args[0]['runs'][:3]:
            probe=run['probe'];profile=run['profiling'];c=probe['columns'][0];p=profile['columns'][0]
            config=['native_tolerance','certificate_tolerance','max_iterations','local_window','pcg_recompute_interval','terminal_relative_tolerance','process_budget_bytes']
            lines=['schema\t2','route\t'+run['route'],'status\tcomplete',
                'config\t'+'\t'.join(str(probe['config'][k]) for k in config),
                'dimensions\t'+'\t'.join(str(probe['dimensions'][k]) for k in ['tuples','coefficients','rhs','depth','terminal_rank']),
                'payload\t'+'\t'.join(str(probe['payload_bytes'][k]) for k in PAYLOAD),
                'profiling_metadata\t1\t8000\t32768\t800']
            lines += ['profile_level\t'+str(x['index'])+'\t'+str(x['tuples'])+'\t'+str(x['coefficients']) for x in profile['levels']]
            lines += ['phase\t'+k+'\t'+str(probe['phases_ns'][k]) for k in PHASES]
            values=[c['index'],c['elapsed_ns'],c['prefix_ns'],str(c['accepted']).lower(),str(c['native_converged']).lower(),
                c['native_stop'],c['iterations'],c['certificate'],c['native_residual'],c['native_secondary'],
                'NA' if c['native_projection'] is None else c['native_projection'],c['coefficient_fnv1a64']]+[c['work'][k] for k in WORK]
            lines.append('column\t'+'\t'.join(map(str,values)))
            lines.append('profile\t0\ttrue\t8\t5')
            lines += ['profile_phase\t0\t'+k+'\t'+'\t'.join(str(p['phases'][k][field]) for field in ['calls','inclusive_ns','exclusive_ns']) for k in PHASE_NAMES]
            if 'gate' in c:lines.append('gate\t0\t'+'\t'.join(str(c['gate'][k]) for k in GATE_KEYS))
            lines.append('total\t100\t20')
            self.assertEqual(parse_profile_output('\n'.join(lines)),(probe,profile))

    def test_incomplete_or_impossible_level_inventory_fails(self):
        self.args[0]['runs'][0]['profiling']['levels'].pop()
        with self.assertRaises(ValueError):self.validate()
        self.setUp();self.args[0]['runs'][0]['profiling']['levels'][1]['tuples']=10**9
        with self.assertRaises(ValueError):self.validate()

    def test_parser_requires_feature_and_rejects_duplicate_or_misplaced_records(self):
        for text in ['schema\t2','profiling_metadata\t1\t10\t20\t30\nprofiling_metadata\t1\t10\t20\t30',
                     'profile\t0\ttrue\t1\t1','profile_phase\t0\tgramian\t1\t1\t1']:
            with self.assertRaises(ValueError):parse_profile_output(text)


if __name__=='__main__':unittest.main()
