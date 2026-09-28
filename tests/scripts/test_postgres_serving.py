"""Explicit real-PG functional checks of the Python async service boundary."""

from __future__ import annotations

import asyncio
import json
import os
import subprocess
from pathlib import Path

import pytest

from postgres_backup import connection_env
from postgres_expand import write_secret

pytestmark = pytest.mark.skipif(
    os.environ.get("LCTX_POSTGRES_TEST") != "1", reason="explicit real PostgreSQL functional check"
)
ROOT = Path(__file__).resolve().parents[2]


@pytest.fixture
def database(tmp_path):
    from postgres_test_support import database as provision

    with provision(tmp_path) as fixture:
        yield fixture


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


def test_cancelled_server_work_retains_one_slot_until_statement_deadline(database, tmp_path):
    from lctx_storage import open_repository

    _serving, command, call, role, _port = database
    config = tmp_path / "one-slot.json"
    write_secret(config, {**role, "max_connections": 1, "statement_timeout_seconds": 2})

    async def scenario():
        repo = await open_repository(config)
        call(
            command,
            input=(
                "ALTER TABLE public._sqlx_migrations RENAME TO retained_migrations; "
                "CREATE VIEW public._sqlx_migrations AS SELECT m.* "
                "FROM public.retained_migrations m CROSS JOIN pg_sleep(5); "
                "GRANT SELECT ON public._sqlx_migrations TO lctx_serving;"
            ),
        )
        pending = repo.check()
        try:
            for _ in range(100):
                sleeping = await asyncio.to_thread(
                    lambda: call(
                        command,
                        input=(
                            "SELECT count(*) FROM pg_stat_activity "
                            "WHERE application_name='lctx-serving' AND wait_event='PgSleep'"
                        ),
                    ).stdout.strip()
                )
                if sleeping == "1":
                    break
                await asyncio.sleep(0.01)
            assert sleeping == "1"
            pending.cancel()
            with pytest.raises(asyncio.CancelledError):
                await pending
            waiting = repo.check()
            await asyncio.sleep(0.15)
            assert not waiting.done(), "cancelled server work must still own the sole pool slot"
            active = await asyncio.to_thread(
                lambda: call(
                    command,
                    input=(
                        "SELECT count(*) FROM pg_stat_activity "
                        "WHERE application_name='lctx-serving' AND state='active'"
                    ),
                ).stdout.strip()
            )
            assert active == "1"
            waiting.cancel()
            with pytest.raises(asyncio.CancelledError):
                await waiting
        finally:
            # DDL waits for the actual query/statement deadline, then restores the fixture.
            await asyncio.to_thread(
                lambda: call(
                    command,
                    input=(
                        "DROP VIEW public._sqlx_migrations; "
                        "ALTER TABLE public.retained_migrations RENAME TO _sqlx_migrations;"
                    ),
                )
            )
        assert json.loads(await repo.check())["role"] == "lctx_serving"
        await repo.close()

    asyncio.run(scenario())
