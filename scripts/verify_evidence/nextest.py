"""Qualified Nextest 0.9.146 capture; stdlib outside bounded config composition.

Native recordings and JUnit remain authorities. This adapter never discovers/builds tests,
uses `latest`, or reads private recording ZIP members. The enclosing run owns storage.
"""

from __future__ import annotations

import json
import math
import re
import subprocess
import tomllib
import uuid
import xml.etree.ElementTree as ET
from collections.abc import Iterator
from pathlib import Path

import workspace_env

from . import cargo_adapter
from .contracts import CaptureUnsupported

VERSION = "0.9.146"
UUID = r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
MANAGED = (
    "--config-file",
    "--user-config-file",
    "--message-format",
    "--message-format-version",
    "--cargo-message-format",
)


def _options(argv: list[str], names: tuple[str, ...]) -> list[str]:
    result = []
    for index, token in enumerate(argv):
        if token == "--":
            break
        for name in names:
            if token == name and index + 1 < len(argv):
                result.append(argv[index + 1])
            elif token.startswith(name + "="):
                result.append(token[len(name) + 1 :])
            elif name == "-P" and token.startswith("-P") and token != "-P":
                result.append(token[2:])
    return result


def _without_managed(argv: list[str]) -> list[str]:
    result = []
    index = 0
    while index < len(argv):
        token = argv[index]
        if token == "--":
            result.extend(argv[index:])
            break
        if token in MANAGED:
            if index + 1 >= len(argv):
                raise CaptureUnsupported(f"missing value for {token}")
            index += 2
            continue
        if not any(token.startswith(name + "=") for name in MANAGED):
            result.append(token)
        index += 1
    return result


def _read_config(path: Path, explicit: bool) -> str:
    try:
        return path.read_text()
    except FileNotFoundError:
        if not explicit:
            return ""
        raise CaptureUnsupported(f"explicit Nextest configuration missing: {path}") from None
    except OSError as error:
        raise CaptureUnsupported(str(error)) from error


def _native(argv: list[str], env: dict[str, str], root: Path) -> str:
    completed = subprocess.run(argv, cwd=root, env=env, capture_output=True, text=True)
    output = completed.stdout + completed.stderr
    if completed.returncode:
        raise CaptureUnsupported(f"native inspection exit {completed.returncode}: {output.strip()}")
    return output


