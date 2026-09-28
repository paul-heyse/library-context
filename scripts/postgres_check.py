#!/usr/bin/env python3
"""Check or regenerate SQLx metadata against a freshly migrated, disposable PG18 database."""

import argparse
import json
import os
import subprocess
import time
from pathlib import Path


def call(
    args: list[str],
    *,
    input: str | None = None,
    capture_output: bool = False,
    stdout: int | None = None,
    env: dict[str, str] | None = None,
    cwd: Path | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        text=True,
        check=True,
        input=input,
        capture_output=capture_output,
        stdout=stdout,
        env=env,
        cwd=cwd,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--prepare", action="store_true", help="write .sqlx metadata instead of checking"
    )
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    image = (root / "specs/postgres-vector-image.txt").read_text().strip()
    try:
        call(["docker", "image", "inspect", image], stdout=subprocess.DEVNULL)
    except (OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(
            "blocked: Docker and pinned PG18 image required; run just postgres-test-setup"
        ) from error
    container = call(
        [
            "docker",
            "run",
            "--rm",
            "--detach",
            "--publish",
            "127.0.0.1::5432",
            "--env",
            "POSTGRES_PASSWORD=fixture-only",
            image,
        ],
        capture_output=True,
    ).stdout.strip()
    try:
        port_data = json.loads(call(["docker", "inspect", container], capture_output=True).stdout)
        port = port_data[0]["NetworkSettings"]["Ports"]["5432/tcp"][0]["HostPort"]
        for _ in range(120):
            result = subprocess.run(
                ["docker", "exec", container, "pg_isready", "-h", "127.0.0.1", "-U", "postgres"],
                capture_output=True,
                check=False,
            )
            if result.returncode == 0:
                break
            time.sleep(0.25)
        else:
            raise SystemExit("blocked: disposable PostgreSQL did not become ready")
        sql = """CREATE ROLE lctx_app LOGIN PASSWORD 'fixture-only';
CREATE ROLE lctx_importer LOGIN PASSWORD 'fixture-only';
CREATE ROLE lctx_serving LOGIN PASSWORD 'fixture-only';
CREATE ROLE lctx_migrator LOGIN PASSWORD 'fixture-only';
CREATE SCHEMA lctx_ext;
CREATE EXTENSION vector WITH SCHEMA lctx_ext VERSION '0.8.6';
GRANT USAGE ON SCHEMA lctx_ext TO lctx_app,lctx_migrator,lctx_importer,lctx_serving;
DO $$ BEGIN
 IF current_setting('server_version_num')::int <> 180006 THEN
  RAISE EXCEPTION 'pinned PG18 patch mismatch';
 END IF;
END $$;"""
        call(
            [
                "docker",
                "exec",
                "-i",
                container,
                "psql",
                "-X",
                "-U",
                "postgres",
                "-v",
                "ON_ERROR_STOP=1",
            ],
            input=sql,
        )
        env = os.environ.copy()
        env["DATABASE_URL"] = f"postgres://postgres:fixture-only@127.0.0.1:{port}/postgres"
        env["SQLX_OFFLINE"] = "false"
        env["CARGO_TARGET_DIR"] = str(root / "target")
        call(
            ["sqlx", "migrate", "run", "--source", "crates/lctx-postgres/migrations"],
            env=env,
            cwd=root,
        )
        command = ["cargo", "sqlx", "prepare", "--workspace"]
        if not args.prepare:
            command.append("--check")
        call(
            [*command, "--", "-p", "lctx-postgres", "--all-targets", "--release"], env=env, cwd=root
        )
    finally:
        call(["docker", "rm", "--force", container], stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    main()
