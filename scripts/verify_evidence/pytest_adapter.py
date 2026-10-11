"""Qualified pytest/reportlog capture and read-only completed-prefix interpretation.

The caller owns directory admission and the child lifecycle. Native JSONL is the
execution authority; the companion hook stream only supplies observations.
"""

from __future__ import annotations

import configparser
import hashlib
import json
import shlex
import subprocess
import tomllib
from collections.abc import Iterator
from pathlib import Path

import workspace_env

from .contracts import CaptureUnsupported

REPORT = "pytest-reportlog.jsonl"
HOOKS = "pytest-hooks.jsonl"
HOOK_PLUGIN = "verify_evidence.pytest_hooks"
REPORT_PLUGINS = {"pytest_reportlog", "pytest_reportlog.plugin"}


def _configured_addopts(
    args: list[str], root: Path, *, configuration: dict | None = None
) -> list[str]:
    """Observe the selected project config without importing/initializing pytest.

    Managed verification runs from root. Explicit configs are supported as well;
    no project config is rewritten or executed.
    """
    explicit = None
    override = None
    for i, arg in enumerate(args):
        if arg in {"-c", "--config-file"} and i + 1 < len(args):
            explicit = args[i + 1]
        elif arg.startswith("--config-file="):
            explicit = arg.partition("=")[2]
        elif arg.startswith("-c") and len(arg) > 2:
            explicit = arg[2:]
        if arg in {"-o", "--override-ini"} and i + 1 < len(args):
            value = args[i + 1]
        elif arg.startswith("--override-ini="):
            value = arg.partition("=")[2]
        elif arg.startswith("-o") and len(arg) > 2:
            value = arg[2:]
        else:
            continue
        if value.startswith("addopts="):
            override = value.partition("=")[2]
    candidates = (
        [root / explicit]
        if explicit
        else [
            root / name
            for name in (
                "pytest.toml",
                ".pytest.toml",
                "pytest.ini",
                ".pytest.ini",
                "pyproject.toml",
                "tox.ini",
                "setup.cfg",
            )
        ]
    )
    for path in candidates:
        if not path.is_file():
            continue
        raw = path.read_bytes()
        if path.suffix == ".toml":
            data = tomllib.loads(raw.decode("utf-8"))
            section = (
                data.get("pytest")
                if path.name in {"pytest.toml", ".pytest.toml"}
                else data.get("tool", {}).get("pytest")
            )
            if (
                section is None
                and not explicit
                and path.name not in {"pytest.toml", ".pytest.toml"}
            ):
                continue
            section = section or {}
            section = section.get("ini_options", section)
            value = section.get("addopts", [])
        else:
            data = configparser.ConfigParser(interpolation=None)
            data.read_string(raw.decode("utf-8"))
            section_name = "tool:pytest" if path.name == "setup.cfg" else "pytest"
            if (
                not data.has_section(section_name)
                and not explicit
                and path.name not in {"pytest.ini", ".pytest.ini"}
            ):
                continue
            value = data[section_name].get("addopts", "") if data.has_section(section_name) else ""
        if configuration is not None:
            configuration.update(path=str(path.absolute()), sha256=hashlib.sha256(raw).hexdigest())
        if override is not None:
            return shlex.split(override)
        return shlex.split(value) if isinstance(value, str) else list(value)
    return shlex.split(override) if override is not None else []


def _tool_versions(root: Path, env: dict) -> dict:
    # The coordinator is intentionally stdlib-only and may run outside .venv.
    # Read exact producer metadata in the selected tools environment, retaining
    # shared ownership only for this bounded helper. Never synchronize here.
    try:
        with workspace_env.ownership("shared", "tools", root=root, env=env) as owner:
            child_env = owner.environment(env)
            readiness = workspace_env.observe("tools", root=root, env=child_env)
            if not readiness.ready:
                raise CaptureUnsupported(readiness.message())
            python = workspace_env.environment_path(root, child_env) / "bin" / "python"
            result = subprocess.run(
                [
                    str(python),
                    "-c",
                    "import importlib.metadata as m,json; "
                    "print(json.dumps({n:m.version(n) for n in ('pytest','pytest-reportlog')}))",
                ],
                cwd=root,
                env=child_env,
                capture_output=True,
                text=True,
                timeout=15,
                check=False,
            )
            if result.returncode:
                raise CaptureUnsupported(
                    "pytest evidence metadata unavailable; repair: just sync tools"
                )
            return json.loads(result.stdout)
    except (OSError, subprocess.TimeoutExpired, ValueError) as error:
        if isinstance(error, CaptureUnsupported):
            raise
        raise CaptureUnsupported(
            "pytest evidence tools probe failed; repair: just sync tools"
        ) from error


