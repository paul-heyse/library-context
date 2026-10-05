"""TerminusDB 12.0.7: load state A and state B as full replacements of the instance graph (one
commit each), diff the two data versions with /api/diff (all_documents), check against ref.json,
test link enforcement and record server memory. Stdlib only.

  docker run -d --name lctx-probe-terminus -e TERMINUSDB_ADMIN_PASS=root -p 127.0.0.1:16363:6363 \
    -v lctx-probe-terminus:/app/terminusdb/storage terminusdb/terminusdb-server:v12.0.7
  python3 .../terminus_harness.py <data_dir> [db]
Cleanup: docker rm -f lctx-probe-terminus; docker volume rm lctx-probe-terminus

Documents: one class per relation, @key Lexical on the hex id (so @id = <Rel>/<hex>), typed
fields, references as Optional links to the target class; pdig stored as a field.
"""
import base64, csv, json, pathlib, subprocess, sys, urllib.request, urllib.error
sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import load, compare

D = pathlib.Path(sys.argv[1]).resolve()
DB = sys.argv[2] if len(sys.argv) > 2 else "snap"
BASE = "http://127.0.0.1:16363/api"
AUTH = "Basic " + base64.b64encode(b"admin:root").decode()
schema, ref = load(D)
CLS = {rel: "".join(p.title() for p in rel.split("_")) for rel in schema}

def call(method, path, body=None, raw=False):
    data = None if body is None else (body if isinstance(body, bytes) else json.dumps(body).encode())
    req = urllib.request.Request(BASE + path, data=data, method=method,
                                 headers={"Authorization": AUTH, "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=24 * 3600) as r:
            out = r.read()
            return (r.headers, out) if raw else json.loads(out or b"null")
    except urllib.error.HTTPError as e:
        raise RuntimeError(f"{e.code} {e.read()[:400]!r}")

def mem():
    return subprocess.run(["docker", "stats", "--no-stream", "lctx-probe-terminus", "--format", "{{.MemUsage}}"],
                          capture_output=True, text=True).stdout.strip()

try:
    call("DELETE", f"/db/admin/{DB}")
except RuntimeError:
    pass
call("POST", f"/db/admin/{DB}", {"label": DB, "schema": True,
                                 "prefixes": {"@base": "terminusdb:///data/", "@schema": "terminusdb:///schema#"}})
nullable = {rel: set() for rel in schema}
for rel in schema:
    for st in ("A", "B"):
        with open(D / st / f"{rel}.csv") as fh:
            for r in csv.DictReader(fh):
                nullable[rel].update(c for c, v in r.items() if v == "")
docs = []
for rel, s in schema.items():
    d = {"@id": CLS[rel], "@type": "Class", "@key": {"@type": "Lexical", "@fields": ["hid"]}, "hid": "xsd:string"}
    for c in s["columns"][1:] + ["pdig"]:
        if c in s["refs"]:
            d[c] = {"@type": "Optional", "@class": CLS[s["refs"][c]]}
        else:
            base = "xsd:integer" if s["types"][c] == "int" else "xsd:string"
            d[c] = {"@type": "Optional", "@class": base} if c in nullable[rel] else base
    docs.append(d)
call("POST", f"/document/admin/{DB}?graph_type=schema&author=probe&message=schema", docs)

def state_docs(state):
    for rel, s in schema.items():
        with open(D / state / f"{rel}.csv") as fh:
            for r in csv.DictReader(fh):
                doc = {"@type": CLS[rel], "hid": r["id"]}
                for c in s["columns"][1:] + ["pdig"]:
                    if c in s["refs"]:
                        if r[c]:
                            doc[c] = f"{CLS[s['refs'][c]]}/{r[c]}"
                    elif r[c] != "":
                        doc[c] = int(r[c]) if s["types"][c] == "int" else r[c]
                yield doc

versions = {}
for state in ("A", "B"):
    body = b"\n".join(json.dumps(d).encode() for d in state_docs(state))
    h, _ = call("POST", f"/document/admin/{DB}?author=probe&message=state{state}&full_replace=true", body, raw=True)
    versions[state] = h.get("TerminusDB-Data-Version")
    print(f"terminus state {state}: data version {versions[state]}; server memory {mem()}")
    sys.stdout.flush()

patch = call("POST", f"/diff/admin/{DB}", {"before_data_version": versions["A"], "after_data_version": versions["B"]})
got = {rel: {"added": set(), "removed": set(), "changed": set()} for rel in schema}
inv = {v: k for k, v in CLS.items()}
kinds = {}
for p in patch:
    op = p.get("@op")
    kinds[op] = kinds.get(op, 0) + 1
    if op == "Insert":
        doc = p["@insert"]; got[inv[doc["@type"]]]["added"].add(doc["hid"])
    elif op == "Delete":
        doc = p["@delete"]; got[inv[doc["@type"]]]["removed"].add(doc["hid"])
    else:  # a field-level patch on a kept document
        cls, hid = p["@id"].split("/", 1)
        got[inv[cls]]["changed"].add(hid)
print("terminus patch ops:", kinds, "| server memory after diff:", mem())
ok = compare("terminus-diff", ref, got)

# Link enforcement: a document linking to a missing target
try:
    call("POST", f"/document/admin/{DB}?author=probe&message=bad",
         {"@type": CLS["occurrences"], "hid": "f" * 32, "source": f"{CLS['source_artifacts']}/{'0' * 32}",
          "start": 0, "end_": 1, "kind": 1, "role": 0, "spath": "0", "pdig": "p"})
    print("terminus link enforcement: dangling link ACCEPTED")
except RuntimeError as e:
    print("terminus link enforcement: dangling link refused", str(e)[:200])
print("terminus OVERALL", "passed" if ok else "failed")
