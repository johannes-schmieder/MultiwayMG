"""Adversarial disjoint diagnostic protocol and isolated-process failure checks."""
import copy,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
from test_prepared_automatic import fixture
from automatic_diagnostic_protocol import REGIONS,GROUPS,parse_output,check_diagnostic,signature
from automatic_diagnostic_runner import run_one,check_reference

def sample():
    _,rows,policy=fixture();row=rows[0];p=row['probe']
    row.update(layout='scalar',build='instrumented');p.update(schema=2,profiling=True,layout='Scalar',grouping=dict.fromkeys(GROUPS,0))
    regions={name:dict(calls=0,elapsed_ns=0) for name in REGIONS};regions['original_certificate']['calls']=1
    p['diagnostic']=dict(report=dict(valid=True,elapsed_ns=1,unattributed_ns=1),tls_bytes=592,kernel_tls_bytes=12345,regions=regions)
    return row,policy

def wire(p):
    boolean=lambda x:str(x).lower()
    lines=[f"schema\t{p['schema']}",f"route\t{p['route']}",f"profiling\t{boolean(p['profiling'])}",f"record_bytes\t{p['record_bytes']}"]
    lines += [f'parameter\t{k}\t{v}' for k,v in p['parameters'].items()]
    d=p['dimensions'];lines.append('dimensions\t'+'\t'.join(map(str,[*d['counts'],d['tuples'],d['rhs'],d['components']])))
    lines += [f'phase\t{k}\t{v}' for k,v in p['phases_ns'].items()]
    lines.append('payload\t'+'\t'.join(map(str,p['payload_bytes'].values())))
    lines += [f'count\t{k}\t{v}' for k,v in p['counts'].items()]
    lines.append('solve_work\t'+'\t'.join(map(str,p['solve_work'].values())))
    lines.append('progress\tComplete\tNA\tNA')
    for c in p['columns']:lines.append(f"column\t{c['index']}\t{boolean(c['accepted'])}\t{c['certificate']}\t{c['initial_certificate']}\t{boolean(c['global_fallback'])}\t{c['fingerprint']}")
    lines.extend([f"total\t{p['total_ns']}\t{p['overhead_ns']}",'status\tcomplete','layout\tScalar','grouping\t0\t0\t0\t0\t0\t0'])
    d=p['diagnostic'];r=d['report'];lines.append(f"diagnostic\t{boolean(r['valid'])}\t{r['elapsed_ns']}\t{r['unattributed_ns']}\t{d['tls_bytes']}\t{d['kernel_tls_bytes']}")
    lines += [f"region\t{k}\t{v['calls']}\t{v['elapsed_ns']}" for k,v in d['regions'].items()]
    return '\n'.join(lines)+'\n'

class DiagnosticProtocolTests(unittest.TestCase):
    def test_complete_diagnostic_and_exact_reference_projection(self):
        row,policy=sample();p=row['probe']
        self.assertEqual(parse_output(wire(p)),p)
        check_diagnostic(p,'scalar',True,row['process_wall_ns'])
        ref=copy.deepcopy(row);ref['build']='reference';q=ref['probe'];q['profiling']=False;q['diagnostic']=dict(report=None,tls_bytes=0,kernel_tls_bytes=0,regions={})
        self.assertTrue(check_reference(ref,policy));self.assertEqual(signature(p),signature(q))
        q['columns'][0]['fingerprint']='1'*16;self.assertNotEqual(signature(p),signature(q))
    def test_omitted_overlap_unvisited_and_invalid_cost_fail(self):
        for mode in ['invalid','missing','extra','sum','unvisited','outer','tls','kernel_tls','bool','unknown_build']:
            row,_=sample();p=row['probe'];d=p['diagnostic']
            if mode=='invalid':d['report']['valid']=False
            elif mode=='missing':d['regions'].pop('partition')
            elif mode=='extra':d['regions']['invented']=dict(calls=0,elapsed_ns=0)
            elif mode=='sum':d['report']['elapsed_ns']+=1
            elif mode=='unvisited':d['regions']['partition']['elapsed_ns']=1
            elif mode=='outer':p['total_ns']+=1
            elif mode=='tls':d['tls_bytes']=0
            elif mode=='kernel_tls':d['kernel_tls_bytes']=0
            elif mode=='bool':d['regions']['partition']['calls']=True
            else:p['profiling']=False
            with self.subTest(mode=mode),self.assertRaises(ValueError):check_diagnostic(p,'scalar',True,row['process_wall_ns'])
    def test_grouped_failures_capacity_and_unused_images_fail(self):
        for mode in ['invented','lost','image','capacity','attempt']:
            row,_=sample();p=row['probe'];p['layout']='AllRow';g=p['grouping']
            if mode=='invented':p['grouping_rejection']=dict(component=None,scope='GlobalBaseline')
            elif mode=='lost':g.update(attempts=1,rejected=1)
            elif mode=='image':g['maximum_image_len']=1
            elif mode=='capacity':g['maximum_payload']=p['payload_bytes']['maximum_admitted']+1
            else:g.update(attempts=1,completed=1,levels=1)
            with self.subTest(mode=mode),self.assertRaises(ValueError):check_diagnostic(p,'all-row',True,row['process_wall_ns'])
    def test_wire_duplicate_unknown_missing_and_wrong_schema_fail(self):
        row,_=sample();text=wire(row['probe'])
        variants=[text+'layout\tScalar\n',text+'region\tpartition\t0\t0\n',text+'region\ttypo\t0\t0\n',
          text.replace('layout\tScalar\n',''),text.replace('schema\t2','schema\t1'),text.replace('diagnostic\ttrue','diagnostic\tmaybe')]
        for changed in variants:
            with self.assertRaises(ValueError):parse_output(changed)
    def test_reference_cannot_smuggle_profiler_state(self):
        row,policy=sample()
        with self.assertRaises(ValueError):check_reference(row,policy)
        row['probe']['profiling']=False
        with self.assertRaises(ValueError):check_reference(row,policy)
    def test_launch_failure_preserves_its_cost_and_raw_error(self):
        row,policy=sample();spec={k:row[k] for k in ['case','kind','repeat','position','route','layout','build']}
        with tempfile.TemporaryDirectory() as path,patch('automatic_diagnostic_runner.subprocess.Popen',side_effect=OSError('launch unavailable')):
            result=run_one(Path('/absent'),b'',policy,'Darwin',Path(path),0,spec,0,{})
            self.assertEqual(result['status'],'launch_error');self.assertGreater(result['process_wall_ns'],0)
            self.assertEqual((Path(path)/result['stderr_path']).read_text(),'launch unavailable')
            self.assertNotIn('probe',result);self.assertNotIn('resources',result)


