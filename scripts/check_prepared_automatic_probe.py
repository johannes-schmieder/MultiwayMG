#!/usr/bin/env python3
"""Actual binary framing, recipe certificates, fallback and numerical-error gates."""
import argparse
import copy
import json
import os
from pathlib import Path
import platform
import struct
import subprocess
import time
from automatic_protocol import ROUTES,parse_output
from automatic_recipes import FAMILIES,generate,case_id,component_count
from prepared_automatic import POLICY,run_one
from prepared_serial import ROOT,THREAD_ENV,dump
from validate_prepared_automatic import check_probe

def packed(counts,keys,weights,targets):
    e=len(keys);k=len(targets)//e
    data=bytearray(struct.pack('<8sIIIQI',b'MG3AUT1\0',*counts,e,k))
    for key in keys:data.extend(struct.pack('<III',*key))
    for x in [*weights,*targets]:data.extend(struct.pack('<d',x))
    return bytes(data),dict(counts=list(counts),tuples=e,coefficients=sum(counts),rhs=k,components=component_count(counts,keys))

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('binary',type=Path);parser.add_argument('output',type=Path);args=parser.parse_args()
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    binary=args.binary.resolve();policy=json.loads((ROOT/POLICY).read_text());env=os.environ.copy();env.update(THREAD_ENV)
    rows=[];origin=time.perf_counter_ns();index=0
    def check(data,dims,case,route,success=True):
        nonlocal index
        spec=dict(case=case,kind='protocol',repeat=0,position=0,route=route)
        row=run_one(binary,data,policy,platform.system(),out,index,spec,origin,env);index+=1;row['dimensions']=dims
        rows.append(row);dump(out/'attempts.json',rows)
        assert row['status']=='returned',row
        accepted=check_probe(row['probe'],row,policy)
        assert accepted==success,(case,route,row['probe'])
        return row['probe']
    for family in FAMILIES:
        for shape in policy['shapes']:
            for weights in policy['weights']:
                case=dict(family=family,shape=shape,weights=weights,seed=10001,width=1)
                data,dims=generate(policy,'smoke',case)
                for route in ROUTES:check(data,dims,case,route)
    for k in [17,32]:
        case=dict(family='uniform',shape='unbalanced',weights='heterogeneous',seed=10001,width=k)
        data,dims=generate(policy,'smoke',case)
        for route in ROUTES:check(data,dims,case,route)
    for family in ['recursive','weak-fallback']:
        if family=='recursive':
            counts=(512,2,2);keys=[(i,j,k) for i in range(512) for j in range(2) for k in range(2)];weights=[1.0]*len(keys)
        else:
            counts=(128,128,128);keys=sorted([(i,i,i) for i in range(128)]+[(i,(i+1)%128,(i+1)%128) for i in range(128)])
            weights=[1.0 if i==j else 2.0**-30 for i,j,k in keys]
        target=[((i*17)%101-50)/64.0 for i in range(len(keys))]
        data,dims=packed(counts,keys,weights,target);case=dict(family=family,shape='protocol',weights='protocol',seed=10001,width=1)
        for route in ROUTES:
            p=check(data,dims,case,route)
            if route=='automatic':
                if family=='recursive':assert p['counts']['accepted_hierarchies']==1 and p['counts']['provisional_input_tuples']>0
                else:assert p['counts']['hierarchy_rejections']==1 and p['counts']['baseline_components']==1
    keys=[(i,j,k) for i in range(2) for j in range(2) for k in range(2)]
    data,dims=packed((2,2,2),keys,[1e300]*8,[1e20]*8)
    case=dict(family='numerical-error',shape='protocol',weights='protocol',seed=10001,width=1)
    for route in ROUTES:
        p=check(data,dims,case,route,False)
        assert p['status']=='error' and p['error_stage']=='driver' and not p['columns']
    good,_=packed((2,2,2),keys,[1.0]*8,[0.0]*8)
    bad=[good[:n] for n in [0,8,31,32,47,len(good)-1]]+[good+b'X']
    for offset,value,fmt in [(8,0,'I'),(20,100001,'Q'),(28,0,'I'),(28,33,'I'),(32+12*8,float('nan'),'d'),(32+20*8,float('inf'),'d')]:
        b=bytearray(good);struct.pack_into('<'+fmt,b,offset,value);bad.append(bytes(b))
    for data in bad:
        process=subprocess.run([str(binary),'automatic'],input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,timeout=20)
        assert process.returncode!=0
        p=parse_output(process.stdout.decode());assert p['status']=='error' and not p['columns']
        (out/f'malformed-{index}.stdout').write_bytes(process.stdout);index+=1
    for arguments in [[],['typo'],['automatic','extra']]:
        process=subprocess.run([str(binary),*arguments],input=good,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,timeout=20)
        assert process.returncode!=0
        p=parse_output(process.stdout.decode());assert p['status']=='error' and not p['columns']
        (out/f'malformed-{index}.stdout').write_bytes(process.stdout);index+=1
    dump(out/'summary.json',dict(complete=True,checks=index,successful_route_calls=220,numerical_failure_calls=5,performance_measurement=False))
    print(f'PASS {index} actual automatic/control protocol checks; not comparative timing evidence')

if __name__=='__main__':main()
