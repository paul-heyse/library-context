#!/usr/bin/env python3
"""Explicit contract families, observed readiness, and keep-going qualification.

Readiness observes each boundary's prerequisites and never prepares the Python environment: a
missing prerequisite is `blocked` with the route that repairs it (`just sync tools|native`,
ADR-0134). Boundaries that use the shared Python environment hold shared ownership of it for
their lifetime (`workspace_env.ownership`); pure-Rust boundaries take none. The qualification
result belongs to the calling plan, not a new receipt register.
"""

from __future__ import annotations

import argparse
import os
import subprocess
from collections.abc import Callable
from dataclasses import dataclass

import workspace_env
from build_environment import ROOT, normalized_env
from surrealdb_fixture import IMAGE


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
            ("uv", "run", "--no-sync", "python", "scripts/native_controls.py", "providers"),
            rust("-p", "cpg-flow", "--lib", "--test", "flow_shapes", "--test", "capture_timing"),
        ),
        frozenset({"tools", "native-store"}),
    ),
    "compiler": Family(
        (
            ("uv", "run", "--no-sync", "python", "scripts/native_controls.py", "compiler"),
            ("uv", "run", "--no-sync", "python", "scripts/native_controls.py", "compiler-cli"),
        ),
        frozenset({"tools", "native-store"}),
    ),
    "store": Family(
        (("uv", "run", "--no-sync", "python", "scripts/native_controls.py", "store"),),
        frozenset({"tools", "native-store"}),
    ),
    "serving": Family(
        (
            ("uv", "run", "--no-sync", "python", "scripts/native_controls.py", "serving"),
            ("uv", "run", "--no-sync", "python", "scripts/native_controls.py", "mcp"),
        ),
        frozenset({"tools", "native-serving", "native-python"}),
    ),
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
    "store": ("rust",),
    "serving": ("rust", "mcp"),
    "oracles": ("flow",),
    "tooling": ("python", "docs"),
}

BOUNDARY_REQUIREMENTS = {
    ("providers", "extract"): frozenset({"tools", "native-store"}),
    ("providers", "flow"): frozenset(),
    ("compiler", "producer"): frozenset({"tools", "native-store"}),
    ("compiler", "cli"): frozenset({"tools", "native-store"}),
    ("serving", "rust"): frozenset({"tools", "native-serving"}),
    ("serving", "mcp"): frozenset({"tools", "native-serving", "native-python"}),
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


# Requirements observed through the shared Python environment, and their repair routes.
PYTHON_REQUIREMENTS = ("tools", "native-python")
REPAIR = {
    "tools": "just sync tools",
    "native-python": "just sync native",
    "native-store": f"docker pull {IMAGE}",
    "native-serving": f"docker pull {IMAGE}",
    "cli": "fix the `cargo build --release -p lctx` failure above",
}


def environment_owner(requirements: set[str] | frozenset[str]):
    """Shared ownership of what the selected boundaries import; pure Rust takes none."""
    return workspace_env.ownership("shared", [r for r in PYTHON_REQUIREMENTS if r in requirements])


Runner = Callable[[tuple[str, ...]], int]
Observer = Callable[[str], "workspace_env.Readiness"]


def readiness(
    requirements: set[str], run: Runner, observe: Observer = workspace_env.observe
) -> dict[str, bool]:
    """Observe each prerequisite; never synchronize the environment.

    `cli` builds the product binary the oracles execute: a build step of the run, not Python
    environment preparation, so it stays here.
    """
    ready = {}
    for requirement in PYTHON_REQUIREMENTS:
        if requirement in requirements:
            observed = observe(requirement)
            ready[requirement] = observed.ready
            if not observed.ready:
                print(observed.message(), flush=True)
    native_ready = None
    if requirements & {"native-store", "native-serving"}:
        native_ready = run(("docker", "image", "inspect", "--format", "{{.Id}}", IMAGE)) == 0
    for native in ("native-store", "native-serving"):
        if native in requirements:
            ready[native] = bool(native_ready)
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
            routes = "; ".join(f"{r}: {REPAIR.get(r, 'unknown repair')}" for r in missing)
            print(f"{name}: blocked (prerequisite {routes})", flush=True)
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
    # Children launch with --no-sync, so no native input key is computed here.
    base = normalized_env(dict(os.environ, INSTA_UPDATE="no", UV_NO_SYNC="1"))

    # Shared ownership spans the live workers, so a managed `just sync` waits for them.
    with environment_owner(requirements) as owned:
        env = owned.environment(base)

        def run(command: tuple[str, ...]) -> int:
            return launch(command, env)

        ready = readiness(requirements, run)
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
