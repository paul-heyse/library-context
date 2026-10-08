#!/usr/bin/env python3
"""Contract verification: one resolved command plan over boundary definitions (plan D4).

    just verify --select FAMILY[:BOUNDARY] [--nextest-args "…"] [--pytest-args "…"] … [options]
    just verify --print …     static plan: commands, environment names, prerequisites, readiness
    just verify --list …      builds: tool discovery (nextest list, pytest --collect-only)
    just verify --rerun ID    the failed, blocked or unexecuted boundaries of run ID's selection
    just verify qualify       every boundary and leaf; no reuse, no filters, no self-preparation
    just verify-<family> [--command BOUNDARY] [-- ARGS]   shortcut; ARGS reach the primary tool

Tool arguments attach to the preceding ``--select`` (before any ``--select`` they apply to every
selected boundary that has that tool). ``BOUNDARIES`` is the only catalogue: plan, ``--print``,
``--list``, execution, ``summary.json`` and this help derive from it.

Readiness observes and never synchronizes (ADR-0134): a missing prerequisite is ``blocked`` with
its repair route. Fixture-backed boundaries each get an owned native SurrealDB attachment
(``surrealdb_fixture``); ``--attach ID`` uses a kept fixture instead. Outcomes are ``passed``,
``failed``, ``blocked`` or ``not_run``; ``blocked`` needs evidence (a failed readiness
observation, a fixture OOM or server end, launch error 127). Interruption is the run's
termination, never an outcome. Execution always runs under a run handle (``runs.py``): logs and
``summary.json`` live in its directory, and ``just runs`` observes, cancels and prunes it.
"""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import shlex
import signal
import subprocess
import sys
import time
from collections.abc import Callable, Iterator, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import runs
import workspace_env
from build_environment import ROOT, normalized_env
from harness import OUTCOMES, write_json_atomic

SCHEMA = 1
NEXTEST_RUN = ("cargo", "nextest", "run", "--release", "--no-fail-fast", "--no-tests=fail")
PYTEST = ("uv", "run", "--no-sync", "pytest")
TOOLS = ("nextest", "pytest")
TAIL_LINES = 25


# ---------------------------------------------------------------------------------------------
# Definitions: the single source


@dataclass(frozen=True)
class Step:
    """One command of a boundary.

    ``tool`` is ``nextest`` (packages + default targets), ``pytest`` (paths), ``cargo`` (a full
    cargo argv such as a build) or ``just`` (a recipe). Only nextest and pytest steps receive
    tool arguments. ``env`` values may use ``{release}`` (the release artifact directory) and
    ``{serving}`` (the serving configuration this boundary produces or reuses); ``option_env``
    entries apply only when their option (e.g. ``cli``) is selected.
    """

    name: str
    tool: str
    argv: tuple[str, ...] = ()
    packages: tuple[str, ...] = ()
    targets: tuple[str, ...] = ()
    env: tuple[tuple[str, str], ...] = ()
    produces_serving: bool = False
    restart_after: bool = False
    when: str | None = None  # an option that enables this step (e.g. "cli")
    option_env: tuple[tuple[str, str, str], ...] = ()  # (option, name, value) when option is set


@dataclass(frozen=True)
class Boundary:
    family: str
    name: str
    description: str
    steps: tuple[Step, ...]
    requirements: frozenset[str] = frozenset()
    fixture: bool = False

    @property
    def id(self) -> str:
        return f"{self.family}:{self.name}"

    def tools(self) -> set[str]:
        return {step.tool for step in self.steps if step.tool in TOOLS}

    def primary_tool(self) -> str | None:
        return next((step.tool for step in self.steps if step.tool in TOOLS), None)


def nextest(name: str, *packages: str, targets: Sequence[str] = ()) -> Step:
    return Step(name, "nextest", packages=packages, targets=tuple(targets))


def tests(*names: str) -> tuple[str, ...]:
    return tuple(item for name in names for item in ("--test", name))


# Requirements follow what a boundary's commands import (D1): `tools` only where a command runs
# the shared environment (`uv run`), never because a family once declared it.
STORE = frozenset({"native-store"})
SERVING = frozenset({"native-serving"})
LEAVES_NEEDING_TOOLS = frozenset(
    {"ruff", "types", "docs-check", "deps", "gold", "adr-lint", "fixtures-check"}
)
LEAVES = (
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
)

