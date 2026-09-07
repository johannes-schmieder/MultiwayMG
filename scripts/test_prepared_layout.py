"""Fabricated adversarial fixtures are protocol tests, never benchmark evidence."""
import copy
import json
import unittest
import prepared_serial as base
from prepared_layout import POLICY, FILES, MEMORY_SCOPES, schedule, effective_policy, parse_layout_output
from validate_prepared_layout import validate_manifest_pair, inventory
from test_prepared_serial_evidence import fixture


def layout_fixture():
    old,_,_=fixture(gated=True)
    policy=json.loads((base.ROOT/POLICY).read_text())
    policy.update(families=['uniform'],weights=['unit'],widths=[1])
    policy['profiles']['smoke']['depth']=2
    hashes={name:base.sha(name.encode()) for name in FILES}
    meta=copy.deepcopy(old['provenance']);meta.update(source_hashes=hashes,build_args=policy['build_args'])
    children={layout:dict(schema=1,scope=policy['scope'],profile='smoke',policy=policy,
        layout=layout,policy_path=POLICY,policy_sha256=hashes[POLICY],provenance=meta,
        memory_scopes=copy.deepcopy(MEMORY_SCOPES),runs=[]) for layout in policy['layouts']}
    manifest=dict(schema=1,scope='prepared_layout_pairing',policy=policy,profile='smoke',
        source_commit=meta['source_commit'],source_hashes=hashes,trace=[],total_ns=1)
    for attempt,(layout,index,case,kind,repeat,position,route) in enumerate(schedule(policy)):
        run=copy.deepcopy(next(r for r in old['runs'] if r['route']==route))
        data,dims=base.generate(policy,'smoke',case)
        run.update(case_id=base.case_id(case),case=case,dimensions=dims,kind=kind,repeat=repeat,
            position=position,input_sha256=base.sha(data))
        p=run['probe'];p.update(schema=3,dimensions=dims|{'terminal_rank':6},total_ns=110)
        p['columns'][0]['prefix_ns']=90;p['phases_ns']['grouping']=10
        levels=inventory(json.dumps(policy,sort_keys=True),'smoke',case['family'],case['seed'])
        prefix=0 if layout=='scalar' else 1 if layout.startswith('fine-') else 2
        retained=64*prefix+sum(8*(v+3)+8*e for e,v in levels[:prefix])
        image=dims['tuples'] if layout.endswith('image') else 0
        payload=p['payload_bytes'];payload['caller_arrays']=12*dims['tuples']+8*(2*dims['tuples']+dims['coefficients'])
        payload['grouping']=retained;payload['hierarchy_workspace']=1000+8*image+(24 if image else 0)
        payload['total']=sum(v for k,v in payload.items() if k!='total')
        live=payload['caller_arrays']-8*dims['coefficients']+3000+64*prefix;peak=live
        for e,v in levels[:prefix]:
            array=8*(v+3)+8*e;peak=max(peak,live+array+8*(v//3));live+=array
        p['layout']=dict(name=layout,prefix=prefix,usize_bytes=8,group_descriptor_bytes=64,
            image_descriptor_bytes=24,record_bytes=32768,retained=retained,
            setup_bound=max(peak,live) if prefix else 0,image_len=image)
        p['layout_levels']=[dict(tuples=e,coefficients=v,index_bytes=4 if i<prefix else 0) for i,(e,v) in enumerate(levels)]
        children[layout]['runs'].append(run)
        manifest['trace'].append(dict(layout=layout,index=index,start_ns=attempt*2000000,end_ns=attempt*2000000+1100000))
    manifest['total_ns']=manifest['trace'][-1]['end_ns']
    return manifest,children,policy,hashes

class LayoutTests(unittest.TestCase):
    def setUp(self):self.args=layout_fixture()
    def validate(self):return validate_manifest_pair(*self.args)
    def probe(self):return self.args[1]['all-image']['runs'][0]['probe']
    def test_complete_gate_still_selects_no_default_or_competitive_claim(self):
        r=self.validate();self.assertTrue(r['complete_layout_gate_passed'])
        self.assertEqual(r['processes'],60);self.assertEqual(r['paired_processes_compared'],48)
        self.assertFalse(r['default_layout_selected']);self.assertFalse(r['competitive_qualification'])
        self.assertIn('grouping',r['layouts']['scalar']['timings'][0]['phase_seconds_median'])
    def test_rotations_balance_every_layout_position_across_five_measured_repetitions(self):
        p=self.args[2];rows=list(schedule(p));positions={k:[] for k in p['layouts']}
        for start in range(0,len(rows),5):
            block=rows[start:start+5]
            if block[0][3]=='measured' and block[0][-1]=='pcg':
                for i,row in enumerate(block):positions[row[0]].append(i)
        self.assertTrue(all(sorted(v)==list(range(5)) for v in positions.values()))
    def test_inventory_prefix_width_and_single_image_are_enforced(self):
        edits=[lambda p:p['layout'].update(prefix=1),lambda p:p['layout'].update(image_len=1),
            lambda p:p['layout_levels'].pop(),lambda p:p['layout_levels'][0].update(index_bytes=8),
            lambda p:p['layout_levels'][1].update(tuples=1)]
        for edit in edits:
            self.setUp();edit(self.probe())
            with self.assertRaises(ValueError):self.validate()
    def test_live_cursor_peak_and_grouping_bytes_cannot_disappear(self):
        for key in ['setup_bound','retained','group_descriptor_bytes','image_descriptor_bytes']:
            self.setUp();self.probe()['layout'][key]=0
            with self.assertRaises(ValueError):self.validate()
    def test_image_payload_and_unrelated_workspace_cannot_be_hidden(self):
        for key in ['hierarchy_workspace','outer_workspace','fine_topology']:
            self.setUp();p=self.probe()['payload_bytes'];p[key]+=8;p['total']+=8
            with self.assertRaises(ValueError):self.validate()
    def test_missing_nonfinite_boolean_negative_and_forged_totals_fail(self):
        for value in [float('nan'),-1,True]:
            self.setUp();self.probe()['layout']['record_bytes']=value
            with self.assertRaises(ValueError):self.validate()
        self.setUp();self.probe()['phases_ns'].pop('grouping')
        with self.assertRaises(ValueError):self.validate()
    def test_exact_work_and_coefficients_are_compared_across_layouts(self):
        self.probe()['columns'][0]['coefficient_fnv1a64']='0'*16
        with self.assertRaises(ValueError):self.validate()
    def test_missing_layout_and_mixed_source_fail(self):
        self.args[1].pop('fine-row')
        with self.assertRaises(ValueError):self.validate()
        self.setUp();self.args[1]['fine-row']=copy.deepcopy(self.args[1]['fine-row'])
        self.args[1]['fine-row']['provenance']['source_commit']='e'*40
        with self.assertRaises(ValueError):self.validate()
    def test_duplicate_or_overlapping_execution_trace_fails(self):
        for edit in [lambda t:t.reverse(),lambda t:t[1].update(start_ns=0),lambda t:t.pop()]:
            self.setUp();edit(self.args[0]['trace'])
            with self.assertRaises(ValueError):self.validate()
    def test_changed_rss_and_record_scope_fail(self):
        run=self.args[1]['all-image']['runs'][0]
        run['resources']['peak_rss_bytes']=run['probe']['payload_bytes']['total']
        with self.assertRaises(ValueError):self.validate()
    def test_rejected_columns_keep_costs_and_prevent_speedup_qualification(self):
        for child in self.args[1].values():
            for run in child['runs']:
                if run['route']=='pcg':
                    run['probe']['columns'][0].update(certificate=1.0,accepted=False)
        r=self.validate();self.assertFalse(r['complete_layout_gate_passed'])
        self.assertIsNone(r['balanced']['all-image']['pcg']['process_geomean'])
        self.assertGreater(r['paired_cells'][0]['attempted_layout_seconds'],0)
    def test_timeout_keeps_complete_attempt_cost_and_prevents_qualification(self):
        manifest,children,policy,_=self.args
        run=next(r for r in children['all-image']['runs'] if r['kind']=='measured')
        run.update(status='timeout',exit_code=-9,process_wall_ns=policy['timeout_seconds']*10**9,
            resource_status='unavailable',resource_reason='test timeout')
        run.pop('probe');run.pop('resources')
        end=0
        for trace in manifest['trace']:
            duration=children[trace['layout']]['runs'][trace['index']]['process_wall_ns']+10
            trace.update(start_ns=end,end_ns=end+duration);end+=duration
        manifest['total_ns']=end
        result=self.validate();self.assertFalse(result['complete_layout_gate_passed'])
        self.assertFalse(result['all_pairs_complete'])
        self.assertFalse(result['exact_numerical_work_fingerprint_agreement'])
        self.assertEqual(result['layouts']['all-image']['failed_measured_runs'],1)
        self.assertIsNone(result['balanced']['all-image']['pcg']['process_geomean'])
        self.assertGreaterEqual(next(r for r in result['paired_cells'] if r['layout']=='all-image' and r['route']=='pcg')['attempted_layout_seconds'],60)
    def test_record_payload_cannot_change_between_fixed_binary_runs(self):
        self.probe()['layout']['record_bytes']+=16
        with self.assertRaises(ValueError):self.validate()
    def test_old_parser_rejects_layout_extension_and_new_parser_requires_every_boundary(self):
        text='schema\t3\nlayout\tscalar\t0\t8\t64\t24\t32768\nphase\tgrouping\t0\nlayout_payload\t0\t0\t0\nstatus\terror\tdecode\ttest\n'
        parsed=parse_layout_output(text);self.assertEqual(parsed['layout']['name'],'scalar')
        with self.assertRaises(ValueError):base.parse_output(text)
        for bad in [text+text,text.replace('layout_payload\t0\t0\t0\n',''),text+'profiling_metadata\t1\t0\t0\t0\n']:
            with self.assertRaises(ValueError):parse_layout_output(bad)
    def test_expanded_width_scope_is_explicit_and_smaller_policy_is_unchanged(self):
        p=json.loads((base.ROOT/POLICY).read_text())
        self.assertEqual(effective_policy(p,'expanded')['widths'],[1])
        self.assertEqual(effective_policy(p,'development')['widths'],[1,2,4,8,16,17,32])
        self.assertEqual(p['widths'],[1,2,4,8,16,17,32])

if __name__=='__main__':unittest.main()
