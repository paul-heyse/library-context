"""Synthetic two-state corpus shaped like the lctx-model key structure, plus the independent
reference diff. Writes CSVs under build/review-probes/snapshot-diff/synthetic/{A,B}/ and ref.json.

Run from the repository root (isolated env, no project sync):
  uv run --no-project --with blake3 python docs/design_review/evidence/2026-10-05_snapshot-diff/gen_synthetic.py [NFILES]

Identity mirrors crates/lctx-model/src/domain (read at f66eb15a):
  source_artifacts key (input, path, content hash) | payload byte_len
  modules          key (source, qualified_name)   [.pyi stubs share the qualified name]
  occurrences      key (source, start, end, kind, role, structural_path)
  callable_entities key Source{declaration occurrence, kind}
  provider_symbols key (provider, context, provider_module, native_key) | payload name, kind
  symbol_declarations key (symbol, declaration occurrence)
  signatures       key (symbol, param_digest) | payload ret   [as the real Signature: parameters digest is a key field]
  signature_parameters key (signature, ordinal) | payload name, kind, has_default
  entity_refs      sum: Callable{callable} | Occurrence{occurrence}
  evidence         key (occurrence, claim) | payload detail
id = BLAKE3(typed length-tagged key)[:16]; pdig = BLAKE3(payload)[:16] (like hash_rows' row digest).

Perturbation A -> B (one input revision, so provider context is stable):
  * file edits: F_SHIFT gets 13 bytes inserted at the top (content and every span change -> cascade);
    F_DEL deletes one function; F_REN renames one function (native key changes).
  * ~1% row level: new evidence/entity_refs (producer change), removed evidence/entity_refs,
    provider_symbols payload changes (same id, new payload), signature changes (a parameter added).
"""
import blake3, csv, json, pathlib, random, sys

NF = int(sys.argv[1]) if len(sys.argv) > 1 else 1000
OCC_PER_FILE, CALL_EVERY = 300, 15
OUT = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "build/review-probes/snapshot-diff/synthetic")
F_SHIFT, F_DEL, F_REN = 7, 11, 13

def key(rel, *parts):
    h = blake3.blake3(b"lctx-model/v1\0" + rel.encode() + b"\0")
    for p in parts:
        b = p if isinstance(p, bytes) else str(p).encode()
        h.update(len(b).to_bytes(8, "little") + b)
    return h.digest()[:16].hex()

def pdig(*parts):
    return key("payload", *parts)

# relation -> (columns, references {column: target relation}, payload columns)
SCHEMA = {
    "source_artifacts": (["id", "input", "path", "content", "byte_len"], {}, ["byte_len"]),
    "modules": (["id", "source", "qualified_name", "stub"], {"source": "source_artifacts"}, []),
    "occurrences": (["id", "source", "start", "end_", "kind", "role", "spath"], {"source": "source_artifacts"}, []),
    "callable_entities": (["id", "declaration", "ckind"], {"declaration": "occurrences"}, []),
    "provider_symbols": (["id", "provider", "context", "pmodule", "native_key", "name", "skind"], {}, ["name", "skind"]),
    "symbol_declarations": (["id", "symbol", "declaration"], {"symbol": "provider_symbols", "declaration": "occurrences"}, []),
    "signatures": (["id", "symbol", "param_digest", "ret"], {"symbol": "provider_symbols"}, ["ret"]),
    "signature_parameters": (["id", "signature", "ordinal", "pname", "pkind", "has_default"], {"signature": "signatures"}, ["pname", "pkind", "has_default"]),
    "entity_refs": (["id", "code", "callable", "occurrence"], {"callable": "callable_entities", "occurrence": "occurrences"}, []),
    "evidence": (["id", "occurrence", "claim", "detail"], {"occurrence": "occurrences"}, ["detail"]),
}
PROVIDER, CONTEXT, INPUT = key("providers", "pyrefly"), key("analysis_contexts", "ctx"), key("input_revisions", "in")