BOUNDARIES: tuple[Boundary, ...] = (
    # presentation.rs runs `uv run --no-sync python -I` against the shared environment.
    Boundary(
        "model",
        "rust",
        "model declarations",
        (nextest("nextest", "lctx-model"),),
        frozenset({"tools"}),
    ),
    Boundary("analytics", "rust", "analytics kernels", (nextest("nextest", "lctx-analytics"),)),
    Boundary(
        "providers",
        "extract",
        "native providers over persisted contributions",
        (
            nextest(
                "nextest",
                "cpg-extract",
                targets=(
                    "--lib",
                    *tests(
                        "acquisition",
                        "bundle",
                        "harness",
                        "typed_conformance",
                        "typed_flow",
                        "typed_calls",
                        "native_overload_origins",
                        "typed_ruff_context",
                    ),
                ),
            ),
        ),
        # tests/harness.rs runs `uv run --no-sync pyrefly`.
        STORE | {"tools"},
        fixture=True,
    ),
    Boundary(
        "providers",
        "flow",
        "flow provider (pure Rust)",
        (
            nextest(
                "nextest", "cpg-flow", targets=("--lib", *tests("flow_shapes", "capture_timing"))
            ),
        ),
    ),
    Boundary(
        "compiler",
        "producer",
        "persisted compiler",
        (nextest("nextest", "cpg-core", targets=("--lib", "--tests")),),
        STORE,
        fixture=True,
    ),
    Boundary(
        "compiler",
        "cli",
        "compiler CLI",
        (
            nextest(
                "nextest",
                "lctx",
                targets=("--bin", "lctx", *tests("acquire", "compile_artifact")),
            ),
        ),
        STORE,
        fixture=True,
    ),
    Boundary(
        "store",
        "rust",
        "native store and publication",
        (
            Step(
                "build-cli",
                "cargo",
                ("cargo", "build", "--release", "--locked", "-p", "lctx", "--bin", "lctx"),
                when="cli",
            ),
            Step(
                "nextest",
                "nextest",
                packages=("lctx-surrealdb", "lctx-publisher"),
                # publication.rs selects through this CLI when set, else in-process.
                option_env=(("cli", "LCTX_REMEDIATION_CLI_BIN", "{release}/lctx"),),
            ),
        ),
        STORE,
        fixture=True,
    ),
    Boundary(
        "serving",
        "rust",
        "native serving",
        (nextest("nextest", "lctx-serving"),),
        SERVING,
        fixture=True,
    ),
    Boundary(
        "serving",
        "mcp",
        "MCP journey: produce serving content, restart, then the Python wire/session suite",
        (
            Step(
                "build",
                "cargo",
                ("cargo", "build", "--release", "--locked", "-p", "lctx-eval", "-p", "lctx"),
            ),
            Step(
                "native_journey",
                "cargo",
                (
                    "cargo",
                    "test",
                    "--release",
                    "--locked",
                    "-p",
                    "lctx-serving",
                    "--test",
                    "native_journey",
                    "--",
                    "--nocapture",
                ),
                env=(("LCTX_RETAIN_NATIVE_FIXTURE_CONFIG", "{serving}"),),
                produces_serving=True,
                restart_after=True,
            ),
            Step(
                "pytest",
                "pytest",
                (
                    "python/lctx_mcp/tests/test_wire_contract.py",
                    "python/lctx_mcp/tests/test_native_session.py",
                    "python/lctx_mcp/tests/test_safe_failure.py",
                    "tests/scripts/test_programmatic_native.py",
                    "tests/scripts/test_programmatic_native_numeric.py",
                ),
                env=(
                    ("LCTX_NATIVE_SERVING_CONFIG", "{serving}"),
                    ("LCTX_NATIVE_TEST_LIBRARY", "synthesis-sources"),
                    ("LCTX_EVAL_WORKER", "{release}/lctx-eval"),
                    ("LCTX_REMEDIATION_CLI_BIN", "{release}/lctx"),
                ),
            ),
        ),
        frozenset({"tools", "native-serving", "native-python"}),
        fixture=True,
    ),
    Boundary(
        "oracles",
        "flow",
        "flow soundness oracle over the product binary",
        (
            Step("build", "cargo", ("cargo", "build", "--release", "--locked", "-p", "lctx")),
            Step("pytest", "pytest", ("tests/scripts/test_flow_soundness.py",)),
        ),
        frozenset({"tools"}),
    ),
    Boundary(
        "tooling",
        "python",
        "harness and tooling scripts",
        (
            Step(
                "pytest",
                "pytest",
                ("tests/scripts", "--ignore=tests/scripts/test_flow_soundness.py"),
            ),
        ),
        frozenset({"tools"}),
    ),
    Boundary(
        "tooling",
        "docs",
        "Rust doc tests",
        (
            Step(
                "doctest",
                "cargo",
                ("cargo", "test", "--release", "--workspace", "--doc", "--no-fail-fast"),
            ),
        ),
    ),
    *(
        Boundary(
            "leaf",
            name,
            f"non-functional leaf `just {name}` (qualification)",
            (Step(name, "just", ("just", name)),),
            frozenset({"tools"}) if name in LEAVES_NEEDING_TOOLS else frozenset(),
        )
        for name in LEAVES
    ),
)

# Requirements observed through the shared Python environment.
PYTHON_REQUIREMENTS = ("tools", "native-python")
NATIVE_REQUIREMENTS = ("native-store", "native-serving")


def boundaries() -> dict[str, Boundary]:
    return {boundary.id: boundary for boundary in BOUNDARIES}


def families() -> dict[str, list[Boundary]]:
    found: dict[str, list[Boundary]] = {}
    for boundary in BOUNDARIES:
        found.setdefault(boundary.family, []).append(boundary)
    return found


# ---------------------------------------------------------------------------------------------
# Workspace targets, as Cargo reports them (read-only; builds nothing)

LIBRARY_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"}
_CARGO_TARGETS: dict[str, dict[str, set[str]]] | None = None


def cargo_targets() -> dict[str, dict[str, set[str]]]:
    """package -> target kind -> names, from `cargo metadata --no-deps --offline`."""
    global _CARGO_TARGETS
    if _CARGO_TARGETS is None:
        done = subprocess.run(
            ("cargo", "metadata", "--no-deps", "--offline", "--format-version", "1"),
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        )
        found: dict[str, dict[str, set[str]]] = {}
        for package in json.loads(done.stdout)["packages"]:
            kinds = found.setdefault(package["name"], {})
            for target in package["targets"]:
                for kind in target["kind"]:
                    kind = "lib" if kind in LIBRARY_KINDS else kind
                    kinds.setdefault(kind, set()).add(target["name"])
        _CARGO_TARGETS = found
    return _CARGO_TARGETS


NAMED_TARGETS = {"--bin": "bin", "--test": "test", "--example": "example", "--bench": "bench"}
PLURAL_TARGETS = {"--lib", "--bins", "--tests", "--examples", "--benches", "--all-targets"}


def target_options(arguments: Sequence[str]) -> list[tuple[str, str | None]]:
    """Cargo target selections inside nextest arguments, as (option, name)."""
    found: list[tuple[str, str | None]] = []
    items = list(arguments)
    index = 0
    while index < len(items):
        item = items[index]
        if item == "--":
            break
        option, _, value = item.partition("=")
        if option in NAMED_TARGETS:
            if not value and index + 1 < len(items):
                index += 1
                value = items[index]
            found.append((option, value))
        elif item in PLURAL_TARGETS:
            found.append((item, None))
        index += 1
    return found


