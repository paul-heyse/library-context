from __future__ import annotations

from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[3]
FIXTURE = REPO / "build" / "py-fixture"


# Serving is suspended until cutover phase 5 (plan P1.3): the fixture generation came from the
# removed Delta pipeline and the MCP server no longer starts. Tests that need a served generation
# or the entry points are reported skipped (not_run); pure unit tests still run.
SUSPENDED = "suspended: P5 serving (no fixture generation or entry point after cutover P1.3)"
_SERVED = {"generation", "pg_serving", "catalog_serving"}
_ENTRY_POINTS = {"test_stdio.py"}


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    for item in items:
        if _SERVED & set(getattr(item, "fixturenames", ())) or item.path.name in _ENTRY_POINTS:
            item.add_marker(pytest.mark.skip(reason=SUSPENDED))


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


@pytest.fixture(scope="session")
def pg_serving(generation):
    """Import the canonical fixture through the same real CLI used by Rust consumer tests."""
    from support import served_bundle

    with served_bundle(generation) as fixture:
        yield fixture