# Collection tests use the existing numerical fixtures, then apply the separate
# diagnostic schedule/provenance. They never validate generated speed claims.
def collection_fixture():
    from prepared_serial import ROOT,sha
    from prepared_automatic_diagnostic import SCOPES,schedule
    from automatic_diagnostic_protocol import LAYOUTS
    from automatic_recipes import generate,case_id
    base,old,_=fixture();policy=json.loads((ROOT/'benchmarks/policies/prepared-automatic-diagnostic-v1.json').read_text())
    policy.update(families=['uniform'],shapes=['balanced'],weights=['unit'],widths=[1])
    meta=base['provenance'];metas={build:dict(meta,build_args=args) for build,args in policy['builds'].items()}
    manifest=dict(schema=1,status='complete',scope=policy['scope'],policy=policy,profile='smoke',scopes=SCOPES,
        provenance=metas,binaries={b:b+'-probe' for b in metas},build_logs={b:dict(path=b+'-build.log',sha256='d'*64) for b in metas},collection_ns=10**9)
    runs=[]
    for i,spec in enumerate(schedule(policy)):
        row=copy.deepcopy(next(r for r in old if r['case']['width']==1 and r['route']==spec['route']))
        data,dims=generate(policy,'smoke',spec['case']);row.update(spec,index=i,case_id=case_id(spec['case']),dimensions=dims,input_sha256=sha(data),start_ns=i*300,end_ns=i*300+200)
        p=row['probe'];p.update(schema=2,layout=LAYOUTS[spec['layout']],profiling=spec['build']=='instrumented',grouping=dict.fromkeys(GROUPS,0))
        regions={n:dict(calls=0,elapsed_ns=0) for n in REGIONS}
        if spec['route']=='global-map':
            regions['global_workspace']['calls']=regions['global_solve']['calls']=1
            if spec['layout']!='scalar':
                p['grouping'].update(attempts=1,completed=1,levels=1,maximum_payload=1000);regions['global_grouping']['calls']=1
        else:regions['original_certificate']['calls']=regions['direct_solve']['calls']=1
        p['diagnostic']=dict(report=dict(valid=True,elapsed_ns=1,unattributed_ns=1),tls_bytes=592,kernel_tls_bytes=12345,regions=regions) if p['profiling'] else dict(report=None,tls_bytes=0,kernel_tls_bytes=0,regions={})
        runs.append(row)
    return manifest,runs,policy