def build(state):
    R = {r: {} for r in SCHEMA}
    def put(rel, row):
        R[rel][row["id"]] = row
    for f in range(NF):
        path = f"pkg/m{f // 50}/f{f}.py"
        qname = path[:-3].replace("/", ".")
        edited = state == "B" and f in (F_SHIFT, F_DEL, F_REN)
        shift = 13 if state == "B" and f == F_SHIFT else 0
        content = key("bytes", path, "v2" if edited else "v1")
        src = key("source_artifacts", INPUT, path, content)
        put("source_artifacts", dict(id=src, input=INPUT, path=path, content=content, byte_len=3000 * 10 + shift))
        put("modules", dict(id=key("modules", src, qname), source=src, qualified_name=qname, stub=0))
        if f % 20 == 0:  # a stub with the same qualified name (distinct module identity, section 15.3)
            spath = path + "i"
            sc = key("bytes", spath, "v1")
            ssrc = key("source_artifacts", INPUT, spath, sc)
            put("source_artifacts", dict(id=ssrc, input=INPUT, path=spath, content=sc, byte_len=500))
            put("modules", dict(id=key("modules", ssrc, qname), source=ssrc, qualified_name=qname, stub=1))
        pmod = key("provider_modules", PROVIDER, CONTEXT, qname)
        for i in range(OCC_PER_FILE):
            is_call = i % CALL_EVERY == 0
            if state == "B" and f == F_DEL and i == 30:
                continue  # deleted function (its declaration occurrence goes)
            start, end, kind, spath = 100 * i + shift, 100 * i + 40 + (i % 7) + shift, (0 if is_call else 1 + i % 40), f"{i // 20}.{i % 20}"
            occ = key("occurrences", src, start, end, kind, 0, spath)
            put("occurrences", dict(id=occ, source=src, start=start, end_=end, kind=kind, role=0, spath=spath))
            if i % 3 == 0:
                ev = key("evidence", occ, "claim")
                put("evidence", dict(id=ev, occurrence=occ, claim="claim", detail=f"d{f}.{i}"))
            if i % 5 == 1:
                put("entity_refs", dict(id=key("entity_refs", 5, occ), code=5, callable="", occurrence=occ))
            if not is_call:
                continue
            ce = key("callable_entities", 0, occ, 0)
            put("callable_entities", dict(id=ce, declaration=occ, ckind=0))
            put("entity_refs", dict(id=key("entity_refs", 1, ce), code=1, callable=ce, occurrence=""))
            fname = f"f{i}" + ("_renamed" if state == "B" and f == F_REN and i == 45 else "")
            nk = f"{qname}.{fname}"
            sym = key("provider_symbols", PROVIDER, CONTEXT, pmod, nk)
            put("provider_symbols", dict(id=sym, provider=PROVIDER, context=CONTEXT, pmodule=pmod, native_key=nk, name=fname, skind=1))
            put("symbol_declarations", dict(id=key("symbol_declarations", sym, occ), symbol=sym, declaration=occ))
            params = [(f"a{k}", 1, 0) for k in range(1 + int(sym[:8], 16) % 4)]
            pd = pdig(*[p for t in params for p in t])
            sig = key("signatures", sym, pd)  # the real Signature keys on its parameters digest
            put("signatures", dict(id=sig, symbol=sym, param_digest=pd, ret="None"))
            for o, (n, k, d) in enumerate(params):
                put("signature_parameters", dict(id=key("signature_parameters", sig, o), signature=sig, ordinal=o, pname=n, pkind=k, has_default=d))
    if state == "B":
        perturb(R)
    return R

