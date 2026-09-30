"""Inventory, preserve services, and rebuild only regenerable generation state for Phase 3."""

import json
import subprocess
import sys
from pathlib import Path

from pilots import CLI, HERE, RAW, ROOT, sql

sys.path.insert(0, str(ROOT / "scripts"))
from postgres_backup import fingerprints


def main() -> None:
    receipt = {"service_fingerprints_before": fingerprints(sql)}
    receipt["generations_before"] = json.loads(sql("SELECT coalesce(json_agg(t),'[]'::json) FROM (SELECT encode(id,'hex') AS generation,state,frontier FROM lctx_model_store.generations ORDER BY id) t"))
    receipt["active_readers_before"] = json.loads(sql("SELECT coalesce(json_agg(t),'[]'::json) FROM (SELECT usename,state,application_name FROM pg_stat_activity WHERE datname='lctx' AND pid<>pg_backend_pid()) t"))
    assert not receipt["active_readers_before"], "quiesce project readers before reset"
    receipt["selection_before"] = sql("SELECT count(generation_id) FROM lctx_model_store.selection")
    assert receipt["selection_before"] == "0"
    path = Path.home() / ".config/library-context/postgres-importer.json"
    assert path.stat().st_mode & 0o077 == 0
    config = json.loads(path.read_text())
    receipt["importer_before"] = {key: value for key, value in config.items() if key != "url"}
    config.update(max_connections=10, provider_connections=8)
    path.write_text(json.dumps(config, indent=2) + "\n")
    path.chmod(0o600)
    receipt["importer_after"] = {key: value for key, value in config.items() if key != "url"}
    for label, args in [("reset", ["store", "reset", "--confirm", "lctx"]), ("check-before", ["store", "check"])]:
        result = subprocess.run([str(CLI), *args], cwd=ROOT, capture_output=True, check=False)
        (RAW / f"{label}.stdout").write_bytes(result.stdout)
        (RAW / f"{label}.stderr").write_bytes(result.stderr)
        receipt[label] = {"command": ["target/release/lctx", *args], "exit": result.returncode}
        assert result.returncode == 0, result.stderr.decode()
    receipt["service_fingerprints_after"] = fingerprints(sql)
    assert receipt["service_fingerprints_before"] == receipt["service_fingerprints_after"]
    receipt["outcome"] = "passed"
    (HERE / "migration-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print("passed: generation reset, current store, unchanged retained services, importer 8+2 slots")


if __name__ == "__main__":
    main()