def prepare(argv: list[str], env: dict[str, str], directory: Path, root: Path) -> dict:
    """Return new argv/environment. Caller holds invocation storage before calling."""
    if len(argv) < 3 or Path(argv[0]).name != "cargo" or argv[1:3] != ["nextest", "run"]:
        raise CaptureUnsupported("capture supports cargo nextest run only")
    command = argv[: argv.index("--")] if "--" in argv else argv
    if any(
        token.split("=", 1)[0]
        in {
            "--no-run",
            "--debugger",
            "--tracer",
            "--no-capture",
            "--stress-count",
            "--stress-duration",
        }
        for token in command
    ):
        raise CaptureUnsupported("Nextest noncapturing/stress execution is not qualified")
    if any(token in {"--nocapture", "--no-capture"} for token in argv):
        raise CaptureUnsupported("Nextest no-capture execution is not qualified")
    resolved_env = dict(env)
    try:
        version = _native([argv[0], "nextest", "--version"], resolved_env, root)
        if not version.startswith(f"cargo-nextest {VERSION} "):
            raise CaptureUnsupported(f"unsupported Nextest version: {version.splitlines()[0]}")
        directory = directory.resolve()
        root = root.resolve()
        repos = _options(argv, ("--config-file",))
        users = _options(argv, ("--user-config-file",))
        profiles = _options(argv, ("--profile", "-P"))
        profile = profiles[-1] if profiles else env.get("NEXTEST_PROFILE", "default")
        repo_path = (root / repos[-1]).resolve() if repos else root / ".config/nextest.toml"
        user_name = users[-1] if users else env.get("NEXTEST_USER_CONFIG_FILE")
        home = Path(env.get("HOME", str(Path.home())))
        config_home = Path(env.get("XDG_CONFIG_HOME", str(home / ".config")))
        user_path = (
            (root / user_name).resolve()
            if user_name and user_name != "none"
            else config_home / "nextest/config.toml"
        )
        repo_text = _read_config(repo_path, bool(repos))
        user_text = "" if user_name == "none" else _read_config(user_path, bool(user_name))
        known_profiles = {"default", "ci", "miri"}
        known_profiles.update(tomllib.loads(repo_text).get("profile", {}))
        for tool in _options(argv, ("--tool-config-file",)):
            _, separator, path = tool.partition(":")
            if not separator or not Path(path).is_absolute():
                raise CaptureUnsupported("invalid explicit Nextest tool config")
            known_profiles.update(tomllib.loads(_read_config(Path(path), True)).get("profile", {}))
        if profile not in known_profiles:
            raise CaptureUnsupported(f"unknown selected Nextest profile: {profile}")
        state = directory / "nextest-state"
        if state.exists() or (directory / "nextest-adapter.json").exists():
            raise CaptureUnsupported("Nextest capture directory has already been used")
        directory.mkdir(parents=True, exist_ok=True)
        request = {
            "repo_text": repo_text,
            "user_text": user_text,
            "profile": profile,
            "junit": str(directory / "junit.xml"),
            "directory": str(directory),
        }
        # Rust-only execution releases tools ownership immediately after composition.
        with workspace_env.ownership("shared", "tools", root=root, env=resolved_env) as owned:
            helper_env = owned.environment(resolved_env)
            interpreter = workspace_env.environment_path(root, helper_env) / "bin/python"
            helper = Path(__file__).with_name("config_helper.py")
            result = subprocess.run(
                [str(interpreter), str(helper)],
                input=json.dumps(request),
                cwd=root,
                env=helper_env,
                capture_output=True,
                text=True,
            )
            if result.returncode:
                raise CaptureUnsupported(
                    f"Nextest config composition unavailable: {result.stderr.strip()}; "
                    "repair: just sync tools"
                )
            composed = json.loads(result.stdout)
        rewritten = _without_managed(argv)
        insertion = rewritten.index("--") if "--" in rewritten else len(rewritten)
        rewritten[insertion:insertion] = [
            "--config-file",
            composed["repo"],
            "--user-config-file",
            composed["user"],
            "--message-format",
            "libtest-json-plus",
            "--message-format-version",
            "0.1",
            "--cargo-message-format",
            "json",
        ]
        resolved_env.update(
            NEXTEST_STATE_DIR=str(state),
            NEXTEST_EXPERIMENTAL_RECORD="1",
            NEXTEST_EXPERIMENTAL_LIBTEST_JSON="1",
        )
        metadata = {
            "tool_version": VERSION,
            "profile": profile,
            "cargo": argv[0],
            "config_sources": {"repo": str(repo_path), "user": user_name or str(user_path)},
            "manifest": _options(argv, ("--manifest-path",)),
            "configs": composed,
        }
        (directory / "nextest-adapter.json").write_text(json.dumps(metadata, indent=2) + "\n")
        return {
            "argv": rewritten,
            "env": resolved_env,
            "tool_version": VERSION,
            "stdout": str(directory / "nextest-events.jsonl"),
            "channels": {
                "junit": "requested",
                "native-recording": "requested",
                "libtest-json-plus": "requested",
                "cargo-json": "requested",
            },
            "artifacts": {
                "junit": str(directory / "junit.xml"),
                "state": str(state),
                "events": str(directory / "nextest-events.jsonl"),
                "recording": str(directory / "nextest-recording.zip"),
                "config": str(directory / "nextest-adapter.json"),
            },
        }
    except CaptureUnsupported:
        raise
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise CaptureUnsupported(f"Nextest capture preparation failed: {error}") from error


def _junit_uuid(path: Path) -> str:
    with path.open("rb") as stream:
        for _, element in ET.iterparse(stream, events=("start",)):
            if element.tag != "testsuites" or "uuid" not in element.attrib:
                raise ValueError("Nextest JUnit has no native UUID")
            return str(uuid.UUID(element.attrib["uuid"]))
    raise ValueError("empty JUnit artifact")


def _info_uuid(output: str) -> str:
    identities = re.findall(r"(?im)^Run (" + UUID + r")\s*$", output)
    if len(identities) != 1:
        raise CaptureUnsupported("native store info did not expose one full Run ID")
    return str(uuid.UUID(identities[0]))