def _plugins(args: list[str]) -> list[str]:
    return [
        args[i + 1] if arg == "-p" and i + 1 < len(args) else arg[2:]
        for i, arg in enumerate(args)
        if arg == "-p" or (arg.startswith("-p") and len(arg) > 2)
    ]


def _canonical_plugins(args: list[str]) -> list[str]:
    result = list(args)
    for i, arg in enumerate(result):
        if arg == "-p" and i + 1 < len(result) and result[i + 1] in REPORT_PLUGINS:
            result[i + 1] = "pytest_reportlog"
        elif arg.startswith("-p") and arg[2:] in REPORT_PLUGINS:
            result[i] = "-ppytest_reportlog"
    return result


def prepare(argv: list[str], env: dict, directory: Path, root: Path) -> dict:
    """Return a capture command without modifying inputs or creating artifacts.

    Reportlog is requested by its *entry point name*: requesting its module name
    while ordinary entry-point discovery is enabled registers the same plugin
    twice. Normalize explicit requests for this one producer, leaving discovery
    and every unrelated plugin intact.
    """
    args = list(argv)
    environment = dict(env)
    if any(
        (directory / name).exists() or (directory / name).is_symlink() for name in (REPORT, HOOKS)
    ):
        raise CaptureUnsupported("pytest capture directory has already been used")
    addopts = shlex.split(str(environment.get("PYTEST_ADDOPTS", "")))
    configuration = {}
    try:
        configured = _configured_addopts(addopts + args, root, configuration=configuration)
    except (OSError, ValueError, configparser.Error) as error:
        raise CaptureUnsupported(
            "pytest evidence could not safely inspect selected config"
        ) from error
    options = configured + addopts + args
    plugins = _plugins(options)
    if any(
        p.removeprefix("no:") in REPORT_PLUGINS | {HOOK_PLUGIN} and p.startswith("no:")
        for p in plugins
    ):
        raise CaptureUnsupported("pytest evidence plugin is explicitly disabled")
    if any(a == "--report-log" or a.startswith("--report-log=") for a in options):
        raise CaptureUnsupported(
            "explicit --report-log destination conflicts with invocation-owned capture"
        )
    if any(
        a in {"-n", "--numprocesses", "--dist", "--reruns", "--force-reruns"}
        or a.startswith(("--numprocesses=", "--dist=", "--reruns=", "--force-reruns="))
        or (a.startswith("-n") and len(a) > 2)
        for a in options
    ):
        raise CaptureUnsupported(
            "pytest evidence currently qualifies serial pytest without retry plugins"
        )
    versions = _tool_versions(root, environment)
    pytest_version = versions["pytest"]
    reportlog_version = versions["pytest-reportlog"]
    if (pytest_version, reportlog_version) != ("9.1.1", "1.0.0"):
        raise CaptureUnsupported(
            f"unqualified pytest/reportlog versions: {pytest_version}/{reportlog_version}"
        )
    args = _canonical_plugins(args)
    normalized_addopts = _canonical_plugins(addopts)
    if normalized_addopts != addopts:
        environment["PYTEST_ADDOPTS"] = shlex.join(normalized_addopts)
    requested = [
        p.strip() for p in str(environment.get("PYTEST_PLUGINS", "")).split(",") if p.strip()
    ]
    if any(p in REPORT_PLUGINS for p in requested):
        remaining = [p for p in requested if p not in REPORT_PLUGINS]
        environment["PYTEST_PLUGINS"] = ",".join(remaining)
    # Insert before the pytest positional separator, so flags cannot become paths.
    flags = [
        "-p",
        "pytest_reportlog",
        "-p",
        HOOK_PLUGIN,
        "--report-log",
        str(directory.absolute() / REPORT),
    ]
    normalized_configured = _canonical_plugins(configured)
    if normalized_configured != configured:
        flags.extend(["-o", "addopts=" + shlex.join(normalized_configured)])
    separator = args.index("--") if "--" in args else len(args)
    args[separator:separator] = flags
    path = str(root.absolute() / "scripts")
    environment["PYTHONPATH"] = path + (
        ":" + str(environment["PYTHONPATH"]) if environment.get("PYTHONPATH") else ""
    )
    environment["LCTX_PYTEST_EVIDENCE_HOOKS"] = str(directory.absolute() / HOOKS)
    return {
        "argv": args,
        "env": environment,
        "channels": {"pytest-reportlog": "requested", "pytest-hooks": "requested"},
        "artifacts": [REPORT, HOOKS],
        "tool_version": {"pytest": pytest_version, "pytest-reportlog": reportlog_version},
        "selection_options": {"configured_addopts": configured, "environment_addopts": addopts},
        "configuration": configuration,
    }


