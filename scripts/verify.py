#!/usr/bin/env python3
"""Explicit contract families, scoped readiness, and keep-going qualification.

Readiness is run-owned preparation, never a persistent assertion-pass cache.
The qualification result belongs to the calling plan, not a new receipt register.
"""

from __future__ import annotations

import argparse
import fcntl
import os
import subprocess
from collections.abc import Callable
from contextlib import contextmanager
from dataclasses import dataclass

from build_environment import ROOT, normalized_env


@dataclass(frozen=True)
class Family:
    commands: tuple[tuple[str, ...], ...]
    requirements: frozenset[str] = frozenset()


def rust(*selection: str) -> tuple[str, ...]:
    return ("cargo", "nextest", "run", "--release", "--no-fail-fast", "--no-tests=fail", *selection)


FAMILIES = {
    "model": Family((rust("-p", "lctx-model"),)),
    "analytics": Family((rust("-p", "lctx-analytics"),)),
    "providers": Family(
        (
            rust(
                "-p",
                "cpg-extract",
                "--lib",
                "--test",
                "acquisition",
                "--test",
                "bundle",
                "--test",
                "typed_conformance",
                "--test",
                "typed_flow",
                "--test",
                "typed_calls",
                "--test",
                "native_overload_origins",
                "--test",
                "typed_ruff_context",
                "--test",
                "harness",
            ),
            rust("-p", "cpg-flow", "--lib", "--test", "flow_shapes", "--test", "capture_timing"),
        ),
        frozenset({"tools"}),
    ),
    "compiler": Family(
        (
            rust("-p", "cpg-core", "--lib", "--tests"),
            rust("-p", "lctx", "--bin", "lctx", "--test", "acquire", "--test", "compile_artifact"),
        ),
        frozenset({"tools"}),
    ),
    # Publication and serving are separate later stages. An absent native implementation
    # must be reported blocked, never covered by retired PostgreSQL tests or an empty pass.
    "store": Family((), frozenset({"native-store"})),
    "serving": Family((), frozenset({"native-serving"})),
    "oracles": Family(
        (("uv", "run", "--no-sync", "pytest", "tests/scripts/test_flow_soundness.py", "-q"),),
        frozenset({"tools", "cli"}),
    ),
    "tooling": Family(
        (
            (
                "uv",
                "run",
                "--no-sync",
                "pytest",
                "tests/scripts",
                "--ignore=tests/scripts/test_flow_soundness.py",
                "-q",
            ),
            ("cargo", "test", "--release", "--workspace", "--doc", "--no-fail-fast"),
        ),
        frozenset({"tools"}),
    ),
}

COMMANDS = {
    "model": ("rust",),
    "analytics": ("rust",),
    "providers": ("extract", "flow"),
    "compiler": ("producer", "cli"),
    "store": (),
    "serving": (),
    "oracles": ("flow",),
    "tooling": ("python", "docs"),
}

BOUNDARY_REQUIREMENTS = {
    ("providers", "extract"): frozenset({"tools"}),
    ("providers", "flow"): frozenset(),
    ("compiler", "producer"): frozenset({"tools"}),
    ("compiler", "cli"): frozenset({"tools"}),
    ("oracles", "flow"): frozenset({"tools", "cli"}),
    ("tooling", "python"): frozenset({"tools"}),
    ("tooling", "docs"): frozenset(),
}


def prerequisites(name: str, command: str | None) -> frozenset[str]:
    return (
        FAMILIES[name].requirements
        if command is None
        else BOUNDARY_REQUIREMENTS.get((name, command), FAMILIES[name].requirements)
    )


@contextmanager
def environment_owner(required: bool):
    if not required:
        yield
        return
    lock_path = ROOT / ".venv" / ".verification.lock"
    lock_path.parent.mkdir(exist_ok=True)
    with lock_path.open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        yield


Runner = Callable[[tuple[str, ...]], int]


