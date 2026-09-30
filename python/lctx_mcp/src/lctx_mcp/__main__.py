"""`python -m lctx_mcp --library NAME [--config FILE] [--generation DIGEST]`.

Serves one generation over stdio (DESIGN §11.3). Nothing is written to stdout but the protocol:
FastMCP's update check is switched off before it is imported, and its banner is not shown.
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

UNAVAILABLE = 3


def main(argv: list[str] | None = None) -> None:
    """Serving is suspended by the semantic-model cutover until phase 5 (plan P1.1)."""
    print("lctx-mcp: unavailable until cutover phase 5 serves typed generations", file=sys.stderr)
    raise SystemExit(UNAVAILABLE)


def serve(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="lctx-mcp", description=__doc__)
    parser.add_argument(
        "--config", type=Path, default=Path.home() / ".config/library-context/postgres-serving.json"
    )
    parser.add_argument("--library", required=True)
    parser.add_argument(
        "--generation", help="full ready generation digest; default: selected at startup"
    )
    parser.add_argument("--profile", help="qualified retrieval profile digest")
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
    build_server(
        args.config.resolve(),
        embedder,
        library=args.library,
        generation=args.generation,
        profile=args.profile,
    ).run(transport="stdio", show_banner=False)


if __name__ == "__main__":
    main()
