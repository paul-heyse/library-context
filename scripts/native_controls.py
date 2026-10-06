#!/usr/bin/env python3
"""Run a selected native contract family on one owned disposable persistent server."""

from __future__ import annotations

import argparse
import subprocess

from build_environment import ROOT, normalized_env
from surrealdb_fixture import fixture


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("boundary", choices=("store", "serving", "mcp"))
    parser.add_argument(
        "--cli",
        action="store_true",
        help="Build and exercise the native selection CLI in the publication journey",
    )
    options, filters = parser.parse_known_args()
    if filters[:1] == ["--"]:
        filters = filters[1:]
    with fixture() as owned:
        env = normalized_env(owned.environment(), native_inputs=False)
        env.update(INSTA_UPDATE="no", UV_NO_SYNC="1")

        def run(command: list[str]) -> int:
            return subprocess.run(command, cwd=ROOT, env=env, check=False).returncode

        if options.cli:
            code = run(["cargo", "build", "--release", "--locked", "-p", "lctx", "--bin", "lctx"])
            if code:
                return code
            env["LCTX_REMEDIATION_CLI_BIN"] = str(ROOT / "target" / "release" / "lctx")

        if options.boundary != "mcp":
            packages = (
                ["-p", "lctx-surrealdb", "-p", "lctx-publisher"]
                if options.boundary == "store"
                else ["-p", "lctx-serving"]
            )
            return run(
                [
                    "cargo",
                    "nextest",
                    "run",
                    "--release",
                    "--no-fail-fast",
                    "--no-tests=fail",
                    *packages,
                    *filters,
                ]
            )
        # The real bridge/MCP prerequisite is produced here, not supplied by an operator store.
        serving = owned.scratch / "serving.json"
        env["LCTX_RETAIN_NATIVE_FIXTURE_CONFIG"] = str(serving)
        code = run(["cargo", "test", "--release", "-p", "lctx-serving", "--test", "native_journey"])
        if code:
            return code
        if not serving.is_file():
            raise RuntimeError("native journey did not publish the MCP fixture")
        owned.restart()
        env["LCTX_NATIVE_SERVING_CONFIG"] = str(serving)
        env["LCTX_NATIVE_TEST_LIBRARY"] = "synthesis-sources"
        return run(
            [
                "uv",
                "run",
                "--no-sync",
                "pytest",
                "python/lctx_mcp/tests/test_wire_contract.py",
                "python/lctx_mcp/tests/test_native_session.py",
                "python/lctx_mcp/tests/test_safe_failure.py",
                "-q",
                *filters,
            ]
        )


if __name__ == "__main__":
    raise SystemExit(main())
