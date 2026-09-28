"""PR0 original-evidence MCP baseline; no generated facts, labels or skill corpus.

Read Git objects at an explicit commit, never working-tree additions. B and C use the same
read/search surface; C supplies the separately pinned structured product tools as well.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from fastmcp import FastMCP
from lctx_mcp.retrieval import Lexical

SUFFIXES = {".py", ".pyi", ".md", ".mdx", ".rst", ".toml", ".yaml", ".yml", ".json", ".txt"}


def git(source: Path, *args: str) -> bytes:
    return subprocess.run(["git", "-C", str(source), *args], check=True,
                          capture_output=True, timeout=60).stdout


class Evidence:
    def __init__(self, source: Path, commit: str):
        if len(commit) != 40 or any(c not in "0123456789abcdef" for c in commit):
            raise ValueError("full Git commit required")
        if git(source, "rev-parse", commit + "^{commit}").decode().strip() != commit:
            raise ValueError("source revision mismatch")
        self.commit = commit
        self.files: dict[str, str] = {}
        total = 0
        for entry in git(source, "ls-tree", "-rz", commit).split(b"\0"):
            if not entry:
                continue
            meta, raw_path = entry.split(b"\t", 1)
            mode, kind, blob = meta.split()
            path = raw_path.decode()
            if kind != b"blob" or mode == b"120000" or Path(path).suffix not in SUFFIXES:
                continue
            content = git(source, "cat-file", "blob", blob.decode())
            total += len(content)
            if total > 128 * 1024 * 1024:
                raise ValueError("original corpus exceeds byte budget")
            try:
                self.files[path] = content.decode()
            except UnicodeDecodeError:
                continue
        self.chunks = []
        for path, text in self.files.items():
            lines = text.splitlines()
            for offset in range(0, len(lines), 60):
                self.chunks.append((path, offset + 1, "\n".join(lines[offset:offset + 80])))
        self.index = Lexical([path + "\n" + text for path, _, text in self.chunks])

    def inventory(self) -> dict:
        files = {p: hashlib.sha256(t.encode()).hexdigest() for p, t in self.files.items()}
        return {"commit": self.commit, "files": files,
                "sha256": hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest(),
                "basis": "original Git blobs; plain text; no derived API relationships"}

    def search(self, query: str, limit: int = 8) -> dict:
        if not 1 <= limit <= 20 or not query.strip() or len(query) > 4096:
            raise ValueError("query/limit outside bounds")
        scores = self.index.scores(query)
        ranked = sorted(range(len(scores)), key=lambda i: (-float(scores[i]), self.chunks[i][0], self.chunks[i][1]))
        result = []
        for i in ranked:
            if scores[i] <= 0 or len(result) == limit:
                break
            path, start, text = self.chunks[i]
            result.append({"path": path, "line": start, "text": text[:6000],
                           "truncated": len(text) > 6000, "commit": self.commit})
        return {"results": result, "complete": False, "basis": "ranked original evidence"}

    def read(self, path: str, start: int = 1, lines: int = 80) -> dict:
        if path not in self.files or start < 1 or not 1 <= lines <= 200:
            raise ValueError("unknown original path or invalid line range")
        original = self.files[path].splitlines()
        text = "\n".join(original[start - 1:start - 1 + lines])
        if len(text.encode()) > 32768:
            raise ValueError("source range exceeds budget; request fewer lines")
        return {"commit": self.commit, "path": path, "start": start,
                "end": min(len(original), start - 1 + lines), "text": text,
                "sha256": hashlib.sha256(self.files[path].encode()).hexdigest()}


def server(evidence: Evidence) -> FastMCP:
    mcp = FastMCP("Original pinned library evidence")
    mcp.tool(evidence.search, name="search_original", annotations={"readOnlyHint": True})
    mcp.tool(evidence.read, name="read_original", annotations={"readOnlyHint": True})
    return mcp


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--port", type=int, default=8091)
    args = parser.parse_args()
    evidence = Evidence(args.source, args.commit)
    with args.inventory.open("x") as output:
        json.dump(evidence.inventory(), output, indent=2, sort_keys=True)
    server(evidence).run(transport="http", host="127.0.0.1", port=args.port)


if __name__ == "__main__":
    main()
