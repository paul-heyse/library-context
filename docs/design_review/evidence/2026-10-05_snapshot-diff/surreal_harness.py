"""SurrealDB 3.3 snapshot representation and identity/semantic diff, checked against ref.json.

Server (pinned image, SurrealKV on a named volume):
  docker volume create lctx-probe-surreal
  docker run -d --name lctx-probe-surreal --user root -p 127.0.0.1:18766:8000 \
    -v lctx-probe-surreal:/data surrealdb/surrealdb:v3.3.0 start --log warn --user root --pass root surrealkv:///data/db
  python3 .../surreal_harness.py <data_dir> [db]       (stdlib only; HTTP /sql)
Cleanup: docker rm -f lctx-probe-surreal; docker volume rm lctx-probe-surreal

Representation (M1, judged the best native fit; VERSION time travel is excluded, see A3b):
  content  <rel>:[id, pdig]   SCHEMAFULL, typed fields, ASSERT >= 0 on ints; write-once via INSERT IGNORE
  member   member:[snap, rel, id] { v: record link to the content version, p: pdig }
  Snapshot read = array-id range scan member:[s, rel, NONE]..[s, rel, ..]; diff = keyset-paged
  range reads of both snapshots merged in the client (no JOIN, no snapshot-diff primitive).
Also: M2 validity fields (lo/hi, indexed) on a copy, an in-database array::complement diff, a
snapshot-scoped reference check (record::exists on a constructed member id) and the keyed
semantic diff via type::record lookups.
"""
import csv, json, pathlib, sys, urllib.request, base64
sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import load, compare

D = pathlib.Path(sys.argv[1]).resolve()
DB = sys.argv[2] if len(sys.argv) > 2 else "snap"
URL = "http://127.0.0.1:18766/sql"
AUTH = "Basic " + base64.b64encode(b"root:root").decode()
schema, ref = load(D)
PAGE, BATCH = 20000, 2000

def q(sql, check=True):
    req = urllib.request.Request(URL, data=sql.encode(), headers={
        "Accept": "application/json", "Surreal-NS": "probe", "Surreal-DB": DB, "Authorization": AUTH})
    res = json.load(urllib.request.urlopen(req, timeout=3600))
    if check:
        for r in res:
            if r["status"] != "OK":
                raise RuntimeError(f"{r['result']} :: {sql[:200]}")
    return res

def lit(rel, col, v):
    t = schema[rel]["types"][col]
    if v == "":
        return "NONE"
    return str(int(v)) if t == "int" else json.dumps(v)

def rows(state, rel):
    with open(D / state / f"{rel}.csv") as fh:
        yield from csv.DictReader(fh)

nullable = {rel: set() for rel in schema}
for rel in schema:
    for st in ("A", "B"):
        for r in rows(st, rel):
            nullable[rel].update(c for c, v in r.items() if v == "")
q("DEFINE NAMESPACE IF NOT EXISTS probe; DEFINE DATABASE IF NOT EXISTS " + DB + ";")  # bootstrap
q(f"REMOVE DATABASE IF EXISTS {DB}; DEFINE DATABASE {DB};")
ddl = ["DEFINE TABLE member SCHEMAFULL; DEFINE FIELD v ON member TYPE record; DEFINE FIELD p ON member TYPE string;"]
for rel, s in schema.items():
    ddl.append(f"DEFINE TABLE {rel} SCHEMAFULL;")
    for c in s["columns"][1:] + ["pdig"]:
        t = s["types"][c]
        st = "int" if t == "int" else "string"
        opt = f"option<{st}>" if c in s["refs"] or c in nullable[rel] else st
        assertion = " ASSERT $value >= 0" if c in s.get("nonneg", [k for k, v in s["types"].items() if v == "int"]) else ""
        ddl.append(f"DEFINE FIELD `{c}` ON {rel} TYPE {opt}{assertion};")
q("\n".join(ddl))

def load_state(snap, state):
    for rel, s in schema.items():
        cols = s["columns"][1:] + ["pdig"]
        content, members, size = [], [], [0]
        def flush():
            # Plain INSERT: INSERT IGNORE silently drops rows that violate field types
            # (raw/surreal_insert_ignore_probe.txt). Versions already stored are skipped client-side.
            if content:
                q(f"INSERT INTO {rel} [" + ",".join(content) + "] RETURN NONE;")
            if members:
                q("INSERT INTO member [" + ",".join(members) + "] RETURN NONE;")
            content.clear(); members.clear()
        for r in rows(state, rel):
            members.append("{id: [%d,'%s','%s'], v: %s:['%s','%s'], p: '%s'}" % (snap, rel, r["id"], rel, r["id"], r["pdig"], r["pdig"]))
            if (r["id"], r["pdig"]) in stored_versions[rel]:
                size[0] += len(members[-1])
                if len(members) >= BATCH or size[0] > 700_000:
                    flush(); size[0] = 0
                continue
            stored_versions[rel].add((r["id"], r["pdig"]))
            content.append("{id: ['%s','%s'], %s}" % (r["id"], r["pdig"], ", ".join(f"{json.dumps(c)}: {lit(rel, c, r[c])}" for c in cols)))
            size[0] += len(content[-1]) + len(members[-1])
            if len(content) >= BATCH or size[0] > 700_000:  # /sql body limit is 1 MiB by default
                flush(); size[0] = 0
        flush()