class PlanError(ValueError):
    pass


def narrow(
    boundary: Boundary,
    step: Step,
    arguments: Sequence[str],
    targets: Mapping[str, Mapping[str, set[str]]] | None = None,
) -> tuple[tuple[str, ...], tuple[str, ...]]:
    """(packages, default targets) for a nextest step once the user's targets are known.

    Explicit target options replace the default targets. A named target narrows the packages to
    those of the boundary's package set that have it; one no package has is a plan error naming
    the actual set and what each package does have."""
    selections = target_options(arguments)
    if not selections:
        return step.packages, step.targets
    known = cargo_targets() if targets is None else targets
    kept: list[str] = []
    for option, name in selections:
        if option in PLURAL_TARGETS and option != "--lib":
            return step.packages, ()
        kind = "lib" if option == "--lib" else NAMED_TARGETS[option]
        owners = [
            package
            for package in step.packages
            if (
                known.get(package, {}).get(kind)
                if option == "--lib"
                else name in known.get(package, {}).get(kind, set())
            )
        ]
        if not owners:
            what = "a library" if option == "--lib" else f"{option} {name}"
            listing = "; ".join(
                f"{package}: {', '.join(sorted(known.get(package, {}).get(kind, ()))) or '-'}"
                for package in step.packages
            )
            raise PlanError(
                f"{boundary.id}: no package in its set ({', '.join(step.packages)}) has {what}"
                f" (its {kind} targets: {listing})"
            )
        kept += [package for package in owners if package not in kept]
    return tuple(package for package in step.packages if package in kept), ()


# ---------------------------------------------------------------------------------------------
# Selection and the resolved plan


@dataclass
class Selection:
    select: str | None
    nextest_args: list[str] = field(default_factory=list)
    pytest_args: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class Options:
    cli: bool = False
    cargo_config: tuple[str, ...] = ()
    attach: str | None = None
    serving: str | None = None
    retain_serving: str | None = None
    qualify: bool = False

    def to_json(self) -> dict[str, Any]:
        return {
            "cli": self.cli,
            "cargo_config": list(self.cargo_config),
            "attach": self.attach,
            "serving": self.serving,
            "retain_serving": self.retain_serving,
            "qualify": self.qualify,
        }


@dataclass(frozen=True)
class PlannedStep:
    step: Step
    argv: tuple[str, ...]
    skipped: str | None = None  # why the step is not part of this plan


@dataclass(frozen=True)
class Planned:
    boundary: Boundary
    selection: dict[str, Any]
    steps: tuple[PlannedStep, ...]


def with_cargo_config(argv: tuple[str, ...], configuration: Sequence[str]) -> tuple[str, ...]:
    if not configuration or argv[:1] != ("cargo",):
        return argv
    options = tuple(item for value in configuration for item in ("--config", value))
    if argv[1:3] == ("nextest", "run") or argv[1:3] == ("nextest", "list"):
        return argv[:3] + options + argv[3:]
    return argv[:1] + options + argv[1:]


def step_argv(
    boundary: Boundary,
    step: Step,
    nextest_args: Sequence[str],
    pytest_args: Sequence[str],
    options: Options,
    targets: Mapping[str, Mapping[str, set[str]]] | None = None,
    *,
    listing: bool = False,
) -> tuple[str, ...]:
    if step.tool == "nextest":
        chosen, defaults = narrow(boundary, step, nextest_args, targets)
        selection = (*(item for p in chosen for item in ("-p", p)), *defaults)
        head = (
            ("cargo", "nextest", "list", "--release", "--message-format", "json")
            if listing
            else NEXTEST_RUN
        )
        argv = (*head, *selection, *nextest_args)
    elif step.tool == "pytest":
        extra = ("--collect-only", "-q") if listing else ("-q",)
        argv = (*PYTEST, *step.argv, *extra, *pytest_args)
    else:
        argv = step.argv
    return with_cargo_config(argv, options.cargo_config)


def expand(select: str) -> list[Boundary]:
    known = boundaries()
    if select in known:
        return [known[select]]
    family = families().get(select)
    if family:
        return list(family)
    raise PlanError(f"unknown selection {select!r}; choose a family or one of: {', '.join(known)}")


def resolve(
    selections: Sequence[Selection],
    options: Options,
    targets: Mapping[str, Mapping[str, set[str]]] | None = None,
) -> list[Planned]:
    """One plan: every selected boundary with its own arguments routed to their owners."""
    shared = [s for s in selections if s.select is None]
    scoped = [s for s in selections if s.select is not None]
    if options.qualify:
        if any(s.nextest_args or s.pytest_args for s in selections) or scoped:
            raise PlanError("qualify requires its complete declared selections (no filters)")
        if options.attach or options.serving or options.retain_serving:
            raise PlanError("qualify refuses reuse: no --attach, --serving or --retain-serving")
        scoped = [Selection(boundary.id) for boundary in BOUNDARIES]
    if not scoped:
        raise PlanError("select at least one boundary: --select FAMILY[:BOUNDARY]")
    if (options.serving or options.retain_serving) and not options.attach:
        raise PlanError("--serving/--retain-serving need a kept fixture: --attach ID")
    common_nextest = [a for s in shared for a in s.nextest_args]
    common_pytest = [a for s in shared for a in s.pytest_args]
    for args in (common_nextest, common_pytest):
        if any(a.startswith("--no-tests") for a in args):
            raise PlanError("empty required selections must fail; --no-tests is fixed")
    plan: list[Planned] = []
    seen: dict[str, str] = {}
    for selection in scoped:
        assert selection.select is not None
        chosen = expand(selection.select)
        for args, tool in ((selection.nextest_args, "nextest"), (selection.pytest_args, "pytest")):
            if any(a.startswith("--no-tests") for a in args):
                raise PlanError("empty required selections must fail; --no-tests is fixed")
            if args and not any(tool in boundary.tools() for boundary in chosen):
                owners = sorted({t for b in chosen for t in b.tools()}) or ["none"]
                raise PlanError(
                    f"{selection.select} has no {tool} step; its tool arguments go to:"
                    f" {', '.join(owners)}"
                )
        for boundary in chosen:
            if boundary.id in seen:
                raise PlanError(
                    f"{boundary.id} selected twice ({seen[boundary.id]}, {selection.select})"
                )
            seen[boundary.id] = selection.select
            nextest_args = [*common_nextest, *selection.nextest_args]
            pytest_args = [*common_pytest, *selection.pytest_args]
            steps = []
            for step in boundary.steps:
                skipped = None
                if step.when == "cli" and not options.cli:
                    skipped = "enabled by --cli"
                elif step.produces_serving and options.serving:
                    skipped = f"reuses retained serving content {options.serving!r}"
                argv = step_argv(boundary, step, nextest_args, pytest_args, options, targets)
                steps.append(PlannedStep(step, argv, skipped))
            plan.append(
                Planned(
                    boundary,
                    {
                        "select": boundary.id,
                        "nextest_args": nextest_args if "nextest" in boundary.tools() else [],
                        "pytest_args": pytest_args if "pytest" in boundary.tools() else [],
                    },
                    tuple(steps),
                )
            )
    if options.serving and not any(p.boundary.id == "serving:mcp" for p in plan):
        raise PlanError("--serving applies to serving:mcp; select it")
    if options.retain_serving and not any(p.boundary.id == "serving:mcp" for p in plan):
        raise PlanError("--retain-serving applies to serving:mcp; select it")
    return plan


