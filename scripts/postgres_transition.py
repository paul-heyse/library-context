#!/usr/bin/env python3
"""The operator database transition (semantic-model cutover plan P1.13, T11).

A database whose migration history predates the service baseline is refused by `lctx`, never
migrated in place. This moves its retained services into a fresh database, offline:

  plan           read-only: the migration history, retained rows and their fingerprints, and
                 the legacy schemas that will stay behind
  prepare        create `lctx_next`, install the store there with `lctx store install`, copy
                 the retained service rows and verify their fingerprints
  switch         with no connection to either database, rename `lctx` to an archive and
                 `lctx_next` to `lctx`, then check the result
  drop-retired   drop an archive: a separate, explicit operator step

The old database is never written; `switch` only renames it. Server administration runs as the
PostgreSQL superuser through `sudo -u postgres psql` on `--port`, or through a protected (0600)
`--admin-config` JSON file holding `{"url": ...}`. Owner work runs as the configured migrator.
"""

from __future__ import annotations

import argparse
import datetime
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from urllib.parse import urlsplit, urlunsplit

sys.path.insert(0, str(Path(__file__).resolve().parent))
from postgres_backup import connection_env, fingerprints, protected  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
# Parents before children: events reference attempts, values reference specs.
RETAINED = ("lctx_cache.specs", "lctx_cache.embedding_values", "lctx_ops.attempts", "lctx_ops.events")
DATABASE = "lctx"
NEXT = "lctx_next"
ARCHIVE = re.compile(r"^lctx_retired_\d{14}$")


class Refused(Exception):
    """A refusal: nothing was changed. Exit status 2."""


def with_database(url: str, database: str) -> str:
    parts = urlsplit(url)
    return urlunsplit(parts._replace(path=f"/{database}"))


class Admin:
    """SQL as the PostgreSQL superuser, on a named database."""

    def __init__(self, admin_config: Path | None, port: int) -> None:
        self.url = None
        if admin_config is not None:
            protected(admin_config)
            self.url = json.loads(admin_config.read_text())["url"]
        self.port = port

    def sql(self, database: str, statement: str) -> str:
        if self.url is None:
            command = ["sudo", "-u", "postgres", "psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1", "-p", str(self.port), "-d", database]
            env = None
        else:
            command = ["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"]
            env = connection_env(with_database(self.url, database))
        result = subprocess.run(command, input=statement, text=True, capture_output=True, env=env, check=False)
        if result.returncode:
            raise RuntimeError(f"administration failed on {database} (exit {result.returncode})")
        return result.stdout.strip()


class Owner:
    """SQL and COPY as the service owner (the migrator) on one database."""

    def __init__(self, migration_url: str, database: str) -> None:
        self.env = connection_env(with_database(migration_url, database))

    def query(self, statement: str) -> str:
        result = subprocess.run(["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"], input=statement, text=True,
                                capture_output=True, env=self.env, check=False)
        if result.returncode:
            raise RuntimeError("owner query failed")
        return result.stdout.strip()

    def copy_out(self, table: str, columns: list[str]) -> bytes:
        statement = f"COPY (SELECT {', '.join(columns)} FROM {table}) TO STDOUT"
        result = subprocess.run(["psql", "-X", "-q", "-v", "ON_ERROR_STOP=1", "-c", statement], capture_output=True, env=self.env, check=False)
        if result.returncode:
            raise RuntimeError(f"reading {table} failed")
        return result.stdout

    def copy_in(self, table: str, columns: list[str], data: bytes) -> None:
        statement = f"COPY {table} ({', '.join(columns)}) FROM STDIN"
        result = subprocess.run(["psql", "-X", "-q", "-v", "ON_ERROR_STOP=1", "-c", statement], input=data, capture_output=True, env=self.env, check=False)
        if result.returncode:
            raise RuntimeError(f"writing {table} failed")

    def columns(self, table: str) -> list[str]:
        schema, name = table.split(".")
        listed = self.query(f"SELECT string_agg(column_name, ',' ORDER BY ordinal_position) FROM information_schema.columns "
                            f"WHERE table_schema = '{schema}' AND table_name = '{name}';")
        return listed.split(",") if listed else []

    def history(self) -> list[int]:
        if self.query("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL;") != "t":
            return []
        listed = self.query("SELECT string_agg(version::text, ',' ORDER BY version) FROM public._sqlx_migrations;")
        return [int(v) for v in listed.split(",")] if listed else []


