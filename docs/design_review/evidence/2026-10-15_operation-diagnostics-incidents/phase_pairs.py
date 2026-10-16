import re, sys, json, collections, datetime as dt
TS = re.compile(r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+)Z\s+(\w+)\s+(.*?)lctx_phase:(.*)$')
FIELD = re.compile(r'\b([a-z_]+)=("(?:[^"\\]|\\.)*"|[^\s{}]+)')
def parse(path, start=1, end=None):
    ev=[]
    with open(path, errors='replace') as f:
        for n,line in enumerate(f,1):
            if n<start: continue
            if end and n>end: break
            if 'lctx_phase' not in line: continue
            m=TS.search(line)
            if not m: 
                ev.append(dict(line=n, bad=True)); continue
            ts=dt.datetime.fromisoformat(m.group(1)); spans=m.group(3).strip().rstrip(':'); rest=m.group(4)
            f={}
            for k,v in FIELD.findall(rest):
                f[k]=v.strip('"')
            ev.append(dict(line=n, ts=ts, level=m.group(2), spans=spans, **f))
    return ev
def summarize(path, start=1, end=None, gap=30):
    ev=parse(path,start,end)
    bad=[e for e in ev if e.get('bad')]; ev=[e for e in ev if not e.get('bad')]
    print(f'## {path} [{start}:{end}] events={len(ev)} unparsed={len(bad)}')
    if not ev: return
    kinds=collections.Counter('phase' if 'phase_id' in e else ('obs:'+e.get('observation','?')) for e in ev)
    print('kinds',dict(kinds))
    print('with invocation_id', sum(1 for e in ev if 'invocation_id' in e))
    print('first',ev[0]['ts'],'last',ev[-1]['ts'], 'line', ev[-1]['line'])
    procs=collections.Counter(e.get('process_id') for e in ev); print('procs',dict(procs))
    open_={}; done=[]
    for e in ev:
        if 'phase_id' not in e or 'status' not in e: continue
        key=(e.get('process_id'),e['phase_id'])
        if e.get('status')=='begin': open_[key]=e
        else:
            b=open_.pop(key,None); done.append((b,e))
    print('terminals',collections.Counter(e['status'] for _,e in done))
    print('unmatched terminals', sum(1 for b,_ in done if b is None))
    print('OPEN at end:', len(open_))
    for k,b in sorted(open_.items(), key=lambda x:x[1]['ts']):
        print('  open', b['ts'].time(), b['phase'], 'id',b['phase_id'], 'line',b['line'], '| spans:', b['spans'][-160:])
    agg=collections.defaultdict(lambda:[0,0.0,0.0])
    for b,e in done:
        a=agg[e['phase']]; a[0]+=1; ms=float(e.get('elapsed_ms',0)); a[1]+=ms; a[2]=max(a[2],ms)
    print('per-phase (count, inclusive_sum_s, max_s) top by sum:')
    for k,(c,s,m) in sorted(agg.items(), key=lambda x:-x[1][1])[:25]:
        print(f'  {k:45s} {c:7d} {s/1000:10.1f} {m/1000:9.1f}')
    longest=sorted(done, key=lambda x:-float(x[1].get('elapsed_ms',0)))[:12]
    print('longest single phases:')
    for b,e in longest:
        print(f"  {e['phase']:40s} id={e['phase_id']} {float(e['elapsed_ms'])/1000:8.1f}s {e['status']} end={e['ts'].time()} line={e['line']} begin_line={b['line'] if b else None}")
    print(f'gaps > {gap}s between consecutive events:')
    for a,b in zip(ev,ev[1:]):
        d=(b['ts']-a['ts']).total_seconds()
        if d>gap: print(f"  {d:7.1f}s {a['ts'].time()} (line {a['line']} {a.get('phase',a.get('observation'))} {a.get('status','')}) -> {b['ts'].time()} (line {b['line']} {b.get('phase',b.get('observation'))} {b.get('status','')})")
    # non-monotonic timestamps
    nm=sum(1 for a,b in zip(ev,ev[1:]) if b['ts']<a['ts'])
    print('non-monotonic adjacent pairs', nm)
    return ev
if __name__=='__main__':
    p=sys.argv[1]; s=int(sys.argv[2]) if len(sys.argv)>2 else 1; e=int(sys.argv[3]) if len(sys.argv)>3 and sys.argv[3]!='0' else None
    g=float(sys.argv[4]) if len(sys.argv)>4 else 30
    summarize(p,s,e,g)
