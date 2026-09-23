from __future__ import annotations

from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[3]
FIXTURE = REPO / "build" / "py-fixture"


@pytest.fixture
def anyio_backend() -> str:
    return "asyncio"


@pytest.fixture(scope="session")
def generation() -> Path:
    """The fixture generation `just py-fixture` builds from `analysis_shapes` (fake vectors)."""
    current = FIXTURE / "CURRENT"
    if not current.exists():
        raise RuntimeError("no fixture generation: run `just py-fixture` (part of `just py-check`)")
    return FIXTURE / current.read_text().strip()
