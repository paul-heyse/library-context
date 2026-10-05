"""Snapshot loader, validation pass and publication for the SurrealDB-native representation.

A snapshot S is a set of shared, content-addressed records whose `spans` contain S. Loading S:
  * identity record per content id (key fields); payload record per [id, payload hash] (non-key
    fields) -- both shared with earlier snapshots when equal, so unchanged content is not copied;
  * a record present in S-1 and S keeps its open span; a record absent from S gets its open span
    closed at S; a record new in S (or re-added) gets an open span starting at S;
  * spans are the only mutable field (content fields are READONLY), and closing a span at S never
    changes what an earlier snapshot sees.
Validation runs as the owner over records live at S; publication flips `selection:current`.
Readers are record users whose session CONTEXT pins the selected ordinal; table PERMISSIONS
restrict every read to records live at that ordinal.

Rows are dicts: {"id": hex, <field>: value}; id/digest values are lowercase hex strings, binary
values are bytes, absent values are None.
"""
import hashlib
import json
import sdb

BATCH = 2000


def lit(f, v):
    if v is None:
        return "NONE"
    if f["list"]:
        return "[" + ", ".join(lit(dict(f, list=False), x) for x in v) + "]"
    t = f["type"]
    if t == "id":
        return f"{f['target']}:`{v}`"
    if t in ("digest", "text"):
        return json.dumps(v, ensure_ascii=False)
    if t == "binary":
        return f'b"{v.hex()}"'
    if t == "bool":
        return "true" if v else "false"
    if t == "finite_f64":
        return repr(float(v)) + "f" if v == v and abs(v) != float("inf") else ("NaN" if v != v else ("math::INF" if v > 0 else "math::NEG_INF"))
    return str(int(v)) if not isinstance(v, str) else v  # str: deliberate raw literal (seeded type violations)


def payload_hash(fields, row):
    canon = [[f["name"], row.get(f["name"]).hex() if isinstance(row.get(f["name"]), bytes) else row.get(f["name"])] for f in fields]
    return hashlib.sha256(json.dumps(canon, sort_keys=False).encode()).hexdigest()[:32]


class Model:
    def __init__(self, path):
        m = json.load(open(path))
        self.rel = {r["name"]: r for r in m["relations"]}
        self.digest = m["digest"]

    def keys(self, name):
        return [f for f in self.rel[name]["fields"] if f["key"]]

    def pay(self, name):
        return [f for f in self.rel[name]["fields"] if not f["key"]]


def _live_open(spans, s):
    return bool(spans) and len(spans[-1]) == 1 and spans[-1][0] <= s


def _existing(table):
    res = sdb.sql(f"SELECT record::id(id) AS k, spans FROM {table};")[0]["result"]
    return {json.dumps(r["k"]) if isinstance(r["k"], list) else r["k"]: (r["k"], r["spans"]) for r in res}


def _insert(table, objs, guard_fields=None):
    """Plain INSERT for new records. With guard_fields, INSERT .. ON DUPLICATE KEY UPDATE that
    re-asserts every content field: READONLY refuses the statement if any stored value differs, so
    sharing an existing record is also a content-equality check."""
    tail = ""
    if guard_fields is not None:
        tail = " ON DUPLICATE KEY UPDATE " + ", ".join([f"`{n}` = $input.`{n}`" for n in guard_fields] + ["spans = $input.spans"])
    for i in range(0, len(objs), BATCH):
        sdb.sql(f"INSERT INTO {table} [{', '.join(objs[i:i + BATCH])}]{tail} RETURN NONE;")


def _set_spans(table, updates):
    """updates: list of (key, spans); key is a string or [id, hash]."""
    for i in range(0, len(updates), BATCH):
        chunk = [{"k": k, "s": s} for k, s in updates[i:i + BATCH]]
        sdb.sql(f"FOR $u IN {json.dumps(chunk)} {{ UPDATE type::record('{table}', $u.k) SET spans = $u.s RETURN NONE; }};")