stored_versions = {rel: set() for rel in schema}
load_state(1, "A")
load_state(2, "B")

counts = {rel: q(f"RETURN count((SELECT VALUE id FROM {rel}));")[0]["result"] for rel in schema}
short = {rel: (counts[rel], ref["relations"][rel]["union_versions"]) for rel in schema
         if counts[rel] != ref["relations"][rel]["union_versions"]}
print("surreal content completeness:", "passed" if not short else f"failed {short}")
stored = sum(counts.values())
members = {s: sum(q(f"RETURN count((SELECT VALUE id FROM member:[{s},'{rel}',NONE]..[{s},'{rel}',..]));")[0]["result"] for rel in schema) for s in (1, 2)}
print(f"surreal-M1 sharing: content records stored {stored} for membership A={members[1]} B={members[2]} "
      f"(two full copies: {members[1] + members[2]})")

def scan(snap, rel, proj="[id[2], p]"):
    """Keyset-paged range scan of one snapshot's membership for one relation."""
    out, last = [], None
    while True:
        start = f"member:[{snap},'{rel}',NONE].." if last is None else f"member:[{snap},'{rel}','{last}']>.."
        page = q(f"SELECT VALUE {proj} FROM {start}[{snap},'{rel}',..] LIMIT {PAGE};")[0]["result"]
        out.extend(page)
        if len(page) < PAGE:
            return out
        last = page[-1][0]

got = {}
for rel in schema:
    a, b = dict(scan(1, rel)), dict(scan(2, rel))
    got[rel] = {"added": b.keys() - a.keys(), "removed": a.keys() - b.keys(),
                "changed": {i for i in a.keys() & b.keys() if a[i] != b[i]}}
ok1 = compare("surreal-M1-rangemerge", ref, got)

# Snapshot-scoped reference integrity (record links from shared content cannot be per-snapshot)
def dangling(snap):
    n = 0
    for rel, s in schema.items():
        for c, t in s["refs"].items():
            n += q(f"RETURN count((SELECT VALUE id FROM member:[{snap},'{rel}',NONE]..[{snap},'{rel}',..] "
                   f"WHERE v.`{c}` != NONE AND !record::exists(type::record('member', [{snap}, '{t}', v.`{c}`]))));")[0]["result"]
    return n
# A single INSERT ... SELECT of a whole snapshot (3.5M rows) is refused on SurrealKV with
# "Memtable arena is full" (raw/surreal_real.txt, first attempt); copy per relation in batches.
for rel in schema:
    page = scan(2, rel, "[id[2], v, p]")
    for b in range(0, len(page), BATCH):
        q("INSERT INTO member [" + ",".join("{id: [3,'%s','%s'], v: %s, p: '%s'}" % (rel, i, v, p_)
                                            for i, v, p_ in page[b:b + BATCH]) + "] RETURN NONE;")
rrel, (rcol, trel) = next((r, next(iter(s["refs"].items()))) for r, s in schema.items() if s["refs"])
tid = q(f"SELECT VALUE v.`{rcol}` FROM member:[3,'{rrel}',NONE]..[3,'{rrel}',..] WHERE v.`{rcol}` != NONE LIMIT 1;")[0]["result"][0]
q(f"DELETE member:[3,'{trel}','{tid}'];")
print(f"surreal-M1 snapshot-scoped refs: dangling(A)={dangling(1)} dangling(B)={dangling(2)} "
      f"dangling(B minus one referenced occurrence)={dangling(3)} (expected 0,0,>=1)")
for rel in schema:
    q(f"DELETE member:[3,'{rel}',NONE]..[3,'{rel}',..];")

# Enforcement: field types and ASSERT on a SCHEMAFULL table (generic over the schema)
def create(rel, override):
    sets = []
    for c in schema[rel]["columns"][1:] + ["pdig"]:
        v = override.get(c, 0 if schema[rel]["types"][c] == "int" else "x")
        sets.append(f"`{c}` = {json.dumps(v)}")
    return f"CREATE {rel}:['probe','probe'] SET " + ", ".join(sets) + ";"