def perturb(R):
    rng = random.Random(7)
    occs = sorted(R["occurrences"])
    # ~1% removals of leaf rows (no referrers)
    for rel in ("evidence", "entity_refs"):
        for i in rng.sample(sorted(R[rel]), len(R[rel]) // 100):
            del R[rel][i]
    # ~1% additions from a "changed producer"
    for rel, n in (("evidence", len(R["evidence"]) // 100), ("entity_refs", len(R["entity_refs"]) // 100)):
        for occ in rng.sample(occs, n):
            if rel == "evidence":
                ev = key("evidence", occ, "claim2")
                R[rel][ev] = dict(id=ev, occurrence=occ, claim="claim2", detail="new")
            else:
                i = key("entity_refs", 6, occ)
                R[rel][i] = dict(id=i, code=6, callable="", occurrence=occ)
    # ~1% payload modifications with stable identity
    for i in rng.sample(sorted(R["provider_symbols"]), len(R["provider_symbols"]) // 100):
        R["provider_symbols"][i]["skind"] = 2
    for i in rng.sample(sorted(R["evidence"]), len(R["evidence"]) // 100):
        R["evidence"][i]["detail"] = R["evidence"][i]["detail"] + "*"
    # semantic signature change: a parameter added to ~1% of signatures. The parameters digest is
    # a key field (as in the real Signature), so the signature and its parameters get new ids.
    for sig in rng.sample(sorted(R["signatures"]), len(R["signatures"]) // 100):
        old = R["signatures"].pop(sig)
        plist = []
        for o in range(64):
            pid = key("signature_parameters", sig, o)
            if pid not in R["signature_parameters"]:
                break
            r = R["signature_parameters"].pop(pid)
            plist.append((r["pname"], r["pkind"], r["has_default"]))
        plist.append(("extra", 3, 1))
        pd = pdig(*[p for t in plist for p in t])
        nsig = key("signatures", old["symbol"], pd)
        R["signatures"][nsig] = dict(old, id=nsig, param_digest=pd)
        for o, (n, k, d) in enumerate(plist):
            pid = key("signature_parameters", nsig, o)
            R["signature_parameters"][pid] = dict(id=pid, signature=nsig, ordinal=o, pname=n, pkind=k, has_default=d)
    # deleted function: drop rows that referenced removed occurrences (cascade)
    live = set(R["occurrences"])
    for rel in ("evidence", "entity_refs", "callable_entities", "symbol_declarations"):
        for i, row in list(R[rel].items()):
            if any(row.get(c) and t == "occurrences" and row[c] not in live for c, t in SCHEMA[rel][1].items()):
                del R[rel][i]
    live_ce = set(R["callable_entities"])
    for i, row in list(R["entity_refs"].items()):
        if row["callable"] and row["callable"] not in live_ce:
            del R["entity_refs"][i]
    live_sym = {r["symbol"] for r in R["symbol_declarations"].values()}
    for i in [i for i in R["provider_symbols"] if i not in live_sym]:
        del R["provider_symbols"][i]
    for i in [i for i, r in R["signatures"].items() if r["symbol"] not in R["provider_symbols"]]:
        del R["signatures"][i]
    for i in [i for i, r in R["signature_parameters"].items() if r["signature"] not in R["signatures"]]:
        del R["signature_parameters"][i]

def rowdig(rel, row):
    return pdig(*[row[c] for c in SCHEMA[rel][2]])

def check_refs(R):
    for rel, (_, refs, _) in SCHEMA.items():
        for row in R[rel].values():
            for c, t in refs.items():
                assert not row[c] or row[c] in R[t], (rel, c, row)

def semantic(R):
    """Correspondence key (module qualified name, native key) -> signature param digest."""
    out = {}
    for s in R["signatures"].values():
        sym = R["provider_symbols"][s["symbol"]]
        k = sym["native_key"]
        assert k not in out, f"correspondence key collision {k}"
        out[k] = s["param_digest"]
    return out

def main():
    states = {s: build(s) for s in ("A", "B")}
    ref = {"nfiles": NF, "relations": {}}
    for s, R in states.items():
        check_refs(R)
        d = OUT / s
        d.mkdir(parents=True, exist_ok=True)
        for rel, (cols, _, _) in SCHEMA.items():
            with open(d / f"{rel}.csv", "w", newline="") as fh:
                w = csv.writer(fh)
                w.writerow(cols + ["pdig"])
                for i in sorted(R[rel]):
                    row = R[rel][i]
                    w.writerow([row[c] for c in cols] + [rowdig(rel, row)])
    A, B = states["A"], states["B"]
    for rel in SCHEMA:
        a, b = set(A[rel]), set(B[rel])
        changed = sorted(i for i in a & b if rowdig(rel, A[rel][i]) != rowdig(rel, B[rel][i]))
        va = {(i, rowdig(rel, A[rel][i])) for i in a}
        vb = {(i, rowdig(rel, B[rel][i])) for i in b}
        ref["relations"][rel] = dict(rows_a=len(a), rows_b=len(b), added=sorted(b - a), removed=sorted(a - b),
                                     changed=changed, shared_versions=len(va & vb), union_versions=len(va | vb))
    sa, sb = semantic(A), semantic(B)
    ref["semantic"] = dict(added=sorted(sb.keys() - sa.keys()), removed=sorted(sa.keys() - sb.keys()),
                           changed=sorted(k for k in sa.keys() & sb.keys() if sa[k] != sb[k]),
                           unchanged=len([k for k in sa.keys() & sb.keys() if sa[k] == sb[k]]))
    mods = [r["qualified_name"] for r in A["modules"].values()]
    ref["module_qname_duplicates"] = len(mods) - len(set(mods))
    ref["semantic_spec"] = {"sig": "signatures", "sym_ref": "symbol", "sym": "provider_symbols",
                            "key_sym": ["native_key"], "key_sig": [], "digest": "param_digest"}
    (OUT / "ref.json").write_text(json.dumps(ref))
    INTS = {"byte_len", "stub", "start", "end_", "kind", "role", "ckind", "skind", "ordinal", "pkind", "has_default", "code"}
    TEXT = {"path", "qualified_name", "spath", "native_key", "name", "ret", "pname", "claim", "detail"}
    (OUT / "schema.json").write_text(json.dumps({rel: {"columns": cols, "refs": refs, "payload": pay,
        "nonneg": [c for c in cols if c in INTS],
        "types": {c: "int" if c in INTS else "text" if c in TEXT else "id" for c in cols + ["pdig"]}}
        for rel, (cols, refs, pay) in SCHEMA.items()}))
    tot = {k: sum(v[k] if isinstance(v[k], int) else len(v[k]) for v in ref["relations"].values())
           for k in ("rows_a", "rows_b", "added", "removed", "changed", "shared_versions", "union_versions")}
    summary = {"totals": tot, "per_relation": {r: {k: (v if isinstance(v, int) else len(v)) for k, v in d.items()}
                                               for r, d in ref["relations"].items()},
               "semantic": {k: (v if isinstance(v, int) else len(v)) for k, v in ref["semantic"].items()},
               "semantic_examples": {k: ref["semantic"][k][:3] for k in ("added", "removed", "changed")},
               "module_qname_duplicates": ref["module_qname_duplicates"]}
    print(json.dumps(summary, indent=1))

if __name__ == "__main__":
    main()
