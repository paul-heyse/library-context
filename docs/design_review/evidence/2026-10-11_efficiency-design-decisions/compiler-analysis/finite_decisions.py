"""Independent finite decision references. No product/service imports or protected inputs."""
import heapq
import itertools
import json
import math
import random
import struct
from collections import deque

# Ordered traversal: first witnesses, parallel edges, all stop boundaries, cycles/isolates.
def traverse(graph, seed, vertices, arcs, depth, prepared=False):
    queue = deque([seed]); depths = {seed: 0}; parents = {}; seen = []; stop = None; depth_limited = False
    while queue:
        node = queue.popleft()
        edges = graph[node] if prepared else sorted(graph[node])
        if depths[node] >= depth:
            depth_limited |= bool(edges)
            continue
        for target, site, arc in edges:
            if len(seen) == arcs:
                return seen, parents, "Arcs", depth_limited
            seen.append((node, target, site, arc))
            if target not in depths:
                if len(depths) >= vertices:
                    return seen, parents, "Vertices", depth_limited
                depths[target] = depths[node] + 1; parents[target] = (node, site, arc); queue.append(target)
    return seen, parents, stop, depth_limited
rng = random.Random(91011)
comparisons = 0
for _ in range(80):
    n = rng.randrange(1, 11); graph = {i: [] for i in range(n)}
    for arc in range(rng.randrange(60)):
        graph[rng.randrange(n)].append((rng.randrange(n), rng.randrange(7), arc))
    ordered = {node: sorted(edges) for node, edges in graph.items()}
    for seed, vertices, arcs, depth in itertools.product(range(n), [1,n], [0,1,3,70], [0,1,3]):
        assert traverse(graph,seed,vertices,arcs,depth) == traverse(ordered,seed,vertices,arcs,depth,True)
        comparisons += 1
# Selection can bound allocation but scans degree for every request; it loses to retained sorted
# adjacency for repeated seeds. It MUST request one extra edge to distinguish exhaustion vs Arcs.
hub = [(t, 0, t) for t in range(100000)]
assert heapq.nsmallest(4, reversed(hub)) == sorted(hub)[:4]
assert len(heapq.nsmallest(4, hub)) > 3

# Exact score: F32 payload projected component by component to F64; identical order.
def f32(x): return struct.unpack('f', struct.pack('f', x))[0]
def dot64(a,b):
    value = 0.0
    for x,y in zip(a,b): value += float(x)*float(y)
    return value
# Small positive terms disappear in an F32 accumulator; they survive the specified F64 kernel.
a = [f32(1.0), f32(1e-4), f32(1e-4)]; b = [f32(1.0)]*3
wide = dot64(a,b); narrow = 0.0
for x,y in zip(a,b): narrow = f32(narrow + f32(x*y))
assert wide != narrow
near_a=[f32(1),f32(2**-24)]; near_b=[f32(1),f32(2**-25)]
assert dot64(near_a,[1,1]) > dot64(near_b,[1,1])
assert f32(dot64(near_a,[1,1])) == f32(dot64(near_b,[1,1]))

# Compact coverage is total over an exact domain, not a default synthesized from missing rows.
def answer(state, domain, subject, requested):
    if subject not in domain: return 'Unknown'
    if state=='NotRequested': return 'NotRequested' if not requested else 'Invalid'
    if not requested: return 'Invalid'
    return state
states=['Complete','Partial','NotRequested','Unknown']; domain={1,2}; checks=0
for state,subject,requested in itertools.product(states,[1,3],[False,True]):
    a=answer(state,domain,subject,requested)
    # Independent expanded-row oracle: only exact in-domain entries have any status.
    rows={s: state for s in domain}
    expected='Unknown' if subject not in rows else ('Invalid' if ((state=='NotRequested')==requested or (state!='NotRequested' and not requested)) else rows[subject])
    assert a == expected
    checks += 1
assert answer('NotRequested',set(),1,False)=='Unknown'
assert answer('Complete',domain,1,True)=='Complete'
assert answer('NotRequested',domain,1,True)=='Invalid'
# Distinguish needed explicit empty unions from no consumer/no demand; exact duplicate conflicts.
def union(demand, contributions):
    if not demand: return None
    result={}
    for rows in contributions:
        for key,value in rows:
            if key in result and result[key]!=value: raise ValueError('nominal conflict')
            result[key]=value
    return sorted(result.items())