def _lines(path: Path, start: int = 0, end: int | None = None) -> Iterator[tuple[int, int, dict]]:
    if not path.exists():
        return
    with path.open("rb") as stream:
        stream.seek(start)
        while True:
            offset = stream.tell()
            if end is not None and offset >= end:
                return
            line = stream.readline(-1 if end is None else end - offset)
            if not line or not line.endswith(b"\n"):
                return
            try:
                event = json.loads(line)
            except ValueError as error:
                raise ValueError(f"malformed pytest evidence {path.name}:{offset}") from error
            if not isinstance(event, dict):
                raise ValueError(f"pytest evidence is not an object {path.name}:{offset}")
            yield offset, stream.tell(), event


def _payload(event: dict) -> str:
    parts = []
    if event.get("longrepr"):
        value = event["longrepr"]
        parts.append(
            value if isinstance(value, str) else json.dumps(value, ensure_ascii=False, indent=2)
        )
    for section in event.get("sections", []):
        if isinstance(section, (tuple, list)) and len(section) == 2:
            parts.append(f"{section[0]}\n{section[1]}")
    if event.get("$report_type") == "WarningMessage":
        parts.append(f"{event.get('category')}: {event.get('message')}")
    if event.get("$report_type") in {"CaptureConfiguration", "CaptureUnsupported"}:
        parts.append(json.dumps(event, ensure_ascii=False, indent=2))
    return "\n\n".join(parts)


def records(
    directory: Path, *, watermarks: dict | None = None, ends: dict | None = None
) -> Iterator[dict]:
    """Yield complete native lines; no partial trailing record is interpreted.

    Watermarks map relative source names to previously consumed byte ends. Native
    attempt identity is retained when present; ordinary pinned pytest has one
    attempt. No start or collected-member observation establishes execution.
    """
    for source in (REPORT, HOOKS):
        start = (watermarks or {}).get(source, 0)
        for offset, end, event in _lines(directory / source, start, (ends or {}).get(source)):
            kind = event.get("$report_type", "unknown")
            native = source == REPORT
            phase = (
                event.get("when")
                if kind == "TestReport"
                else {
                    "CollectReport": "collection",
                    "CaseStart": "start",
                    "CollectionMember": "collection",
                    "Deselected": "collection",
                    "CollectionFinished": "collection",
                    "WarningMessage": event.get("when", "warning"),
                }.get(kind, "session")
            )
            nodeid = event.get("nodeid", "")
            outcome = event.get("outcome", "observed")
            if kind == "TestReport" and "wasxfail" in event:
                outcome = (
                    "xfail" if outcome == "skipped" else "xpass" if outcome == "passed" else outcome
                )
            yield {
                "case_key": nodeid,
                "nodeid": nodeid,
                "phase": phase,
                "rawphase": event.get("when"),
                "event_kind": kind,
                "attempt": int(event.get("rerun", 0)) + 1,
                "outcome": outcome,
                "native_outcome": event.get("outcome"),
                "wasxfail": event.get("wasxfail"),
                "duration": event.get("duration"),
                "start": event.get("start"),
                "stop": event.get("stop"),
                "payload": _payload(event),
                "source": source,
                "source_start": offset,
                "source_end": end,
                "source_offset": offset,
                "source_length": end - offset,
                "location": event.get("location"),
                "coverage": {
                    "authority": "native-report" if native else "supplemental-observation",
                    "completed_phase": native and kind == "TestReport",
                    "unfinished_output": "unavailable",
                    "attempt_identity": "native" if "rerun" in event else "not-reported",
                    "warning_case_attribution": "unavailable"
                    if kind == "WarningMessage" and not nodeid
                    else "native",
                },
            }


def finalize(directory: Path, env: dict | None = None, root: Path | None = None) -> dict:
    """Describe availability, independently of pytest/process success."""
    channels: dict[str, dict] = {}
    effective = None
    for source in (REPORT, HOOKS):
        path = directory / source
        count = 0
        committed = 0
        finished = False
        unsupported = []
        try:
            for _, line_end, event in _lines(path):
                committed = line_end
                count += 1
                finished |= event.get("$report_type") == "SessionFinish"
                if event.get("$report_type") == "CaptureUnsupported":
                    unsupported.append(event.get("reason", "unknown"))
                if event.get("$report_type") == "CaptureConfiguration":
                    effective = {
                        key: value for key, value in event.items() if key != "$report_type"
                    }
        except ValueError as error:
            channels[source] = {"status": "failed", "error": str(error), "committed_end": committed}
            continue
        channels[source] = {
            "status": (
                "unsupported"
                if unsupported
                else "available"
                if finished and path.stat().st_size == committed
                else "partial"
            )
            if path.exists()
            else "unavailable",
            "available_prefix": path.exists() and count > 0,
            "unsupported": unsupported,
            "records": count,
            "committed_end": committed,
            "partial_tail": path.exists() and path.stat().st_size > committed,
            "session_finished": finished,
        }
    return {
        "channels": channels,
        "effective": effective,
        "unfinished_output": "unavailable",
        "xdist": "unsupported",
    }