# ---------------------------------------------------------------------------------------------
# Readiness (observation only)


# A readiness observation is `workspace_env.Readiness` or `surrealdb_fixture.Readiness`: both have
# `requirement`, `ready`, `repair` and `message()`. A static plan leaves `native-python` unobserved
# (observing it needs the native input fingerprint); it is reported, not invented.
UNOBSERVED_STATIC = ("native-python",)


def observe_requirements(
    requirements: set[str],
    *,
    static: bool = False,
    python: Callable[[str], Any] = workspace_env.observe,
    native: Callable[[str], Any] | None = None,
) -> dict[str, Any]:
    """Observe each prerequisite; never synchronize or start anything."""
    found: dict[str, Any] = {}
    for requirement in sorted(requirements):
        if static and requirement in UNOBSERVED_STATIC:
            continue
        if requirement in PYTHON_REQUIREMENTS:
            found[requirement] = python(requirement)
        elif requirement in NATIVE_REQUIREMENTS:
            if native is None:
                import surrealdb_fixture

                native = surrealdb_fixture.substrate_readiness
            found[requirement] = native(requirement)
        else:
            raise PlanError(f"unknown prerequisite {requirement}")
    return found


def environment_owner(requirements: set[str] | frozenset[str], report=None):
    """Shared ownership of what a boundary imports; pure Rust takes none."""
    needed = [r for r in PYTHON_REQUIREMENTS if r in requirements]
    if report is None:
        return workspace_env.ownership("shared", needed)
    return workspace_env.ownership("shared", needed, report=report)


# ---------------------------------------------------------------------------------------------
# Execution


class Interrupted(Exception):
    pass


@dataclass
class Runtime:
    """Everything execution touches outside this module; tests replace it with fakes."""

    observe: Callable[[set[str]], dict[str, Any]]
    run: Callable[[Sequence[str], Mapping[str, str], Path, bool], int]
    fixture: Callable[[Options], contextlib.AbstractContextManager[Any]]
    owner: Callable[[frozenset[str], Callable[[str], None]], contextlib.AbstractContextManager[Any]]
    progress: Callable[..., None] = lambda **_: None
    report: Callable[[str], None] = lambda message: print(message, flush=True)
    base_env: Mapping[str, str] = field(default_factory=dict)
    release_dir: Path = ROOT / "target" / "release"
    stop: Callable[[], bool] = lambda: False


def _fixture_blocked(error: BaseException) -> str | None:
    kind = getattr(error, "kind", None)
    detail = getattr(error, "detail", None)
    if kind is None or detail is None:
        return None
    repair = getattr(error, "repair", None)
    return f"fixture {kind}: {detail}" + (f"; repair: {repair}" if repair else "")


def _server_end(attachment: Any) -> tuple[str, str] | None:
    """(outcome, reason) when the fixture server ended during a step. Only the fixture's OOM
    report is infrastructure evidence (plan §5.8.5); any other end is a product failure."""
    server = getattr(attachment, "server", None)
    if server is None:
        return None
    ended = server.exited()
    if ended is None:
        return None
    if ended == "oom-kill":
        return "blocked", f"fixture OOM: server {server.id} was oom-killed (evidence: oom-kill)"
    return "failed", f"fixture server exited ({ended}) during the step"


def run_boundary(
    planned: Planned,
    runtime: Runtime,
    observations: Mapping[str, Any],
    options: Options,
    log: Path,
    live: bool,
) -> dict[str, Any]:
    boundary = planned.boundary
    started = time.monotonic()
    result: dict[str, Any] = {
        "boundary": boundary.id,
        "outcome": "not_run",
        "reason": "",
        "log": str(log),
        "duration": 0.0,
        "fixture": None,
        "selection": planned.selection,
        "steps": [],
    }

    def done(outcome: str, reason: str) -> dict[str, Any]:
        assert outcome in OUTCOMES
        result.update(outcome=outcome, reason=reason)
        result["duration"] = round(time.monotonic() - started, 3)
        return result

    if runtime.stop():
        return done("not_run", "interrupted before it started")
    missing = [observations[r] for r in sorted(boundary.requirements) if not observations[r].ready]
    if missing:
        reason = "; ".join(
            f"prerequisite {o.requirement}: {o.repair or o.message()}" for o in missing
        )
        return done("blocked", reason)
    log.parent.mkdir(parents=True, exist_ok=True)
    runtime.progress(current_command=boundary.id, waiting_reason=None)

    def waiting(message: str) -> None:
        runtime.report(message)
        runtime.progress(waiting_reason=message.splitlines()[0])

    try:
        with runtime.owner(boundary.requirements, waiting) as owned:
            runtime.progress(waiting_reason=None)
            # Children reuse this ownership instead of acquiring again (§5.8.4).
            base = owned.environment(runtime.base_env)
            fixture_scope = (
                runtime.fixture(options) if boundary.fixture else contextlib.nullcontext()
            )
            with fixture_scope as attachment:
                return _run_steps(
                    planned, runtime, options, attachment, base, log, live, result, done
                )
    except Interrupted:
        return done("not_run", "interrupted")
    except Exception as error:  # evidence-bearing fixture failures are infrastructure
        blocked = _fixture_blocked(error)
        if blocked is None:
            raise
        if result["outcome"] == "passed":
            return done("blocked", f"after passing steps: {blocked}")
        return done("blocked", blocked)


