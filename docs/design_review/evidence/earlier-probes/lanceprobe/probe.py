import hashlib, os, sys, shutil
import numpy as np, pyarrow as pa, lancedb
def build(root):
    shutil.rmtree(root, ignore_errors=True)
    db = lancedb.connect(root)
    rng = np.random.default_rng(0)
    v = rng.standard_normal((2000, 64)).astype("float32")
    t = pa.table({"id": pa.array(range(2000), pa.int64()), "text": [f"op {i} serialize json" for i in range(2000)],
                  "vector": pa.FixedSizeListArray.from_arrays(pa.array(v.ravel()), 64)})
    tbl = db.create_table("ops", t)
    tbl.create_fts_index("text")
    return tbl
def listing(root):
    out = {}
    for d, _, fs in os.walk(root):
        for f in fs:
            p = os.path.join(d, f); out[os.path.relpath(p, root)] = hashlib.sha256(open(p,'rb').read()).hexdigest()[:12]
    return out
a = build("a"); b = build("b")
la, lb = listing("a"), listing("b")
print("lancedb", lancedb.__version__, "files a/b:", len(la), len(lb))
print("same relative names:", sorted(la) == sorted(lb))
same = [k for k in la if lb.get(k) == la[k]]
print("byte-identical files:", len(same), "of", len(la))
for k in sorted(la)[:8]: print(" ", k, la[k], lb.get(k))
q = np.random.default_rng(1).standard_normal(64).astype("float32")
ra = a.search(q).limit(5).to_arrow()["id"].to_pylist(); rb = b.search(q).limit(5).to_arrow()["id"].to_pylist()
print("flat search equal:", ra == rb, ra)
