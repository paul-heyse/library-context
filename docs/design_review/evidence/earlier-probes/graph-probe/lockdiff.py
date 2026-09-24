import sys, tomllib
def load(p):
    d = tomllib.load(open(p,'rb'))
    out = {}
    for pk in d['package']:
        out.setdefault(pk['name'], set()).add(pk['version'])
    return out
a, b = load(sys.argv[1]), load(sys.argv[2])
added = sorted(f"{n} {v}" for n in b for v in b[n] if v not in a.get(n, set()))
removed = sorted(f"{n} {v}" for n in a for v in a[n] if v not in b.get(n, set()) and n not in ('zz-probe',))
print("ADDED:", len(added)); [print("  +", x) for x in added]
# a 'changed' existing crate = name existed and its old version vanished
print("REMOVED/CHANGED:", len(removed)); [print("  -", x) for x in removed]