assert union(True,[[],[]])==[] and union(False,[[],[]]) is None
assert union(True,[[(1,'a')],[(1,'a')]])==[(1,'a')]
try: union(True,[[(1,'a')],[(1,'b')]])
except ValueError: pass
else: raise AssertionError('lost conflict')
results={'adjacency_reference_comparisons':comparisons,'hub_degree':len(hub),'coverage_truth_cases':checks,'scalar_f64':wide,'f32_accumulator':narrow,'near_tie_f64_distinguished':True,'explicit_empty_union':True}

# C04 independent SQL three-valued finite reference versus candidate extraction.
def sql(expr, row):
    op,*args=expr
    if op=='eq': return None if args[0] is None else row==args[0]
    if op=='in': return True if row in args[0] else (None if None in args[0] else False)
    if op=='other': return row%2==0
    left,right=(sql(arg,row) for arg in args)
    if op=='and': return False if left is False or right is False else (None if left is None or right is None else True)
    if op=='or': return True if left is True or right is True else (None if left is None or right is None else False)
    raise AssertionError(op)
def keys(expr):
    op,*args=expr
    if op=='eq': return set() if args[0] is None else {args[0]}
    if op=='in': return set(args[0])-{None}
    if op=='and':
        a,b=map(keys,args)
        return b if a is None else a if b is None else a&b
    return None
atoms=[('eq',1),('eq',None),('in',[1,3,None]),('in',[]),('other',)]
expressions=atoms+[(op,a,b) for op,a,b in itertools.product(['and','or'],atoms,atoms)]
for expr in expressions:
    demand=keys(expr); rows=list(range(5)); candidates=rows if demand is None else sorted(demand&set(rows))
    assert [r for r in candidates if sql(expr,r) is True]==[r for r in rows if sql(expr,r) is True]
assert keys(('or',('eq',1),('other',))) is None
results.update({'mandatory_demand_three_valued_cases':len(expressions),'or_keeps_residual':True,'null_positive_in_dropped':True,'cast_rule':'no general cast stripping'})

# Relation-local identity must ignore sibling configuration and producer lineage but include
# local semantic result contract, exact complete row content, outcome and completeness.
import hashlib
def relation_value(contract, outcome, domain, rows):
    return hashlib.sha256(json.dumps([contract,outcome,domain,sorted(rows)],sort_keys=True).encode()).hexdigest()
def group_request(operation, settings, dependencies):
    return hashlib.sha256(json.dumps([operation,settings,sorted(dependencies.items())],sort_keys=True).encode()).hexdigest()
config_a={'catalog': [('Catalog', 'params0')], 'retrieval': [('Retrieval', 'params1')], 'analytics': [('seeds', 4)]}
config_b={**config_a, 'analytics': [('seeds', 9)]}
for group in ['catalog','retrieval']:
    a=relation_value('cfg/'+group,'Complete','exact-all',config_a[group])
    b=relation_value('cfg/'+group,'Complete','exact-all',config_b[group])
    assert a==b
    assert group_request(group,{}, {'config':a})==group_request(group,{}, {'config':b})
assert relation_value('cfg/analytics','Complete','exact-all',config_a['analytics'])!=relation_value('cfg/analytics','Complete','exact-all',config_b['analytics'])
a=relation_value('local/v1','Complete','d',[(1,'source-id')])
for changed in [('local/v2','Complete','d',[(1,'source-id')]),('local/v1','Partial','d',[(1,'source-id')]),('local/v1','Complete','d2',[(1,'source-id')]),('local/v1','Complete','d',[(1,'new-source-id')])]:
    assert a!=relation_value(*changed)
assert relation_value('local/v1','Complete','d',[])!=relation_value('local/v1','NotRequested','d',[])
results.update({'relation_local_configuration_cutoff':True,'consumed_fields_invalidate':True,'exact_receipt_binding':'separate; not probed as production'})
print(json.dumps(results,sort_keys=True))
