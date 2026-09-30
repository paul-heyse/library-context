"""A local, read-only FastMCP server over the library-utilization catalog (stdio).

Register it with `.mcp.json`; it reads `docs/library-utilization.jsonl` and, when present, the
usage index `build/library-usage.sqlite` that a full `scripts/library_utilization.py --write` run
writes. It never writes either file. All behaviour lives in `library_catalog_hints`; this module
only declares the tools and their schemas. Separate from any product server.
"""

from __future__ import annotations

import argparse
import json
import os
import threading
from pathlib import Path
from typing import Annotated, Any, Literal

os.environ.setdefault("FASTMCP_CHECK_FOR_UPDATES", "off")  # before fastmcp is imported

from fastmcp import FastMCP
from fastmcp.exceptions import ToolError
from mcp_types import ToolAnnotations
from pydantic import Field

import library_catalog_db as db
import library_catalog_hints as hints
import library_unitgraph as unitgraph
import library_utilization as utilization

READ_ONLY = ToolAnnotations(read_only_hint=True, idempotent_hint=True, open_world_hint=False)
ROOT = Path(__file__).resolve().parents[1]

INSTRUCTIONS = (
    "Read-only lookup over this repository's library-utilization catalog: curated capabilities "
    "(which library features the code uses, through which local abstraction, in which files) and, "
    "when built, the complete index of library items the workspace source references. Every "
    "tool returns the full result set; every response carries a `state` and `hints`."
)


class Catalog:
    """Loads the store and answer context, reloading when the files behind them change."""

    def __init__(
        self,
        root: Path,
        jsonl: Path | None = None,
        index: Path | None = None,
        graph_loader=unitgraph.load,
    ) -> None:
        self.root = root
        self.jsonl = jsonl or root / db.JSONL
        self.index = index or root / db.INDEX
        self.graph = hints.GraphCache(root, graph_loader)
        self.lock = threading.Lock()  # the store's connection is shared, so use is serialised
        self._ctx: hints.Context | None = None

    def context(self) -> hints.Context:
        if not self.jsonl.exists():
            raise ToolError(f"the catalog file {self.jsonl} does not exist")
        if self._ctx is None or self._ctx.store.stale:
            keep = self._ctx.index if self._ctx else None
            self._ctx = hints.Context(
                root=self.root,
                store=db.Store(self.jsonl, self.index),
                graph=self.graph,
                index=keep,
                _index_loaded=self._ctx is not None,
            )
        return self._ctx

    def call(self, function, *args, **kwargs) -> dict[str, Any]:
        with self.lock:
            return function(self.context(), *args, **kwargs)


def schema_document() -> dict[str, Any]:
    """Field lists and vocabularies, drawn from the maintainer script so they cannot drift."""
    return {
        "library_fields": utilization.LIBRARY_KEYS,
        "capability_fields": utilization.CAPABILITY_KEYS,
        "library_status": ["used", "test-only", "dormant-only", "declared-unused", "not-used"],
        "capability_status": ["used", "test-only", "dormant-only"],
        "file_roles": [
            "impl",
            "wrapper",
            "consumer",
            "test",
            "dormant",
            "macro-expansion",
            "sql-string",
        ],
        "usage_row": ["file", "package", "version", "path", "leaf", "kind", "via", "lines", "role"],
        "blind_spots": list(db.BLIND_SPOTS),
    }


def build_server(
    root: Path = ROOT,
    jsonl: Path | None = None,
    index: Path | None = None,
    graph_loader=unitgraph.load,
) -> FastMCP:
    catalog = Catalog(root, jsonl, index, graph_loader)
    mcp = FastMCP("library-catalog", instructions=INSTRUCTIONS, mask_error_details=True)

    @mcp.tool(annotations=READ_ONLY)
    def capabilities_by_library() -> dict[str, Any]:
        """Every capability the workspace uses, grouped by library: the local precedent."""
        return catalog.call(hints.capabilities_by_library)

    @mcp.tool(annotations=READ_ONLY)
    def get_library(
        lib: Annotated[str, Field(description="Library key as Cargo spells it, e.g. sqlx")],
    ) -> dict[str, Any]:
        """A library's catalog record with all its capabilities, wrappers, features and skill."""
        return catalog.call(hints.get_library, lib)

    @mcp.tool(annotations=READ_ONLY)
    def library_usage(
        lib: Annotated[str, Field(description="Library key, e.g. sqlx")],
        group_by: Annotated[
            Literal["item", "file"], Field(description="Group rows by item or by file")
        ] = "item",
    ) -> dict[str, Any]:
        """Every item of the library the workspace source references, with all files and lines."""
        return catalog.call(hints.library_usage, lib, group_by)

    @mcp.tool(annotations=READ_ONLY)
    def find_item(
        item: Annotated[
            str,
            Field(
                description="A path, facade spelling, macro with `!`, `Type::method` or bare name"
            ),
        ],
    ) -> dict[str, Any]:
        """Capabilities and resolved usage for an item, whatever spelling the caller has."""
        return catalog.call(hints.find_item, item)

    @mcp.tool(annotations=READ_ONLY)
    def find_by_file(
        path: Annotated[str, Field(description="A repository file or a directory prefix")],
    ) -> dict[str, Any]:
        """Capabilities recorded for a file or directory and every library item its files use."""
        return catalog.call(hints.find_by_file, path)

    @mcp.tool(annotations=READ_ONLY)
    def search_capabilities(
        query: Annotated[str, Field(description="Words or identifiers; matches any, ranked")] = "",
        lib: Annotated[str | None, Field(description="Only this library")] = None,
        skill: Annotated[str | None, Field(description="Only libraries this skill covers")] = None,
        role: Annotated[
            str | None, Field(description="Only capabilities with a file in this role")
        ] = None,
        status: Annotated[str | None, Field(description="Only this capability status")] = None,
        limit: Annotated[
            int | None, Field(description="Cap the results; the response counts what it omits")
        ] = None,
    ) -> dict[str, Any]:
        """Every curated capability matching the query and filters, best first."""
        return catalog.call(hints.search_capabilities, query, lib, skill, role, status, limit)

    @mcp.tool(annotations=READ_ONLY)
    def catalog_gaps(
        kind: Annotated[
            Literal[
                "not_used",
                "declared_unused",
                "pin_delta",
                "no_wrapper",
                "dependency_without_record",
            ],
            Field(description="Which kind of gap to list"),
        ],
    ) -> dict[str, Any]:
        """Libraries and capabilities the catalog records with a gap, all of them."""
        return catalog.call(hints.catalog_gaps, kind)

    @mcp.tool(annotations=READ_ONLY)
    def catalog_status() -> dict[str, Any]:
        """Counts, what the usage index covers, and the blind spots of that index."""
        return catalog.call(hints.catalog_status)

    @mcp.resource("catalog://schema", mime_type="application/json")
    def schema() -> str:
        """Catalog field lists and vocabularies."""
        return json.dumps(schema_document(), indent=2)

    return mcp


def serve(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--jsonl", type=Path)
    parser.add_argument("--index", type=Path)
    args = parser.parse_args(argv)
    build_server(args.root, args.jsonl, args.index).run(transport="stdio", show_banner=False)


if __name__ == "__main__":
    serve()