def _expand(value: str, release: Path, serving: Path | None) -> str:
    value = value.replace("{release}", str(release))
    if "{serving}" in value:
        if serving is None:
            raise PlanError("a step needs {serving} outside a fixture")
        value = value.replace("{serving}", str(serving))
    return value


def _run_steps(
    planned: Planned,
    runtime: Runtime,
    options: Options,
    attachment: Any,
    base: Mapping[str, str],
    log: Path,
    live: bool,
    result: dict[str, Any],
    done: Callable[[str, str], dict[str, Any]],
) -> dict[str, Any]:
    boundary = planned.boundary
    env = dict(base)
    serving: Path | None = None
    if attachment is not None:
        result["fixture"] = {
            "id": attachment.server.id,
            "kind": attachment.server.kind,
            "attachment": attachment.id,
        }
        producing = any(s.step.produces_serving for s in planned.steps)
        if producing and options.serving:
            reuse = attachment.use_serving(options.serving)
            result["fixture"]["serving_reuse"] = reuse
            serving = Path(attachment.extra_env["LCTX_NATIVE_SERVING_CONFIG"])
        elif producing and options.retain_serving:
            serving = attachment.retain_serving(options.retain_serving)
        elif producing:
            serving = attachment.scratch / "serving.json"
        env = attachment.environment(env)
    for planned_step in planned.steps:
        step = planned_step.step
        record: dict[str, Any] = {"name": step.name, "argv": list(planned_step.argv)}
        result["steps"].append(record)
        if planned_step.skipped:
            record.update(outcome="not_run", reason=planned_step.skipped)
            if step.produces_serving and options.serving:
                record["identity"] = result["fixture"]["serving_reuse"]
            continue
        if runtime.stop():
            raise Interrupted
        step_env = dict(env)
        for key, value in step.env:
            step_env[key] = _expand(value, runtime.release_dir, serving)
        for option, key, value in step.option_env:
            if getattr(options, option):
                step_env[key] = _expand(value, runtime.release_dir, serving)
        runtime.progress(
            current_command=f"{boundary.id} {step.name}: {shlex.join(planned_step.argv)}"
        )
        began = time.monotonic()
        code = runtime.run(planned_step.argv, step_env, log, live)
        record.update(exit=code, duration=round(time.monotonic() - began, 3))
        if runtime.stop():
            record["outcome"] = "not_run"
            raise Interrupted
        ended = _server_end(attachment) if attachment is not None else None
        if ended:
            record.update(outcome=ended[0], server_end=ended[1])
            return done(*ended)
        if code == 127:
            record["outcome"] = "blocked"
            return done("blocked", f"launch: cannot start {planned_step.argv[0]} (exit 127)")
        if code != 0:
            record["outcome"] = "failed"
            return done("failed", f"step {step.name} exited {code}")
        record["outcome"] = "passed"
        if step.produces_serving and options.retain_serving:
            identity = attachment.record_serving(options.retain_serving, planned_step.argv)
            result["fixture"]["serving_retained"] = identity
        if step.restart_after and attachment is not None:
            attachment.restart()
    return done("passed", "")


def execute(
    plan: Sequence[Planned],
    runtime: Runtime,
    options: Options,
    log_dir: Path,
    *,
    live: bool = False,
    on_result: Callable[[list[dict[str, Any]]], None] = lambda _results: None,
) -> list[dict[str, Any]]:
    """Run every planned boundary; one boundary's outcome never decides another's."""
    requirements = set().union(*(p.boundary.requirements for p in plan))
    observations = runtime.observe(requirements)
    for observation in observations.values():
        if not observation.ready:
            runtime.report(observation.message())
    results: list[dict[str, Any]] = []
    for planned in plan:
        log = log_dir / f"{planned.boundary.family}-{planned.boundary.name}.log"
        runtime.report(f"-- {planned.boundary.id}: running (log {log})")
        result = run_boundary(planned, runtime, observations, options, log, live)
        results.append(result)
        _report_result(result, runtime.report, live)
        on_result(results)
    runtime.progress(current_command=None, waiting_reason=None)
    return results


def _report_result(result: dict[str, Any], report: Callable[[str], None], live: bool) -> None:
    line = f"{result['boundary']}: {result['outcome']} ({result['duration']:.1f}s)"
    if result["reason"]:
        line += f" - {result['reason']}"
    report(line)
    if result["outcome"] == "failed" and not live:
        try:
            lines = Path(result["log"]).read_text(errors="replace").splitlines()
        except OSError:
            return
        report("\n".join(f"   | {text}" for text in lines[-TAIL_LINES:]))
        report(f"   full log: {result['log']}")


# ---------------------------------------------------------------------------------------------
# Real runtime pieces


