#!/usr/bin/env python3
"""Black-box wire/correctness tests; test fixtures are not performance evidence."""
import argparse
import copy
import json
import os
from pathlib import Path
import platform
import prepared_serial as base
from prepared_layout import POLICY,run_one
from validate_prepared_layout import check_layout_probe,compare_complete
from validate_prepared_serial import check_probe,numerical_signature

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary',type=Path);parser.add_argument('output',type=Path)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    policy=json.loads((base.ROOT/POLICY).read_text())
    policy.update(families=['uniform','chain'],weights=['unit','heterogeneous'],widths=[1,17,32],repetitions=1)
    policy['profiles']['smoke']=dict(levels=8,draws=64,depth=2)
    env=os.environ.copy();env.update(base.THREAD_ENV)
    system=platform.system();count=0;record_bytes=set()
    for layout in ['legacy']+policy['layouts']:(out/layout).mkdir()
    for case in base.cases(policy):
        data,dims=base.generate(policy,'smoke',case)
        for position,route in enumerate(policy['routes']):
            name=dict(case_id=base.case_id(case),case=case,dimensions=dims,route=route,kind='measured',repeat=0,position=position)
            old=base.run_one(args.binary.resolve(),data,policy,system,out/'legacy',name,env)
            assert old['status']=='returned' and check_probe(old['probe'],old,policy)
            reference=None
            for layout in policy['layouts']:
                r=run_one(args.binary.resolve(),data,policy,system,out/layout,name,env,layout)
                assert r['status']=='returned' and check_layout_probe(r['probe'],r,policy,'smoke',layout)
                record_bytes.add(r['probe']['layout']['record_bytes'])
                if layout=='scalar':
                    reference=r['probe']
                    a=copy.deepcopy(numerical_signature(old['probe']));b=copy.deepcopy(numerical_signature(reference))
                    assert b['payload_bytes'].pop('grouping')==0
                    assert a==b,'explicit scalar protocol changed legacy numerical/work/payload records'
                else:compare_complete(reference,r['probe'],layout)
                count+=1
    # A malformed input still emits the new failure protocol with charged decode.
    bad=out/'malformed';bad.mkdir()
    r=run_one(args.binary.resolve(),b'',policy,system,bad,name,env,'all-image')
    assert r['status']=='returned' and r['exit_code']!=0
    assert not check_layout_probe(r['probe'],r,policy,'smoke','all-image')
    # Unknown layout arguments fail before decode and cannot become valid records.
    bad=out/'unknown-layout';bad.mkdir()
    r=run_one(args.binary.resolve(),data,policy,system,bad,name,env,'auto')
    assert r['status']=='protocol_error' and r['exit_code']==2
    assert len(record_bytes)==1
    print(f'PASS {count} layout protocol comparisons; legacy numerical/work/payload identity; rejection boundaries; fixed record bytes={record_bytes.pop()}')

if __name__=='__main__':main()
