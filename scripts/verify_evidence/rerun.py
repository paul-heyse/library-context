"""Read-only exact-case experiments over verification's existing dependency catalog."""

from __future__ import annotations

import json
import shlex
from typing import Any

from .contracts import CaptureUnsupported


def cargo_target(invocation: dict[str, Any], case: dict[str, Any]) -> dict[str, Any]:
    """Join the stable native binary ID to retained Cargo artifact declarations.

    No current metadata/build discovery is allowed in a printed experiment plan.
    Nextest 0.9.146 IDs distinguish libraries, integration tests and other kinds.
    """
    binary = case.get("binary_id")
    if not binary:
        raise CaptureUnsupported("exact native binary identity unavailable")
    package, separator, suffix = binary.partition("::")
    if not separator:
        kind, name = "lib", None
    elif "/" in suffix:
        kind, name = suffix.split("/", 1)
    else:
        kind, name = "test", suffix
    matches = {}
    for row in invocation.get("binaries", []):
        if (row.get("profile") or {}).get("test") is False:
            continue  # Cargo also emits ordinary companion executables for integration tests.
        target = row.get("target") or {}
        kinds = target.get("kind", [])
        actual_kind = (
            "lib"
            if set(kinds) & {"lib", "proc-macro", "rlib", "dylib", "staticlib", "cdylib"}
            else kinds[0]
            if len(kinds) == 1
            else None
        )
        if (
            row.get("package_name") == package
            and actual_kind == kind
            and (name is None or target.get("name") == name)
            and row.get("package_id")
        ):
            identity = (
                row["package_id"],
                tuple(kinds),
                target.get("name"),
                json.dumps(row.get("profile"), sort_keys=True),
                json.dumps(row.get("features"), sort_keys=True),
            )
            matches[identity] = row
    if len(matches) != 1:
        raise CaptureUnsupported(
            f"recorded Cargo target for {binary} is missing or ambiguous ({len(matches)} matches)"
        )
    row = next(iter(matches.values()))
    if kind not in {"lib", "test", "bin", "example", "bench"}:
        raise CaptureUnsupported(f"unsupported recorded Cargo target kind: {kind}")
    return row


def narrow_cargo(argv: list[str], artifact: dict[str, Any]) -> list[str]:
    target = artifact["target"]
    kinds = set(target["kind"])
    kind = (
        "lib"
        if kinds & {"lib", "proc-macro", "rlib", "dylib", "staticlib", "cdylib"}
        else next(iter(kinds))
    )
    if kind not in {"lib", "test", "bin", "example", "bench"} or not target.get("name"):
        raise CaptureUnsupported("unsupported or incomplete Cargo target declaration")
    flags = {
        "--workspace",
        "--all",
        "--all-targets",
        "--lib",
        "--bins",
        "--tests",
        "--examples",
        "--benches",
    }
    values = {"-p", "--package", "--exclude", "--bin", "--test", "--example", "--bench"}
    result = []
    index = 0
    while index < len(argv):
        token = argv[index]
        if token == "--":
            result.extend(argv[index:])
            break
        option = token.split("=", 1)[0]
        if option in values:
            if "=" not in token:
                if index + 1 == len(argv):
                    raise CaptureUnsupported(f"missing value for {token}")
                index += 1
        elif token.startswith("-p") and not token.startswith("--") and token != "-p":
            pass
        elif token not in flags:
            result.append(token)
        index += 1
    at = result.index("--") if "--" in result else len(result)
    selection = ["--package", artifact["package_id"], "--" + kind]
    if kind != "lib":
        selection.append(target["name"])
    result[at:at] = selection
    return result


def _matcher(value: str) -> str:
    # Nextest's exact matcher accepts escaped delimiters. This is DSL escaping;
    # shell quoting is applied independently to the completed argv.
    escapes = {"\\": "\\\\", ")": "\\)", ",": "\\,", "\n": "\\n", "\r": "\\r", "\t": "\\t"}
    return "=" + "".join(escapes.get(char, char) for char in value)


