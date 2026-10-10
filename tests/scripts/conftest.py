"""Tooling controls never publish fixture objects into the operator's lifecycle state."""

import pytest


@pytest.fixture(autouse=True)
def isolated_storage_state(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "storage-state"))
    monkeypatch.setenv("LCTX_STORAGE_CONFIG", str(tmp_path / "storage-host.toml"))
