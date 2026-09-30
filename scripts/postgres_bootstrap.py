#!/usr/bin/env python3
"""Provision this application's database in the existing PG18 cluster; never change HBA.

Run as the operator. sudo authenticates the postgres OS account when required. Secrets are
written only to separate mode-0600 configs. SQL travels on stdin, never in argv or diagnostics.
"""

import argparse
import json
import os
import secrets
import subprocess
from collections.abc import Mapping
from pathlib import Path


def write_secret(path: Path, data: dict) -> None:
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w") as stream:
        json.dump(data, stream, indent=2)
        stream.write("\n")


def provision_sql(passwords: Mapping[str, str]) -> str:
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


def bootstrap_sql(passwords: Mapping[str, str]) -> str:
    if set(passwords) != {"lctx_app", "lctx_migrator", "lctx_importer", "lctx_serving"} or any(
        len(value) != 64 or any(c not in "0123456789abcdef" for c in value)
        for value in passwords.values()
    ):
        raise ValueError("invalid provisioning credentials")
    return rf"""
\set ON_ERROR_STOP on
SET password_encryption = 'scram-sha-256';
DO $$ BEGIN
 IF current_setting('server_version_num')::int / 10000 <> 18 THEN
  RAISE EXCEPTION 'PostgreSQL 18 is required';
 END IF;
 IF NOT EXISTS (SELECT FROM pg_available_extension_versions
                WHERE name='vector' AND version='0.8.6') THEN
  RAISE EXCEPTION 'Install pgvector 0.8.6 before provisioning';
 END IF;
 IF EXISTS (SELECT FROM pg_roles WHERE rolname IN
            ('lctx_app','lctx_migrator','lctx_importer','lctx_serving'))
    OR EXISTS (SELECT FROM pg_database WHERE datname='lctx') THEN
  RAISE EXCEPTION 'lctx roles/database already exist: inspect before provisioning; nothing changed';
 END IF;
 CREATE ROLE lctx_app LOGIN PASSWORD '{passwords["lctx_app"]}'
  NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
 CREATE ROLE lctx_migrator LOGIN PASSWORD '{passwords["lctx_migrator"]}'
  NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
END $$;
CREATE DATABASE lctx OWNER lctx_migrator;
REVOKE ALL ON DATABASE lctx FROM PUBLIC;
GRANT CONNECT ON DATABASE lctx TO lctx_app;
"""


def configurations(passwords: Mapping[str, str], port: int) -> dict[str, dict]:
    # Validate the same closed credentials used by the SQL before constructing URLs.
    bootstrap_sql(passwords)
    if not 1 <= port <= 65535:
        raise ValueError("port must be between 1 and 65535")

    def url(role: str) -> str:
        return f"postgresql://{role}:{passwords[role]}@127.0.0.1:{port}/lctx?sslmode=disable"

    limits = {
        "acquire_timeout_seconds": 5,
        "statement_timeout_seconds": 30,
        "lock_timeout_seconds": 5,
    }
    result = {
        "postgres.json": {
            "application_url": url("lctx_app"),
            "max_connections": 6,
            "max_receipt_bytes": 268435456,
            **limits,
        },
        "postgres-admin.json": {"migration_url": url("lctx_migrator")},
    }
    # The reader keeps two of its connections for generation-bound provider sessions (T13).
    for role, connections, provider in [("importer", 2, 0), ("serving", 6, 2)]:
        result[f"postgres-{role}.json"] = {
            "format": 1,
            "role": role,
            "url": url(f"lctx_{role}"),
            "max_connections": connections,
            "provider_connections": provider,
            **limits,
        }
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--config", type=Path, default=Path.home() / ".config/library-context/postgres.json"
    )
    parser.add_argument("--port", type=int, default=5432)
    args = parser.parse_args()
    if not 1 <= args.port <= 65535:
        raise SystemExit("Port must be between 1 and 65535")
    passwords = {
        role: secrets.token_hex(32)
        for role in ("lctx_app", "lctx_migrator", "lctx_importer", "lctx_serving")
    }
    files = {
        args.config if name == "postgres.json" else args.config.parent / name: data
        for name, data in configurations(passwords, args.port).items()
    }
    if len(files) != 4 or any(path.exists() or path.is_symlink() for path in files):
        raise SystemExit(
            "Configuration already exists or names collide; inspect before provisioning"
        )
    args.config.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    # Reserve separate credentials before server changes. Keep them on a partial failure;
    # this is not an automatic rotation or an old-generation recovery path.
    for path, data in files.items():
        write_secret(path, data)
    sql = (
        bootstrap_sql(passwords)
        + "\n\\connect lctx\n"
        + provision_sql({role: passwords[role] for role in ("lctx_importer", "lctx_serving")})
    )
    result = subprocess.run(
        ["sudo", "-u", "postgres", "psql", "-X", "-p", str(args.port), "-d", "postgres"],
        input=sql,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        # PostgreSQL error context can contain the password-bearing DO statement.
        raise SystemExit(
            f"Bootstrap failed (exit {result.returncode}); protected credentials retained in "
            f"{args.config.parent}. Inspect the cluster before retrying. No HBA changes were made."
        )
    print(f"Provisioned lctx database, four roles and pgvector 0.8.6 on PG18 port {args.port}")
    print(f"Separate protected configurations written in {args.config.parent}")
    print(
        "Next: lctx store install, then lctx store check. "
        "No runtime migrations occur automatically."
    )


if __name__ == "__main__":
    main()
