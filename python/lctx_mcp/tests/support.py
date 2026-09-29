"""Test clients and independent canonical inputs; no alternate serving implementation."""

from __future__ import annotations

import asyncio
import json
from collections.abc import Callable
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from tempfile import TemporaryDirectory
from threading import Thread
from typing import Any

import pyarrow as pa
import pyarrow.ipc as ipc
from lctx_semantics import SemanticExecutor, serving_schemas
from lctx_storage import open_repository

from lctx_mcp import operations as ops
from lctx_mcp.generation import NATIVE_IPC_FILES
from lctx_mcp.generation import load as load_pinned
from lctx_mcp.server import Capability, build_server
from postgres_bootstrap import write_secret
from postgres_test_support import ROOT, database
from projection_reference import ReferenceBundle


def expected_schemas(dimensions):
    return {
        name: ipc.open_file(pa.BufferReader(raw)).schema
        for name, raw in serving_schemas(dimensions)
    }


@dataclass(frozen=True)
class NativeReference:
    condition_graph: SemanticExecutor
    tables: dict
    manifest: dict
    snapshot_id: str
    key: str


def load_native(path: Path, _spec=None):
    reference = ReferenceBundle(path)
    manifest = reference.manifest["projection"]
    tables = {name: reference.table(name) for name in reference.manifest["files"]}
    native = SemanticExecutor.from_ipc(
        1,
        manifest["snapshot_id"],
        manifest["entry_value_effect_digest"],
        [(name, (path / f"{name}.arrow").read_bytes()) for name in sorted(NATIVE_IPC_FILES)],
    )
    return NativeReference(
        condition_graph=native,
        tables=tables,
        manifest=manifest,
        snapshot_id=manifest["snapshot_id"],
        key=reference.manifest["projection_generation"],
    )


class PgFixture:
    """Synchronous test calls cross the real PyO3 async repository on an owned event loop."""

    importer: Path
    run: Callable[..., Any]

    def __init__(self, config: Path, path: Path):
        self.config, self.reference = config, ReferenceBundle(path)
        self.manifest = self.reference.manifest
        self.library = self.manifest["projection"]["context"]["library"]
        self.generation = self.manifest["projection_generation"]
        self.loop = asyncio.new_event_loop()
        self.thread = Thread(target=self.loop.run_forever, daemon=True)
        self.thread.start()

        async def open_pin():
            self.repository = await open_repository(config)
            self.pinned = await self.repository.pin(self.library, self.generation, None)
            self.inputs = await self.pinned.inputs()

        self.call(open_pin)

    def call(self, method, *args):
        async def invoke():
            return await method(*args)

        return asyncio.run_coroutine_threadsafe(invoke(), self.loop).result(timeout=40)

    def load(self, spec=None):
        return load_pinned(self.pinned, json.loads(self.pinned.descriptor()), self.inputs, spec)

    def operation(self, gen, snapshot, operation, *, expanded=False):
        return ops.OperationPacket.model_validate_json(
            self.call(self.pinned.get_operation, snapshot, operation, expanded)
        )

    def section(self, snapshot, operation, section):
        """Follow real section cursors; no reconstructed legacy operation packet."""
        cursor = None
        records = []
        while True:
            page = ops.OperationSectionPage.model_validate_json(
                self.call(
                    self.pinned.get_operation,
                    snapshot,
                    operation,
                    True,
                    json.dumps({"kind": "section", "section": section, "cursor": cursor}),
                )
            )
            records.extend(page["items"])
            cursor = page.next_cursor
            if cursor is None:
                return records

    def find(self, gen, selection, limit, cursor):
        result = json.loads(
            self.call(self.pinned.find_operations, selection.model_dump_json(), limit, cursor)
        )
        return ops.SelectionResults.model_validate(result)

    def capability(self, capability_id):
        return Capability.model_validate_json(
            self.call(self.pinned.get_capability, self.manifest["snapshot_id"], capability_id)
        )

    def findings(self):
        return {
            bytes.fromhex(support.finding.finding_id): support.finding
            for brief in self.reference.table("briefs").to_pylist()
            for assertion in self.capability(brief["brief_id"].hex()).assertions
            for support in assertion.supports
            if support.finding is not None
        }

    def server(self, embedder):
        return build_server(self.config, embedder, library=self.library, generation=self.generation)

    def close(self):
        try:
            self.call(self.repository.close)
        finally:
            self.loop.call_soon_threadsafe(self.loop.stop)
            self.thread.join()
            self.loop.close()


@contextmanager
def served_bundle(path: Path):
    """Import a canonical test bundle and exercise production hydration against disposable PG18."""
    with TemporaryDirectory(prefix="lctx-serving-test-") as directory:
        tmp = Path(directory)
        with database(tmp) as (_serving, _command, call, role, _port):
            importer = tmp / "importer.json"
            write_secret(
                importer,
                {
                    **role,
                    "role": "importer",
                    "url": role["url"].replace("lctx_serving:", "lctx_importer:"),
                    "statement_timeout_seconds": 30,
                },
            )
            serving = tmp / "runtime-serving.json"
            write_secret(serving, {**role, "statement_timeout_seconds": 30})
            call(
                [
                    str(ROOT / "target/release/lctx"),
                    "serving",
                    "--importer-config",
                    str(importer),
                    "import-bundle",
                    "--bundle",
                    str(path),
                    "--artifacts",
                    str(tmp / "artifacts"),
                ]
            )
            fixture = PgFixture(serving, path)
            fixture.importer, fixture.run = importer, call
            try:
                yield fixture
            finally:
                fixture.close()
