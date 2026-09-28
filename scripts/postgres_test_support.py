"""Explicit disposable PostgreSQL fixture shared by runtime boundary tests."""

from __future__ import annotations

import json
import subprocess
import time
from contextlib import contextmanager
from pathlib import Path

from postgres_expand import provision_sql, write_secret

ROOT = Path(__file__).resolve().parents[1]


@contextmanager
def database(tmp_path):
    def call(args, **kw):
        return subprocess.run(args, text=True, capture_output=True, check=True, **kw)

    image = (ROOT / "specs/postgres-vector-image.txt").read_text().strip()
    container = call(
        [
            "docker",
            "run",
            "--rm",
            "-d",
            "-p",
            "127.0.0.1::5432",
            "-e",
            "POSTGRES_PASSWORD=fixture-only",
            image,
        ]
    ).stdout.strip()
    try:
        port = json.loads(call(["docker", "inspect", container]).stdout)[0]["NetworkSettings"][
            "Ports"
        ]["5432/tcp"][0]["HostPort"]
        command = [
            "docker",
            "exec",
            "-i",
            container,
            "psql",
            "-X",
            "-qAt",
            "-U",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
        ]
        for _ in range(120):
            if (
                subprocess.run(
                    ["docker", "exec", container, "pg_isready", "-h", "127.0.0.1"],
                    capture_output=True,
                ).returncode
                == 0
            ):
                break
            time.sleep(0.25)
        call(
            command,
            input=(
                "CREATE ROLE lctx_app LOGIN PASSWORD 'fixture-only'; "
                "CREATE ROLE lctx_migrator LOGIN PASSWORD 'fixture-only'; "
                "CREATE DATABASE lctx OWNER lctx_migrator;"
            ),
        )
        command += ["-d", "lctx"]
        call(
            command, input=provision_sql({r: "ab" * 32 for r in ("lctx_importer", "lctx_serving")})
        )
        config = tmp_path / "postgres.json"
        url = f"postgres://lctx_app:fixture-only@127.0.0.1:{port}/lctx"
        write_secret(
            config,
            dict(
                application_url=url,
                max_connections=2,
                acquire_timeout_seconds=1,
                statement_timeout_seconds=1,
                lock_timeout_seconds=1,
                max_receipt_bytes=268435456,
            ),
        )
        write_secret(
            tmp_path / "postgres-admin.json",
            {"migration_url": url.replace("lctx_app:", "lctx_migrator:")},
        )
        call([str(ROOT / "target/release/lctx"), "--database-config", str(config), "db", "migrate"])
        role = dict(
            format=1,
            role="serving",
            url=f"postgres://lctx_serving:{'ab' * 32}@127.0.0.1:{port}/lctx",
            max_connections=2,
            provider_connections=0,
            acquire_timeout_seconds=1,
            statement_timeout_seconds=1,
            lock_timeout_seconds=1,
        )
        serving = tmp_path / "serving.json"
        write_secret(serving, role)
        yield serving, command, call, role, port
    finally:
        call(["docker", "rm", "-f", container])