def _load_relation(model, s, name, rows, stats, lock, verify_shared=True, base_manifest=None, journal=None):
    """Load one relation into staging snapshot s. If the relation's key digest equals the base
    snapshot's manifest digest, every record is already live and open: nothing is written
    (relation-level structural sharing, like an unchanged Merkle subtree)."""
    if not isinstance(verify_shared, bool):  # a set of relations to verify
        verify_shared = name in verify_shared
    keys, pay = model.keys(name), model.pay(name)
    tables = [(name, keys, "identity")] + ([(name + "__p", pay, "payload")] if pay else [])
    local = {"identity": {"new": 0, "shared": 0, "readded": 0, "closed": 0},
             "payload": {"new": 0, "shared": 0, "readded": 0, "closed": 0}}
    wants = []
    h = hashlib.sha256()
    for table, fields, kind in tables:
        want = {}
        for r in rows:
            if kind == "identity":
                k = r["id"]
                obj = "{" + ", ".join([f"id: {table}:`{k}`"] + [f"`{f['name']}`: {lit(f, r.get(f['name']))}" for f in fields])
            else:
                ph = payload_hash(fields, r)
                k = [r["id"], ph]
                obj = "{" + ", ".join([f"id: {table}:['{r['id']}', '{ph}']", f"identity_of: {name}:`{r['id']}`"] + [f"`{f['name']}`: {lit(f, r.get(f['name']))}" for f in fields])
            kk = json.dumps(k) if isinstance(k, list) else k
            if kk in want:
                if kind == "identity":
                    raise ValueError(f"duplicate content id {k} in {name} within snapshot {s}")
                continue
            want[kk] = (k, obj)
        for kk in sorted(want):
            h.update(kk.encode())
        wants.append((table, fields, kind, want))
    digest = h.hexdigest()
    unchanged = (not verify_shared and base_manifest is not None and name in base_manifest
                 and base_manifest[name]["digest"] == digest)
    if unchanged:
        for table, fields, kind, want in wants:
            local[kind]["shared"] += len(want)
    else:
        for table, fields, kind, want in wants:
            have = _existing(table)
            new, shared, upd = [], [], []
            for kk, (k, obj) in want.items():
                if kk not in have:
                    new.append(obj + f", spans: [[{s}]]}}")
                    local[kind]["new"] += 1
                    if journal is not None:
                        journal.setdefault(table, {"new": [], "spans": []})["new"].append(k)
                else:
                    spans = have[kk][1]
                    if _live_open(spans, s - 1) or _live_open(spans, s):
                        if verify_shared:  # re-assert content: READONLY refuses any difference
                            shared.append(obj + f", spans: {json.dumps(spans)}}}")
                        local[kind]["shared"] += 1
                    else:
                        shared.append(obj + f", spans: {json.dumps(spans + [[s]])}}}")
                        local[kind]["readded"] += 1
                        if journal is not None:
                            journal.setdefault(table, {"new": [], "spans": []})["spans"].append((k, spans))
            for kk, (k, spans) in have.items():
                if kk not in want and spans and len(spans[-1]) == 1:
                    upd.append((k, spans[:-1] + [[spans[-1][0], s]]))
                    local[kind]["closed"] += 1
                    if journal is not None:
                        journal.setdefault(table, {"new": [], "spans": []})["spans"].append((k, spans))
            _insert(table, new)
            guard = [f["name"] for f in fields] + (["identity_of"] if kind == "payload" else [])
            _insert(table, shared, guard_fields=guard)
            _set_spans(table, upd)
    with lock:
        for kind in local:
            for c in local[kind]:
                stats[kind][c] += local[kind][c]
    return {"rows": len(rows), "digest": digest, "unchanged": unchanged}


def load(model, s, data, source="synthetic", log=print, workers=1, verify_shared=True, relations=None,
         base=None, journal=None, order=None, carry=None):
    """Load snapshot ordinal s. `data` is {relation: [rows]} or a callable relation -> rows.
    relations: the relations making up the snapshot (default: all model relations).
    base: ordinal of the published snapshot to share unchanged relations with (by manifest digest).
    journal: dict filled with every write, so abort() can undo exactly this load.
    carry: relations the caller declares unchanged from base; their manifest entries are carried
    without reading or comparing them (used for seeded single-relation changes)."""
    import threading
    from concurrent.futures import ThreadPoolExecutor
    provider = data if callable(data) else (lambda n: data.get(n, []))
    base_manifest = None
    if base is not None:
        base_manifest = sdb.sql(f"SELECT VALUE manifest FROM ONLY snapshot:{base};")[0]["result"]
    sdb.sql(f"CREATE snapshot:{s} SET ordinal = {s}, state = 'staging', source = {json.dumps(source)}, model_digest = '{model.digest}';")
    stats = {"identity": {"new": 0, "shared": 0, "readded": 0, "closed": 0},
             "payload": {"new": 0, "shared": 0, "readded": 0, "closed": 0}}
    lock = threading.Lock()
    names = list(order) if order else sorted(relations if relations is not None else model.rel)

    def one(name):
        if carry and name in carry and base_manifest and name in base_manifest:
            m = dict(base_manifest[name], unchanged=True, carried=True)
            with lock:
                stats["identity"]["shared"] += m["rows"]
            return name, m
        m = _load_relation(model, s, name, provider(name), stats, lock, verify_shared, base_manifest, journal)
        if workers > 1:
            log(f"  loaded {name}: {m['rows']} rows{' (unchanged, shared)' if m['unchanged'] else ''}")
        return name, m
    if workers > 1:
        with ThreadPoolExecutor(workers) as ex:
            manifest = dict(ex.map(one, names))
    else:
        manifest = dict(map(one, names))
    sdb.sql(f"UPDATE snapshot:{s} SET manifest = {json.dumps(manifest)};")
    log(f"loaded snapshot {s}: {json.dumps(stats)}")
    return stats, manifest