def configs(config: Path) -> dict[str, dict]:
    files = {}
    for name in ("postgres.json", "postgres-admin.json", "postgres-serving.json", "postgres-importer.json"):
        path = config.with_name(name)
        if path.exists():
            protected(path)
            files[name] = json.loads(path.read_text())
    if "postgres-admin.json" not in files:
        raise Refused("the owner's postgres-admin.json is required beside the configuration")
    return files


def redirected(files: dict[str, dict], database: str, directory: Path) -> Path:
    """Protected copies of the configuration naming `database`."""
    directory.chmod(0o700)
    for name, data in files.items():
        data = dict(data)
        for key in ("application_url", "migration_url", "url"):
            if key in data:
                data[key] = with_database(data[key], database)
        path = directory / name
        path.write_text(json.dumps(data))
        path.chmod(0o600)
    return directory / "postgres.json"


def lctx(binary: Path, config: Path, *args: str) -> subprocess.CompletedProcess:
    env = {k: v for k, v in os.environ.items() if k != "LCTX_DATABASE_CONFIG"}
    return subprocess.run([str(binary), "--database", str(config), *args], capture_output=True, text=True, env=env, check=False)


def plan(files: dict[str, dict], admin: Admin) -> dict:
    owner = Owner(files["postgres-admin.json"]["migration_url"], DATABASE)
    history = owner.history()
    present = [t for t in RETAINED if owner.query(f"SELECT to_regclass('{t}') IS NOT NULL;") == "t"]
    # Every schema the owner owns stays behind except the retained services.
    legacy = owner.query("SELECT coalesce(string_agg(nspname, ',' ORDER BY nspname), '') FROM pg_namespace "
                         "WHERE nspowner = (SELECT oid FROM pg_roles WHERE rolname = current_user) AND nspname NOT IN ('lctx_cache', 'lctx_ops');")
    return {
        "database": DATABASE,
        "history": history,
        "legacy_history": bool(history) and min(history) < 202609300014,
        "retained": fingerprints(owner.query, tuple(present)),
        "left_behind": [s for s in legacy.split(",") if s],
        # Any role may read pg_database: the plan needs no server administration.
        "next_exists": owner.query(f"SELECT EXISTS (SELECT FROM pg_database WHERE datname = '{NEXT}');") == "t",
    }


def prepare(files: dict[str, dict], admin: Admin, binary: Path) -> dict:
    planned = plan(files, admin)
    if not planned["legacy_history"]:
        raise Refused(f"{DATABASE} carries no legacy migration history; there is nothing to transition")
    if planned["next_exists"]:
        raise Refused(f"{NEXT} already exists; inspect or drop it before preparing again")
    admin.sql("postgres", f"CREATE DATABASE {NEXT} OWNER lctx_migrator;")
    admin.sql(NEXT, f"REVOKE ALL ON DATABASE {NEXT} FROM PUBLIC; GRANT CONNECT ON DATABASE {NEXT} TO lctx_app, lctx_importer, lctx_serving; "
                    f"GRANT TEMP ON DATABASE {NEXT} TO lctx_importer;")
    with tempfile.TemporaryDirectory() as directory:
        installed = lctx(binary, redirected(files, NEXT, Path(directory)), "store", "install")
        if installed.returncode:
            raise RuntimeError(f"`lctx store install` on {NEXT} failed: {installed.stderr.strip()}")
    old = Owner(files["postgres-admin.json"]["migration_url"], DATABASE)
    new = Owner(files["postgres-admin.json"]["migration_url"], NEXT)
    for table in planned["retained"]:
        columns = new.columns(table)
        new.copy_in(table, columns, old.copy_out(table, columns))
    copied = fingerprints(new.query, tuple(planned["retained"]))
    if copied != planned["retained"]:
        raise RuntimeError(f"retained fingerprints differ after the copy; {NEXT} kept for inspection")
    return {"prepared": NEXT, "retained": copied}


