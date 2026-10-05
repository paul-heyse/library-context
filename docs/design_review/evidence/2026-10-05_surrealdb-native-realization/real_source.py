"""Read one relation of a published generation, read-only, as loader rows.

Rows come from `SELECT to_jsonb(t)::text` over the generation schema (psql with FETCH_COUNT, so
psql streams). bytea arrives as "\\x<hex>"; it becomes a hex string (ids, digests) or bytes
(binary). Columns that are not model fields (generation_id, vocabulary introduced_epoch) are
dropped; see README for the mapping gap."""
import json
import subprocess


def schema_of(generation_hex):
    return f"lctx_g{generation_hex}"


def _conv(f, v):
    if v is None:
        return None
    if f["list"]:
        return [_conv(dict(f, list=False), x) for x in v]
    t = f["type"]
    if t in ("id", "digest"):
        return v[2:]
    if t == "binary":
        return bytes.fromhex(v[2:])
    return v


def provider(model, schema, db="lctx", physical=None):
    """physical: optional {relation: physical table} (e.g. a sealed but unmerged vocabulary delta)."""
    physical = physical or {}

    def rows(name):
        table = physical.get(name, name)
        sql = ("SET default_transaction_read_only = on;\n\\set FETCH_COUNT 20000\n"
               f"SELECT to_jsonb(t)::text FROM {schema}.{table} t;\n")
        p = subprocess.run(["psql", "-h", "127.0.0.1", "-U", "lctx_superuser", "-d", db, "-At", "-q",
                            "-v", "ON_ERROR_STOP=1", "-f", "-"], input=sql, capture_output=True, text=True, check=True)
        fields = model.rel[name]["fields"]
        out = []
        for line in p.stdout.split("\n"):  # not splitlines(): JSON text may hold U+2028 etc.
            if not line.startswith("{"):
                continue
            d = json.loads(line)
            r = {"id": d["id"][2:]}
            for f in fields:
                r[f["name"]] = _conv(f, d.get(f["name"]))
            out.append(r)
        return out
    return rows


def facts_frontier(generation_hex):
    """The staging generation's facts frontier: ordinary relations with stage receipts (main tables)
    plus vocabulary relations sealed by their stage but never merged (rows only in the private
    delta table named __delta_<blake3("<stage>/<relation>")[:40]>, ddl.rs delta_name).
    Returns ({relation: physical table}, {relation: receipted row count})."""
    import hashlib  # noqa: F401  (blake3 is not stdlib: computed by `uv run --with blake3`)
    gid = f"decode('{generation_hex}','hex')"
    q = lambda sql: [l for l in subprocess.run(
        ["psql", "-h", "127.0.0.1", "-U", "lctx_superuser", "-d", "lctx", "-At", "-F", "|", "-q", "-v", "ON_ERROR_STOP=1", "-f", "-"],
        input="SET default_transaction_read_only = on;\n" + sql, capture_output=True, text=True, check=True).stdout.splitlines() if "|" in l]
    counts, physical = {}, {}
    for l in q(f"SELECT relation_name, row_count FROM lctx_model_store.stage_receipts WHERE generation_id = {gid};"):
        r, c = l.split("|")
        counts[r] = counts.get(r, 0) + int(c)
    sealed = [l.split("|") for l in q(f"SELECT stage_name, relation_name, row_count FROM lctx_model_store.publication_outputs WHERE generation_id = {gid} AND sealed;")]
    names = subprocess.run(["uv", "run", "--no-project", "-q", "--with", "blake3", "python3", "-c",
                            "import sys,blake3\nfor l in sys.stdin: print(blake3.blake3(l.strip().encode()).hexdigest()[:40])"],
                           input="\n".join(f"{st}/{r}" for st, r, _ in sealed), capture_output=True, text=True, check=True).stdout.split()
    for (st, r, c), h in zip(sealed, names):
        physical[r] = f"__delta_{h}"
        counts[r] = int(c)
    return physical, counts
