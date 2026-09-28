"""Explicit real-PG functional checks of the Python async service boundary."""

from __future__ import annotations

import asyncio
import json
import os
import subprocess
import time
from pathlib import Path

import pytest

from postgres_backup import connection_env
from postgres_expand import provision_sql, write_secret

pytestmark = pytest.mark.skipif(
    os.environ.get("LCTX_POSTGRES_TEST") != "1", reason="explicit real PostgreSQL functional check"
)
ROOT = Path(__file__).resolve().parents[2]


@pytest.fixture
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


def test_async_lifetime_cancellation_and_sanitized_errors(database, tmp_path):
    from lctx_storage import StorageError, open_repository

    serving, command, call, role, port = database

    async def scenario():
        repo = await open_repository(serving)
        answers = await asyncio.gather(*(repo.check() for _ in range(10)))
        assert all(json.loads(answer)["role"] == "lctx_serving" for answer in answers)
        # Hold a lock on a real query dependency; cancellation must release/discard the lease.
        env = connection_env(f"postgres://postgres:fixture-only@127.0.0.1:{port}/lctx")
        lock = subprocess.Popen(
            ["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"],
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        assert lock.stdin is not None and lock.stdout is not None
        try:
            lock.stdin.write(
                "BEGIN; LOCK TABLE public._sqlx_migrations IN ACCESS EXCLUSIVE MODE; "
                "SELECT 'locked';\n"
            )
            lock.stdin.flush()
            assert await asyncio.to_thread(lock.stdout.readline) == "locked\n"
            pending = repo.check()
            beats = 0
            for _ in range(10):
                await asyncio.sleep(0.01)
                beats += 1
            assert not pending.done() and beats == 10
            pending.cancel()
            with pytest.raises(asyncio.CancelledError):
                await pending
            for _ in range(50):
                count = await asyncio.to_thread(
                    lambda: call(
                        command,
                        input=(
                            "SELECT count(*) FROM pg_stat_activity "
                            "WHERE usename='lctx_serving' AND state='active';"
                        ),
                    ).stdout.strip()
                )
                if count == "0":
                    break
                await asyncio.sleep(0.05)
            assert count == "0", "cancelled work must finish or disconnect before reuse"
            lock.stdin.write("ROLLBACK;\n\\q\n")
            lock.stdin.flush()
            await asyncio.to_thread(lock.wait, 5)
        finally:
            if lock.poll() is None:
                lock.terminate()
                lock.wait(timeout=5)
        assert json.loads(await repo.check())["schema_current"]
        await repo.close()
        await repo.close()
        with pytest.raises(StorageError) as stopped:
            await repo.check()
        assert stopped.value.kind == "unavailable"
        wrong = tmp_path / "wrong.json"
        write_secret(
            wrong,
            {
                **role,
                "url": "postgres://lctx_serving:do-not-leak@remote.invalid/lctx?sslmode=disable",
            },
        )
        with pytest.raises(StorageError) as rejected:
            await open_repository(wrong)
        assert rejected.value.kind == "incompatible"
        assert "do-not-leak" not in str(rejected.value)

    asyncio.run(scenario())
