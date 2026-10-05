"""Explicit real-PG functional checks of the Python async service boundary."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest

from postgres_backup import connection_env
from postgres_bootstrap import bootstrap_sql, configurations, write_secret

ROOT = Path(__file__).resolve().parents[2]


@pytest.fixture
def database(tmp_path):
    from postgres_test_support import database as provision

    with provision(tmp_path) as fixture:
        yield fixture


def test_current_bootstrap_split_credentials_and_existing_database_refusal(database, tmp_path):
    _serving, command, call, _role, port = database
    passwords = {
        role: "ab" * 32 for role in ("lctx_app", "lctx_migrator", "lctx_importer", "lctx_serving")
    }
    folder = tmp_path / "fresh-configs"
    folder.mkdir()
    for name, config in configurations(passwords, int(port)).items():
        path = folder / name
        write_secret(path, config)
        assert path.stat().st_mode & 0o777 == 0o600
        url_keys = set(config) & {"url", "application_url", "migration_url"}
        assert len(url_keys) == 1
        connection = connection_env(config[next(iter(url_keys))])
        user = call(
            ["psql", "-X", "-qAt"], env=connection, input="SELECT current_user;"
        ).stdout.strip()
        assert user == connection["PGUSER"]
    call(
        [
            "just",
            "store-check",
            "--database",
            str(folder / "postgres.json"),
        ]
    )
    with pytest.raises(subprocess.CalledProcessError) as refused:
        call(command, input=bootstrap_sql(passwords))
    assert "already exist" in refused.value.stderr
    # Refusing an existing deployment cannot rotate or elevate any role.
    assert (
        call(
            command,
            input="SELECT count(*) FROM pg_roles WHERE rolname LIKE 'lctx_%' "
            "AND (rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls);",
        ).stdout.strip()
        == "0"
    )


def test_retained_service_backup_roundtrip_and_corrupt_receipt_refusal(database, tmp_path):
    import sys

    from postgres_backup import TABLES

    archive = tmp_path / "services.dump"
    command = [sys.executable, str(ROOT / "scripts/postgres_backup.py")]
    config = tmp_path / "postgres.json"
    made = subprocess.run(
        [*command, "backup", str(archive), "--config", str(config)], capture_output=True, text=True
    )
    assert made.returncode == 0, made.stderr
    receipt_path = archive.with_suffix(".dump.json")
    receipt = json.loads(receipt_path.read_text())
    assert receipt["format"] == 4 and receipt["scope"] == "retained-services"
    assert set(receipt["tables"]) == set(TABLES)
    restored = subprocess.run(
        [*command, "restore-drill", str(archive)], capture_output=True, text=True
    )
    assert restored.returncode == 0, restored.stderr
    assert json.loads(restored.stdout)["semantic_generations"] == "rebuild_from_pinned_inputs"
    receipt_path.write_text(json.dumps({**receipt, "tables": {}}))
    refused = subprocess.run(
        [*command, "restore-drill", str(archive)], capture_output=True, text=True
    )
    assert refused.returncode != 0 and "unsupported" in refused.stderr