def nextest_case(argv: list[str], case: dict[str, Any]) -> list[str]:
    binary = case.get("binary_id")
    test = case.get("test") or case.get("name")
    if not binary or not test:
        raise CaptureUnsupported("exact native binary/test identity unavailable")
    result, filters = [], []
    index = 0
    while index < len(argv):
        item = argv[index]
        if item == "--":
            result.extend(argv[index:])
            break
        if item in ("-E", "--filterset"):
            if index + 1 >= len(argv):
                raise CaptureUnsupported("incomplete original filterset")
            filters.append(argv[index + 1])
            index += 2
            continue
        if item.startswith("--filterset="):
            filters.append(item.split("=", 1)[1])
        elif item.startswith("-E") and item != "-E":
            filters.append(item[2:])
        else:
            result.append(item)
        if item == "--partition" and (
            index + 1 >= len(argv) or not argv[index + 1].startswith("hash:")
        ):
            raise CaptureUnsupported(
                "exact rerun supports hash partition only; count/slice membership changes"
            )
        if item.startswith("--partition=") and not item.split("=", 1)[1].startswith("hash:"):
            raise CaptureUnsupported("exact rerun supports hash partition only")
        index += 1
    exact = f"binary_id({_matcher(binary)}) & test({_matcher(test)})"
    if filters:
        exact = "(" + " | ".join(f"({part})" for part in filters) + ") & (" + exact + ")"
    at = result.index("--") if "--" in result else len(result)
    result[at:at] = ["-E", exact]
    return result


PYTEST_VALUES = {
    "-k",
    "-m",
    "-p",
    "-c",
    "-o",
    "-r",
    "--override-ini",
    "--rootdir",
    "--confcutdir",
    "--ignore",
    "--ignore-glob",
    "--deselect",
    "--maxfail",
    "--tb",
    "--capture",
    "--color",
    "--code-highlight",
    "--log-level",
    "--log-format",
    "--log-date-format",
    "--log-cli-level",
    "--log-cli-format",
    "--log-cli-date-format",
    "--log-file",
    "--log-file-level",
    "--log-file-format",
    "--log-file-date-format",
    "--junitxml",
    "--junit-prefix",
    "--basetemp",
    "--import-mode",
    "--assert",
    "--durations",
    "--durations-min",
    "--verbosity",
    "--cache-show",
    "--report-log",
    "--show-capture",
}
PYTEST_FLAGS = {
    "-q",
    "-qq",
    "-v",
    "-vv",
    "-s",
    "-x",
    "-l",
    "--showlocals",
    "--strict-markers",
    "--strict-config",
    "--strict",
    "--disable-warnings",
    "--full-trace",
    "--runxfail",
    "--lf",
    "--last-failed",
    "--ff",
    "--failed-first",
    "--nf",
    "--new-first",
    "--cache-clear",
    "--no-header",
    "--no-summary",
    "--continue-on-collection-errors",
    "--keep-duplicates",
    "--pyargs",
    "--noconftest",
    "--collect-in-virtualenv",
}


def pytest_case(argv: list[str], nodeid: str, *, require_no_targets: bool = False) -> list[str]:
    if not nodeid or nodeid.startswith("-"):
        raise CaptureUnsupported("exact pytest node identity unavailable")
    try:
        at = argv.index("pytest") + 1
    except ValueError as error:
        raise CaptureUnsupported("unsupported pytest launcher") from error
    result = argv[:at]
    index = at
    while index < len(argv):
        token = argv[index]
        if token == "--":
            if require_no_targets and index + 1 < len(argv):
                raise CaptureUnsupported(
                    "positional targets in pytest addopts prevent an exact printed rerun"
                )
            break  # all remaining tokens are target operands
        option = token.split("=", 1)[0]
        if option in PYTEST_VALUES:
            result.append(token)
            if "=" not in token:
                if index + 1 >= len(argv):
                    raise CaptureUnsupported(f"missing value for {option}")
                result.append(argv[index + 1])
                index += 1
        elif token in PYTEST_FLAGS or token.startswith("-r"):
            result.append(token)
        elif token.startswith("-"):
            raise CaptureUnsupported(
                f"cannot safely distinguish operands for pytest option {token}"
            )
        elif require_no_targets:
            raise CaptureUnsupported(
                "positional targets in pytest addopts prevent an exact printed rerun"
            )
        # Replace all original target operands; never append to their union.
        index += 1
    return [*result, "--", nodeid]


