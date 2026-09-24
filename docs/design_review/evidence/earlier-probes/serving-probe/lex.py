from pathlib import Path
from collections import Counter
from lctx_mcp.generation import load
from lctx_mcp.retrieval import tokenize
g = load(Path("build/generations/438801c473c4d6d6"), None)
for bid, text in zip(g.brief_ids, g.lexical):
    title = g.briefs[bid]["title"]
    body, names = text.rsplit("\n", 1)
    c = Counter(tokenize(names))
    if title.endswith((".tool", ".http_app", ".from_function", ".run")):
        print(title, "| name tokens:", len(tokenize(names)), "| body tokens:", len(tokenize(body)), "|", dict(c.most_common(6)))
import json
print(json.dumps(1e-7), json.dumps(0.1+0.2), json.dumps({"é":"\u0001"}, ensure_ascii=False, separators=(",",":")))
