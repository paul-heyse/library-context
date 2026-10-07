#!/usr/bin/env python3
"""Owned disposable persistent SurrealDB prerequisite for native functional controls.

Run ``python3 scripts/surrealdb_fixture.py -- COMMAND ...`` or use ``fixture()``.
No operator endpoint/configuration is read. The child receives a private 0600
configuration through LCTX_SURREAL_TEST_CONFIG; its existing build/uv environment
is preserved, with the repository's ordinary Cargo-path normalization applied.
"""

from __future__ import annotations

import argparse
import base64
import contextlib
import json
import os
import secrets
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from collections.abc import Iterator, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

# Exact image: selected 3.3.0 protocol/DDL/restore parity against inspected source
# 238bfeb11f5725bebed370167656748df8067595. Revisit when qualifying another server
# release; a floating image would silently transfer these functional contracts.
IMAGE = (
    "surrealdb/surrealdb@sha256:681c6c22c287421b5c7d99e0fde79b6e0d32c36c1ddeaab2762a1661cb04cd20"
)
VERSION = "surrealdb-3.3.0"


def _docker(*args: str) -> str:
    result = subprocess.run(
        ("docker", *args), check=False, capture_output=True, text=True, timeout=60
    )
    if result.returncode:
        raise RuntimeError(f"Docker {args[0]} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def _identifier(value: str) -> str:
    if not value or not value.isascii() or not all(c.isalnum() or c == "_" for c in value):
        raise ValueError("fixture database/namespace must be an ASCII identifier")
    return value


@dataclass
class SurrealFixture:
    scratch: Path
    container: str
    container_id: str = ""
    config: dict[str, Any] = field(default_factory=dict, repr=False)

    @property
    def config_path(self) -> Path:
        return self.scratch / "runtime.json"

    @property
    def endpoint(self) -> str:
        return str(self.config["endpoint"])

    def environment(self, source: Mapping[str, str] | None = None) -> dict[str, str]:
        env = dict(os.environ if source is None else source)
        env["LCTX_SURREAL_TEST_CONFIG"] = str(self.config_path)
        return env

    def ready(self, timeout: float = 30) -> None:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                with urllib.request.urlopen(self.endpoint + "/ready", timeout=1) as response:
                    healthy = response.status == 200
                if healthy:
                    with urllib.request.urlopen(self.endpoint + "/version", timeout=1) as response:
                        version = response.read().decode().strip()
                    if version != VERSION:
                        raise RuntimeError(f"fixture version mismatch: {version}")
                    return
            except urllib.error.URLError, TimeoutError, ConnectionError:
                time.sleep(0.1)
        raise RuntimeError("owned SurrealDB readiness timed out")

    def restart(self) -> None:
        if not self.container_id:
            raise RuntimeError("fixture has no owned container")
        _docker("restart", "--time", "30", self.container_id)
        self.ready()

    def remove(self) -> None:
        if self.container_id:
            # The immutable ID came only from this instance's docker run; no name,
            # inherited endpoint or pre-existing container can enter this cleanup.
            _docker("rm", "--force", self.container_id)
            self.container_id = ""

    def _request(
        self, route: str, data: bytes, database: str | None = None
    ) -> urllib.request.Request:
        credentials = f"{self.config['admin_user']}:{self.config['admin_password']}"
        return urllib.request.Request(
            self.endpoint + route,
            data=data,
            headers={
                "Authorization": "Basic " + base64.b64encode(credentials.encode()).decode(),
                "Surreal-NS": str(self.config["namespace"]),
                "Surreal-DB": database or str(self.config["database"]),
                "Accept": "application/json",
                "Content-Type": "text/plain",
            },
        )

    def query(self, sql: str, *, database: str | None = None) -> list[dict[str, Any]]:
        with urllib.request.urlopen(self._request("/sql", sql.encode(), database), timeout=25) as r:
            result = json.load(r)
        if not isinstance(result, list):
            raise RuntimeError("fixture query returned no statement results")
        failures = [row for row in result if row.get("status") != "OK"]
        if failures:
            # SQL/errors can contain bound secrets; retain the shape, not the body.
            kinds = ", ".join(str(row.get("kind", "statement")) for row in failures)
            raise RuntimeError(f"fixture query failed: {kinds}")
        return result

    def export(self, path: Path, *, database: str | None = None) -> None:
        """Export a fixture atomically; restore/reconciliation establishes logical completeness."""
        request = self._request("/export", b"", database)
        request.data = None
        request.method = "GET"
        temporary = path.with_name(path.name + ".partial")
        try:
            with (
                urllib.request.urlopen(request, timeout=25) as r,
                temporary.open("wb") as out,
            ):
                shutil.copyfileobj(r, out)
            temporary.replace(path)
        finally:
            temporary.unlink(missing_ok=True)

    def import_dump(self, path: Path, *, database: str) -> None:
        """Restore into a fresh owned database; callers reconcile content/index readiness."""
        database = _identifier(database)
        existing = self.query("INFO FOR NAMESPACE;")[0]["result"].get("databases", {})
        if database in existing:
            raise ValueError("restore requires a fresh fixture database")
        self.query(f"DEFINE DATABASE {database} STRICT;")
        with urllib.request.urlopen(
            self._request("/import", path.read_bytes(), database), timeout=25
        ) as response:
            body = response.read()
        if body:
            result = json.loads(body)
            if isinstance(result, list) and any(row.get("status") != "OK" for row in result):
                raise RuntimeError("fixture import reported a statement error")
            if isinstance(result, dict) and result.get("code", 200) >= 400:
                raise RuntimeError("fixture import reported a request error")

    def run(self, command: Sequence[str], *, cwd: Path | None = None) -> int:
        env = self.environment()
        try:
            from build_environment import normalized_env
        except ModuleNotFoundError as error:
            if error.name != "build_environment":
                raise
        else:
            env = normalized_env(env)
        return subprocess.run(command, cwd=cwd, env=env, check=False).returncode


@contextlib.contextmanager
def fixture() -> Iterator[SurrealFixture]:
    with tempfile.TemporaryDirectory(prefix="lctx-surrealdb-fixture-") as directory:
        scratch = Path(directory)
        (scratch / "data").mkdir()
        suffix = secrets.token_hex(8)
        instance = SurrealFixture(scratch, "lctx-surrealdb-fixture-" + suffix)
        password = secrets.token_urlsafe(32)
        env_file = scratch / "server.env"
        # Deliberate fixture allocation policy for the selected server. Keep the
        # default durable Every sync mode and background maintenance behavior.
        # The tracked-memory threshold complements the container RSS boundary;
        # neither is a claim about peak RSS or a universal tuning prescription.
        env_file.write_text(
            "SURREAL_USER=fixture_admin\nSURREAL_PASS=" + password + "\n"
            "SURREAL_ROCKSDB_BLOCK_CACHE_SIZE=67108864\n"
            "SURREAL_ROCKSDB_WRITE_BUFFER_SIZE=33554432\n"
            "SURREAL_ROCKSDB_MAX_WRITE_BUFFER_NUMBER=2\n"
            "SURREAL_MEMORY_THRESHOLD=512MiB\n"
            # Native rows may travel alone up to 64 MiB, with RPC framing. The SDK follows
            # the advertised server limit; its 4 MiB default cannot carry admitted rows.
            "SURREAL_GRPC_MAX_MESSAGE_SIZE=128MiB\n"
        )
        env_file.chmod(0o600)
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        try:
            instance.container_id = _docker(
                "run",
                "--detach",
                "--memory",
                "1g",
                "--name",
                instance.container,
                "--user",
                f"{os.getuid()}:{os.getgid()}",
                "--publish",
                f"127.0.0.1:{port}:8000",
                "--env-file",
                str(env_file),
                "--mount",
                f"type=bind,src={scratch / 'data'},dst=/data",
                IMAGE,
                "start",
                "--bind",
                "0.0.0.0:8000",
                "--query-timeout",
                "20s",
                "--transaction-timeout",
                "10s",
                "rocksdb:///data/store",
            )
            instance.config = {
                "scratch": str(scratch),
                "container": instance.container,
                "endpoint": f"http://127.0.0.1:{port}",
                "grpc_endpoint": f"grpc://127.0.0.1:{port}",
                "namespace": "fixture_" + suffix,
                "database": "core",
                "admin_user": "fixture_admin",
                "admin_password": password,
                "image": IMAGE,
            }
            instance.config_path.write_text(json.dumps(instance.config))
            instance.config_path.chmod(0o600)
            instance.ready()
            namespace = _identifier(str(instance.config["namespace"]))
            instance.query(f"DEFINE NAMESPACE {namespace}; DEFINE DATABASE core STRICT;")
            yield instance
        finally:
            instance.remove()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("provide a test command after --")
    with fixture() as instance:
        print("SurrealDB fixture ready:", instance.endpoint, flush=True)
        return instance.run(command, cwd=Path.cwd())


if __name__ == "__main__":
    raise SystemExit(main())
