"""Query-time embedding client (httpx) for the ADR-0010 spike. Applies the spec, validates, writes JSON."""
import json, math, sys
import httpx

SPEC = {
    "model": "Qwen/Qwen3-Embedding-8B",
    "dimensions": 4096,
    "query_template": "Instruct: {task_description}\nQuery:{query}",
    "task_description": "Given a coding task, retrieve capability briefs of a Python library that solve it",
}

def format_input(kind: str, text: str) -> str:
    if kind == "query":
        return SPEC["query_template"].format(task_description=SPEC["task_description"], query=text)
    return text

def embed(url: str, texts: list[str]) -> list[list[float]]:
    r = httpx.post(f"{url}/v1/embeddings", json={"model": SPEC["model"], "input": texts, "encoding_format": "float"}, timeout=120)
    r.raise_for_status()
    body = r.json()
    if body.get("model") != SPEC["model"]:
        raise ValueError(f"model mismatch: {body.get('model')}")
    data = body["data"]
    if len(data) != len(texts) or sorted(d["index"] for d in data) != list(range(len(texts))):
        raise ValueError("wrong count or index mapping")
    out = [None] * len(texts)
    for d in data:
        v = d["embedding"]
        if len(v) != SPEC["dimensions"] or not all(math.isfinite(x) for x in v):
            raise ValueError("wrong length or non-finite value")
        out[d["index"]] = v
    return out

if __name__ == "__main__":
    url, inputs, dest = sys.argv[1], json.load(open(sys.argv[2])), sys.argv[3]
    texts = [format_input(i["kind"], i["text"]) for i in inputs]
    vecs = embed(url, texts)
    norms = {i["id"]: math.sqrt(sum(x * x for x in v)) for i, v in zip(inputs, vecs)}
    json.dump({"vectors": {i["id"]: v for i, v in zip(inputs, vecs)}, "norms": norms, "texts": texts}, open(dest, "w"), ensure_ascii=False)
    print(json.dumps(norms))
