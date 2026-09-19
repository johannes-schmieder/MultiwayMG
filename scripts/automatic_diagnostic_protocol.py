"""Separate disjoint-driver diagnostics; never an authoritative speedup protocol."""
from automatic_protocol import parse_output as parse_base,signature as base_signature
from validate_prepared_serial import require,integer
REGIONS=['partition','local_root','local_frame','direct_factor','direct_solve',
 'structural_preparation','hierarchy_grouping','component_grouping','global_grouping',
 'numerical_replay','coarse_factor','screening','local_workspace','local_solve',
 'original_certificate','global_workspace','global_solve']
LAYOUTS={'scalar':'Scalar','fine-row':'FineRow','all-row':'AllRow','fine-image':'FineImage','all-image':'AllImage'}
GROUPS=['attempts','completed','rejected','levels','maximum_payload','maximum_image_len']
EXTRA={'layout','grouping','grouping_rejection','diagnostic'}

def parse_output(text):
    extra={};regions={};base=[]
    for line in text.splitlines():
        f=line.split('\t');tag=f[0]
        if tag in EXTRA:
            require(tag not in extra,'duplicate diagnostic '+tag)
            if tag=='layout' and len(f)==2:extra[tag]=f[1]
            elif tag=='grouping' and len(f)==7:extra[tag]=dict(zip(GROUPS,map(int,f[1:])))
            elif tag=='grouping_rejection' and len(f)==3:extra[tag]=dict(component=None if f[1]=='NA' else int(f[1]),scope=f[2])
            elif tag=='diagnostic' and len(f)==6:
                require(f[1] in ['true','false','NA'],'invalid diagnostic status')
                if f[1]=='NA':
                    require(f[2:4]==['NA','NA'],'invented missing diagnostic');extra[tag]=dict(report=None,tls_bytes=int(f[4]),kernel_tls_bytes=int(f[5]))
                else:extra[tag]=dict(report=dict(valid=f[1]=='true',elapsed_ns=int(f[2]),unattributed_ns=int(f[3])),tls_bytes=int(f[4]),kernel_tls_bytes=int(f[5]))
            else:raise ValueError('malformed diagnostic '+tag)
        elif tag=='region':
            require(len(f)==4 and f[1] in REGIONS and f[1] not in regions,'invalid/duplicate diagnostic region')
            regions[f[1]]=dict(calls=int(f[2]),elapsed_ns=int(f[3]))
        else:base.append(line)
    require({'layout','grouping','diagnostic'}<=extra.keys(),'missing diagnostic boundary')
    p=parse_base('\n'.join(base));require(p['schema']==2,'wrong diagnostic schema')
    extra['diagnostic']['regions']=regions
    return p|extra

def signature(p):
    """Exact reference/profile comparison: discard clocks and observer metadata only."""
    return {k:v for k,v in base_signature(p).items() if k not in ['profiling','diagnostic']}

def scalar_signature(p):
    """Semantic comparison with frozen scalar ABI; do not assert equal inline size."""
    return {k:v for k,v in base_signature(p).items() if k not in EXTRA|{'record_bytes','schema','profiling'}}

def check_diagnostic(p,layout,instrumented,process_ns):
    require(p['schema']==2 and p['profiling'] is instrumented,'wrong diagnostic build/schema')
    require(p['layout']==LAYOUTS[layout],'wrong explicit layout')
    integer(process_ns,'process time',1)
    integer(p['total_ns'],'complete inner time',1);integer(p['overhead_ns'],'overhead')
    for v in p['phases_ns'].values():integer(v,'outer phase')
    require(sum(p['phases_ns'].values())+p['overhead_ns']==p['total_ns']<=process_ns,'incomplete outer cost')
    g=p['grouping'];require(set(g)==set(GROUPS),'wrong grouping scopes')
    for value in g.values():integer(value,'group inventory')
    require(g['completed']+g['rejected']<=g['attempts'],'inconsistent group attempts')
    require(g['completed']<=g['levels']<=g['completed']*max(1,p['parameters']['maximum_transitions']),'inconsistent completed groups')
    if g['completed']:
        require(g['maximum_payload']>0,'unaccounted completed group')
    else:require(g['maximum_payload']==g['maximum_image_len']==0,'invented completed group storage')
    if layout=='scalar':require(all(v==0 for v in g.values()),'scalar route built groups')
    if g['rejected']:
        require('grouping_rejection' in p,'lost grouping rejection')
        x=p['grouping_rejection'];require(x['scope'] in ['Hierarchy','ComponentBaseline','GlobalBaseline'],'unknown grouping scope')
        if x['component'] is not None:integer(x['component'],'group component')
    else:require('grouping_rejection' not in p,'invented grouping rejection')
    if layout in ['fine-row','all-row'] or p['route']!='automatic':
        require(g['maximum_image_len']==0,'unused or forbidden Gramian image')
    if p['status']=='complete':require(g['completed']+g['rejected']==g['attempts'],'lost completed group attempt')
    if p.get('dimensions') is not None:
        require(g['maximum_image_len']<=p['dimensions']['tuples'],'oversized logical image')
        require(g['maximum_payload']<=p['payload_bytes']['maximum_admitted'],'uncharged grouping capacity')
    d=p['diagnostic'];integer(d['tls_bytes'],'automatic TLS bytes');integer(d['kernel_tls_bytes'],'kernel TLS bytes')
    require(d['kernel_tls_bytes']>0 if instrumented else d['kernel_tls_bytes']==0,'wrong kernel TLS scope')
    r=d['report'];regions=d['regions']
    if not instrumented:
        require(r is None and not regions and d['tls_bytes']==0,'observer present in reference build')
    elif r is None:
        require(p['status']=='error' and p.get('error_stage') not in ['driver','materialize','complete'],'missing attempted driver diagnostic')
        require(not regions and d['tls_bytes']>0,'wrong absent diagnostic scope')
    else:
        require(r['valid'] is True and set(regions)==set(REGIONS),'invalid/incomplete disjoint regions')
        integer(d['tls_bytes'],'active TLS',1);integer(r['elapsed_ns'],'driver profile',1);integer(r['unattributed_ns'],'unattributed driver time')
        for value in regions.values():
            integer(value['calls'],'region calls');integer(value['elapsed_ns'],'region elapsed')
            if value['calls']==0:require(value['elapsed_ns']==0,'unvisited region has elapsed time')
        require(r['elapsed_ns']==r['unattributed_ns']+sum(v['elapsed_ns'] for v in regions.values()),'overlapping or omitted diagnostic cost')
        require(r['elapsed_ns']<=p['phases_ns']['driver'],'profile exceeds complete driver span')
        calls=lambda name:regions[name]['calls']
        require(sum(calls(name) for name in ['hierarchy_grouping','component_grouping','global_grouping'])==g['attempts'],'lost grouping diagnostic attempts')
        if p['route'].startswith('global-'):
            require(all(calls(name)==0 for name in REGIONS if name not in ['global_grouping','global_workspace','global_solve']),'invented component diagnostic on global route')
        elif p['status']=='complete':
            require(calls('structural_preparation')==p['counts']['hierarchy_attempts'],'lost structural attempt')
            require(calls('screening')<=calls('coarse_factor')<=calls('numerical_replay')<=calls('structural_preparation'),'inconsistent staged diagnosis')
    return p
