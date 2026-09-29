"""Replace only this project's operator schemas after both current profiles qualify."""
import asyncio
import json
from pathlib import Path
import subprocess
import sys

from packet_probe import check
import tempfile

ROOT = Path.cwd()
sys.path.insert(0, str(ROOT / "scripts"))
from postgres_backup import connection_env, protected


def main():
    pilots, output = (ROOT / argument for argument in sys.argv[1:3])
    rows = json.loads((pilots / "receipt.json").read_text())
    assert {r["profile"] for r in rows} == {"catalog", "behavioral"}
    assert all(r["outcome"] == "passed" and all(j["outcome"] == "passed" for j in r["journeys"]) for r in rows)
    assert json.loads((pilots / "restore.json").read_text())["outcome"] == "passed"
    output.mkdir(exist_ok=False)
    config = Path.home() / ".config/library-context"
    admin = config / "postgres-admin.json"
    protected(admin)
    env = connection_env(json.loads(admin.read_text())["migration_url"])
    assert env["PGDATABASE"] == "lctx" and env["PGUSER"] == "lctx_migrator"

    def call(label, command, **kwargs):
        result = subprocess.run(command, capture_output=True, text=True, **kwargs)
        (output / (label + ".log")).write_text(result.stdout + result.stderr)
        if result.returncode:
            raise RuntimeError(f"{label} failed; inspect the local cutover log")
        return result.stdout

    psql = ["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"]
    active = call("quiescence", psql, env=env, input="SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid();")
    assert int(active.strip()) == 0, "project readers/writers must stop before replacement"
    call("replace-schemas", psql, env=env, input="BEGIN; DROP SCHEMA lctx_report,lctx_serving,lctx_cache,lctx_ops CASCADE; DROP TABLE public._sqlx_migrations; COMMIT;")
    cli = [str(ROOT / "target/release/lctx"), "--database-config", str(config / "postgres.json")]
    publisher = [*cli, "serving", "--importer-config", str(config / "postgres-importer.json")]
    call("migrate", [*cli, "db", "migrate"])
    call("check", [*cli, "db", "check"])
    evidence = []
    for row in rows:
        generation = Path(row["generation"])
        name = row["profile"]
        call(name + "-import", [*publisher, "import-bundle", "--bundle", str(generation)])
        call(name + "-reconcile", [*publisher, "reconcile", "--generation", row["projection_generation"]])
        call(name + "-smoke", [sys.executable, "-m", "lctx_mcp.smoke", str(generation), "--embedder", "vllm", "--config", str(config / "postgres-serving.json")])
        evidence.append(asyncio.run(check(generation, config / "postgres-serving.json", "vllm")))
    selected = next(r["projection_generation"] for r in rows if r["profile"] == "behavioral")
    selection = json.loads(call("select", [*publisher, "select", "--library", "fastmcp", "--generation", selected]))
    assert selection["selected"] and selection["generation"] == selected
    current = call("inventory", psql, env=env, input="SELECT encode(generation_digest,'hex') FROM lctx_serving.generations ORDER BY generation_digest;")
    assert set(current.split()) == {r["projection_generation"] for r in rows}
    with tempfile.TemporaryDirectory(prefix="lctx-pr5-current-reconstruction-") as temporary:
        backup = Path(temporary) / "current.dump"
        call("current-backup", [sys.executable, str(ROOT / "scripts/postgres_backup.py"), "backup", str(backup), "--config", str(config / "postgres.json")])
        restored = json.loads(call("current-restore", [sys.executable, str(ROOT / "scripts/postgres_backup.py"), "restore-drill", str(backup)]))
        assert restored["outcome"] == "passed"
    receipt = {"outcome": "passed", "selection": selection, "current_generations": len(rows), "obsolete_generations": 0, "evidence": evidence, "reconstruction": restored}
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt))


if __name__ == "__main__":
    main()