def finalize(directory: Path, env: dict[str, str], root: Path) -> dict:
    """Inspect the sole native candidate and export it, independent of test exit."""
    directory = directory.resolve()
    result: dict = {
        "status": "partial",
        "channels": {},
        "native_uuid": None,
        "tool_version": VERSION,
        "losses": [],
    }
    channels = result["channels"]
    junit = directory / "junit.xml"
    junit_id = None
    if junit.is_file():
        try:
            junit_id = _junit_uuid(junit)
            # Parse through EOF without materializing the whole report.
            for _, element in ET.iterparse(junit, events=("end",)):
                element.clear()
            channels["junit"] = {"status": "available", "path": str(junit), "uuid": junit_id}
        except (ET.ParseError, ValueError, OSError) as error:
            channels["junit"] = {"status": "failed", "reason": str(error)}
    else:
        channels["junit"] = {"status": "missing", "reason": "no completed JUnit artifact"}
    events = directory / "nextest-events.jsonl"
    channels["libtest-json-plus"] = {
        "status": "available" if events.is_file() else "missing",
        "path": str(events),
        "coverage": "readable complete-line prefix",
    }
    try:
        metadata = json.loads((directory / "nextest-adapter.json").read_text())
        native_env = dict(env) | {
            "NEXTEST_STATE_DIR": str(directory / "nextest-state"),
            "NEXTEST_EXPERIMENTAL_RECORD": "1",
            "CARGO_TERM_COLOR": "never",
        }
        command = [metadata["cargo"], "nextest", "--no-pager", "store"]
        options = [
            "--config-file",
            metadata["configs"]["repo"],
            "--user-config-file",
            metadata["configs"]["user"],
        ]
        if metadata["manifest"]:
            options += ["--manifest-path", metadata["manifest"][-1]]

        def inspect(operation: str, *args: str) -> str:
            return _native([*command, operation, *args, *options], native_env, root)

        listing = inspect("list")
        (directory / "nextest-store-list.txt").write_text(listing)
        candidates = re.findall(r"(?m)^\s*([0-9a-f]{8,32}(?:-[0-9a-f-]+)?)\s+", listing)
        if len(candidates) != 1:
            raise CaptureUnsupported(
                f"private native store has {len(candidates)} candidates; expected one"
            )
        info = inspect("info", candidates[0])
        (directory / "nextest-store-info.txt").write_text(info)
        native_id = _info_uuid(info)
        result["native_uuid"] = native_id
        if junit_id and junit_id != native_id:
            raise CaptureUnsupported("native UUID disagrees with completed JUnit UUID")
        recording = directory / "nextest-recording.zip"
        inspect("export", native_id, "--archive-file", str(recording))
        archive_info = inspect("info", str(recording))
        (directory / "nextest-recording-info.txt").write_text(archive_info)
        if _info_uuid(archive_info) != native_id:
            raise CaptureUnsupported("exported native recording UUID disagrees with store UUID")
        channels["native-recording"] = {
            "status": "available",
            "path": str(recording),
            "uuid": native_id,
            "truncation": "unknown per output; producer maximum 256MiB",
        }
        trace = directory / "nextest-trace.json"
        try:
            inspect("export-chrome-trace", native_id, "--output", str(trace))
            channels["chrome-trace"] = {"status": "available", "path": str(trace)}
        except (CaptureUnsupported, OSError) as error:
            channels["chrome-trace"] = {"status": "failed", "reason": str(error)}
        result["status"] = "available" if channels["junit"]["status"] == "available" else "partial"
    except (CaptureUnsupported, OSError, ValueError, KeyError) as error:
        channels["native-recording"] = {"status": "failed", "reason": str(error)}
        result["status"] = "failed"
    result["losses"] = [
        "JUnit UTF-8/XML normalization; original-byte fidelity unavailable",
        "libtest JSON omits intermediate retries, setup and passing output",
        "JUnit/libtest channels overlap; observations are not unique executions",
        "unfinished output and native per-output truncation are not established",
    ]
    (directory / "nextest-capture.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def _payload(element: ET.Element) -> str:
    parts = []
    if element.text and element.text.strip():
        parts.append(element.text)
    for child in element:
        if child.tag in {"system-out", "system-err", "failure", "error", "skipped"}:
            text = "".join(child.itertext())
            attrs = " ".join(f"{key}={value}" for key, value in child.attrib.items())
            parts.append(f"[{child.tag}{' ' + attrs if attrs else ''}]\n{text}")
    return "\n".join(parts)


def _duration(value: object) -> float | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, (float, int, str)):
        raise ValueError("invalid native Nextest duration")
    duration = float(value)
    if not math.isfinite(duration) or duration < 0:
        raise ValueError("invalid native Nextest duration")
    return duration


