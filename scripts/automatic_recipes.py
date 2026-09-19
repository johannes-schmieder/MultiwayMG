"""Deterministic development-only recipes for the complete automatic benchmark."""
import itertools
from bisect import bisect_right
import struct
from functools import lru_cache
from prepared_serial import mix

FAMILIES = ['uniform','communities','chain','pair-dominant','nested','hubs','ragged',
            'tensor','worker-firm-occupation','exporter-importer-product']
SHAPES = ['balanced','unbalanced']

def cases(policy):
    for family,shape,weights,seed,width in itertools.product(policy['families'],policy['shapes'],
            policy['weights'],policy['seeds'],policy['widths']):
        yield dict(family=family,shape=shape,weights=weights,seed=seed,width=width)

def case_id(c):
    return f"{c['family']}-{c['shape']}-{c['weights']}-s{c['seed']}-k{c['width']}"

def partition(n,components):
    sizes=[n//2,n//4]
    left=n-sum(sizes)
    sizes += [left//(components-2)+(i<left%(components-2)) for i in range(components-2)]
    offsets=[0]
    for size in sizes: offsets.append(offsets[-1]+size)
    assert offsets[-1]==n and min(sizes)>0
    return offsets

def component_count(counts,keys):
    offsets=[0,counts[0],counts[0]+counts[1]]
    parent=list(range(sum(counts)))
    def root(i):
        while parent[i]!=i:
            parent[i]=parent[parent[i]]; i=parent[i]
        return i
    for key in keys:
        nodes=[root(offsets[q]+key[q]) for q in range(3)]
        for node in nodes[1:]: parent[root(node)]=root(nodes[0])
    return len({root(i) for i in range(len(parent))})

@lru_cache(maxsize=16)
def topology(counts,draws,family,seed):
    if family not in FAMILIES: raise ValueError('unknown automatic family')
    n0,n1,n2=counts
    if min(counts)<4 or any(n%4 for n in counts): raise ValueError('recipe needs multiples of four')
    keys=set()
    blocks=None
    if family=='ragged':
        components=3 if min(counts)==4 else max(4,min(counts)//4)
        blocks=[partition(n,components) for n in counts]
        for c in range(components):
            starts=[b[c] for b in blocks];sizes=[b[c+1]-b[c] for b in blocks]
            for q in range(3):
                for i in range(sizes[q]):
                    key=starts.copy();key[q]+=i;keys.add(tuple(key))
    elif family=='tensor':
        for i in range(n0): keys.add((i,0,i%n2))
        for j in range(n1): keys.add((0,j,j%n2))
    elif family=='hubs':
        for q,n in enumerate(counts):
            for i in range(n):
                key=[0,0,0];key[q]=i;keys.add(tuple(key))
    else:
        for i in range(max(counts)):
            j=i%n1
            k=j*n2//n1 if family=='nested' else i%n2
            keys.add((i%n0,j,k));keys.add(((i-1)%n0,j,k))
    state=seed
    for _ in range(draws):
        values=[]
        for _ in range(3): state=mix(state);values.append(state)
        a,b,c=values
        if family=='uniform': key=(a%n0,b%n1,c%n2)
        elif family=='communities':
            group=(a>>32)%4
            key=tuple(group*(n//4)+x%(n//4) for x,n in zip(values,counts))
        elif family=='chain':
            anchor=a%n0
            key=(anchor,(anchor*n1//n0+b%9-4)%n1,(anchor*n2//n0+c%9-4)%n2)
        elif family=='pair-dominant':
            i,j=a%n0,b%n1
            k=(i*n2//n0) if (c>>32)%64 else c%n2
            key=(i,j,k)
        elif family=='nested':
            i,j=a%n0,b%n1
            # Rare within-block deviations retain extra exact block null modes.
            k=j*n2//n1
            if (c>>32)%64==0:
                k=(j//(n1//4))*(n2//4)+c%(n2//4)
            key=(i,j,k)
        elif family=='hubs':
            key=tuple((x%n)**3//(n*n) for x,n in zip(values,counts))
        elif family=='ragged':
            comp=bisect_right(blocks[0],a%n0)-1
            key=tuple(blocks[q][comp]+x%(blocks[q][comp+1]-blocks[q][comp]) for q,x in enumerate(values))
        elif family=='tensor':
            i,j=a%n0,b%n1
            key=(i,j,(i+j)%n2)
        elif family=='worker-firm-occupation':
            worker=a%n0
            firm=worker*n1//n0
            if (b>>32)%16==0: firm=(firm+b%5-2)%n1
            occupation=worker*n2//n0
            if (c>>32)%64==0: occupation=c%n2
            key=(worker,firm,occupation)
        else:
            exporter=a%n0;group=exporter//(n0//4)
            importer=group*(n1//4)+b%(n1//4)
            if (b>>32)%64==0: importer=b%n1
            product=(exporter+3*importer)%n2
            if (c>>32)%16==0: product=c%n2
            key=(exporter,importer,product)
        keys.add(key)
    keys=tuple(sorted(keys))
    assert all({k[q] for k in keys}==set(range(n)) for q,n in enumerate(counts))
    assert len(keys)<=100_000
    return keys,component_count(counts,keys)

def generate(policy,profile,case):
    recipe=policy['profiles'][profile]
    counts=tuple(recipe[case['shape']]);width=case['width']
    keys,components=topology(counts,recipe['draws'],case['family'],case['seed'])
    e=len(keys)
    result=bytearray(struct.pack('<8sIIIQI',b'MG3AUT1\0',*counts,e,width))
    packed=[(i<<24)|(j<<12)|k for i,j,k in keys]
    for key in keys: result.extend(struct.pack('<III',*key))
    for key,index in zip(keys,packed):
        w=1.0
        if case['weights']=='heterogeneous':
            w=2.0**(int(mix(index+case['seed'])%13)-6)
            if case['family']=='communities' and len({key[q]//(counts[q]//4) for q in range(3)})>1:
                w*=2.0**-10
        elif case['weights']!='unit': raise ValueError('unknown automatic weights')
        result.extend(struct.pack('<d',w))
    for j in range(width):
        for key,index in zip(keys,packed):
            if j in (16,31): y=0.0
            elif j%4==1:
                y=sum(((mix(case['seed']+(j+1)*0x100000000+q*4096+v)&65535)-32768)/32768.0 for q,v in enumerate(key))
            else: y=((mix(index+case['seed']+(j+1)*0x100000000)&65535)-32768)/32768.0
            result.extend(struct.pack('<d',y))
    return bytes(result),dict(counts=list(counts),tuples=e,coefficients=sum(counts),rhs=width,components=components)
