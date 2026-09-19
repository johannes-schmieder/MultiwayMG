"""Strict machine-readable boundary for the separate automatic/control probe."""
PHASES=['decode','fine_topology','fine_frame','output','driver','materialize']
ROUTES=['automatic','components-map','global-map','global-diagonal','global-identity']
COUNTS=['components','singleton_components','dense_components','large_components',
    'hierarchy_attempts','accepted_hierarchies','hierarchy_rejections','quality_rejections',
    'baseline_components','global_fallback_columns','rejections','structural_attempts',
    'structural_input_tuples','provisional_input_tuples','candidate_tuple_visits',
    'candidate_pair_entries','candidate_proposals','candidate_unique_candidates',
    'candidate_truncated_neighbors','candidate_accepted_pairs','screen_gramian','screen_cycle',
    'screen_energy','screen_projections','screen_defects','global_projection',
    'global_certificate_incidence','global_certificate_adjoint']
WORK=['weighted_incidence','weighted_adjoint','action','final_certificate_incidence',
    'final_certificate_adjoint','candidate_checks','candidate_vetoes','gate_projection',
    'gate_certificate_incidence','gate_certificate_adjoint']
INT_PARAMETERS=['max_iterations','local_window','maximum_transitions',
    'maximum_tuple_multiplier','maximum_coefficient_multiplier','test_vectors',
    'power_iterations','tail_iterations','screen_seed','process_budget_bytes','usize_bytes','report_slot_bytes']
FLOAT_PARAMETERS=['native_tolerance','certificate_tolerance','terminal_relative_tolerance',
    'minimum_affinity','maximum_estimated_energy_factor','maximum_observed_energy_factor',
    'maximum_structural_defect','correction_damping','relative_zero_tolerance']
STAGES=['Validation','Partition','LocalRoot','DenseTerminal','Construction','NumericalReplay',
    'Screening','ComponentSolve','Certification','GlobalFallback','Complete']
MEMORY_SCOPES={
    'requested_setup_and_checked_capacity_peak':'fine input, caller arrays and maximum admitted requested or checked actual array payload; not allocator high-water',
    'fine_capacity':'actual original topology/frame array capacities',
    'caller_capacity':'retained decoded tuples/weights/targets and output/report array capacities',
    'retained_component_forest':'none; one component numerical owner at a time',
    'exact_allocator_peak':'unavailable in timing binary; separate isolated allocation qualification',
    'logical_lengths':'input/output lengths known; opaque internal array lengths not separately exposed',
    'stack_and_runtime':'inline record size reported; fixed dense stack scratch, error strings, allocator metadata/rounding and runtime excluded from array bounds',
    'dense_dependency':'bounded requested factorization arrays admitted; internal allocation failures remain infallible',
    'old_new_generation_overlap':'not applicable; one fresh fixed-weight generation',
    'workspace_pool':'one serial reusable workspace per current component or whole-problem baseline',
    'rss':'isolated process including decode, setup, failed attempts, solves, certification, output and destruction',
    'subphase_times':'automatic construction/replay/screen/local/fallback times unavailable separately; all included in measured whole-driver span',
}

def boolean(s):
    if s not in ('true','false'): raise ValueError('invalid boolean')
    return s=='true'

def optional(s,convert): return None if s=='NA' else convert(s)

