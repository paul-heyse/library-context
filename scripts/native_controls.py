#!/usr/bin/env python3
"""Run a selected native contract family on one owned disposable persistent server."""

from __future__ import annotations

import argparse
import subprocess
import json
import os

from build_environment import ROOT, normalized_env
from surrealdb_fixture import fixture


def compiler_packages(boundary: str, filters: list[str]) -> list[str]:
    defaults = {
        "compiler": ["-p", "cpg-core", "--lib", "--tests"],
        "compiler-cli": ["-p", "lctx", "--bin", "lctx", "--test", "acquire", "--test", "compile_artifact"],
        "providers": ["-p", "cpg-extract", "--lib", "--test", "acquisition", "--test", "bundle", "--test", "harness", "--test", "typed_conformance", "--test", "typed_flow", "--test", "typed_calls", "--test", "native_overload_origins", "--test", "typed_ruff_context"],
    }[boundary]
    target_options = {"--lib", "--test", "--tests", "--bin", "--bins", "--example", "--examples", "--bench", "--benches", "--all-targets"}
    # Explicit Cargo targets replace family defaults; Nextest filters alone still select
    # within the family's ordinary targets. Combining --tests with --test builds all tests.
    if any(argument.split("=", 1)[0] in target_options for argument in filters):
        return defaults[:2]
    return defaults


def cargo_command(command: list[str], configuration: list[str]) -> list[str]:
    if not configuration or command[0] != "cargo":
        return command
    options = [argument for value in configuration for argument in ("--config", value)]
    if command[1:3] == ["nextest", "run"]:
        return command[:3] + options + command[3:]
    return command[:1] + options + command[1:]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("boundary", choices=("store", "serving", "mcp", "compiler", "compiler-cli", "providers"))
    parser.add_argument(
        "--cli",
        action="store_true",
        help="Build and exercise the native selection CLI in the publication journey",
    )
    parser.add_argument(
        "--cargo-config", action="append", default=[], metavar="KEY=VALUE",
        help="Temporary Cargo configuration for this control invocation",
    )
    options, filters = parser.parse_known_args()
    if filters[:1] == ["--"]:
        filters = filters[1:]
    with fixture() as owned:
        env = normalized_env(owned.environment(), native_inputs=False)
        env.update(INSTA_UPDATE="no", UV_NO_SYNC="1")
        runtime=owned.scratch / "compiler-runtime.json"
        runtime.write_text(json.dumps({
            "endpoint":owned.config["grpc_endpoint"],
            "username":owned.config["admin_user"],"password":owned.config["admin_password"],
            "viewer_username":"fixture_viewer","viewer_password":owned.config["admin_password"]+"_viewer",
            "namespace":owned.config["namespace"],"cache_database":"compiler_cache",
            "selection":str(owned.scratch / "selected.json"),
        }))
        os.chmod(runtime,0o600)
        owned.query("DEFINE DATABASE compiler_cache STRICT;")
        env["LCTX_COMPILER_RUNTIME_CONFIG"]=str(runtime)

        def run(command: list[str]) -> int:
            return subprocess.run(cargo_command(command, options.cargo_config), cwd=ROOT, env=env, check=False).returncode

        if options.cli:
            code = run(["cargo", "build", "--release", "--locked", "-p", "lctx", "--bin", "lctx"])
            if code:
                return code
            env["LCTX_REMEDIATION_CLI_BIN"] = str(ROOT / "target" / "release" / "lctx")

        if options.boundary in {"compiler", "compiler-cli", "providers"}:
            packages = compiler_packages(options.boundary, filters)
            return run(["cargo","nextest","run","--release","--no-fail-fast","--no-tests=fail",*packages,*filters])
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
        code = run(["cargo", "build", "--release", "--locked", "-p", "lctx-eval"])
        if code:
            return code
        env["LCTX_EVAL_WORKER"] = str(ROOT / "target" / "release" / "lctx-eval")
        serving = owned.scratch / "serving.json"
        env["LCTX_RETAIN_NATIVE_FIXTURE_CONFIG"] = str(serving)
        code = run(["cargo", "test", "--release", "-p", "lctx-serving", "--test", "native_journey", "--", "--nocapture"])
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
                "tests/scripts/test_programmatic_native.py",
                "tests/scripts/test_programmatic_native_numeric.py",
                "-q",
                *filters,
            ]
        )


if __name__ == "__main__":
    raise SystemExit(main())