def _junit_records(path: Path) -> Iterator[dict]:
    native_id = _junit_uuid(path)
    suite = ""
    for event, element in ET.iterparse(path, events=("start", "end")):
        if event == "start" and element.tag == "testsuite":
            suite = element.get("name", "")
        if event != "end":
            continue
        if element.tag == "testcase":
            binary = element.get("classname", suite)
            name = element.get("name", "")
            phase = "setup" if suite.startswith("@setup-script:") else "call"
            retries = [
                child
                for child in element
                if child.tag in {"flakyFailure", "flakyError", "rerunFailure", "rerunError"}
            ]
            failure = element.find("failure")
            error = element.find("error")
            skipped = element.find("skipped")
            outcome = (
                "skipped"
                if skipped is not None
                else "error"
                if error is not None
                else "failed"
                if failure is not None
                else "passed"
            )
            flaky = any(child.tag.startswith("flaky") for child in retries)
            attempts = [*retries, element] if flaky else [element, *retries]
            key = json.dumps([binary, name], separators=(",", ":"), ensure_ascii=False)
            for attempt, item in enumerate(attempts, 1):
                attempt_outcome = (
                    outcome
                    if item is element
                    else ("error" if item.tag.endswith("Error") else "failed")
                )
                # A flaky-fail envelope is policy failure over a successful last execution.
                if (
                    item is element
                    and failure is not None
                    and failure.get("type") == "flaky failure"
                ):
                    attempt_outcome = "passed"
                locator = {"suite": suite, "test": name, "phase": phase, "attempt": attempt}
                yield {
                    "case_key": key,
                    "binary_id": binary if phase == "call" else None,
                    "test": name,
                    "nodeid": f"{binary}::{name}",
                    "phase": phase,
                    "event_kind": "nextest-junit-attempt",
                    "duration": _duration(item.get("time")),
                    "attempt": attempt,
                    "outcome": attempt_outcome,
                    "case_outcome": outcome,
                    "payload": _payload(item),
                    "source": str(path),
                    "native": {
                        "uuid": native_id,
                        "locator": locator,
                        "format": "nextest-junit",
                        "normalization": "UTF-8/XML normalized text",
                        "truncation": "unknown",
                        "coverage": "completed attempt; unfinished cases unavailable",
                    },
                }
            element.clear()
        elif element.tag == "testsuite":
            element.clear()


def _json_records(path: Path, watermark: int = 0, end_bound: int | None = None) -> Iterator[dict]:
    suites: dict[str, dict] = {}
    with path.open("rb") as stream:
        stream.seek(watermark)
        while stream.tell() < (end_bound if end_bound is not None else float("inf")):
            line = stream.readline()
            if not line:
                break
            end = stream.tell()
            start = end - len(line)
            if end_bound is not None and end > end_bound:
                break
            if not line.endswith(b"\n"):
                break
            record = json.loads(line)
            if not isinstance(record, dict):
                raise ValueError("Nextest JSON event must be an object")
            if record.get("type") == "suite" and record.get("nextest"):
                meta = record["nextest"]
                suites[f"{meta['crate']}::{meta['test_binary']}"] = meta
            if record.get("type") != "test":
                continue
            prefix, separator, name = record.get("name", "").partition("$")
            if not separator:
                raise ValueError("Nextest libtest test has no qualified binary prefix")
            attempt = 1
            suffix = re.search(r"#([0-9]+)$", name)
            if suffix:
                attempt = int(suffix[1])
                name = name[: suffix.start()]
            meta = suites.get(prefix, {})
            # The native format omits exact binary identity on test events. Keep its
            # synthetic identity stable across append preparations, without claiming
            # that it disambiguates targets sharing the same package/binary name.
            key = json.dumps(["libtest-suite", prefix, name], separators=(",", ":"))
            event = record.get("event", "unknown")
            outcome = {
                "ok": "passed",
                "failed": "failed",
                "ignored": "skipped",
                "started": "started",
            }.get(event, "unknown")
            yield {
                "case_key": key,
                "binary_id": None,
                "test": name,
                "nodeid": f"{prefix}${name}",
                "phase": "call",
                "attempt": attempt,
                "event_kind": "nextest-test-" + event,
                "duration": _duration(record.get("exec_time"))
                if event in {"ok", "failed"}
                else None,
                "outcome": outcome,
                "case_outcome": outcome,
                "payload": record.get("stdout", "") or record.get("reason", ""),
                "source": str(path),
                "source_start": start,
                "source_end": end,
                "native": {
                    "uuid": None,
                    "locator": {"name": record["name"], "event": event},
                    "suite": meta,
                    "format": "nextest-libtest-json-plus-0.1",
                    "binary_identity": "unavailable",
                    "truncation": "unknown",
                    "normalization": "lossy UTF-8; human harness text stripped",
                    "coverage": (
                        "complete-line prefix; synthetic identity may alias targets; "
                        "no intermediate retry, setup or passing output"
                    ),
                    "outcome_scope": "native case envelope; final attempt status may differ",
                },
            }


def records(
    directory: Path, *, watermarks: dict[str, int] | None = None, ends: dict[str, int] | None = None
) -> Iterator[dict]:
    """Retain independent completed JUnit and JSON prefixes, with explicit overlap."""
    directory = directory.resolve()
    junit = directory / "junit.xml"
    watermarks = watermarks or {}
    if junit.is_file() and "junit.xml" not in watermarks:
        yield from _junit_records(junit)
    events = directory / "nextest-events.jsonl"
    if events.is_file():
        yield from _json_records(
            events,
            watermarks.get("nextest-events.jsonl", 0),
            (ends or {}).get("nextest-events.jsonl"),
        )
    yield from cargo_adapter.records(directory, watermarks=watermarks, ends=ends)