def switch(files: dict[str, dict], admin: Admin, binary: Path, config: Path) -> dict:
    if admin.sql("postgres", f"SELECT EXISTS (SELECT FROM pg_database WHERE datname = '{NEXT}');") != "t":
        raise Refused(f"{NEXT} does not exist; run prepare first")
    busy = admin.sql("postgres", f"SELECT count(*) FROM pg_stat_activity WHERE datname IN ('{DATABASE}', '{NEXT}') AND pid <> pg_backend_pid();")
    if busy != "0":
        raise Refused(f"{busy} connections to {DATABASE} or {NEXT}; stop every reader and writer first")
    archive = f"lctx_retired_{datetime.datetime.now(datetime.UTC).strftime('%Y%m%d%H%M%S')}"
    retained = fingerprints(Owner(files["postgres-admin.json"]["migration_url"], NEXT).query, RETAINED)
    admin.sql("postgres", f"ALTER DATABASE {DATABASE} RENAME TO {archive};")
    admin.sql("postgres", f"ALTER DATABASE {NEXT} RENAME TO {DATABASE};")
    switched = fingerprints(Owner(files["postgres-admin.json"]["migration_url"], DATABASE).query, RETAINED)
    checked = lctx(binary, config, "store", "check")
    runs = lctx(binary, config, "runs", "list")
    if switched != retained or checked.returncode or runs.returncode:
        raise RuntimeError(f"the switched database did not verify; the archive {archive} is intact")
    return {"archive": archive, "retained": switched, "store_check": "clean", "runs": "listed"}


def drop_retired(admin: Admin, name: str) -> dict:
    if not ARCHIVE.match(name):
        raise Refused(f"{name} is not a transition archive")
    if admin.sql("postgres", f"SELECT EXISTS (SELECT FROM pg_database WHERE datname = '{name}');") != "t":
        raise Refused(f"{name} does not exist")
    admin.sql("postgres", f"DROP DATABASE {name};")
    return {"dropped": name}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=["plan", "prepare", "switch", "drop-retired"])
    parser.add_argument("--config", type=Path, default=Path(os.environ.get("LCTX_DATABASE_CONFIG", str(Path.home() / ".config/library-context/postgres.json"))))
    parser.add_argument("--admin-config", type=Path)
    parser.add_argument("--port", type=int, default=5432)
    parser.add_argument("--lctx", type=Path, default=ROOT / "target/release/lctx")
    parser.add_argument("--confirm-switch")
    parser.add_argument("--confirm-drop")
    args = parser.parse_args()
    admin = Admin(args.admin_config, args.port)
    try:
        if args.command == "drop-retired":
            if not args.confirm_drop:
                raise Refused("name the archive with --confirm-drop")
            result = drop_retired(admin, args.confirm_drop)
        else:
            files = configs(args.config)
            if args.command == "plan":
                result = plan(files, admin)
            elif args.command == "prepare":
                result = prepare(files, admin, args.lctx)
            else:
                if args.confirm_switch != DATABASE:
                    raise Refused(f"confirm the switch with --confirm-switch {DATABASE}")
                result = switch(files, admin, args.lctx, args.config)
    except Refused as refusal:
        print(f"refused: {refusal}", file=sys.stderr)
        sys.exit(2)
    except (OSError, subprocess.SubprocessError, RuntimeError, KeyError, json.JSONDecodeError) as error:
        # Utility error text can carry credentials or row payloads.
        reason = str(error) if isinstance(error, RuntimeError) else type(error).__name__
        raise SystemExit(f"failed: transition {args.command} ({reason})") from None
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
