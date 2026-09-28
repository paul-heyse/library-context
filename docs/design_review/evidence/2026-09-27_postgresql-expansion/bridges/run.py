"""Run the bounded Arrow/SQLx/provider probe against a disposable PG18 container."""

import os
import subprocess
import sys
import time
from pathlib import Path

here = Path(__file__).resolve().parent
repo = here.parents[4]
target = Path.home() / ".cache/lctx-pg-expansion-target"
env = dict(os.environ, CARGO_TARGET_DIR=str(target))
command = ["cargo", "build", "--release", "--locked", "--manifest-path", str(here / "Cargo.toml")]
if "--providers" in sys.argv:
    command += [
        "--features",
        "datafusion,datafusion-federation,datafusion-table-providers-postgres",
    ]
subprocess.run(command, env=env, cwd=repo, check=True)
image = "postgres:" + (repo / "specs/postgres-image.txt").read_text().strip()
container = subprocess.check_output(
    [
        "docker",
        "run",
        "--rm",
        "-d",
        "-e",
        "POSTGRES_PASSWORD=bridge_probe",
        "-p",
        "127.0.0.1::5432",
        image,
    ],
    text=True,
).strip()
try:
    for _ in range(120):
        ready = subprocess.run(
            ["docker", "exec", container, "pg_isready", "-U", "postgres"], capture_output=True
        )
        if ready.returncode == 0:
            break
        time.sleep(0.25)
    else:
        raise RuntimeError("Disposable PostgreSQL did not become ready")
    port = (
        subprocess.check_output(["docker", "port", container, "5432/tcp"], text=True)
        .strip()
        .rsplit(":", 1)[1]
    )
    env["BRIDGE_PG_PORT"] = port
    env["BRIDGE_DATABASE_URL"] = f"postgres://postgres:bridge_probe@127.0.0.1:{port}/postgres"
    subprocess.run([str(target / "release/lctx-pg-bridge-qualification")], env=env, check=True)
finally:
    subprocess.run(["docker", "stop", container], capture_output=True, check=True)
