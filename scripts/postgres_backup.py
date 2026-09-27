#!/usr/bin/env python3
"""Back up the application database, or verify a backup in a disposable PG18 instance."""

import argparse
import hashlib
import json
import os
import subprocess
import time
from pathlib import Path
from urllib.parse import parse_qs, unquote, urlsplit

from postgres_check import call

TABLES = (
    "public._sqlx_migrations",
    "lctx_cache.specs",
    "lctx_cache.embedding_values",
    "lctx_ops.attempts",
    "lctx_ops.events",
    "lctx_ops.snapshots",
    "lctx_ops.generations",
)


def connection_env(url: str) -> dict[str, str]:
    parts = urlsplit(url)
    env = {k: v for k, v in os.environ.items() if not k.startswith("PG")}
    env.update(
        PGHOST=parts.hostname or "localhost",
        PGPORT=str(parts.port or 5432),
        PGUSER=unquote(parts.username or ""),
        PGPASSWORD=unquote(parts.password or ""),
        PGDATABASE=unquote(parts.path.lstrip("/")),
        PGCONNECT_TIMEOUT="10",
        PGTZ="UTC",
    )
    for key, values in parse_qs(parts.query).items():
        if key in {"sslmode", "sslrootcert", "sslcert", "sslkey"}:
            env["PG" + key.upper()] = values[-1]
        else:
            raise SystemExit("unsupported libpq backup URL option; use a dedicated configuration")
    return env


def protected(path: Path) -> None:
    if path.stat().st_mode & 0o077:
        raise SystemExit(f"protected file requires mode 600: {path}")


def fingerprints(query) -> dict[str, dict[str, object]]:
    # Identifier list is fixed by the schema owner. Hash rows before aggregation, so large bytea
    # payloads do not accumulate in string_agg's state. These are recovery checks, not semantic IDs.
    return {
        table: json.loads(
            query(
                "SELECT json_build_object('rows',count(*),'digest',"
                "md5(coalesce(string_agg(h,'' ORDER BY h),''))) FROM "
                f"(SELECT md5(to_jsonb(t)::text) AS h FROM {table} t) q;"
            )
        )
        for table in TABLES
    }


def backup(config: Path, archive: Path) -> None:
    protected(config)
    settings = json.loads(config.read_text())
    env = connection_env(settings["migration_url"])
    archive.parent.mkdir(parents=True, exist_ok=True)
    receipt = archive.with_suffix(archive.suffix + ".json")
    if archive.exists() or receipt.exists():
        raise SystemExit("refusing to overwrite an existing backup or receipt")
    archive.touch(mode=0o600, exist_ok=False)
    process = subprocess.Popen(
        ["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"],
        env=env,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    assert process.stdin is not None and process.stdout is not None

    def query(sql: str) -> str:
        assert process.stdin is not None and process.stdout is not None
        process.stdin.write(sql + "\n")
        process.stdin.flush()
        answer = process.stdout.readline().strip()
        if not answer:
            raise RuntimeError("PostgreSQL backup snapshot query failed")
        return answer

    try:
        snapshot = query(
            "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY; SELECT pg_export_snapshot();"
        )
        state = fingerprints(query)
        call(
            ["pg_dump", "--format=custom", "--snapshot", snapshot, "--file", str(archive)],
            env=env,
            capture_output=True,
        )
        process.stdin.write("COMMIT;\n\\q\n")
        process.stdin.flush()
        if process.wait(timeout=10):
            raise RuntimeError("PostgreSQL backup transaction failed")
        receipt.touch(mode=0o600, exist_ok=False)
        receipt.write_text(
            json.dumps(
                {
                    "sha256": hashlib.file_digest(archive.open("rb"), "sha256").hexdigest(),
                    "tables": state,
                },
                indent=2,
            )
            + "\n"
        )
        print(f"passed: consistent application backup and recovery fingerprints: {archive}")
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=10)


def restore_drill(archive: Path) -> None:
    protected(archive)
    receipt = archive.with_suffix(archive.suffix + ".json")
    protected(receipt)
    expected = json.loads(receipt.read_text())
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if digest != expected["sha256"]:
        raise SystemExit("backup checksum mismatch")
    image = (
        "postgres:"
        + (Path(__file__).resolve().parent.parent / "specs/postgres-image.txt").read_text().strip()
    )
    call(["docker", "image", "inspect", image], stdout=subprocess.DEVNULL)
    started = time.monotonic()
    container = call(
        ["docker", "run", "--rm", "--detach", "--env", "POSTGRES_PASSWORD=fixture-only", image],
        capture_output=True,
    ).stdout.strip()
    try:
        for _ in range(120):
            ready = subprocess.run(
                ["docker", "exec", container, "pg_isready", "-h", "127.0.0.1", "-U", "postgres"],
                capture_output=True,
                check=False,
            )
            if ready.returncode == 0:
                break
            time.sleep(0.25)
        else:
            raise RuntimeError("disposable restore PostgreSQL did not become ready")
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
        call(
            command,
            input="CREATE ROLE lctx_app LOGIN; CREATE ROLE lctx_migrator LOGIN;",
            capture_output=True,
        )
        call(["docker", "cp", str(archive), f"{container}:/tmp/lctx.dump"], capture_output=True)
        call(
            [
                "docker",
                "exec",
                container,
                "pg_restore",
                "-U",
                "postgres",
                "--dbname=postgres",
                "--exit-on-error",
                "/tmp/lctx.dump",
            ],
            capture_output=True,
        )
        actual = fingerprints(
            lambda sql: call(command, input=sql, capture_output=True).stdout.strip()
        )
        if actual != expected["tables"]:
            raise RuntimeError(
                "restored tables differ: "
                + ", ".join(t for t in TABLES if actual[t] != expected["tables"][t])
            )
        print(
            json.dumps(
                {
                    "outcome": "passed",
                    "tables": actual,
                    "restore_seconds": time.monotonic() - started,
                }
            )
        )
    finally:
        call(["docker", "rm", "--force", container], stdout=subprocess.DEVNULL)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["backup", "restore-drill"])
    parser.add_argument("archive", type=Path)
    parser.add_argument(
        "--config",
        type=Path,
        default=Path(
            os.environ.get(
                "LCTX_DATABASE_CONFIG", str(Path.home() / ".config/library-context/postgres.json")
            )
        ),
    )
    args = parser.parse_args()
    try:
        if args.command == "backup":
            backup(args.config, args.archive)
        else:
            restore_drill(args.archive)
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        # Driver/utility error text can contain credentials, identifiers or row payloads.
        reason = str(error) if isinstance(error, RuntimeError) else type(error).__name__
        raise SystemExit(
            f"failed: PostgreSQL {args.command} ({reason}); archive retained for inspection"
        ) from None


if __name__ == "__main__":
    main()