def launch(argv: Sequence[str], env: Mapping[str, str], log: Path, live: bool) -> int:
    """Run one step in this process group; its output goes to the boundary log (and stdout with
    --live). A missing executable is exit 127."""
    with open(log, "ab") as handle:
        handle.write(f"$ {shlex.join(argv)}\n".encode())
        handle.flush()
        try:
            child = subprocess.Popen(
                list(argv),
                cwd=ROOT,
                env=dict(env),
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE if live else handle,
                stderr=subprocess.STDOUT,
            )
        except OSError as error:
            handle.write(f"cannot launch {argv[0]}: {error}\n".encode())
            return 127
        if live:
            assert child.stdout is not None
            for chunk in iter(lambda: child.stdout.read1(65536), b""):  # type: ignore[union-attr]
                handle.write(chunk)
                sys.stdout.buffer.write(chunk)
                sys.stdout.buffer.flush()
        return child.wait()


@contextlib.contextmanager
def real_fixture(options: Options) -> Iterator[Any]:
    import surrealdb_fixture as sf

    if options.attach is None:
        with sf.fixture(report=lambda message: print(message, flush=True)) as attachment:
            yield attachment
        return
    server = sf.Server.open(options.attach)
    if server.kind != "kept":
        raise sf.FixtureBlocked("record", f"{options.attach} is not a kept fixture")
    ended = server.exited()
    if ended is not None:
        raise (
            server.oom("before attaching")
            if ended == "oom-kill"
            else sf.FixtureBlocked(
                "readiness", f"kept fixture {server.id} is not running ({ended})"
            )
        )
    server.ready()
    attachment = sf.Attachment.create(server)
    try:
        yield attachment
    finally:
        attachment.release()


def base_environment() -> dict[str, str]:
    # Test parallelism is never constrained here: nextest and pytest keep their own defaults.
    return normalized_env(dict(os.environ, INSTA_UPDATE="no", UV_NO_SYNC="1"))


def release_directory(env: Mapping[str, str]) -> Path:
    target = env.get("CARGO_TARGET_DIR")
    return (Path(target) if target else ROOT / "target") / "release"


# ---------------------------------------------------------------------------------------------
# Inspection


# Variables verify itself sets. The fixture's attachment variables belong to surrealdb_fixture.
VERIFY_ENVIRONMENT = ("INSTA_UPDATE", "UV_NO_SYNC")


def fixture_variables() -> list[str]:
    import surrealdb_fixture

    names = getattr(surrealdb_fixture, "ATTACHMENT_VARIABLES", None)
    return list(names) if names else ["<surrealdb_fixture attachment variables>"]


def describe(plan: Sequence[Planned], observations: Mapping[str, Any], options: Options):
    rows = []
    for planned in plan:
        boundary = planned.boundary
        names = list(VERIFY_ENVIRONMENT)
        if boundary.requirements & {"tools", "native-python"}:
            names.append(workspace_env.OWNERSHIP_KEY)
        if boundary.fixture:
            names += fixture_variables()
        fixture = None
        if boundary.fixture:
            fixture = (
                f"kept fixture {options.attach} (new attachment)"
                if options.attach
                else "run-owned native SurrealDB (one per boundary)"
            )
        rows.append(
            {
                "boundary": boundary.id,
                "description": boundary.description,
                "requirements": sorted(boundary.requirements),
                "fixture": fixture,
                "steps": [
                    {
                        "name": s.step.name,
                        "argv": list(s.argv),
                        "env": sorted(
                            {
                                *names,
                                *(k for k, _ in s.step.env),
                                *(k for o, k, _ in s.step.option_env if getattr(options, o)),
                            }
                        ),
                        "skipped": s.skipped,
                    }
                    for s in planned.steps
                ],
            }
        )
    return {
        "boundaries": rows,
        "readiness": {
            r: {"ready": o.ready, "message": o.message(), "repair": o.repair}
            for r, o in observations.items()
        },
        "unobserved": sorted(
            {r for p in plan for r in p.boundary.requirements} & set(UNOBSERVED_STATIC)
        ),
        "options": options.to_json(),
    }


def print_plan(description: Mapping[str, Any]) -> None:
    print("static plan: builds nothing, starts no fixture, computes no fingerprint")
    for row in description["boundaries"]:
        needs = ", ".join(row["requirements"]) or "none"
        print(f"{row['boundary']}  ({row['description']}; requires {needs})")
        if row["fixture"]:
            print(f"  fixture: {row['fixture']}")
        for step in row["steps"]:
            mark = f"  [not run: {step['skipped']}]" if step["skipped"] else ""
            print(f"  $ {shlex.join(step['argv'])}{mark}")
            print(f"    env: {', '.join(step['env'])}")
    if description["readiness"] or description["unobserved"]:
        print("readiness:")
        for observation in description["readiness"].values():
            print(f"  {observation['message']}")
        for requirement in description["unobserved"]:
            print(f"  {requirement}: observed at execution (needs the native input fingerprint)")


def list_plan(plan: Sequence[Planned], options: Options, env: Mapping[str, str]) -> int:
    """Each tool's own discovery. This builds the selected test binaries (reusing what is
    current; cargo rebuilds what changed) and starts no fixture."""
    print("listing builds: cargo nextest list compiles the selected test binaries")
    status = 0
    for planned in plan:
        print(f"{planned.boundary.id}")
        for planned_step in planned.steps:
            step = planned_step.step
            if step.tool not in TOOLS:
                continue
            argv = step_argv(
                planned.boundary,
                step,
                planned.selection["nextest_args"],
                planned.selection["pytest_args"],
                options,
                listing=True,
            )
            done = subprocess.run(argv, cwd=ROOT, env=dict(env), capture_output=True, text=True)
            if done.returncode:
                status = 1
                tail = (done.stderr or done.stdout).strip().splitlines()[-5:]
                print(f"  {step.name}: listing failed (exit {done.returncode})")
                print("\n".join(f"    {line}" for line in tail))
                continue
            names = (
                nextest_matches(done.stdout)
                if step.tool == "nextest"
                else [line for line in done.stdout.splitlines() if "::" in line]
            )
            print(f"  {step.name}: {len(names)} test(s)")
            for name in names:
                print(f"    {name}")
    return status


