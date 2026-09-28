"""Explicit FastMCP deployment observations in a disposable locked environment.

No compiler/query invokes this script. It executes only the selected upstream config
example under the two named policies, with real stdio MCP list/call operations.
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

SOURCE = "examples/fastmcp_config/server.py"
POLICY = "fastmcp-stdio-v1"
TIMEOUT = 45
OUTPUT_LIMIT = 65536


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def bounded(
    command: list[str], *, cwd: Path, env: dict[str, str], timeout: int
) -> tuple[int, str, str]:
    """Own the entire subprocess group, bounded output and wall time on all exits."""
    with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
        process = subprocess.Popen(
            command, cwd=cwd, env=env, stdout=out, stderr=err, start_new_session=True
        )
        started = time.monotonic()
        failure = None
        try:
            while process.poll() is None:
                if time.monotonic() - started > timeout:
                    failure = "task timeout"
                    break
                if os.fstat(out.fileno()).st_size + os.fstat(err.fileno()).st_size > OUTPUT_LIMIT:
                    failure = "task output limit"
                    break
                time.sleep(0.05)
        finally:
            # Also terminate any orphaned children after a successful worker exit.
            with contextlib.suppress(ProcessLookupError):
                os.killpg(process.pid, signal.SIGKILL)
            process.wait()
        out.seek(0)
        err.seek(0)
        stdout = out.read(OUTPUT_LIMIT).decode("utf-8", errors="replace")
        stderr = err.read(OUTPUT_LIMIT).decode("utf-8", errors="replace")
        if (
            not failure
            and os.fstat(out.fileno()).st_size + os.fstat(err.fileno()).st_size > OUTPUT_LIMIT
        ):
            failure = "task output limit"
        if failure:
            return -1, stdout, failure
        return process.returncode, stdout, stderr


def worker(task: str, source: Path) -> None:
    # Imported only inside the disposable analyzed-library environment.
    import asyncio

    from fastmcp import Client
    from fastmcp.client.transports import StdioTransport

    async def observe() -> None:
        command = sys.executable
        args = (
            [str(source)]
            if task == "programmatic"
            else [
                "-c",
                "from fastmcp.cli import app; app()",
                "run",
                f"{source}:mcp",
                "--transport",
                "stdio",
            ]
        )
        transport = StdioTransport(
            command, args, cwd=str(source.parent), env=dict(os.environ), keep_alive=False
        )
        async with Client(transport, timeout=20, init_timeout=20) as client:
            tools = sorted(tool.name for tool in await client.list_tools())
            result = await client.call_tool("add", {"a": 2, "b": 3})
            if result.is_error or result.data != 5:
                raise RuntimeError("add(2,3) did not return 5")
            print(json.dumps({"tools": tools, "result": "5"}))

    asyncio.run(observe())


def run(args: argparse.Namespace) -> None:
    root = Path(__file__).resolve().parents[1]
    project = root / "libraries/fastmcp"
    cli = args.cli.resolve()
    source = args.source.resolve()
    args.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="lctx-task-") as directory:
        temp = Path(directory)
        libraries = temp / "libraries"
        target = libraries / "fastmcp"
        target.mkdir(parents=True)
        for name in ("pyproject.toml", "uv.lock", ".python-version"):
            shutil.copyfile(project / name, target / name)
        environment = temp / "envs/fastmcp"
        home = temp / "home"
        home.mkdir()
        env = {
            "PATH": os.environ.get("PATH", ""),
            "HOME": str(home),
            "LANG": "C.UTF-8",
            "UV_PROJECT_ENVIRONMENT": str(environment),
        }
        uv = shutil.which("uv")
        if uv is None:
            raise RuntimeError("uv is required")
        code, _, error = bounded(
            [uv, "sync", "--frozen", "--no-build", "--project", str(target)],
            cwd=temp,
            env=env,
            timeout=180,
        )
        if code:
            raise RuntimeError(f"disposable environment provisioning failed: {error[-8192:]}")
        identity_cmd = [
            str(cli),
            "--libraries",
            str(libraries),
            "--envs",
            str(temp / "envs"),
            "deployment-identity",
            "fastmcp",
        ]
        code, stdout, error = bounded(identity_cmd, cwd=temp, env=env, timeout=45)
        if code:
            raise RuntimeError(f"environment identity failed: {error[-8192:]}")
        identity = json.loads(stdout)
        task_source = temp / "source/server.py"
        task_source.parent.mkdir()
        shutil.copyfile(source / SOURCE, task_source)
        for task in ("programmatic", "cli"):
            started = time.monotonic()
            command = [
                str(environment / "bin/python"),
                str(Path(__file__).resolve()),
                "--worker",
                task,
                str(task_source),
            ]
            code, stdout, error = bounded(command, cwd=temp, env=env, timeout=TIMEOUT)
            receipt: dict[str, object] = {
                "format": 1,
                "policy": POLICY,
                "task": task,
                "runner_sha256": sha(Path(__file__)),
                "source_path": SOURCE,
                "source_sha256": sha(task_source),
                "environment": identity,
                "command": ["<environment>/bin/python", "<source>/server.py"]
                if task == "programmatic"
                else [
                    "<environment>/bin/python",
                    "-c",
                    "from fastmcp.cli import app; app()",
                    "run",
                    "<source>/server.py:mcp",
                    "--transport",
                    "stdio",
                ],
                "tool": "add",
                "arguments": {"a": 2, "b": 3},
                "elapsed_ms": int((time.monotonic() - started) * 1000),
                "timeout_seconds": TIMEOUT,
                "execution": "failed",
                "tools": [],
                "result": None,
                "diagnostic": None,
            }
            try:
                if code:
                    raise RuntimeError(error[-8192:] or f"worker exited {code}")
                observed = json.loads(stdout)
                receipt.update(tools=observed["tools"], result=observed["result"])
                code, after, error = bounded(identity_cmd, cwd=temp, env=env, timeout=45)
                if (
                    code
                    or json.loads(after) != identity
                    or sha(task_source) != receipt["source_sha256"]
                ):
                    raise RuntimeError("task changed verified environment or source")
                receipt["execution"] = "passed"
            except (ValueError, KeyError, RuntimeError) as error:
                receipt["diagnostic"] = str(error)[:8192]
            path = args.out / f"{task}.json"
            path.write_text(json.dumps(receipt, sort_keys=True, indent=2) + "\n")
            print(f"{task}: {receipt['execution']} ({path})")


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--worker":
        worker(sys.argv[2], Path(sys.argv[3]))
    else:
        parser = argparse.ArgumentParser(description=__doc__)
        parser.add_argument("--cli", type=Path, default=Path("target/release/lctx"))
        parser.add_argument("--source", type=Path, required=True)
        parser.add_argument("--out", type=Path, required=True)
        options = parser.parse_args()
        try:
            run(options)
        except (RuntimeError, OSError, ValueError) as error:
            options.out.mkdir(parents=True, exist_ok=True)
            (options.out / "preflight.json").write_text(
                json.dumps({"status": "blocked", "policy": POLICY, "diagnostic": str(error)[:8192]})
                + "\n"
            )
            raise
