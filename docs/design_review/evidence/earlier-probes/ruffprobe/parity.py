import collections, sys
D = sys.argv[1]
ty = {}
for line in open(f"{D}/ty_uses.tsv"):
    path, s, e, name, defs, undef = line.rstrip("\n").split("\t")
    ty[(path, int(s), int(e))] = (name, set(defs.split(",")) - {""}, undef == "true")
ours = collections.defaultdict(lambda: {"name": None, "cands": set(), "kinds": set(), "builtin": False})
for line in open(f"{D}/ours.txt"):
    if not line.startswith("| ") or line.startswith("| path"):
        continue
    cells = [c.strip() for c in line.strip().strip("|").split("|")]
    path, s, e, name, bs, be, bk, bn = cells
    r = ours[(path, int(s), int(e))]
    r["name"] = name
    if bs:
        r["cands"].add(f"{bs}:{be}"); r["kinds"].add(bk)
    if bn:
        r["builtin"] = True
missing = [k for k in ours if k not in ty]
extra = [k for k in ty if k not in ours]
print(f"our release references: {len(ours)}; ty name uses: {len(ty)}")
print(f"ours without a ty use (parity residue): {len(missing)}; ty uses we do not place (annotations etc.): {len(extra)}")
subset = within = beyond = nodefs = 0
beyond_kinds = collections.Counter()
examples = []
for k, r in ours.items():
    if k not in ty:
        continue
    _, defs, undef = ty[k]
    if not defs:
        nodefs += 1
        continue
    if defs <= r["cands"]:
        subset += 1
        if len(defs) < len(r["cands"]):
            within += 1
    else:
        beyond += 1
        beyond_kinds[frozenset(r["kinds"]) if r["cands"] else "no-candidate"] += 1
        if len(examples) < 8:
            examples.append((k, r["name"], sorted(defs), sorted(r["cands"])))
print(f"ty reaching defs within our candidates: {subset} (strictly narrower: {within}); outside: {beyond}; ty none (builtin/undefined): {nodefs}")
print("outside, by our candidate binding kinds:", beyond_kinds.most_common(8))
for x in examples: print("  e.g.", x)
print("--- residue (ours without a ty use), by name:")
print(collections.Counter(ours[k]["name"] for k in missing).most_common(12))
site = "/home/paul/library-context/build/envs/fastmcp/lib/python3.14/site-packages/"
def text(path, rng):
    s, e = map(int, rng.split(":")); return open(site + path, "rb").read()[s:e].decode(errors="replace").replace("\n", " ")[:60]
print("--- outside: ty focus text vs our candidate text (first 10):")
n = 0
for k, r in ours.items():
    if k not in ty: continue
    _, defs, _ = ty[k]
    if defs and not defs <= r["cands"]:
        extra_defs = sorted(defs - r["cands"])
        print("  ", k[0], r["name"], "| ty:", [text(k[0], d) for d in extra_defs][:2], "| ours:", [text(k[0], c) for c in sorted(r["cands"])][:2])
        n += 1
        if n >= 10: break
