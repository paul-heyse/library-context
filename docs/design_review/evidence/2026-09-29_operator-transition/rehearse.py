#!/usr/bin/env python3
"""Rehearse the operator transition (cutover plan P1.13) on a disposable PG18 restored from the
operator database. Read-only on the operator database: an owner `pg_dump` of the retained
services, the report schema and the migration history, plus the serving schema without its
rows. Prints one JSON receipt."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
from postgres_backup import connection_env, fingerprints  # noqa: E402
from postgres_bootstrap import bootstrap_sql, configurations, provision_sql, write_secret  # noqa: E402

RETAINED = ("lctx_cache.specs", "lctx_cache.embedding_values", "lctx_ops.attempts", "lctx_ops.events")


def run(args, **kw):
    return subprocess.run(args, capture_output=True, **kw)


def main() -> None:
    operator = json.loads((Path.home() / ".config/library-context/postgres-admin.json").read_text())["migration_url"]
    source = connection_env(operator)
    before = fingerprints(lambda q: run(["psql", "-X", "-qAt", "-c", q], env=source, text=True, check=True).stdout.strip(), RETAINED)
    data = run(["pg_dump", "--no-owner", "--no-privileges", "-n", "lctx_cache", "-n", "lctx_ops", "-n", "lctx_report", "-t", "public._sqlx_migrations"],
               env=source, check=True).stdout
    serving = run(["pg_dump", "--no-owner", "--no-privileges", "--schema-only", "-n", "lctx_serving"], env=source, check=True).stdout
    image = (ROOT / "specs/postgres-vector-image.txt").read_text().strip()
    container = run(["docker", "run", "--rm", "-d", "-p", "127.0.0.1::5432", "-e", "POSTGRES_PASSWORD=rehearsal", image], text=True, check=True).stdout.strip()
    receipt = {"operator_retained": before, "dump_bytes": len(data) + len(serving)}
    try:
        port = int(json.loads(run(["docker", "inspect", container], text=True, check=True).stdout)[0]["NetworkSettings"]["Ports"]["5432/tcp"][0]["HostPort"])
        for _ in range(120):
            if run(["docker", "exec", container, "pg_isready", "-h", "127.0.0.1"]).returncode == 0:
                break
            time.sleep(0.25)
        admin = ["docker", "exec", "-i", container, "psql", "-X", "-qAt", "-U", "postgres", "-v", "ON_ERROR_STOP=1"]
        passwords = {role: "cd" * 32 for role in ("lctx_app", "lctx_migrator", "lctx_importer", "lctx_serving")}
        run(admin, input=bootstrap_sql(passwords).encode(), check=True)
        run(admin + ["-d", "lctx"], input=provision_sql({r: passwords[r] for r in ("lctx_importer", "lctx_serving")}).encode(), check=True)
        owner = connection_env(f"postgres://lctx_migrator:{'cd' * 32}@127.0.0.1:{port}/lctx?sslmode=disable")
        for dump in (serving, data):
            run(["psql", "-X", "-q", "-v", "ON_ERROR_STOP=1"], input=dump, env=owner, check=True)
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            for name, config in configurations(passwords, port).items():
                write_secret(directory / name, config)
            write_secret(directory / "admin.json", {"url": f"postgres://postgres:rehearsal@127.0.0.1:{port}/postgres?sslmode=disable"})
            step = lambda *a: run([sys.executable, str(ROOT / "scripts/postgres_transition.py"), *a, "--config", str(directory / "postgres.json"),  # noqa: E731
                                   "--admin-config", str(directory / "admin.json")], text=True)
            lctx = lambda *a: run([str(ROOT / "target/release/lctx"), "--database", str(directory / "postgres.json"), *a], text=True)  # noqa: E731
            receipt["legacy_install_refused"] = lctx("store", "install").returncode
            for name, args in [("plan", ["plan"]), ("prepare", ["prepare"]), ("switch", ["switch", "--confirm-switch", "lctx"])]:
                result = step(*args)
                receipt[name] = {"exit": result.returncode, "result": json.loads(result.stdout) if result.returncode == 0 else result.stderr.strip()}
                if result.returncode:
                    break
            receipt["store_check"] = lctx("store", "check").returncode
            receipt["runs_list"] = lctx("runs", "list").returncode
    finally:
        run(["docker", "rm", "-f", container])
    print(json.dumps(receipt, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
