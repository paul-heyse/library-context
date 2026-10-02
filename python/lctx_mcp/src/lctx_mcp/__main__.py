"""Serve a current Rust generation over stdio; stdout contains only the MCP protocol."""

from __future__ import annotations

import argparse
import os
from pathlib import Path


def generation_key(value: str) -> str:
    try:
        decoded = bytes.fromhex(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("generation must be 32 hexadecimal characters") from exc
    if len(value) != 32 or len(decoded) != 16:
        raise argparse.ArgumentTypeError("generation must be 32 hexadecimal characters")
    return decoded.hex()


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="lctx-mcp", description=__doc__)
    parser.add_argument(
        "--config", type=Path, default=Path.home() / ".config/library-context/postgres-serving.json"
    )
    parser.add_argument(
        "--generation",
        type=generation_key,
        help="actual 16-byte generation key; default: selected at startup",
    )
    parser.add_argument(
        "--library", help="operator default library; tool requests still name their library"
    )
    parser.add_argument(
        "--vectors", action="store_true", help="read an explicitly prepared vector artifact"
    )
    parser.add_argument("--embedder", choices=["vllm", "fake", "none"], default="none")
    parser.add_argument("--embed-url", default="http://127.0.0.1:8000")
    parser.add_argument(
        "--embedding-spec",
        type=Path,
        help="configured query model's explicit canonical embedding spec",
    )
    args = parser.parse_args(argv)
    if args.embedder == "vllm" and args.embedding_spec is None:
        parser.error("--embedder vllm requires --embedding-spec")
    if args.embedding_spec is not None and args.embedder != "vllm":
        parser.error("--embedding-spec applies to --embedder vllm")
    if args.embedder != "none" and not args.vectors:
        parser.error("a query embedder requires --vectors")
    os.environ["FASTMCP_CHECK_FOR_UPDATES"] = "off"

    from lctx_mcp.embedder import FakeEmbedder, HttpEmbedder, Spec
    from lctx_mcp.server import build_server

    embedder = None
    if args.embedder == "fake":
        embedder = FakeEmbedder()
    elif args.embedder == "vllm":
        spec = Spec.from_json(args.embedding_spec.read_text(encoding="utf-8"))
        embedder = HttpEmbedder(args.embed_url, spec=spec)
    build_server(
        args.config.resolve(),
        embedder,
        library=args.library,
        generation=args.generation,
        vectors=args.vectors,
    ).run(transport="stdio", show_banner=False)


if __name__ == "__main__":
    main()