enf = []
nn_rel = next((r for r, s in schema.items() if s.get("nonneg", [k for k, v in s["types"].items() if v == "int"])), None)
tests = []
if nn_rel:
    c = schema[nn_rel].get("nonneg", [k for k, v in schema[nn_rel]["types"].items() if v == "int"])[0]
    tests += [(nn_rel, {c: -1}, "negative int vs ASSERT"), (nn_rel, {c: "notint"}, "string into int field")]
tests.append((rrel, {rcol: "f" * 32}, "dangling reference value (plain string field)"))
for rel, ov, what in tests:
    r = q(create(rel, ov), check=False)[0]
    enf.append(f"{'refused' if r['status'] != 'OK' else 'ACCEPTED'}: {what} on {rel} -> {str(r['result'])[:110]}")
    q(f"DELETE {rel}:['probe','probe'];", check=False)
print("surreal enforcement:", *enf, sep="\n  ")

# M2: validity fields on content records (linear history), indexed
for rel in schema:
    q(f"DEFINE FIELD `lo` ON {rel} TYPE option<int>; DEFINE FIELD `hi` ON {rel} TYPE option<int>;"
      f"DEFINE INDEX {rel}_lo ON {rel} FIELDS lo; DEFINE INDEX {rel}_hi ON {rel} FIELDS hi;")
    for snap in (1, 2):
        q(f"FOR $m IN (SELECT VALUE v FROM member:[{snap},'{rel}',NONE]..[{snap},'{rel}',..]) "
          f"{{ UPDATE $m SET lo = IF lo = NONE THEN {snap} ELSE lo END, hi = {snap + 1}; }};")
got2 = {}
for rel in schema:
    lo2 = {r[0] for r in q(f"SELECT VALUE [id[0]] FROM {rel} WHERE lo = 2;")[0]["result"]}
    in1 = {r[0] for r in q(f"SELECT VALUE [id[0]] FROM {rel} WHERE lo <= 1 AND hi > 1;")[0]["result"]}
    hi2 = {r[0] for r in q(f"SELECT VALUE [id[0]] FROM {rel} WHERE hi = 2;")[0]["result"]}
    in2 = {r[0] for r in q(f"SELECT VALUE [id[0]] FROM {rel} WHERE lo <= 2 AND hi > 2;")[0]["result"]}
    got2[rel] = {"added": lo2 - in1, "removed": hi2 - in2, "changed": lo2 & in1}
ok2 = compare("surreal-M2-fields", ref, got2)

# Semantic keyed diff: no JOIN, so resolve the symbol through a constructed member id per row
okS = True
sp = ref.get("semantic_spec")
if sp and "semantic" in ref:
    def sem(snap):
        look = f"type::record('member', [{snap}, '{sp['sym']}', v.`{sp['sym_ref']}`]).v"
        proj = ", ".join([f"id[2]", f"v.`{sp['digest']}`"] + [f"{look}.`{c}`" for c in sp["key_sym"]] + [f"v.`{c}`" for c in sp["key_sig"]])
        out = {}
        for row in scan(snap, sp["sig"], f"[{proj}]"):
            if row[2] is None:
                continue
            out["|".join("" if v is None else str(v) for v in row[2:])] = row[1]
        return out
    sa, sb = sem(1), sem(2)
    semantic = {"added": sb.keys() - sa.keys(), "removed": sa.keys() - sb.keys(),
                "changed": {k for k in sa.keys() & sb.keys() if sa[k] != sb[k]}}
    okS = compare("surreal-semantic", {"relations": {}, "semantic": ref["semantic"]}, {}, semantic)
# In-database diff with array::complement (materialises both arrays in one query)
gotc, notes = {}, []
for rel in schema:
    rng = lambda s: f"(SELECT VALUE [id[2], p] FROM member:[{s},'{rel}',NONE]..[{s},'{rel}',..])"
    try:
        res = q(f"LET $a = {rng(1)}; LET $b = {rng(2)}; RETURN [array::complement($b, $a), array::complement($a, $b)];")[-1]["result"]
        addv, remv = res
        ai, ri = {i for i, _ in addv}, {i for i, _ in remv}
        gotc[rel] = {"added": ai - ri, "removed": ri - ai, "changed": ai & ri}
    except Exception as e:  # record qualitative feasibility
        notes.append(f"{rel}: {str(e)[:120]}")
        gotc[rel] = {"added": set(), "removed": set(), "changed": set()}
okc = compare("surreal-array-complement", ref, gotc)
print("surreal-array-complement failures:", notes or "none")

print("surreal OVERALL", "passed" if all((ok1, okc, ok2, okS)) else "failed")
