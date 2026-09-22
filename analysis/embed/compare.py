"""Conformance: Rust vs Python vectors; batch-composition variance; retrieval sanity."""
import json, sys
import numpy as np
sys.path.insert(0, ".")
import py_client as P

inputs = json.load(open("conformance_inputs.json"))
py = {k: np.array(v, dtype=np.float64) for k, v in json.load(open("py_vectors.json"))["vectors"].items()}
rs = {k: np.array(v, dtype=np.float64) for k, v in json.load(open("rs_vectors.json"))["vectors"].items()}
ids = [i["id"] for i in inputs]
cos = {k: float(py[k] @ rs[k] / (np.linalg.norm(py[k]) * np.linalg.norm(rs[k]))) for k in ids}
maxabs = {k: float(np.max(np.abs(py[k] - rs[k]))) for k in ids}
print("rust vs python: min cosine %.9f, max |diff| %.3e" % (min(cos.values()), max(maxabs.values())))

url = "http://localhost:8011"
texts = [P.format_input(i["kind"], i["text"]) for i in inputs]
single = {i["id"]: np.array(P.embed(url, [t])[0]) for i, t in zip(inputs, texts)}
rev = P.embed(url, list(reversed(texts)))
rev = {i["id"]: np.array(v) for i, v in zip(reversed(inputs), rev)}
for name, other in [("single-input batches", single), ("reversed batch", rev)]:
    c = min(float(py[k] @ other[k]) for k in ids)
    d = max(float(np.max(np.abs(py[k] - other[k]))) for k in ids)
    print(f"batch variance vs full batch ({name}): min cosine {c:.9f}, max |diff| {d:.3e}")

Q = [k for k in ids if k.startswith("q")]
D = [k for k in ids if k.startswith("d")]
for q in Q:
    s = {d: float(py[q] @ py[d]) for d in D}
    best = max(s, key=s.get)
    print(q, "->", best, {d: round(v, 4) for d, v in s.items()})
