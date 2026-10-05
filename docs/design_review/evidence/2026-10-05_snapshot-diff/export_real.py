"""Export a subset of a real lctx generation (read-only) as state A, derive state B by a realistic
perturbation, and write the same layout the harnesses read (A/, B/, schema.json, ref.json).

  uv run --no-project --with 'psycopg[binary]' --with blake3 python \
    docs/design_review/evidence/2026-10-05_snapshot-diff/export_real.py <generation_id_hex> \
    [out_dir=build/review-probes/snapshot-diff/real] [rel,rel,...]

Reads schema lctx_g<generation> in database lctx with default_transaction_read_only=on as
lctx_superuser (~/.pgpass). Never writes to lctx.

Identity in the real model is BLAKE3 over the typed key; this script cannot re-encode it, so the
cascade is simulated structurally: a changed row gets a fresh 16-byte id, and every row whose
reference column points at a changed id is rewritten and gets a fresh id too (references are
treated as key fields), to a fixpoint. Reference columns are inferred: a 16-byte column whose
non-null values all resolve to ids of exactly one exported relation (ids are type-tagged, so an id
belongs to one relation). Columns resolving partly or to several relations stay plain values.

Perturbation A -> B:
  1. edit one source artifact (its content hash changes) -> cascade;
  2. ~1% of each signature relation's rows get a new parameters digest (a key field) -> cascade;
  3. ~1% of rows of each large relation (>= 10k rows) not referenced by any exported row are
     removed; ~1% are added (copies with fresh ids); ~1% get a payload change with stable id on a
     non-key scalar column when one exists (only if the column is not a reference).
"""
import blake3, csv, io, json, pathlib, random, sys, collections
import psycopg, re

def model_fields():
    """Relation name -> (key fields, payload fields), parsed from #[model(name = ...)] structs in
    crates/lctx-model/src/domain. Sum types (DomainSum enums) are left out (treated as all-key)."""
    out = {}
    for f in pathlib.Path("crates/lctx-model/src/domain").rglob("*.rs"):
        t = f.read_text()
        for m in re.finditer(r'#\[model\(name = "(\w+)"[^\n]*\n(?:\s*(?:#\[[^\n]*|///[^\n]*)\n)*\s*pub struct \w+\s*\{', t):
            i, d = m.end() - 1, 0
            for j in range(i, len(t)):
                d += {"{": 1, "}": -1}.get(t[j], 0)
                if d == 0:
                    break
            body, keys, pay, attrs = t[i + 1:j], [], [], ""
            for line in body.splitlines():
                line = line.strip()
                if line.startswith("#["):
                    attrs += line
                    continue
                fm = re.match(r"pub (\w+):", line)
                if fm:
                    (keys if "key" in attrs else pay).append(fm.group(1))
                    attrs = ""
                elif not line.startswith("//"):
                    attrs = ""
            out[m.group(1)] = (keys, pay)
    return out
MODEL = model_fields()

GEN = sys.argv[1].lower()
OUT = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 and not sys.argv[2].startswith("--") else "build/review-probes/snapshot-diff/real")
DEFAULT = ("occurrences,evidence,entity_refs,syntax_placements,source_artifacts,modules,provider_modules,"
           "provider_symbols,symbol_declarations,signature_observations,signature_parameters,parameter_shapes,"
           "callable_entities,class_entities,parameter_entities,occurrence_ownership,catalog_callables,"
           "catalog_callable_aspects,serving_callables,serving_parameters")
ARGS = [a for a in sys.argv[1:] if not a.startswith("--")]
RELS = (ARGS[2] if len(ARGS) > 2 else DEFAULT).split(",")
SCHEMA = f"lctx_g{GEN}"
con = psycopg.connect(host="127.0.0.1", user="lctx_superuser", dbname="lctx",
                      options="-c default_transaction_read_only=on")
present = {r[0] for r in con.execute("select table_name from information_schema.tables where table_schema=%s", (SCHEMA,))}
missing = [r for r in RELS if r not in present]
RELS = [r for r in RELS if r in present]
top = con.execute("""select c.relname from pg_class c join pg_namespace n on n.oid=c.relnamespace
    where n.nspname=%s and c.relkind='r' and c.relname not like '\\_\\_%%' order by c.reltuples desc limit 6""", (SCHEMA,)).fetchall()
