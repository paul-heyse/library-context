#!/usr/bin/env python3
"""Explicit contract families, scoped readiness, and keep-going qualification.

Readiness is run-owned preparation, never a persistent assertion-pass cache.
The qualification result belongs to the calling plan, not a new receipt register.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
import fcntl
import os
import subprocess
from dataclasses import dataclass
from typing import Callable

from build_environment import ROOT, native_input_fingerprints, normalized_env


@dataclass(frozen=True)
class Family:
    commands: tuple[tuple[str, ...], ...]
    requirements: frozenset[str] = frozenset()


def rust(*selection: str) -> tuple[str, ...]:
    return ("cargo", "nextest", "run", "--release", "--no-fail-fast", "--no-tests=fail", *selection)


FAMILIES = {
    "model": Family((rust("-p", "lctx-model"),)),
    "analytics": Family((rust("-p", "lctx-analytics"),)),
    "providers": Family((rust("-p", "cpg-extract", "--lib", "--test", "typed_flow", "--test", "typed_calls",
        "--test", "native_overload_origins", "--test", "typed_ruff_context", "--test", "harness"),
        rust("-p", "cpg-flow", "--lib", "--test", "flow_shapes", "--test", "capture_timing")), frozenset({"tools"})),
    "store": Family((rust("-p", "lctx-postgres", "--lib", "--test", "generation_stages", "--test", "vocabulary_epochs",
        "--test", "stage_reads", "--test", "stage_validation", "--test", "publication_checks", "--test", "lifecycle",
        "--test", "generation_catalog", "--test", "installation", "--test", "generations", "--test", "serving_shapes",
        "--test", "analysis_publication"),
        ("uv", "run", "--no-sync", "pytest", "tests/scripts/test_postgres_serving.py", "-q")), frozenset({"postgres", "cli", "tools"})),
    "serving": Family((rust("-p", "cpg-core", "--test", "structural", "--test", "behavioral_frontiers",
        "--test", "catalog_selection", "--test", "validation_views", "--test", "facts_generation", "--test", "facts_admission"),
        rust("-p", "lctx", "--test", "model_describe", "--test", "serving_admission", "--test", "serving_cohort",
        "--test", "serving_packets", "--test", "serving_evidence", "--test", "serving_native", "--test", "serving_qualification",
        "--test", "conditional_output_packets", "--test", "terminal_question_packets", "--test", "raised_type_selection"),
        ("uv", "run", "--no-sync", "pytest", "python/lctx_mcp/tests", "-q")), frozenset({"mcp", "postgres", "tools"})),
    "oracles": Family((("uv", "run", "--no-sync", "pytest", "tests/scripts/test_flow_soundness.py", "-q"),
        rust("-p", "lctx", "--test", "serving_soundness")), frozenset({"mcp", "postgres", "cli", "tools"})),
    "tooling": Family((("uv", "run", "--no-sync", "pytest", "tests/scripts", "--ignore=tests/scripts/test_flow_soundness.py",
        "--ignore=tests/scripts/test_semantic_soundness.py", "--ignore=tests/scripts/test_postgres_serving.py", "-q"),
        ("cargo", "test", "--release", "--workspace", "--doc", "--no-fail-fast")), frozenset({"tools"})),
}

COMMANDS = {
    "model": ("rust",), "analytics": ("rust",), "providers": ("extract", "flow"),
    "store": ("rust", "python"), "serving": ("producer", "serving", "python"),
    "oracles": ("flow", "served"), "tooling": ("python", "docs"),
}

BOUNDARY_REQUIREMENTS = {
    ("store", "rust"): frozenset({"postgres"}),
    ("store", "python"): frozenset({"postgres", "cli", "tools"}),
    ("providers", "extract"): frozenset({"tools"}),
    ("providers", "flow"): frozenset(),
    ("serving", "producer"): frozenset({"postgres"}),
    ("serving", "serving"): frozenset({"mcp", "postgres", "tools"}),
    ("serving", "python"): frozenset({"mcp", "tools"}),
    ("oracles", "flow"): frozenset({"tools", "cli"}),
    ("oracles", "served"): frozenset({"mcp", "postgres", "tools"}),
    ("tooling", "python"): frozenset({"tools"}),
    ("tooling", "docs"): frozenset(),
}


def prerequisites(name: str, command: str | None) -> frozenset[str]:
    return FAMILIES[name].requirements if command is None else BOUNDARY_REQUIREMENTS.get((name, command), FAMILIES[name].requirements)


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
    # MCP owns the union of the two native adapters. Inexact subset preparation
    # never removes another selected family's packages from this shared environment.
    packages = ["mcp"] if "mcp" in requirements else [r for r in ("semantics", "storage") if r in requirements]
    if "tools" in requirements:
        ready["tools"] = run(("uv", "sync", "--locked", "--inexact", "--only-group", "dev")) == 0
    before = native_input_fingerprints(ROOT) if packages else {}
    for package in packages:
        command = ("uv", "sync", "--locked", "--inexact", "--package", f"lctx-{package}")
        okay = run(command) == 0
        if okay:
            imports = "import lctx_mcp, lctx_semantics, lctx_storage" if package == "mcp" else f"import lctx_{package}"
            okay = run(("uv", "run", "--no-sync", "python", "-c", imports)) == 0
        ready[package] = okay
        if package == "mcp":
            ready.update(semantics=okay, storage=okay)
    if packages and before != native_input_fingerprints(ROOT):
        for requirement in ("mcp", "semantics", "storage"):
            if requirement in ready:
                ready[requirement] = False
        print("blocked: native source inputs changed during readiness", flush=True)
    if "postgres" in requirements:
        ready["postgres"] = run(("just", "postgres-test-ready")) == 0
    if "cli" in requirements:
        ready["cli"] = run(("cargo", "build", "--release", "-p", "lctx")) == 0
    return ready


def execute(selected: list[str], ready: dict[str, bool], run: Runner,
            arguments: tuple[str, ...] = (), command: str | None = None) -> dict[str, str]:
    results = {}
    for name in selected:
        family = FAMILIES[name]
        missing = sorted(r for r in prerequisites(name, command) if not ready.get(r, False))
        if missing:
            results[name] = "blocked"
            print(f"{name}: blocked (prerequisite: {', '.join(missing)})", flush=True)
            continue
        commands = family.commands if command is None else (family.commands[COMMANDS[name].index(command)],)
        codes = [run(invocation + arguments) for invocation in commands]
        results[name] = "passed" if all(code == 0 for code in codes) else "blocked" if all(code in (0, 127) for code in codes) else "failed"
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
    parser.add_argument("--command", help="select a family boundary before passing its ordinary tool filters")
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
    env = normalized_env(dict(os.environ, INSTA_UPDATE="no", UV_NO_SYNC="1"),
                         native_inputs=bool(requirements & {"mcp", "semantics", "storage"}))

    def run(command: tuple[str, ...]) -> int:
        return launch(command, env)

    # Hold ownership through live workers, not just sync. This prevents another
    # family launcher preparing the shared environment while these fixtures run.
    with environment_owner(bool(requirements & {"tools", "mcp", "semantics", "storage"})):
        ready = prepare(requirements, run)
        results = execute(selected, ready, run, arguments, args.command)
        if args.family == "qualify":
            # Independent non-functional leaves still run after functional failures.
            for name in ("clippy", "lint-agents", "adr-lint", "fixtures-check", "gold",
                         "rules-scan", "rules-test", "ruff", "types", "docs-check", "deps"):
                if name in ("ruff", "types", "docs-check", "deps", "gold", "adr-lint", "fixtures-check") and not ready.get("tools"):
                    results[name] = "blocked"
                else:
                    code = run(("just", name))
                    results[name] = "passed" if code == 0 else "blocked" if code == 127 else "failed"
                print(f"{name}: {results[name]}", flush=True)
    return 0 if all(result == "passed" for result in results.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())
