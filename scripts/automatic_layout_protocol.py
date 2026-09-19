"""Frozen uninstrumented automatic-layout experiment schedule and comparisons."""
from automatic_protocol import MEMORY_SCOPES
from automatic_recipes import cases
from automatic_diagnostic_protocol import LAYOUTS
from validate_prepared_serial import require
SCOPES=dict(memory=MEMORY_SCOPES,
 timing='complete isolated uninstrumented processes; all setup, failed work, certificates and teardown charged',
 observer='profiling false; no regions; both TLS capacities zero; common inline record reported',
 pairing='same case/repeat; all warmups and measured partners must certify and meet RSS budget',
 selection='no per-cell best layout, default promotion or competitive qualification')

def widths(policy,profile):return policy['profiles'][profile]['widths']

def validate_policy(policy,profile):
    require(profile in policy['profiles'],'unknown layout profile')
    require(type(policy['workers']) is int and policy['workers']==1,'serial collection required')
    require(type(policy['warmups']) is int and policy['warmups']==1 and type(policy['repetitions']) is int and policy['repetitions']==5,'wrong paired repetition contract')
    ks=widths(policy,profile);require(ks and ks==sorted(set(ks)) and all(type(k) is int and 1<=k<=32 for k in ks),'invalid profile widths')
    arms=policy['arms'];names=[a['name'] for a in arms];require(len(names)==len(set(names)) and arms,'duplicate/empty layout arms')
    for a in arms:
        require(a['name']==a['route']+'/'+a['layout'] and a['layout'] in LAYOUTS,'invalid explicit arm')
        require(a['route'] in ['automatic','components-map','global-map'],'unknown layout route')
    comps=policy['comparisons'];pairs=[(c['candidate'],c['control']) for c in comps]
    require(pairs and len(pairs)==len(set(pairs)) and all(a!=b and a in names and b in names for a,b in pairs),'invalid prespecified comparisons')
    require(set(policy['hard_families'])<=set(policy['families']),'unknown hard family')

def schedule(policy,profile):
    validate_policy(policy,profile)
    for ci,case in enumerate(cases(dict(policy,widths=widths(policy,profile)))):
        for kind,count in [('warmup',policy['warmups']),('measured',policy['repetitions'])]:
            for repeat in range(count):
                arms=policy['arms'];shift=(ci+repeat)%len(arms)
                for position,a in enumerate(arms[shift:]+arms[:shift]):
                    yield dict(case=case,kind=kind,repeat=repeat,position=position,arm=a['name'],route=a['route'],layout=a['layout'])
