"""Tooling controls never publish fixture objects into the operator's lifecycle state."""

import pytest


@pytest.fixture(autouse=True)
def isolated_storage_state(tmp_path, monkeypatch):
    # A managed launcher may retain admission for its real lifecycle registry.
    # These controls deliberately select a different registry; that capability
    # cannot be transferred into it. Individual inheritance controls supply their
    # own matching admission after this fixture establishes the test's scope.
    monkeypatch.delenv("LCTX_STORAGE_ADMISSION", raising=False)
    # Nested harness controls must create their own run receipts, rather than fold
    # temporary capture state into the managed run executing this test suite.
    monkeypatch.delenv("LCTX_RUN_DIR", raising=False)
    monkeypatch.delenv("LCTX_RUN_ID", raising=False)
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "storage-state"))
    monkeypatch.setenv("LCTX_STORAGE_CONFIG", str(tmp_path / "storage-host.toml"))
