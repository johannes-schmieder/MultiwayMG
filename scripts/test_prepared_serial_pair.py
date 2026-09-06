import copy
import json
import unittest

from prepared_serial import ROOT, sha
from prepared_serial_pair import PAIR_POLICY, PAIR_FILES, schedule
from validate_prepared_serial_pair import validate_pair
from test_prepared_serial_evidence import fixture


def paired_fixture():
    original, _, _ = fixture(gated=True)
    policy = json.loads((ROOT/PAIR_POLICY).read_text())
    original['policy_sha256'] = policy['baseline_policy_sha256']
    original['provenance'].update(source_commit=policy['baseline_source'],target='test',binary_sha256='d'*64)
    policy['baseline_binaries']={'test':'d'*64}
    for run in original['runs']:
        payload=run['probe']['payload_bytes']
        payload['total'] += 10000-payload['hierarchy_workspace'];payload['hierarchy_workspace']=10000
    candidate=copy.deepcopy(original)
    candidate['provenance'].update(source_commit='b'*40,binary_sha256='e'*64)
    for run in candidate['runs']:
        n,d=run['dimensions']['coefficients'],run['dimensions']['depth']
        reduction=32*sum(n//(1<<i) for i in range(d))+72*d+24*(d+1)
        for key in ['total','hierarchy_workspace']:run['probe']['payload_bytes'][key]-=reduction
    children=dict(baseline=original,candidate=candidate)
    hashes={key:sha(key.encode()) for key in PAIR_FILES}
    manifest=dict(schema=1,scope=policy['scope'],profile=original['profile'],policy=policy,
        source_commit='b'*40,source_hashes=hashes,trace=[],total_ns=0)
    for role,index,*_ in schedule(original['policy']):
        run=children[role]['runs'][index]
        begin=manifest['total_ns']+100
        end=begin+run['process_wall_ns']
        manifest['trace'].append(dict(role=role,index=index,begin_ns=begin,end_ns=end,
            run_sha256=sha(json.dumps(run,sort_keys=True,allow_nan=False).encode())))
        manifest['total_ns']=end
    return manifest,children,{r:dict(eligible_routes_certified=True) for r in children},policy,hashes


class PairTests(unittest.TestCase):
    def setUp(self):self.args=paired_fixture()
    def validate(self):return validate_pair(*self.args)
    def rehash(self):
        for entry in self.args[0]['trace']:
            run=self.args[1][entry['role']]['runs'][entry['index']]
            entry['run_sha256']=sha(json.dumps(run,sort_keys=True,allow_nan=False).encode())
    def test_valid_full_matrix_and_exact_capacity_reduction(self):
        result=self.validate()
        self.assertTrue(result['m5a_gate_passed'])
        self.assertFalse(result['competitive_qualification'])
        self.assertEqual(result['attempted_processes'],24)
        self.assertEqual(result['eligible_route_inner_geomean_speedup'],{'pcg':1.0,'lsmr-gated':1.0})
    def test_missing_or_reordered_actual_pairs_fail(self):
        self.args[0]['trace'].pop()
        with self.assertRaises(ValueError):self.validate()
        self.setUp();t=self.args[0]['trace'];t[0],t[1]=t[1],t[0]
        with self.assertRaises(ValueError):self.validate()
    def test_uncharged_or_overlapping_process_fails(self):
        self.args[0]['trace'][1]['begin_ns']=0
        with self.assertRaises(ValueError):self.validate()
    def test_source_policy_binary_and_runtime_mismatch_fail(self):
        for change in [lambda a:a[0].update(source_commit='f'*40),
                       lambda a:a[0].update(policy={}),
                       lambda a:a[1]['baseline']['provenance'].update(binary_sha256='f'*64),
                       lambda a:a[1]['baseline']['provenance'].update(hardware='another host')]:
            self.setUp();change(self.args)
            with self.assertRaises(ValueError):self.validate()
    def test_unpaired_inputs_and_unlinked_raw_records_fail(self):
        self.args[1]['candidate']['runs'][0]['input_sha256']='f'*64
        with self.assertRaises(ValueError):self.validate()
        self.rehash()
        with self.assertRaises(ValueError):self.validate()
    def test_changed_numerics_are_preserved_negative_gate(self):
        self.args[1]['candidate']['runs'][0]['probe']['columns'][0]['coefficient_fnv1a64']='e'*16
        self.rehash();result=self.validate()
        self.assertFalse(result['exact_numerical_and_work_equivalence'])
        self.assertFalse(result['m5a_gate_passed'])
    def test_wrong_or_hidden_memory_reduction_is_negative(self):
        self.args[1]['candidate']['runs'][0]['probe']['payload_bytes']['hierarchy_workspace']+=1
        self.rehash();self.assertFalse(self.validate()['derived_payload_reduction'])
    def test_failed_candidate_coverage_cannot_be_dropped(self):
        self.args[2]['candidate']['eligible_routes_certified']=False
        self.assertFalse(self.validate()['m5a_gate_passed'])
    def test_signed_zero_diagnostics_and_work_changes_are_detected(self):
        for field,value in [('native_residual',-0.0),('work',{})]:
            self.setUp()
            for role in ['baseline','candidate']:
                self.args[1][role]['runs'][0]['probe']['columns'][0]['native_residual']=0.0
            self.args[1]['candidate']['runs'][0]['probe']['columns'][0][field]=value
            self.rehash();self.assertFalse(self.validate()['exact_numerical_and_work_equivalence'])
    def test_failed_process_stays_in_attempts_and_blocks_aggregate_win(self):
        self.args[1]['candidate']['runs'][1]['probe']['status']='error'
        self.rehash();result=self.validate()
        self.assertFalse(result['m5a_gate_passed'])
        self.assertFalse(result['complete_paired_timing_matrix'])
        self.assertTrue(all(v is None for v in result['eligible_route_inner_geomean_speedup'].values()))


if __name__=='__main__':unittest.main()
