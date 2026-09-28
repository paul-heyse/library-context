#!/usr/bin/env python3
"""Explicit PG8-PG11 administrator upgrade of the existing lctx PG18 database.

Preserve a PG7 backup first. No HBA changes or automatic runtime migrations. Passwords and
SQL go through protected files/stdin/environment, never argv or printed diagnostics.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import secrets
import subprocess
from pathlib import Path
from urllib.parse import quote, urlsplit, urlunsplit

from postgres_backup import connection_env, protected

PACKAGE = "postgresql-18-pgvector=0.8.6-1.pgdg24.04+2"


def write_secret(path: Path, data: dict) -> None:
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w") as stream:
        json.dump(data, stream, indent=2)
        stream.write("\n")


def run_private(args: list[str], *, text: str | None = None, env=None) -> None:
    result = subprocess.run(args, input=text, text=True, env=env, capture_output=True, check=False)
    if result.returncode:
        raise SystemExit(
            f"Upgrade step failed (exit {result.returncode}); protected pending files retained. "
            "No credentials printed."
        )


def provision_sql(passwords: dict[str, str]) -> str:
    # Fixed identifiers and generated hex passwords only; never accept interpolated SQL input.
    if set(passwords) != {"lctx_importer", "lctx_serving"} or any(
        len(v) != 64 or any(c not in "0123456789abcdef" for c in v) for v in passwords.values()
    ):
        raise ValueError("invalid provisioning credentials")
    return f"""\n\\set ON_ERROR_STOP on
BEGIN;
SET password_encryption = 'scram-sha-256';
DO $$ BEGIN
 IF current_setting('server_version_num')::int / 10000 <> 18 THEN
  RAISE EXCEPTION 'PostgreSQL 18 required';
 END IF;
 IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname='lctx_importer') THEN
  CREATE ROLE lctx_importer LOGIN PASSWORD '{passwords["lctx_importer"]}'
   NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
 END IF;
 IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname='lctx_serving') THEN
  CREATE ROLE lctx_serving LOGIN PASSWORD '{passwords["lctx_serving"]}'
   NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
 END IF;
 IF EXISTS (SELECT FROM pg_roles WHERE rolname IN ('lctx_importer','lctx_serving')
     AND (rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls)) THEN
  RAISE EXCEPTION 'runtime role has elevated privileges';
 END IF;
END $$;
CREATE SCHEMA IF NOT EXISTS lctx_ext AUTHORIZATION postgres;
REVOKE ALL ON SCHEMA lctx_ext FROM PUBLIC;
CREATE EXTENSION IF NOT EXISTS vector WITH SCHEMA lctx_ext VERSION '0.8.6';
DO $$ BEGIN
 IF NOT EXISTS (SELECT FROM pg_extension e JOIN pg_namespace n ON n.oid=e.extnamespace
    WHERE e.extname='vector' AND e.extversion='0.8.6' AND n.nspname='lctx_ext') THEN
  RAISE EXCEPTION 'extension version/schema mismatch';
 END IF;
END $$;
GRANT USAGE ON SCHEMA lctx_ext TO lctx_app,lctx_migrator,lctx_importer,lctx_serving;
GRANT CONNECT ON DATABASE lctx TO lctx_importer,lctx_serving;
REVOKE TEMP ON DATABASE lctx FROM PUBLIC;
GRANT TEMP ON DATABASE lctx TO lctx_importer;
ALTER ROLE lctx_serving SET default_transaction_read_only = on;
COMMIT;
"""


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--config", type=Path, default=Path.home() / ".config/library-context/postgres.json"
    )
    parser.add_argument(
        "--backup", type=Path, default=Path("build/postgresql-pg8-baseline-6403b60/pg7.dump")
    )
    args = parser.parse_args()
    protected(args.config)
    protected(args.backup)
    receipt = args.backup.with_suffix(args.backup.suffix + ".json")
    protected(receipt)
    with args.backup.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if json.loads(receipt.read_text())["sha256"] != digest:
        raise SystemExit("PG7 backup checksum mismatch")
    settings = json.loads(args.config.read_text())
    parts = urlsplit(settings["application_url"])
    if parts.hostname not in {"127.0.0.1", "localhost", "::1"} or parts.path != "/lctx":
        raise SystemExit("This upgrade is for the existing local lctx database")
    folder = args.config.parent
    pending = folder / "postgres-expansion-pending.json"
    if not pending.exists():
        if "migration_url" not in settings:
            raise SystemExit(
                "Already split configuration; run lctx db migrate/check with the new binary"
            )
        write_secret(
            pending,
            {
                "migration_url": settings["migration_url"],
                "passwords": {r: secrets.token_hex(32) for r in ("lctx_importer", "lctx_serving")},
            },
        )
    protected(pending)
    state = json.loads(pending.read_text())
    subprocess.run(["sudo", "apt-get", "install", "-y", PACKAGE], check=True)
    run_private(
        ["sudo", "-u", "postgres", "psql", "-X", "-p", str(parts.port or 5432), "-d", "lctx"],
        text=provision_sql(state["passwords"]),
    )
    for role, password in state["passwords"].items():
        host = parts.hostname
        if ":" in host:
            host = f"[{host}]"
        url = urlunsplit(
            (
                parts.scheme,
                f"{role}:{quote(password, safe='')}@{host}:{parts.port or 5432}",
                parts.path,
                parts.query,
                "",
            )
        )
        run_private(
            ["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"],
            text="SELECT 1;",
            env=connection_env(url),
        )
        path = folder / f"postgres-{role.removeprefix('lctx_')}.json"
        data = {
            "format": 1,
            "role": role.removeprefix("lctx_"),
            "url": url,
            "max_connections": 2 if role == "lctx_importer" else 6,
            "provider_connections": 0,
            "acquire_timeout_seconds": 5,
            "statement_timeout_seconds": 30,
            "lock_timeout_seconds": 5,
        }
        if not path.exists():
            write_secret(path, data)
        else:
            protected(path)
            if json.loads(path.read_text()) != data:
                raise SystemExit("Role config differs; inspect before replacing")
    admin = folder / "postgres-admin.json"
    if not admin.exists():
        write_secret(admin, {"migration_url": state["migration_url"]})
    else:
        protected(admin)
        if json.loads(admin.read_text()) != {"migration_url": state["migration_url"]}:
            raise SystemExit("Admin config differs")
    settings.pop("migration_url", None)
    temporary = folder / "postgres.json.expanded"
    if temporary.exists():
        raise SystemExit("Pending config replacement exists; inspect it")
    write_secret(temporary, settings)
    os.replace(temporary, args.config)
    print("Provisioned pgvector 0.8.6 and importer/serving roles; split protected credentials.")
    print(
        "Next: target/release/lctx db migrate, then target/release/lctx db check. "
        "No schema migration ran automatically."
    )


if __name__ == "__main__":
    main()