def parse_output(text):
    r=dict(parameters={},phases_ns={},counts={},columns=[],native=[])
    seen=set()
    def unique(tag):
        if tag in seen: raise ValueError('duplicate '+tag)
        seen.add(tag)
    for line in text.splitlines():
        f=line.split('\t');tag=f[0]
        if tag not in ('parameter','phase','count','column','native'): unique(tag)
        if tag=='schema' and len(f)==2: r['schema']=int(f[1])
        elif tag=='route' and len(f)==2: r['route']=f[1]
        elif tag=='profiling' and len(f)==2: r['profiling']=boolean(f[1])
        elif tag=='record_bytes' and len(f)==2: r['record_bytes']=int(f[1])
        elif tag=='parameter' and len(f)==3:
            key=f[1];unique('parameter:'+key)
            if key not in INT_PARAMETERS+FLOAT_PARAMETERS: raise ValueError('unknown parameter')
            r['parameters'][key]=(int if key in INT_PARAMETERS else float)(f[2])
        elif tag=='dimensions' and len(f)==7:
            r['dimensions']=dict(counts=list(map(int,f[1:4])),tuples=int(f[4]),coefficients=sum(map(int,f[1:4])),rhs=int(f[5]),components=int(f[6]))
        elif tag=='phase' and len(f)==3 and f[1] in PHASES:
            unique('phase:'+f[1]);r['phases_ns'][f[1]]=int(f[2])
        elif tag=='payload' and len(f)==5:
            r['payload_bytes']=dict(zip(['fine','caller','maximum_requested','maximum_admitted'],[optional(x,int) for x in f[1:]]))
        elif tag=='count' and len(f)==3 and f[1] in COUNTS:
            unique('count:'+f[1]);r['counts'][f[1]]=int(f[2])
        elif tag=='solve_work' and len(f)==11: r['solve_work']=dict(zip(WORK,map(int,f[1:])))
        elif tag=='progress' and len(f)==4:
            r['progress']=dict(stage=optional(f[1],str),component=optional(f[2],int),column=optional(f[3],int))
        elif tag=='rejection' and len(f)==4:
            r['rejection']=dict(stage=f[1],component=optional(f[2],int),error=f[3])
        elif tag=='quality' and len(f)==11:
            keys=['component','level','dimension','completed_starts','annihilated_starts',
                'maximum_estimated_energy_factor','maximum_observed_energy_factor',
                'maximum_absolute_final_rayleigh','maximum_structural_defect','accepted']
            r['quality']=dict(zip(keys,list(map(int,f[1:6]))+list(map(float,f[6:10]))+[boolean(f[10])]))
        elif tag=='column' and len(f)==7:
            unique('column:'+f[1]);r['columns'].append(dict(index=int(f[1]),accepted=boolean(f[2]),certificate=float(f[3]),initial_certificate=optional(f[4],float),global_fallback=boolean(f[5]),fingerprint=f[6]))
        elif tag=='native' and len(f)==7:
            unique('native:'+f[1]);r['native'].append(dict(index=int(f[1]),converged=boolean(f[2]),stop=f[3],iterations=int(f[4]),residual=float(f[5]),normal_residual=float(f[6])))
        elif tag=='total' and len(f)==3: r['total_ns'],r['overhead_ns']=map(int,f[1:])
        elif tag=='status' and len(f) in (2,4):
            if f[1]=='complete' and len(f)==2: r['status']='complete'
            elif f[1]=='error' and len(f)==4: r.update(status='error',error_stage=f[2],error=f[3])
            else: raise ValueError('invalid terminal status')
        else: raise ValueError('unknown/malformed automatic TSV record: '+tag)
    required={'schema','route','profiling','record_bytes','payload','solve_work','progress','total','status'}
    if not required<=seen: raise ValueError('missing automatic boundary fields')
    if set(r['parameters'])!=set(INT_PARAMETERS+FLOAT_PARAMETERS) or set(r['phases_ns'])!=set(PHASES) or set(r['counts'])!=set(COUNTS):
        raise ValueError('missing automatic parameters/phases/counts')
    return r

def signature(probe):
    return {key:value for key,value in probe.items() if key not in ('phases_ns','total_ns','overhead_ns')}

def schedule(policy):
    from automatic_recipes import cases
    for ci,case in enumerate(cases(policy)):
        for kind,count in [('warmup',policy['warmups']),('measured',policy['repetitions'])]:
            for repeat in range(count):
                shift=(ci+repeat)%len(policy['routes'])
                order=policy['routes'][shift:]+policy['routes'][:shift]
                for position,route in enumerate(order):
                    yield dict(case=case,kind=kind,repeat=repeat,position=position,route=route)
