#!/usr/bin/env python3
"""Provision this application's database in the existing PG18 cluster; never change HBA.

Run as the operator. sudo authenticates the postgres OS account when required. Secrets are
written only to a mode-0600 config; SQL travels on stdin, never in argv or printed diagnostics.
"""

import argparse
import json
import os
import secrets
import subprocess
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--config", type=Path, default=Path.home() / ".config/library-context/postgres.json"
    )
    parser.add_argument("--port", type=int, default=5432)
    args = parser.parse_args()
    if not 1 <= args.port <= 65535:
        raise SystemExit("Port must be between 1 and 65535")
    if args.config.exists():
        raise SystemExit(
            "Configuration already exists; use its credentials, or review rotation separately."
        )
    runtime_password = secrets.token_hex(32)
    migration_password = secrets.token_hex(32)
    # Passwords are generated hex; all identifiers are application-owned constants.
    sql = rf"""
\set ON_ERROR_STOP on
SET password_encryption = 'scram-sha-256';
DO $$ BEGIN
 IF current_setting('server_version_num')::int / 10000 <> 18 THEN
  RAISE EXCEPTION 'PostgreSQL 18 is required';
 END IF;
 IF EXISTS (SELECT FROM pg_roles WHERE rolname IN ('lctx_app', 'lctx_migrator'))
    OR EXISTS (SELECT FROM pg_database WHERE datname = 'lctx') THEN
  RAISE EXCEPTION 'lctx roles/database already exist: inspect before provisioning; nothing changed';
 END IF;
 CREATE ROLE lctx_app LOGIN PASSWORD '{runtime_password}'
  NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;
 CREATE ROLE lctx_migrator LOGIN PASSWORD '{migration_password}'
  NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;
END $$;
CREATE DATABASE lctx OWNER lctx_migrator;
REVOKE ALL ON DATABASE lctx FROM PUBLIC;
GRANT CONNECT ON DATABASE lctx TO lctx_app;
"""
    args.config.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    # Reserve the secret file before making server changes. On a partial server error retain
    # credentials so an operator can recover; never silently rotate an existing deployment.
    config = {
        "application_url": f"postgresql://lctx_app:{runtime_password}@127.0.0.1:{args.port}/lctx?sslmode=disable",
        "migration_url": f"postgresql://lctx_migrator:{migration_password}@127.0.0.1:{args.port}/lctx?sslmode=disable",
        "max_connections": 6,
        "acquire_timeout_seconds": 5,
        "statement_timeout_seconds": 30,
        "lock_timeout_seconds": 5,
        "max_receipt_bytes": 268435456,
    }
    fd = os.open(args.config, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w") as stream:
        json.dump(config, stream, indent=2)
        stream.write("\n")
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
            f"Bootstrap failed (exit {result.returncode}); credentials retained at {args.config}. "
            "Inspect the cluster before retrying. No HBA changes were made."
        )
    print(f"Provisioned lctx roles/database on PG18 port {args.port}; config: {args.config}")
    print(
        "Next: lctx db migrate, then lctx db check. No runtime schema changes occur automatically."
    )


if __name__ == "__main__":
    main()
