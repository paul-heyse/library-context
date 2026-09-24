import json, numpy as np
from pathlib import Path
from lctx_mcp.retrieval import Lexical, tokenize
from lctx_mcp.generation import load
from lctx_mcp.embedder import Spec, parse_embeddings, EmbedderError, FAKE, fake_vector

g = load(Path("build/generations/438801c473c4d6d6"), None)
lx = Lexical(g.lexical)
ind = lx.retriever.scores["indptr"]
df2 = {t: int(ind[i + 1] - ind[i]) for t, i in lx.retriever.vocab_dict.items() if t}
print("df from indptr equals ours:", df2 == lx.df, len(df2))

spec = Spec.packaged(FAKE)
v = fake_vector("x", spec.dimensions).tolist()
# boolean index accepted?
body = json.dumps({"model": spec.model, "data": [{"index": True, "embedding": v}, {"index": 0, "embedding": v}]}).encode()
try:
    out = parse_embeddings(spec, 2, body); print("bool index accepted, shape", out.shape)
except EmbedderError as e:
    print("rejected:", e)
# pydantic strict alternative
from pydantic import BaseModel, ConfigDict, ValidationError, NonNegativeInt
class Datum(BaseModel):
    model_config = ConfigDict(strict=True)
    index: NonNegativeInt
    embedding: list[float]
class Emb(BaseModel):
    model_config = ConfigDict(strict=True)
    model: str
    data: list[Datum]
try:
    Emb.model_validate_json(body); print("pydantic accepted bool")
except ValidationError as e:
    print("pydantic strict rejects bool index:", e.errors()[0]["type"])
body2 = json.dumps({"model": spec.model, "data": [{"index": 0, "embedding": [1, 0] }]}).encode()
print("pydantic strict int->float in list:", Emb.model_validate_json(body2).data[0].embedding)