for (r,) in ([] if "--no-top" in sys.argv else top):
    if r not in RELS:
        RELS.append(r)
print("relations:", RELS, "| not present:", missing)

cols, data = {}, {}
for rel in RELS:
    meta = con.execute("""select column_name, data_type from information_schema.columns
        where table_schema=%s and table_name=%s and column_name not in ('generation_id','introduced_epoch')
        order by ordinal_position""", (SCHEMA, rel)).fetchall()
    sel = ", ".join(f"encode(\"{c}\", 'hex')" if t == "bytea" else f"\"{c}\"::text" for c, t in meta)
    buf = io.StringIO()
    with con.cursor().copy(f'copy (select {sel} from {SCHEMA}."{rel}") to stdout (format csv)') as cp:
        for chunk in cp:
            buf.write(bytes(chunk).decode())
    buf.seek(0)
    cols[rel] = meta
    data[rel] = {row[0]: row for row in csv.reader(buf)}
    print(f"exported {rel}: {len(data[rel])} rows")
    sys.stdout.flush()

owner = {}
for rel, rows in data.items():
    for i in rows:
        owner[i] = rel
refs, types, nonneg = {}, {}, {}
for rel, meta in cols.items():
    refs[rel], types[rel], nonneg[rel] = {}, {}, []
    for k, (c, t) in enumerate(meta):
        vals = [row[k] for row in data[rel].values() if row[k] != ""]
        if t == "bytea":
            if vals and all(len(v) == 32 for v in vals):
                targets = collections.Counter(owner.get(v) for v in vals)
                if len(targets) == 1 and None not in targets and k > 0:
                    refs[rel][c] = next(iter(targets))
                types[rel][c] = "id"
            else:
                types[rel][c] = "text"
        elif t in ("smallint", "integer", "bigint"):
            types[rel][c] = "int"
            if all(int(v) >= 0 for v in vals):
                nonneg[rel].append(c)
        else:
            types[rel][c] = "text"
    types[rel]["pdig"] = "id"
print("inferred references:", json.dumps(refs))
keycols = {rel: set(MODEL[rel][0]) if rel in MODEL else {c for c, _ in cols[rel][1:]} for rel in cols}
paycols = {rel: [c for c in MODEL.get(rel, ([], []))[1] if c in idx_name] for rel, idx_name in
           ((r, {c for c, _ in cols[r]}) for r in cols)}
print("model payload fields (non-key):", json.dumps({r: v for r, v in paycols.items() if v}))

def fresh(i, salt):
    return blake3.blake3(bytes.fromhex(i) + salt.encode()).digest()[:16].hex()
def pdig(rel, row):
    return blake3.blake3("\x1f".join(row[1:]).encode()).digest()[:16].hex()

A = {rel: {i: list(r) for i, r in rows.items()} for rel, rows in data.items()}
B = {rel: {i: list(r) for i, r in rows.items()} for rel, rows in data.items()}
idx = {rel: {c: k for k, (c, _) in enumerate(cols[rel])} for rel in cols}
referrers = collections.defaultdict(list)  # target rel -> [(rel, col index)]
for rel, rr in refs.items():
    for c, t in rr.items():
        referrers[t].append((rel, idx[rel][c]))

def cascade(seed):
    """seed: {rel: {old_id: new_id}} already applied to B. A referrer whose reference column is a
    key field re-identifies (fresh id) and propagates; one whose reference is a payload field keeps
    its id and only its payload changes."""
    frontier = {old: new for rel, m in seed.items() for old, new in m.items()}
    rounds = 0
    while frontier:
        rounds += 1
        nxt = {}
        changed_rels = {owner_b.get(n) for n in frontier.values()}
        for trel in changed_rels:
            for rel, k in referrers.get(trel, []):
                is_key = cols[rel][k][0] in keycols[rel]
                for i, row in list(B[rel].items()):
                    if row[k] in frontier:
                        new = list(row); new[k] = frontier[row[k]]
                        if is_key:
                            nid = fresh(i, "cascade")
                            new[0] = nid
                            del B[rel][i]; B[rel][nid] = new
                            owner_b[nid] = rel
                            nxt[i] = nid
                        else:
                            B[rel][i] = new
        frontier = nxt
    return rounds

