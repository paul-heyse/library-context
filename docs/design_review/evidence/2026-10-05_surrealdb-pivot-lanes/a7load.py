import json
M=json.load(open('/home/paul/library-context/build/review-probes/surrealdb-native-realization/model-describe.json'))
X=json.load(open('/tmp/claude-1000/-home-paul-library-context/3307aa74-dd84-437f-93be-1d92a29d0cce/scratchpad/a7meta.json'))
F={k:set(v) for k,v in X['frontiers'].items()}
def layer(n):
    if n in F['facts']: return 'L0'
    if n in F['normalized']: return 'L1'
    if n in F['analysis']: return 'L2'
    return 'L3'
R={r['name']:r for r in M['relations']}
for n,r in R.items():
    r['layer']=layer(n); r.update(X['relations'][n])
    r['refs']=[f for f in r['fields'] if f['type']=='id']
