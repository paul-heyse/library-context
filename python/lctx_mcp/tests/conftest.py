"""Current transport controls use the real native snapshot supplied by Rust fixtures."""

import pytest


@pytest.fixture
def anyio_backend() -> str:
    return "asyncio"
