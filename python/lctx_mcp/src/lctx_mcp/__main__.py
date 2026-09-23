"""`python -m lctx_mcp --generation DIR [--embedder vllm|fake|none] [--embed-url URL]`.

Serves one generation over stdio (DESIGN §11.3). Nothing is written to stdout but the protocol:
FastMCP's update check is switched off before it is imported, and its banner is not shown.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="lctx-mcp", description=__doc__)
    parser.add_argument("--generation", type=Path, required=True, help="a generation directory")
    parser.add_argument("--embedder", choices=["vllm", "fake", "none"], default="vllm")
    parser.add_argument("--embed-url", default="http://127.0.0.1:8000")
    args = parser.parse_args(argv)
    os.environ["FASTMCP_CHECK_FOR_UPDATES"] = "off"

    from lctx_mcp.embedder import FakeEmbedder, HttpEmbedder
    from lctx_mcp.server import build_server

    embedder = {
        "vllm": lambda: HttpEmbedder(args.embed_url),
        "fake": FakeEmbedder,
        "none": lambda: None,
    }[args.embedder]()
    build_server(args.generation.resolve(), embedder).run(transport="stdio", show_banner=False)


if __name__ == "__main__":
    main()