def plan(
    invocation: dict[str, Any],
    case: dict[str, Any],
    summary: dict[str, Any],
    invocations: list[dict[str, Any]],
) -> dict[str, Any]:
    import verify

    if invocation.get("boundary") is None:
        raise CaptureUnsupported(
            "ad hoc capture has no declared prerequisites; exact-case rerun plan is unsupported"
        )
    boundary = next((b for b in verify.BOUNDARIES if b.id == invocation["boundary"]), None)
    if boundary is None:
        raise CaptureUnsupported("recorded boundary no longer exists in current catalog")
    steps = boundary.steps
    target = next((s for s in boundary.steps if s.name == invocation["step"]), None)
    if target is None:
        raise CaptureUnsupported("recorded step no longer exists in current catalog")
    artifact: dict[str, Any] | None = None
    if invocation.get("tool") == "nextest":
        artifact = cargo_target(invocation, case)
        argv = nextest_case(narrow_cargo(invocation["argv"], artifact), case)
    elif invocation.get("tool") == "pytest":
        for tokens in (invocation.get("selection_options") or {}).values():
            pytest_case(["pytest", *tokens], "exact-node", require_no_targets=True)
        # Explicit CLI addopts overrides can also introduce a union of targets.
        for index, arg in enumerate(invocation["argv"]):
            override = (
                invocation["argv"][index + 1]
                if arg in ("-o", "--override-ini") and index + 1 < len(invocation["argv"])
                else arg.partition("=")[2]
                if arg.startswith("--override-ini=")
                else arg[2:]
                if arg.startswith("-o") and len(arg) > 2
                else ""
            )
            if override.startswith("addopts="):
                pytest_case(
                    ["pytest", *shlex.split(override.partition("=")[2])],
                    "exact-node",
                    require_no_targets=True,
                )
        argv = pytest_case(invocation["argv"], case.get("nodeid", case.get("case_key", "")))
    else:
        raise CaptureUnsupported("this producer has no qualified exact-case selector")
    options = summary.get("options", {})
    original = {row["step"]: row for row in invocations if row["boundary"] == boundary.id}
    operations = []
    visited: set[str] = set()

    def require(step: Any) -> None:
        if step.name in visited:
            return
        visited.add(step.name)
        for requirement in step.consumes:
            name, _, conditional = requirement.partition("?")
            if conditional and not options.get(conditional):
                continue
            if name == "serving" and options.get("serving"):
                operations.append(
                    {
                        "operation": "admit-retained-serving",
                        "path": options["serving"],
                        "status": "requires current owner admission",
                    }
                )
                continue
            providers = [provider for provider in steps if name in provider.produces]
            if len(providers) != 1:
                raise CaptureUnsupported(f"unresolved prerequisite {name}")
            provider = providers[0]
            require(provider)
            if provider.name not in original:
                raise CaptureUnsupported(f"prerequisite {provider.name} lacks a recorded command")
            command = original[provider.name]["argv"]
            if not any(operation.get("step") == provider.name for operation in operations):
                operations.append(
                    {
                        "step": provider.name,
                        "argv": command,
                        "command": shlex.join(command),
                        "case_filter": False,
                    }
                )
                if original[provider.name].get("restart_after"):
                    operations.append({"operation": "fixture-restart", "after": provider.name})

    require(target)
    selected: dict[str, Any] = {
        "step": target.name,
        "argv": argv,
        "command": shlex.join(argv),
        "case_filter": True,
    }
    if artifact is not None:
        selected["cargo_target"] = {
            key: artifact.get(key)
            for key in ("package_id", "package_name", "target", "features", "profile")
        }
    operations.append(selected)
    by_name = {step.name: step for step in boundary.steps}
    for operation in operations:
        if operation.get("step") not in by_name:
            continue
        step = by_name[operation["step"]]
        bindings = dict(step.env)
        bindings.update(
            {key: value for option, key, value in step.option_env if options.get(option)}
        )
        operation["environment_templates"] = bindings
        operation["bindings"] = {
            "artifacts": "resolved Cargo profile artifact directory under current checkout",
            "serving": (
                "new fixture-owned serving destination or independently admitted retained content"
            ),
            "fixture_environment": "supplied by existing fixture owner"
            if boundary.fixture
            else "not required",
        }
    return {
        "schema": 1,
        "invocation": invocation["id"],
        "case": case.get("case_key"),
        "operations": operations,
        "requirements": sorted(boundary.requirements),
        "fixture": boundary.fixture,
        "environment": {
            **invocation.get("settings", {}),
            "LCTX_SOURCE_CONTEXT": invocation.get("source_context", "invocation"),
        },
        "readiness": "observe before execution; repair only by explicit just sync/service commands",
        "remaining_obligations": (
            "all original cases and boundaries outside this selection remain outstanding"
        ),
        "source_to_binary": (
            "unknown; a printed current-code experiment is not recorded-output replay"
        ),
        "original_context": invocation.get("context_before"),
        "executes": False,
    }