def validation_queries(model, s, relations=None, changed=None):
    """Generated validation pass for snapshot s: list of (check name, SurrealQL returning a count).
    relations: restrict to these relations. changed: delta validation -- keep only checks on a
    changed relation or whose reference target is a changed relation."""
    out = []
    for name, rel in sorted(model.rel.items()):
        if relations is not None and name not in relations:
            continue
        has_pay = any(not f["key"] for f in rel["fields"])
        for f in rel["fields"]:
            if f["type"] != "id" or f["list"]:
                continue
            table = name if f["key"] else name + "__p"
            cond = f"`{f['name']}` != NONE AND !fn::live(`{f['name']}`.spans, {s})"
            if "subtype" in f:
                tag = model.rel[f["target"]]["sum"]["tag"]
                cond = f"`{f['name']}` != NONE AND (!fn::live(`{f['name']}`.spans, {s}) OR `{f['name']}`.`{tag}` != {f['subtype']})"
            if changed is not None and name not in changed and f["target"] not in changed:
                continue
            out.append((f"closure:{table}.{f['name']}->{f['target']}",
                        f"SELECT VALUE count() FROM {table} WHERE fn::live(spans, {s}) AND {cond} GROUP ALL;"))
        if has_pay and (changed is None or name in changed):
            out.append((f"one-payload:{name}",
                        f"SELECT VALUE count() FROM {name} WHERE fn::live(spans, {s}) AND array::len((<~({name}__p FIELD identity_of)).filter(|$p| fn::live($p.spans, {s}))) != 1 GROUP ALL;"))
            out.append((f"payload-owner:{name}__p",
                        f"SELECT VALUE count() FROM {name}__p WHERE fn::live(spans, {s}) AND !fn::live(identity_of.spans, {s}) GROUP ALL;"))
    return out


def validate(model, s, log=print, workers=1, relations=None, changed=None):
    failures = []
    qs = validation_queries(model, s, relations, changed)
    from concurrent.futures import ThreadPoolExecutor
    step = 50 if workers == 1 else 1
    chunks = [qs[i:i + step] for i in range(0, len(qs), step)]

    def run(chunk):
        res = sdb.sql("\n".join(q for _, q in chunk))
        return [(name, (r["result"] or [0])[0]) for (name, _), r in zip(chunk, res)]
    with ThreadPoolExecutor(workers) as ex:
        for part in ex.map(run, chunks):
            failures += [(n, c) for n, c in part if c]
    state = "validated" if not failures else "failed"
    sdb.sql(f"UPDATE snapshot:{s} SET state = '{state}';")
    log(f"validated snapshot {s}: {len(qs)} checks, failures={failures}")
    return failures


def publish(s):
    st = sdb.sql(f"SELECT VALUE state FROM ONLY snapshot:{s};")[0]["result"]
    if st != "validated":
        raise RuntimeError(f"snapshot {s} is {st}; refusing to publish")
    sdb.sql(f"BEGIN; UPDATE snapshot:{s} SET state = 'published'; UPSERT selection:current SET snap = {s}; COMMIT;")


def abort(model, s, journal=None):
    """Undo a failed staging snapshot. With the load journal: delete exactly the records it created
    and restore exactly the spans it changed. Without: scan every table."""
    if journal is not None:
        for table, j in journal.items():
            for i in range(0, len(j["new"]), BATCH):
                keys = j["new"][i:i + BATCH]
                sdb.sql(f"FOR $k IN {json.dumps(keys)} {{ DELETE type::record('{table}', $k) RETURN NONE; }};")
            _set_spans(table, j["spans"])
        sdb.sql(f"UPDATE snapshot:{s} SET state = 'failed';")
        return
    for name in model.rel:
        for table in [name] + ([name + "__p"] if model.pay(name) else []):
            sdb.sql(f"DELETE {table} WHERE spans = [[{s}]] RETURN NONE;"
                    # note: spans[-1] is NONE in SurrealQL 3.3 (no negative indexing); use array::last
                    f"UPDATE {table} SET spans = array::push(array::slice(spans, 0, -1), [array::last(spans)[0]]) WHERE array::len(array::last(spans)) = 2 AND array::last(spans)[1] = {s} RETURN NONE;"
                    f"UPDATE {table} SET spans = array::slice(spans, 0, -1) WHERE array::len(spans) > 1 AND array::last(spans) = [{s}] RETURN NONE;")
    sdb.sql(f"UPDATE snapshot:{s} SET state = 'failed';")