owner_b = dict(owner)
rng = random.Random(11)
# 1. edit one source artifact
src = sorted(B.get("source_artifacts", {}).values(), key=lambda r: r[0])
log = {}
if src:
    ci = idx["source_artifacts"]["content"]
    oc = collections.Counter(r[idx["occurrences"]["source"]] for r in B.get("occurrences", {}).values())
    cand = sorted((r for r in src if oc.get(r[0], 0) > 0 and r[idx["source_artifacts"]["path"]].endswith(".py")),
                  key=lambda r: (oc[r[0]], r[0]))
    victim = cand[len(cand) // 2]
    new = list(victim); new[ci] = fresh(victim[ci], "edit"); new[0] = fresh(victim[0], "edit")
    del B["source_artifacts"][victim[0]]; B["source_artifacts"][new[0]] = new; owner_b[new[0]] = "source_artifacts"
    log["edited_source"] = victim[idx["source_artifacts"]["path"]]
    log["edited_source_occurrences"] = oc[victim[0]]
    log["edit_cascade_rounds"] = cascade({"source_artifacts": {victim[0]: new[0]}})
# 2. signature parameter digests (a key field) for ~1%
if "signature_observations" in B and "parameters" in idx["signature_observations"]:
    k = idx["signature_observations"]["parameters"]
    seed = {}
    for i in rng.sample(sorted(B["signature_observations"]), max(1, len(B["signature_observations"]) // 100)):
        row = B["signature_observations"].pop(i); new = list(row)
        new[k] = fresh(row[k], "param"); new[0] = fresh(i, "param")
        B["signature_observations"][new[0]] = new; owner_b[new[0]] = "signature_observations"; seed[i] = new[0]
    log["signature_changes"] = len(seed)
    log["signature_cascade_rounds"] = cascade({"signature_observations": seed})
# 3. row-level add/remove/modify on large relations
referenced = set()
for rel, rr in refs.items():
    for c in rr:
        k = idx[rel][c]
        referenced.update(row[k] for row in B[rel].values() if row[k])
for rel in list(B):
    n = len(B[rel])
    if n < 10000:
        continue
    leaves = sorted(i for i in B[rel] if i not in referenced)
    for i in rng.sample(leaves, min(len(leaves), n // 100)):
        del B[rel][i]
    # An added row must differ in a key field (content addressing: one key, one id). Change a
    # non-reference key column, integers first.
    kc = [c for c, _ in cols[rel][1:] if c in keycols[rel] and c not in refs[rel]]
    kc.sort(key=lambda c: (types[rel][c] != "int", c))
    for i in rng.sample(sorted(B[rel]), n // 100):
        new = list(B[rel][i]); new[0] = fresh(i, "added")
        live = [c for c in kc if new[idx[rel][c]] != ""]
        if live:
            c = live[0]; k = idx[rel][c]; t = types[rel][c]
            if t == "int":
                new[k] = str(int(new[k]) + 1000000000)
            elif t == "id":
                new[k] = fresh(new[k], "added-key")
            else:
                new[k] = new[k] + "+added"
        B[rel][new[0]] = new
    scal = [idx[rel][c] for c in paycols[rel] if c not in refs[rel] and types[rel][c] in ("int", "text")]
    if scal:
        k = scal[-1]
        for i in rng.sample(sorted(i for i in B[rel] if i in A[rel]), n // 100):
            v = B[rel][i][k]
            if types[rel][cols[rel][k][0]] == "int" and v:
                B[rel][i][k] = str(int(v) + 1)
            elif v in ("true", "false"):
                B[rel][i][k] = "false" if v == "true" else "true"
            else:
                B[rel][i][k] = v + "*"
        log.setdefault("payload_changes", {})[rel] = cols[rel][k][0]
log["note"] = ("row-level perturbation on relations >= 10k rows; ids are simulated "
               "(fresh = BLAKE3(old id || salt)), not re-encoded from the model's typed keys")

ref = {"relations": {}, "log": log, "generation": GEN}
for rel in RELS:
    a, b = A[rel], B[rel]
    pa = {i: pdig(rel, r) for i, r in a.items()}; pb = {i: pdig(rel, r) for i, r in b.items()}
    va, vb = set(pa.items()), set(pb.items())
    ref["relations"][rel] = dict(rows_a=len(a), rows_b=len(b), added=sorted(b.keys() - a.keys()),
                                 removed=sorted(a.keys() - b.keys()),
                                 changed=sorted(i for i in a.keys() & b.keys() if pa[i] != pb[i]),
                                 shared_versions=len(va & vb), union_versions=len(va | vb))
# Semantic keyed diff. The model declares no cross-version correspondence key and the provider's
# native_key is positional (see real_keys.sql), so this uses the within-version signature key:
# the symbol identity plus the Signature key minus `parameters`. It makes a parameter change a
# "changed" signature instead of a remove+add, but symbol ids still move with their module/source.
SPEC = {"sig": "signature_observations", "sym_ref": "symbol", "sym": "provider_symbols",
        "key_sym": ["provider", "context", "module", "native_key"],
        "key_sig": ["role", "variant", "form", "qualification", "scope", "native"], "digest": "parameters"}
def semantic(S):
    if not {SPEC["sig"], SPEC["sym"]} <= S.keys():
        return None, None
    si, pi = idx[SPEC["sig"]], idx[SPEC["sym"]]
    out, dup = {}, 0
    for row in S[SPEC["sig"]].values():
        sym = S[SPEC["sym"]].get(row[si[SPEC["sym_ref"]]])
        if sym is None:
            continue
        key = "|".join([sym[pi[c]] for c in SPEC["key_sym"]] + [row[si[c]] for c in SPEC["key_sig"]])
        dup += key in out
        out[key] = row[si[SPEC["digest"]]]
    return out, dup
sa, da = semantic(A); sb, db = semantic(B)
if sa is not None:
    ref["semantic_spec"] = SPEC
    ref["semantic"] = dict(added=sorted(sb.keys() - sa.keys()), removed=sorted(sa.keys() - sb.keys()),
                           changed=sorted(k for k in sa.keys() & sb.keys() if sa[k] != sb[k]),
                           unchanged=len([k for k in sa.keys() & sb.keys() if sa[k] == sb[k]]),
                           key_collisions_a=da, key_collisions_b=db)
def check_types(S):
    bad = 0
    for rel in RELS:
        for row in S[rel].values():
            for k, (c, _) in enumerate(cols[rel]):
                v, t = row[k], types[rel][c]
                if v == "":
                    continue
                if (t == "int" and not v.lstrip("-").isdigit()) or (t == "id" and (len(v) != 32 or not all(ch in "0123456789abcdef" for ch in v))):
                    bad += 1
    return bad
assert check_types(A) == 0 and check_types(B) == 0, "a value does not match its column type"
for st, S in (("A", A), ("B", B)):
    d = OUT / st; d.mkdir(parents=True, exist_ok=True)
    for rel in RELS:
        with open(d / f"{rel}.csv", "w", newline="") as fh:
            w = csv.writer(fh); w.writerow([c for c, _ in cols[rel]] + ["pdig"])
            for i in sorted(S[rel]):
                w.writerow(S[rel][i] + [pdig(rel, S[rel][i])])
(OUT / "ref.json").write_text(json.dumps(ref))
(OUT / "schema.json").write_text(json.dumps({rel: {"columns": [c for c, _ in cols[rel]], "refs": refs[rel], "payload": [],
                                                   "types": types[rel], "nonneg": nonneg[rel]} for rel in RELS}))
summary = {rel: {k: (v if isinstance(v, int) else len(v)) for k, v in d.items()} for rel, d in ref["relations"].items()}
print(json.dumps({"log": log, "relations": summary,
                  "semantic": {k: (v if isinstance(v, int) else len(v)) for k, v in ref.get("semantic", {}).items()}}, indent=1))
