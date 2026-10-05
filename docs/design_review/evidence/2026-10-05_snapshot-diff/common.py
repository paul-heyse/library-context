"""Shared helpers: load schema/ref, compare a store's diff to the reference."""
import json, pathlib, sys
def load(data_dir):
    d = pathlib.Path(data_dir)
    return json.loads((d / "schema.json").read_text()), json.loads((d / "ref.json").read_text())
def compare(store, ref, got, semantic=None):
    """got: {rel: {"added": set, "removed": set, "changed": set}} of hex ids (lowercase, no dashes)."""
    ok, lines = True, []
    for rel, r in ref["relations"].items():
        g = got.get(rel)
        if g is None:
            lines.append(f"{store} {rel}: MISSING"); ok = False; continue
        for k in ("added", "removed", "changed"):
            exp, act = set(r[k]), {x.replace("-", "").lower() for x in g[k]}
            if exp != act:
                ok = False
                lines.append(f"{store} {rel}.{k}: expected {len(exp)} got {len(act)} "
                             f"(missing {len(exp - act)}, extra {len(act - exp)})")
        lines.append(f"{store} {rel}: added={len(g['added'])} removed={len(g['removed'])} changed={len(g['changed'])}")
    if semantic is not None:
        for k in ("added", "removed", "changed"):
            if set(ref["semantic"][k]) != set(semantic[k]):
                ok = False; lines.append(f"{store} semantic.{k}: expected {len(ref['semantic'][k])} got {len(semantic[k])}")
        lines.append(f"{store} semantic: " + " ".join(f"{k}={len(semantic[k])}" for k in ("added", "removed", "changed")))
    lines.append(f"{store} RESULT {'passed' if ok else 'failed'}")
    print("\n".join(lines)); sys.stdout.flush()
    return ok
