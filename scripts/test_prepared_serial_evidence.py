"""Adversarial evidence tests; fabricated fixtures are never benchmark results."""
import copy
import json
from pathlib import Path
import unittest

from prepared_serial import (ROOT, POLICY, SOURCE_FILES, THREAD_ENV, PHASES, PAYLOAD, WORK,
                             MEMORY_SCOPES, cases, case_id, generate, sha, resources, parse_output)
from validate_prepared_serial import validate_manifest, unique_object


def fixture():
    policy = json.loads((ROOT / POLICY).read_text())
    policy.update(families=['uniform'], weights=['unit'], widths=[1], repetitions=2)
    hashes = {name: sha(name.encode()) for name in SOURCE_FILES}
    meta = dict(source_clean=True, source_commit='a'*40, source_tree='b'*40, binary_sha256='c'*64,
                source_hashes=hashes, rustc='rustc 1.85.0 test\nhost: test', target='test',
                rustflags='', build_args=policy['build_args'], thread_env=THREAD_ENV,
                compiler_overrides={}, cargo_config_hashes={},
                system='Linux', hardware='test CPU', uname={'system':'test'}, scheduler={'JOB_ID':None},
                affinity=dict(status='unavailable', reason='unit test fixture'))
    manifest = dict(schema=1, scope=policy['scope'], profile='smoke', policy=policy,
                    policy_sha256=hashes[POLICY], provenance=meta, memory_scopes=copy.deepcopy(MEMORY_SCOPES), runs=[])
    case = next(cases(policy))
    data, dims = generate(policy, 'smoke', case)
    config = {key: policy[key] for key in ['native_tolerance', 'certificate_tolerance', 'max_iterations',
              'local_window', 'pcg_recompute_interval', 'terminal_relative_tolerance', 'process_budget_bytes']}
    for kind, reps in [('warmup',1), ('measured',2)]:
        for repeat in range(reps):
            order = policy['routes'][repeat%2:] + policy['routes'][:repeat%2]
            for pos, route in enumerate(order):
                work = dict.fromkeys(WORK, 0)
                work.update(certificate_incidence=1, certificate_adjoint=2, hierarchy=2)
                payload = dict.fromkeys(PAYLOAD, 1000)
                payload['caller_arrays'] = 1000000
                payload['total'] = sum(payload[k] for k in PAYLOAD[:-1])
                probe = dict(schema=1, route=route, status='complete', config=config,
                             dimensions=dims | {'terminal_rank':20}, payload_bytes=payload,
                             phases_ns=dict.fromkeys(PHASES,10), total_ns=100, overhead_ns=20,
                             columns=[dict(index=0, elapsed_ns=10, prefix_ns=80, accepted=True,
                                native_converged=True, native_stop='Converged' if route=='pcg' else 'NormalEquationTolerance',
                                iterations=2, certificate=1e-10, native_residual=1e-10, native_secondary=1e-10,
                                native_projection=0.0 if route=='pcg' else None, coefficient_fnv1a64='d'*16, work=work)])
                manifest['runs'].append(dict(case_id=case_id(case), case=case, dimensions=dims,
                    route=route, kind=kind, repeat=repeat, position=pos, input_sha256=sha(data),
                    process_wall_ns=1000000, exit_code=0, raw_stdout_sha256='e'*64, raw_stderr_sha256='f'*64,
                    status='returned', resource_status='measured', probe=probe,
                    resources=dict(peak_rss_bytes=10000000, user_seconds=0.0, system_seconds=0.0,
                        scope='isolated_process_including_startup_teardown', method='gnu_time_v')))
    return manifest, policy, hashes


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.manifest, self.policy, self.hashes = fixture()

    def validate(self):
        return validate_manifest(self.manifest, self.policy, self.hashes)

    def test_complete_scope_is_never_competitive_qualification(self):
        result = self.validate()
        self.assertTrue(result['all_measured_columns_certified'])
        self.assertFalse(result['competitive_qualification'])
        self.assertEqual(result['expected_runs'], 6)

    def test_rejects_each_missing_top_level_scope(self):
        for key in self.manifest:
            with self.subTest(key=key):
                changed = copy.deepcopy(self.manifest)
                del changed[key]
                with self.assertRaises(ValueError):
                    validate_manifest(changed, self.policy, self.hashes)

    def test_missing_duplicate_and_unexpected_cells(self):
        for alter in [lambda r:r.pop(), lambda r:r.append(copy.deepcopy(r[0])),
                      lambda r:r[0].update(case_id='made-up')]:
            changed = copy.deepcopy(self.manifest)
            alter(changed['runs'])
            with self.assertRaises(ValueError):
                validate_manifest(changed, self.policy, self.hashes)

    def test_actual_rotated_order_is_checked(self):
        self.manifest['runs'].reverse()
        with self.assertRaises(ValueError): self.validate()

    def test_input_and_compiler_provenance(self):
        alterations = [lambda m:m['runs'][0].update(input_sha256='0'*64),
                       lambda m:m.update(policy_sha256='0'*64),
                       lambda m:m['provenance'].update(source_clean=False),
                       lambda m:m['provenance'].update(target='different'),
                       lambda m:m['provenance'].update(rustflags='-C target-cpu=native'),
                       lambda m:m['provenance'].update(compiler_overrides={'RUSTC':'different'}),
                       lambda m:m['runs'][0]['probe']['config'].update(local_window=0)]
        for alter in alterations:
            changed = copy.deepcopy(self.manifest); alter(changed)
            with self.assertRaises(ValueError): validate_manifest(changed, self.policy, self.hashes)

    def test_missing_memory_and_misleading_zero_are_rejected(self):
        del self.manifest['memory_scopes']['construction_peak_live_allocations']
        with self.assertRaises(ValueError): self.validate()
        self.manifest['memory_scopes'] = copy.deepcopy(MEMORY_SCOPES)
        self.manifest['memory_scopes']['construction_peak_live_allocations'] = 0
        with self.assertRaises(ValueError): self.validate()

    def test_payload_totals_callers_and_resource_units(self):
        alterations = [lambda r:r['probe']['payload_bytes'].update(total=1),
                       lambda r:r['resources'].update(peak_rss_bytes=0),
                       lambda r:r['resources'].update(method='darwin_time_l'),
                       lambda r:r['probe']['payload_bytes'].pop('outer_workspace'),
                       lambda r:r['resources'].pop('scope')]
        for alter in alterations:
            changed = copy.deepcopy(self.manifest); alter(changed['runs'][0])
            with self.assertRaises(ValueError): validate_manifest(changed, self.policy, self.hashes)

    def test_native_success_does_not_override_certificate(self):
        self.manifest['runs'][0]['probe']['columns'][0]['certificate'] = 2e-8
        with self.assertRaises(ValueError): self.validate()

    def test_valid_certificate_rejection_remains_in_denominator(self):
        for run in self.manifest['runs']:
            run['probe']['columns'][0].update(certificate=2e-8, accepted=False)
        result = self.validate()
        self.assertFalse(result['all_measured_columns_certified'])
        self.assertEqual(result['certified_columns'],0)
        self.assertEqual(result['rejected_columns'],4)
        self.assertEqual(result['measured_columns'],4)

    def test_nan_inf_negative_and_fake_booleans(self):
        for key, value in [('certificate',float('nan')), ('native_residual',float('inf')),
                           ('elapsed_ns',-1), ('iterations',True), ('accepted',1)]:
            changed = copy.deepcopy(self.manifest)
            changed['runs'][0]['probe']['columns'][0][key] = value
            with self.assertRaises(ValueError): validate_manifest(changed, self.policy, self.hashes)

    def test_complete_cost_and_prefix_accounting(self):
        for key, value in [('total_ns',99), ('overhead_ns',0), ('phases_ns',{'decode':1})]:
            changed = copy.deepcopy(self.manifest)
            changed['runs'][0]['probe'][key] = value
            with self.assertRaises(ValueError): validate_manifest(changed, self.policy, self.hashes)
        self.manifest['runs'][0]['probe']['columns'][0]['prefix_ns'] = 10
        with self.assertRaises(ValueError): self.validate()

    def test_missing_certificate_work_or_numerical_repeatability(self):
        for key, value in [('coefficient_fnv1a64','0'*16), ('iterations',3)]:
            changed = copy.deepcopy(self.manifest)
            changed['runs'][0]['probe']['columns'][0][key] = value
            with self.assertRaises(ValueError): validate_manifest(changed, self.policy, self.hashes)
        self.manifest['runs'][0]['probe']['columns'][0]['work']['certificate_incidence'] = 0
        with self.assertRaises(ValueError): self.validate()

    def test_charged_setup_failure_is_valid_negative_evidence(self):
        for run in self.manifest['runs']:
            run['exit_code'] = 1
            p = run['probe']
            p.update(status='error', error_stage='decode', error='bad input', columns=[],
                     phases_ns={key:10 if key=='decode' else 0 for key in PHASES}, overhead_ns=90)
            del p['payload_bytes']; del p['dimensions']
        self.assertEqual(self.validate()['failed_measured_runs'],4)
        self.manifest['runs'][0]['probe']['phases_ns']['decode'] = 0
        self.manifest['runs'][0]['probe']['overhead_ns'] = 100
        with self.assertRaises(ValueError): self.validate()

    def test_failed_action_cannot_be_hidden_in_overhead(self):
        for run in self.manifest['runs']:
            run['exit_code'] = 1
            p = run['probe']
            p.update(status='error', error_stage='solve_certificate_output', error='breakdown', columns=[],
                     failed_action_ns=10, failed_work=dict.fromkeys(WORK,0))
        self.assertEqual(self.validate()['failed_measured_runs'],4)
        p = self.manifest['runs'][0]['probe']
        p['failed_action_ns'] = 0; p['overhead_ns'] = 30
        with self.assertRaises(ValueError): self.validate()

    def test_timeout_preserves_cost_and_coverage_without_invented_memory(self):
        for run in self.manifest['runs']:
            run.update(status='timeout', exit_code=-9, process_wall_ns=60000000000,
                       resource_status='unavailable',resource_reason='wrapper killed')
            del run['probe']; del run['resources']
        self.assertEqual(self.validate()['failed_measured_runs'],4)
        self.manifest['runs'][0]['process_wall_ns'] = 1
        with self.assertRaises(ValueError): self.validate()

    def test_peak_over_budget_is_valid_evidence_but_not_coverage_pass(self):
        for run in self.manifest['runs']:
            run['resources']['peak_rss_bytes'] = self.policy['process_budget_bytes']+1
        self.assertFalse(self.validate()['all_measured_columns_certified'])

    def test_rejects_duplicate_json_and_malformed_tsv(self):
        with self.assertRaises(ValueError): unique_object([('x',1),('x',2)])
        for text in ['schema\t1\n', 'schema\t1\nschema\t1\nstatus\tcomplete\n',
                     'phase\tdecode\t1\nphase\tdecode\t2\nstatus\tcomplete\n',
                     'status\tcomplete\tfake\tmessage\n']:
            with self.assertRaises(ValueError): parse_output(text)

    def test_platform_peak_units_are_explicit(self):
        mac = resources('0.01 real 0.00 user 0.01 sys\n 1048576 maximum resident set size\n','Darwin')
        linux = resources('Maximum resident set size (kbytes): 1024\nUser time (seconds): 0.0\nSystem time (seconds): 0.01','Linux')
        self.assertEqual(mac['peak_rss_bytes'], linux['peak_rss_bytes'])
        with self.assertRaises(ValueError): resources('missing','Linux')

    def test_recipe_is_repeatable_and_rhs_prefixes_match(self):
        case = next(cases(self.policy))
        first, dims = generate(self.policy, 'smoke', case)
        second, _ = generate(self.policy, 'smoke', case | {'width':17})
        offset = 36 + dims['tuples'] * 20
        self.assertEqual(first[offset:], second[offset:offset+dims['tuples']*8])
        self.assertEqual(first, generate(self.policy, 'smoke', case)[0])
        self.assertEqual(sha(first), 'cf7ead4d18d3790b0a81d5e5331cf15eee918e453fb272e3295237ff53aafbf8')
        self.assertEqual(second[-dims['tuples']*8:], b'\0'*(dims['tuples']*8))


if __name__ == '__main__': unittest.main()
