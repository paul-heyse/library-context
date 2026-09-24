import asyncio, sys
from pathlib import Path
sys.path.insert(0, "scripts")
import score_gold
from lctx_mcp.embedder import Spec, QWEN, EmbedderError, fake_vector
import numpy as np

class Flaky:
    def __init__(self, url, timeout=0):
        self.spec = Spec.packaged(QWEN); self.n = 0
    async def embed(self, texts):
        self.n += 1
        if self.n % 2 == 1:
            raise EmbedderError("service hiccup")
        return np.stack([fake_vector(t, 4096) for t in texts])

score_gold.HttpEmbedder = Flaky
out = asyncio.run(score_gold.score(Path("build/generations/438801c473c4d6d6"), "vllm", "x", score_gold.SOURCES))
print("label:", out["label"], "| mode:", out["mode"], "| aliases:", out["b"]["aliases"])