def nextest_matches(output: str) -> list[str]:
    data = json.loads(output)
    found = []
    for binary, suite in sorted(data.get("rust-suites", {}).items()):
        for name, case in sorted(suite.get("testcases", {}).items()):
            if case.get("filter-match", {}).get("status") == "matches":
                found.append(f"{binary} {name}")
    return found


# ---------------------------------------------------------------------------------------------
# Command line


class _Select(argparse.Action):
    def __call__(self, parser, namespace, values, option_string=None):
        namespace.selections.append(Selection(str(values)))


class _ToolArgs(argparse.Action):
    """Tool arguments join the preceding --select; before any, they are shared by all."""

    def __call__(self, parser, namespace, values, option_string=None):
        selections = namespace.selections
        if selections and selections[-1].select is not None:
            target = selections[-1]
        elif selections and selections[0].select is None:
            target = selections[0]
        else:
            target = Selection(None)
            selections.insert(0, target)
        getattr(target, self.dest).extend(shlex.split(str(values)))


def help_epilog() -> str:
    lines = ["boundaries (FAMILY:BOUNDARY; a family selects all of its boundaries):"]
    for boundary in BOUNDARIES:
        needs = ", ".join(sorted(boundary.requirements)) or "none"
        tools = "/".join(sorted(boundary.tools())) or "-"
        fixture = ", fixture" if boundary.fixture else ""
        lines.append(f"  {boundary.id:<22} {boundary.description} [{tools}; {needs}{fixture}]")
    return "\n".join(lines)