def prepare(requirements: set[str], run: Runner) -> dict[str, bool]:
    ready = {}
    if "tools" in requirements:
        ready["tools"] = run(("uv", "sync", "--locked", "--inexact", "--only-group", "dev")) == 0
    for native in ("native-store", "native-serving"):
        if native in requirements:
            ready[native] = False
            print(
                f"blocked: {native} implementation belongs to a later graph-native stage",
                flush=True,
            )
    if "cli" in requirements:
        ready["cli"] = run(("cargo", "build", "--release", "-p", "lctx")) == 0
    return ready


def execute(
    selected: list[str],
    ready: dict[str, bool],
    run: Runner,
    arguments: tuple[str, ...] = (),
    command: str | None = None,
) -> dict[str, str]:
    results = {}
    for name in selected:
        family = FAMILIES[name]
        missing = sorted(r for r in prerequisites(name, command) if not ready.get(r, False))
        if missing:
            results[name] = "blocked"
            print(f"{name}: blocked (prerequisite: {', '.join(missing)})", flush=True)
            continue
        commands = (
            family.commands
            if command is None
            else (family.commands[COMMANDS[name].index(command)],)
        )
        codes = [run(invocation + arguments) for invocation in commands]
        results[name] = (
            "passed"
            if all(code == 0 for code in codes)
            else "blocked"
            if all(code in (0, 127) for code in codes)
            else "failed"
        )
        print(f"{name}: {results[name]}", flush=True)
    return results


def launch(command: tuple[str, ...], env: dict[str, str]) -> int:
    print("$ " + " ".join(command), flush=True)
    try:
        return subprocess.run(command, cwd=ROOT, env=env, check=False).returncode
    except OSError as error:
        print(f"blocked: cannot launch {command[0]} ({error})", flush=True)
        return 127


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("family", choices=[*FAMILIES, "qualify"])
    parser.add_argument(
        "--command", help="select a family boundary before passing its ordinary tool filters"
    )
    args, rest = parser.parse_known_args()
    arguments = tuple(rest)
    if arguments[:1] == ("--",):
        arguments = arguments[1:]
    if args.family == "qualify" and (arguments or args.command):
        parser.error("qualification requires its complete declared selections")
    if any(a == "--no-tests=pass" or a.startswith("--no-tests") for a in arguments):
        parser.error("empty required selections must fail")
    if args.family != "qualify":
        choices = COMMANDS[args.family]
        if args.command is not None and args.command not in choices:
            parser.error("family commands: " + ", ".join(choices))
        if arguments and len(choices) > 1 and args.command is None:
            parser.error("choose --command " + "|".join(choices) + " for boundary-specific filters")
    selected = list(FAMILIES) if args.family == "qualify" else [args.family]
    requirements = set().union(*(prerequisites(name, args.command) for name in selected))
    env = normalized_env(
        dict(os.environ, INSTA_UPDATE="no", UV_NO_SYNC="1"),
        native_inputs=False,
    )

    def run(command: tuple[str, ...]) -> int:
        return launch(command, env)

    # Hold ownership through live workers, not just sync. This prevents another
    # family launcher preparing the shared environment while these fixtures run.
    with environment_owner("tools" in requirements):
        ready = prepare(requirements, run)
        results = execute(selected, ready, run, arguments, args.command)
        if args.family == "qualify":
            # Independent non-functional leaves still run after functional failures.
            for name in (
                "clippy",
                "lint-agents",
                "adr-lint",
                "fixtures-check",
                "gold",
                "rules-scan",
                "rules-test",
                "ruff",
                "types",
                "docs-check",
                "deps",
            ):
                if name in (
                    "ruff",
                    "types",
                    "docs-check",
                    "deps",
                    "gold",
                    "adr-lint",
                    "fixtures-check",
                ) and not ready.get("tools"):
                    results[name] = "blocked"
                else:
                    code = run(("just", name))
                    results[name] = (
                        "passed" if code == 0 else "blocked" if code == 127 else "failed"
                    )
                print(f"{name}: {results[name]}", flush=True)
    return 0 if all(result == "passed" for result in results.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())
