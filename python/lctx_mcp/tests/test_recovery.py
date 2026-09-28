"""Complete populated recovery through the deployed CLI and editable native boundary."""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path

from postgres_recovery import artifact_target

ROOT = Path(__file__).resolve().parents[3]


def test_populated_recovery_and_corruption_refusal(pg_serving, tmp_path):
    config = pg_serving.importer.parent
    shutil.copy2(pg_serving.importer, config / "postgres-importer.json")
    pg_serving.run(
        [
            str(ROOT / "target/release/lctx"),
            "serving",
            "--importer-config",
            str(pg_serving.importer),
            "select",
            "--library",
            pg_serving.library,
            "--generation",
            pg_serving.generation,
        ]
    )
    from postgres_backup import connection_env

    migration = json.loads((config / "postgres-admin.json").read_text())["migration_url"]
    subprocess.run(
        ["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"],
        input="ALTER ROLE lctx_migrator SET TimeZone='America/New_York';",
        env=connection_env(migration),
        capture_output=True,
        text=True,
        check=True,
    )
    archive = tmp_path / "recovery.dump"
    command = [sys.executable, str(ROOT / "scripts/postgres_backup.py")]
    result = subprocess.run(
        [*command, "backup", str(archive), "--config", str(config / "postgres.json")],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    receipt = json.loads(archive.with_suffix(".dump.json").read_text())
    assert receipt["format"] == 2
    assert [g["generation"] for g in receipt["inventory"]["generations"]] == [pg_serving.generation]
    assert all(not name.split(".")[1].startswith(("v_", "o_")) for name in receipt["tables"])
    restored = subprocess.run(
        [*command, "restore-drill", str(archive)], capture_output=True, text=True
    )
    assert restored.returncode == 0, restored.stderr
    outcome = json.loads(restored.stdout)
    assert outcome["outcome"] == "passed" and outcome["rto_passed"]
    assert outcome["serving"]["generations"][0]["native_loaded"]
    missing = {**receipt, "inventory": {"format": 1, "generations": []}}
    receipt_path = archive.with_suffix(".dump.json")
    receipt_path.write_text(json.dumps(missing))
    omitted = subprocess.run(
        [*command, "restore-drill", str(archive)], capture_output=True, text=True
    )
    assert omitted.returncode != 0 and "inventory" in omitted.stderr
    receipt_path.write_text(json.dumps(receipt))
    artifact = receipt["inventory"]["generations"][0]["artifacts"][0]
    path = artifact_target(archive.with_suffix(".dump.artifacts"), artifact)
    original = path.read_bytes()
    path.write_bytes(b"corrupt")
    corrupt = subprocess.run(
        [*command, "restore-drill", str(archive)], capture_output=True, text=True
    )
    assert corrupt.returncode != 0 and "artifact" in corrupt.stderr
    path.write_bytes(original)
    receipt_path = archive.with_suffix(".dump.json")
    receipt_path.rename(archive.with_suffix(".dump.json.incomplete"))
    incomplete = subprocess.run(
        [*command, "restore-drill", str(archive)], capture_output=True, text=True
    )
    assert incomplete.returncode != 0