class DiagnosticCollectionTests(unittest.TestCase):
    def validate(self,args):
        from validate_prepared_automatic_diagnostic import validate_records
        return validate_records(*args)
    def test_complete_exact_pairs_without_performance_claim(self):
        r=self.validate(collection_fixture());self.assertTrue(r['complete_certification_gate_passed'])
        self.assertEqual((r['processes'],r['exact_reference_profile_pairs'],r['certified_columns']),(18,9,18))
        self.assertFalse(r['performance_measurement']);self.assertFalse(r['competitive_qualification']);self.assertFalse(r['default_selected'])
        self.assertTrue(all(c['diagnostic'] is not None for c in r['cells']))
    def test_missing_reordered_wrong_input_and_changed_observer_result_fail(self):
        for mode in ['drop','duplicate','swap','input','fingerprint','layout']:
            args=collection_fixture();rows=args[1]
            if mode=='drop':rows.pop()
            elif mode=='duplicate':rows.append(copy.deepcopy(rows[-1]))
            elif mode=='swap':rows[0],rows[1]=rows[1],rows[0]
            elif mode=='input':rows[0]['input_sha256']='f'*64
            elif mode=='fingerprint':rows[0]['probe']['columns'][0]['fingerprint']='f'*16
            else:rows[0]['layout']='typo'
            with self.subTest(mode=mode),self.assertRaises(ValueError):self.validate(args)
    def test_failed_process_stays_charged_without_profile_share(self):
        args=collection_fixture();row=args[1][0];row.pop('probe');row.pop('resources');row.update(status='launch_error',error='test launch',exit_code=None,resource_status='unavailable_after_launch_error')
        r=self.validate(args);self.assertFalse(r['complete_certification_gate_passed']);self.assertEqual(r['failed_processes'],1)
        self.assertEqual(r['failed_process_seconds'],row['process_wall_ns']/1e9);self.assertIsNone(r['cells'][0]['diagnostic'])
    def test_both_builds_require_correct_provenance_and_real_certificates(self):
        for mode in ['source','hardware','threads','compiler','scope','certificate']:
            args=collection_fixture();meta=args[0]['provenance']['instrumented']
            if mode=='source':meta['source_commit']='f'*40
            elif mode=='hardware':meta['hardware']='different CPU'
            elif mode=='threads':meta['thread_env']={}
            elif mode=='compiler':meta['rustflags']='-C target-cpu=native'
            elif mode=='scope':args[0]['scopes']={}
            else:next(r for r in args[1] if r['build']=='instrumented')['probe']['columns'][0]['certificate']=1.0
            with self.subTest(mode=mode),self.assertRaises(ValueError):self.validate(args)

class HardwareIdentityTests(unittest.TestCase):
    snapshot='Model name: Example CPU\nCPU(s): 4\nCPU(s) scaling MHz: 152%\nCPU max MHz: 2300.0000\nCPU min MHz: 800.0000\nCPU MHz: 1400.25\n'
    def identity(self,s,system='Linux'):
        from validate_prepared_automatic_diagnostic import hardware_identity
        return hardware_identity(system,s)
    def test_only_instantaneous_clocks_may_differ(self):
        changed=self.snapshot.replace('152%','156%').replace('1400.25','1700.50')
        self.assertEqual(self.identity(self.snapshot),self.identity(changed))
        self.assertIn('152%',self.snapshot) # Raw input is retained unchanged.
    def test_model_topology_limits_and_unknown_fields_remain_exact(self):
        for old,new in [('Example CPU','Other CPU'),('CPU(s): 4','CPU(s): 8'),('2300.0000','2400.0000'),('800.0000','900.0000')]:
            with self.subTest(old=old):self.assertNotEqual(self.identity(self.snapshot),self.identity(self.snapshot.replace(old,new)))
        self.assertNotEqual(self.identity(self.snapshot),self.identity(self.snapshot+'Unknown MHz: 12\n'))
    def test_clock_presence_and_platform_remain_exact(self):
        self.assertNotEqual(self.identity(self.snapshot),self.identity(self.snapshot.replace('CPU MHz: 1400.25\n','')))
        self.assertNotEqual(self.identity(self.snapshot,'Darwin'),self.identity(self.snapshot.replace('152%','156%'),'Darwin'))
    def test_malformed_and_duplicate_clock_snapshots_fail(self):
        for changed in [self.snapshot.replace('152%','NaN%'),self.snapshot.replace('152%','156'),self.snapshot.replace('1400.25','inf'),self.snapshot.replace('1400.25','-1'),self.snapshot.replace('1400.25','9'*400),self.snapshot+'CPU MHz: 1500\n']:
            with self.subTest(changed=changed),self.assertRaises(ValueError):self.identity(changed)
    def test_collection_allows_clock_change_but_rejects_cpu_change(self):
        from validate_prepared_automatic_diagnostic import validate_records
        args=collection_fixture()
        for meta in args[0]['provenance'].values():meta.update(system='Linux',hardware=self.snapshot,affinity=dict(status='measured',cpus=[0,1]))
        for row in args[1]:row['resources']['method']='gnu_time_v'
        args[0]['provenance']['instrumented']['hardware']=self.snapshot.replace('152%','156%')
        self.assertTrue(validate_records(*args)['complete_certification_gate_passed'])
        args[0]['provenance']['instrumented']['hardware']=self.snapshot.replace('Example CPU','Other CPU')
        with self.assertRaisesRegex(ValueError,'provenance differs: hardware'):validate_records(*args)

if __name__=='__main__':unittest.main()
