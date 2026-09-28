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
    if path.is_symlink() or not path.is_file() or path.stat().st_mode & 0o077:
        raise SystemExit(f"protected file requires mode 600: {path}")


def fingerprints(query, tables=TABLES) -> dict[str, dict[str, object]]:
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
        for table in tables
    }


def legacy_restore_drill(archive: Path) -> None:
    protected(archive)
    receipt = archive.with_suffix(archive.suffix + ".json")
    protected(receipt)
    expected = json.loads(receipt.read_text())
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if digest != expected["sha256"]:
        raise SystemExit("backup checksum mismatch")
    image = expected.get(
        "image",
        "postgres:"
        + (Path(__file__).resolve().parent.parent / "specs/postgres-image.txt").read_text().strip(),
    )
    allowed = {
        "postgres:"
        + (Path(__file__).resolve().parent.parent / "specs/postgres-image.txt").read_text().strip(),
        (Path(__file__).resolve().parent.parent / "specs/postgres-vector-image.txt")
        .read_text()
        .strip(),
    }
    if image not in allowed:
        raise RuntimeError("backup image is not one of the declared recovery pins")
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
            input=(
                "CREATE ROLE lctx_app LOGIN; CREATE ROLE lctx_migrator LOGIN; "
                "CREATE ROLE lctx_importer LOGIN; CREATE ROLE lctx_serving LOGIN;"
            ),
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
            lambda sql: call(command, input=sql, capture_output=True).stdout.strip(),
            expected["tables"],
        )
        if actual != expected["tables"]:
            raise RuntimeError(
                "restored tables differ: "
                + ", ".join(t for t in expected["tables"] if actual[t] != expected["tables"][t])
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
            from postgres_recovery import backup

            backup(args.config, args.archive)
        else:
            receipt = args.archive.with_suffix(args.archive.suffix + ".json")
            protected(receipt)
            version = json.loads(receipt.read_text()).get("format")
            if version == 3:
                from postgres_recovery import restore

                restore(args.archive)
            elif version == 2:
                raise RuntimeError(
                    "schema008 receipt requires retained pre-PR1 recovery scripts, CLI "
                    "and Python/native runtime"
                )
            else:
                legacy_restore_drill(args.archive)
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        # Driver/utility error text can contain credentials, identifiers or row payloads.
        reason = str(error) if isinstance(error, RuntimeError) else type(error).__name__
        raise SystemExit(
            f"failed: PostgreSQL {args.command} ({reason}); archive retained for inspection"
        ) from None


if __name__ == "__main__":
    main()
