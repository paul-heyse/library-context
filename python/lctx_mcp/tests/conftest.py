"""Current transport controls use the real generation supplied by Rust fixtures."""
import pytest


@pytest.fixture
def anyio_backend() -> str:
    return "asyncio"
