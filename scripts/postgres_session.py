"""Explicit PostgreSQL evaluation lifespan pinned to a verified portable reference identity."""

import json
from contextlib import asynccontextmanager
from pathlib import Path

from lctx_storage import open_repository

from lctx_mcp.generation import load
from lctx_mcp.server import NativeWorkers, serve
from projection_reference import ReferenceBundle

DEFAULT_CONFIG = Path.home() / ".config/library-context/postgres-serving.json"


@asynccontextmanager
async def session(bundle: Path, embedder, config: Path = DEFAULT_CONFIG):
    workers = NativeWorkers()
    repository = await open_repository(config)
    try:
        reference = await workers.run(ReferenceBundle, bundle)
        pinned = await repository.pin(
            reference.manifest["library"], reference.manifest["projection_generation"]
        )
        state = await workers.run(
            load,
            pinned,
            json.loads(pinned.descriptor()),
            await pinned.inputs(),
            embedder.spec if embedder else None,
        )
        yield await workers.run(serve, state, embedder, workers), reference
    finally:
        try:
            await workers.close()
        finally:
            await repository.close()