def parser() -> argparse.ArgumentParser:
    top = argparse.ArgumentParser(
        prog="just verify",
        description=(__doc__ or "").split("\n\n")[0],
        epilog=help_epilog(),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    top.set_defaults(selections=[])
    top.add_argument("--select", action=_Select, metavar="FAMILY[:BOUNDARY]")
    top.add_argument("--nextest-args", dest="nextest_args", action=_ToolArgs, metavar="ARGS")
    top.add_argument("--pytest-args", dest="pytest_args", action=_ToolArgs, metavar="ARGS")
    mode = top.add_mutually_exclusive_group()
    mode.add_argument("--print", action="store_true", help="static plan; builds nothing")
    mode.add_argument("--list", action="store_true", help="tool discovery; builds")
    top.add_argument("--json", action="store_true", help="--print as JSON")
    top.add_argument("--live", action="store_true", help="stream step output")
    top.add_argument("--rerun", metavar="RUN", help="failed/blocked/unexecuted boundaries of RUN")
    top.add_argument("--qualify", action="store_true", help="every boundary and leaf")
    top.add_argument("--cli", action="store_true", help="store: build and exercise the native CLI")
    top.add_argument("--cargo-config", action="append", default=[], metavar="KEY=VALUE")
    top.add_argument("--attach", metavar="FIXTURE", help="use a kept fixture (just fixture --keep)")
    serving = top.add_mutually_exclusive_group()
    serving.add_argument("--serving", metavar="NAME", help="serving:mcp reuses retained content")
    serving.add_argument("--retain-serving", metavar="NAME", help="serving:mcp retains its content")
    return top


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    return parser().parse_args(_bind_tool_args(argv))


def _bind_tool_args(argv: Sequence[str]) -> list[str]:
    """Tool argument values often start with `-` (`--nextest-args --lib`); bind them first."""
    items = list(argv)
    bound: list[str] = []
    index = 0
    while index < len(items):
        item = items[index]
        if item in ("--nextest-args", "--pytest-args") and index + 1 < len(items):
            bound.append(f"{item}={items[index + 1]}")
            index += 2
            continue
        bound.append(item)
        index += 1
    return bound


def options_from(args: argparse.Namespace) -> Options:
    return Options(
        cli=args.cli,
        cargo_config=tuple(args.cargo_config),
        attach=args.attach,
        serving=args.serving,
        retain_serving=args.retain_serving,
        qualify=args.qualify,
    )


def legacy(argv: Sequence[str]) -> list[str]:
    """`verify.py FAMILY [--command B] [verify options] [-- ARGS]` and `verify.py qualify`.

    Everything before `--` is verify's own options, parsed by the plan parser like any `just
    verify` invocation; only what follows `--` passes through, to the selected boundary's
    primary tool. An option verify does not know, before `--`, is a usage error."""
    items = list(argv)
    if not items or items[0].startswith("-"):
        return items
    head = items.pop(0)
    if head == "qualify":
        return ["--qualify", *items]
    if head not in families():
        raise PlanError(f"unknown family {head!r}; choose from {', '.join(families())}")
    split = items.index("--") if "--" in items else len(items)
    before, passthrough = items[:split], items[split + 1 :]
    command = None
    options: list[str] = []
    index = 0
    while index < len(before):
        item = before[index]
        if item == "--command" and index + 1 < len(before):
            command = before[index + 1]
            index += 2
            continue
        if item.startswith("--command="):
            command = item.split("=", 1)[1]
        elif item in ("--select", "--rerun") or item.startswith(("--select=", "--rerun=")):
            raise PlanError(
                f"{item.split('=')[0]} is not a family-shortcut option; use"
                f" `just verify {item.split('=')[0]} …`"
            )
        else:
            options.append(item)
        index += 1
    members = families()[head]
    if command is not None and command not in {b.name for b in members}:
        raise PlanError(f"{head} boundaries: {', '.join(b.name for b in members)}")
    chosen = [b for b in members if command is None or b.name == command]
    out = ["--select", head if command is None else f"{head}:{command}", *options]
    _, unknown = parser().parse_known_args(_bind_tool_args(out))
    if unknown:
        raise PlanError(
            f"unknown verify option(s) {' '.join(unknown)}; tool arguments go after `--`"
            f" (`just verify-{head} [--command B] [options] -- ARGS`)"
        )
    if passthrough:
        owners = {b.primary_tool() for b in chosen}
        if len(chosen) > 1 or None in owners:
            raise PlanError(
                f"choose --command {'|'.join(b.name for b in members)} for boundary filters"
            )
        out += [f"--{owners.pop()}-args", shlex.join(passthrough)]
    return out


def rerun_selection(reference: str) -> tuple[list[Selection], Options]:
    summary_file = runs.resolve(reference) / runs.SUMMARY
    data = json.loads(summary_file.read_text())
    selections = [
        Selection(
            item["selection"]["select"],
            item["selection"]["nextest_args"],
            item["selection"]["pytest_args"],
        )
        for item in data["boundaries"]
        if item["outcome"] != "passed"
    ]
    selected = {item["selection"]["select"] for item in data["boundaries"]}
    for planned in data.get("plan", []):
        if planned["select"] not in selected:  # never reached (interrupted before it)
            selections.append(
                Selection(planned["select"], planned["nextest_args"], planned["pytest_args"])
            )
    saved = data.get("options", {})
    options = Options(
        cli=saved.get("cli", False),
        cargo_config=tuple(saved.get("cargo_config", ())),
        attach=saved.get("attach"),
        serving=saved.get("serving"),
        retain_serving=None,  # retained content is recorded once; a rerun does not re-retain
        qualify=False,
    )
    return selections, options


class Summary:
    """`summary.json` in the current run directory (written only here, plan §5.8.5)."""

    def __init__(self, path: Path | None, plan: Sequence[Planned], options: Options) -> None:
        self.path = path
        self.data: dict[str, Any] = {
            "schema": SCHEMA,
            "run_id": os.environ.get("LCTX_RUN_ID"),
            "argv": sys.argv[1:],
            "options": options.to_json(),
            "plan": [p.selection for p in plan],
            "started": runs.now(),
            "ended": None,
            "termination": None,
            "boundaries": [],
        }
        self.write()

    def write(self) -> None:
        if self.path is not None:
            write_json_atomic(self.path, self.data)

    def update(self, results: list[dict[str, Any]]) -> None:
        self.data["boundaries"] = results
        self.write()

    def finish(self, results: list[dict[str, Any]], termination: str) -> None:
        self.data.update(boundaries=results, ended=runs.now(), termination=termination)
        self.write()


def publish_progress(**fields: Any) -> None:
    runs.set_progress(**fields)


def wrap_in_run(argv: Sequence[str]) -> int:
    """Re-execute under a foreground run handle so logs, summary and rerun always exist."""
    label = "verify " + shlex.join(argv)
    command = [sys.executable, str(Path(runs.__file__).resolve()), "run", "--label", label[:120]]
    command += ["--", sys.executable, str(Path(__file__).resolve()), *argv]
    os.execv(sys.executable, command)
    return 1  # unreachable


def main(argv: Sequence[str] | None = None) -> int:
    raw = list(sys.argv[1:] if argv is None else argv)
    try:
        translated = legacy(raw)
    except PlanError as error:
        print(f"verify: {error}", file=sys.stderr)
        return 2
    args = parse_args(translated)
    options = options_from(args)
    selections: list[Selection] = args.selections
    try:
        if args.rerun:
            if any(s.select for s in selections):
                raise PlanError("--rerun repeats a prior selection; do not add --select")
            selections, options = rerun_selection(args.rerun)
            if not selections:
                print(f"verify: nothing to rerun: every boundary of {args.rerun} passed")
                return 0
        plan = resolve(selections, options)
    except (PlanError, runs.RunNotFound, OSError, ValueError) as error:
        print(f"verify: {error}", file=sys.stderr)
        return 2
    env = base_environment()
    requirements = set().union(*(p.boundary.requirements for p in plan))
    if args.print:
        observations = observe_requirements(requirements, static=True)
        description = describe(plan, observations, options)
        if args.json:
            print(json.dumps(description, indent=2))
        else:
            print_plan(description)
        return 0
    if args.list:
        return list_plan(plan, options, env)
    run_dir = runs.current_run()
    if run_dir is None and not os.environ.get("LCTX_VERIFY_UNWRAPPED"):
        return wrap_in_run(raw)
    log_dir = (run_dir or Path(os.environ.get("TMPDIR", "/tmp"))) / "verify"

    stopping: list[int] = []

    def on_signal(signum: int, _frame: Any) -> None:
        """Stop the run: as the run's group leader (always, under a run handle), terminate the
        whole group so steps and their children stop; the fixture then tears down normally."""
        first = not stopping
        stopping.append(signum)
        if first and os.getpgrp() == os.getpid():
            signal.signal(signal.SIGTERM, signal.SIG_IGN)
            with contextlib.suppress(ProcessLookupError):
                os.killpg(os.getpgrp(), signal.SIGTERM)

    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(sig, on_signal)
    summary = Summary(runs.summary_path(), plan, options)
    runtime = Runtime(
        observe=lambda required: observe_requirements(required),
        run=launch,
        fixture=real_fixture,
        owner=lambda required, report: environment_owner(required, report),
        progress=publish_progress,
        base_env=env,
        release_dir=release_directory(env),
        stop=lambda: bool(stopping),
    )
    results = execute(plan, runtime, options, log_dir, live=args.live, on_result=summary.update)
    # Boundaries the interruption kept from starting are recorded, not dropped.
    termination = "interrupted" if stopping else "completed"
    summary.finish(results, termination)
    failed = [r["boundary"] for r in results if r["outcome"] != "passed"]
    print(
        f"verify: {len(results) - len(failed)}/{len(results)} passed"
        + (
            f"; not passed: {', '.join(failed)} (rerun: just verify --rerun "
            f"{os.environ.get('LCTX_RUN_ID', '<run>')})"
            if failed
            else ""
        )
    )
    if stopping:
        return 128 + stopping[0]
    return 0 if not failed else 1


if __name__ == "__main__":
    raise SystemExit(main())
