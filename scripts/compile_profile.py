#!/usr/bin/env python3
"""Run-owned compilation diagnostics, live inspection and local reports.

Uses runs.py for launch/cancellation and compile_profile_capture for selected rustc units.
Telemetry is independent of the product command's exit status. Reports never modify raw captures.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import re
import shutil
import signal
import struct
import subprocess
import sys
import threading
import time
import tomllib
from contextlib import suppress
from pathlib import Path
from typing import Any

import runs
from build_environment import explain, normalized_env
from harness import ProcessIdentity, read_json, write_json_atomic

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = Path(__file__).resolve()
MIN_FREE = 8 * 1024**3
FORMAT = 9


def capture():
    import compile_profile_capture

    return compile_profile_capture


def tools_root() -> Path:
    import compile_profile_tools

    return compile_profile_tools.tools_root()


def execute(
    argv: list[str],
    *,
    cwd: Path = ROOT,
    timeout: float | None = 30,
    env: dict[str, str] | None = None,
) -> dict[str, Any]:
    try:
        proc = subprocess.run(
            argv, cwd=cwd, capture_output=True, text=True, timeout=timeout, env=env
        )
        return {
            "argv": argv,
            "exit_code": proc.returncode,
            "stdout": proc.stdout,
            "stderr": proc.stderr,
            "outcome": "passed" if proc.returncode == 0 else "failed",
        }
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"argv": argv, "outcome": "blocked", "reason": str(error)}


def cargo_command(command: list[str]) -> tuple[list[str], bool]:
    """Only direct known Cargo commands: never move filters, rustc args or global settings."""
    if not command or Path(command[0]).name != "cargo":
        return command.copy(), False
    index = 1
    if index < len(command) and command[index].startswith("+"):
        index += 1
    # Cargo global arguments may precede its command; values are not subcommands.
    while index < len(command):
        token = command[index]
        if token in ("--config", "--color", "-Z", "-C"):
            index += 2
        elif token.startswith("-"):
            index += 1
        else:
            break
    if index >= len(command):
        return command.copy(), False
    subcommand = command[index]
    nextest = subcommand == "nextest" and command[index + 1 : index + 2] in (["run"], ["list"])
    if subcommand not in ("build", "test", "rustc") and not nextest:
        return command.copy(), False
    flags = ["-Zbuild-analysis", "-Zsection-timings", "--config", "build.analysis.enabled=true"]
    insertion = index + 2 if nextest else index
    updated = command[:insertion] + flags + command[insertion:]
    boundary = updated.index("--") if "--" in updated else len(updated)
    if not any(arg == "--timings" or arg.startswith("--timings=") for arg in updated[:boundary]):
        updated.insert(boundary, "--timings")
    message_option = "--cargo-message-format" if nextest else "--message-format"
    if not any(
        arg == message_option or arg.startswith(message_option + "=") for arg in updated[:boundary]
    ):
        updated.insert(
            updated.index("--") if "--" in updated else len(updated),
            message_option
            + ("=json,json-render-diagnostics" if nextest else "=json-render-diagnostics"),
        )
    return updated, True


def focus_manifests(focus: str, command: list[str] | None = None) -> list[str]:
    metadata_args = ["cargo", "metadata", "--no-deps", "--offline", "--format-version", "1"]
    manifest = option_value(command, "--manifest-path") if command else None
    if manifest:
        metadata_args += ["--manifest-path", manifest]
    result = execute(metadata_args)
    if result["outcome"] != "passed":
        raise ValueError(
            f"Cargo metadata unavailable: {result.get('stderr') or result.get('reason')}"
        )
    metadata = json.loads(result["stdout"])
    members = set(metadata["workspace_members"])
    packages = [
        p
        for p in metadata["packages"]
        if p["id"] in members and (focus == "workspace" or p["name"] == focus)
    ]
    if not packages:
        raise ValueError(f"focus {focus!r} is not a workspace package")
    return sorted(str(Path(p["manifest_path"]).resolve().parent) for p in packages)


def doctor(*, check_analysis: bool = True, diagnostic_root: Path | None = None) -> dict[str, Any]:
    checks: dict[str, Any] = {}
    observed = capture().doctor()
    checks["capture"] = {
        **observed,
        "outcome": "passed" if observed.get("status") in ("ready", "passed") else "blocked",
    }
    rustc = execute(["rustc", "-vV"])
    checks["rustc"] = rustc
    cargo = execute(["cargo", "-V"])
    checks["cargo"] = cargo
    if check_analysis:
        unstable = execute(["cargo", "-Z", "help"])
        content = unstable.get("stdout", "") + unstable.get("stderr", "")
        checks["cargo_analysis"] = {
            **unstable,
            "outcome": "passed"
            if unstable["outcome"] == "passed"
            and all(flag in content for flag in ("build-analysis", "section-timings"))
            else "blocked",
            "repair": (
                "use the pinned Cargo toolchain with build-analysis and section-timings support"
            ),
        }
    try:
        import compile_profile_tools

        checked = compile_profile_tools.check()
        checks["measureme"] = {**checked, "outcome": checked["status"]}
    except ImportError, AttributeError:
        checks["measureme"] = {
            "outcome": "blocked",
            "reason": "exact measureme tools receipt unavailable",
            "repair": "just compile-profile-tools",
        }
    destination = diagnostic_root or runs.runs_root()
    while not destination.exists():
        destination = destination.parent
    free = shutil.disk_usage(destination).free
    checks["disk"] = {
        "outcome": "passed",
        "free_bytes": free,
        "sampler_floor_bytes": MIN_FREE,
        "destinations": diagnostic_headroom(diagnostic_root),
        "advisory": free < advisory_threshold("recording"),
        "advisory_bytes": advisory_threshold("recording"),
        "support_limit": (
            "headroom is advisory; sampler floor cannot bound self-profile or Cargo writers"
        ),
    }
    return {
        "outcome": "passed"
        if all(c.get("outcome") == "passed" for c in checks.values())
        else "blocked",
        "checks": checks,
    }


EXCLUDED = {"fixtures", "gold", "heldout", ".venv", "target", "build", ".git"}


def source_allowed(name: str) -> bool:
    return not EXCLUDED.intersection(Path(name).parts) and not any(
        part.startswith(".venv") for part in Path(name).parts
    )


def provenance(
    directory: Path, command: list[str], effective: list[str], focus: str, manifests: list[str]
) -> dict[str, Any]:
    head = execute(["git", "rev-parse", "HEAD"])
    names = (
        subprocess.run(
            ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
            cwd=ROOT,
            capture_output=True,
            check=True,
        )
        .stdout.decode()
        .split("\0")
    )
    hashes = {}
    for name in sorted(set(names)):
        if not name or not source_allowed(name):
            continue
        path = ROOT / name
        if path.is_file():
            hashes[name] = hashlib.sha256(path.read_bytes()).hexdigest()
    status = execute(["git", "status", "--porcelain=v1", "--untracked-files=all"])
    # Explicit allowlist avoids including heldout/gold fixture bytes in the retained patch.
    allowed = [name for name in sorted(set(names)) if name and source_allowed(name)]
    patch = subprocess.run(
        ["git", "diff", "HEAD", "--binary", "--", *allowed],
        cwd=ROOT,
        capture_output=True,
        check=True,
    ).stdout
    (directory / "source.patch").write_bytes(patch)
    untracked = (
        subprocess.run(
            ["git", "ls-files", "--others", "--exclude-standard", "-z"],
            cwd=ROOT,
            capture_output=True,
            check=True,
        )
        .stdout.decode()
        .split("\0")
    )
    for name in untracked:
        if name and source_allowed(name) and (ROOT / name).is_file():
            target = directory / "untracked-source" / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, target)
    for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo/config.toml"):
        target = directory / "configuration" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / name, target)
    import compile_profile_tools

    observed_tools = {
        "measureme": compile_profile_tools.check(),
        "perf": execute([capture().resolve_perf(), "--version"]),
        "sccache": execute([shutil.which("sccache") or "sccache", "--version"]),
        "samply": execute([shutil.which("samply") or "samply", "--version"]),
        "nextest": execute(["cargo", "nextest", "--version"]),
    }
    return {
        "version": 1,
        "tools": observed_tools,
        "started": runs.now(),
        "head": head.get("stdout", "").strip(),
        "dirty": status.get("stdout", ""),
        "source_sha256": hashes,
        "source_patch": str(directory / "source.patch"),
        "input_argv": command,
        "effective_argv": effective,
        "focus": focus,
        "manifest_dirs": manifests,
        "rustc": execute(["rustc", "-vV"]),
        "cargo": execute(["cargo", "-V"]),
        "build_environment": explain(dict(os.environ)),
        "tool_root": str(tools_root()),
    }


def option_value(command: list[str], option: str) -> str | None:
    values = [arg.split("=", 1)[1] for arg in command if arg.startswith(option + "=")]
    for index, arg in enumerate(command[:-1]):
        if arg == option:
            values.append(command[index + 1])
    return values[-1] if values else None


def retain_cargo_sessions(
    directory: Path, sessions: list[str], env: dict[str, str], command: list[str]
) -> dict[str, Any]:
    """Copy only IDs emitted by our command, validating their workspace and cwd."""
    if not sessions:
        return {
            "outcome": "blocked",
            "reason": "Cargo emitted no build-started session ID; no shared log guessed",
        }
    manifest = next(
        (arg.split("=", 1)[1] for arg in command if arg.startswith("--manifest-path=")), None
    )
    if "--manifest-path" in command:
        manifest = command[command.index("--manifest-path") + 1]
    metadata_args = ["cargo", "metadata", "--offline", "--no-deps", "--format-version=1"]
    if manifest:
        metadata_args += ["--manifest-path", manifest]
    metadata = execute(metadata_args, cwd=Path.cwd())
    if metadata["outcome"] != "passed":
        return {
            "outcome": "blocked",
            "reason": "cannot validate Cargo workspace",
            "metadata": metadata,
        }
    workspace = Path(json.loads(metadata["stdout"])["workspace_root"]).resolve()
    cargo_home = Path(env.get("CARGO_HOME", str(Path.home() / ".cargo"))).expanduser().resolve()
    collected = []
    for session in sorted(set(sessions)):
        result: dict[str, Any] = {"id": session, "outcome": "blocked"}
        try:
            if not re.fullmatch(r"[A-Za-z0-9_.-]+", session):
                raise ValueError("invalid Cargo session ID")
            source = cargo_home / "log" / f"{session}.jsonl"
            raw = source.read_bytes()
            destination = directory / "cargo" / f"{session}.jsonl"
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(raw)
            result.update(path=str(destination), sha256=hashlib.sha256(raw).hexdigest())
            messages = []
            lines = raw.splitlines(keepends=True)
            dropped_tail_bytes = 0
            for index, line in enumerate(lines):
                if not line.strip():
                    continue
                try:
                    messages.append(json.loads(line))
                except ValueError, UnicodeDecodeError:
                    if index == len(lines) - 1 and not raw.endswith(b"\n"):
                        dropped_tail_bytes = len(line)
                        break
                    raise
            partial = dropped_tail_bytes > 0
            result.update(partial=partial, dropped_tail_bytes=dropped_tail_bytes)

            if not all(isinstance(message, dict) for message in messages):
                raise ValueError("Cargo session entries must be JSON objects")
            result.update(
                analysis_scope="complete_prefix" if partial else "complete_log",
                parsed_entries=len(messages),
            )
            starts = [m for m in messages if m.get("reason") == "build-started"]
            if len(starts) != 1 or any(m.get("run_id") != session for m in messages):
                raise ValueError("Cargo session log identity mismatch")
            start = starts[0]
            if (
                Path(start["workspace_root"]).resolve() != workspace
                or Path(start["cwd"]).resolve() != Path.cwd().resolve()
            ):
                raise ValueError("Cargo session workspace/cwd mismatch")
            actual = start["command"]
            is_nextest = "nextest" in command
            expected = command[1:]
            if expected and expected[0].startswith("+"):
                expected = expected[1:]
            if not is_nextest and actual[1:] != expected:
                raise ValueError("Cargo session command differs from the effective invocation")
            if is_nextest and "test" not in actual:
                raise ValueError("nextest session did not run Cargo test")
            for option in ("--cargo-profile",) if is_nextest else ("--profile",):
                requested = option_value(command, option)
                if requested is not None and start["profile"] != requested:
                    raise ValueError("Cargo session profile differs from the requested profile")
            if "--release" in command and start["profile"] != "release":
                raise ValueError("Cargo session did not use the requested release profile")
            requested_target = (
                option_value(command, "--target-dir")
                or env.get("CARGO_TARGET_DIR")
                or env.get("CARGO_BUILD_TARGET_DIR")
            )
            if (
                requested_target
                and Path(start["target_dir"]).resolve() != Path(requested_target).resolve()
            ):
                raise ValueError(
                    "Cargo session target directory differs from the requested directory"
                )
            timing = Path(start["target_dir"]) / "cargo-timings" / f"cargo-timing-{session}.html"
            timing_destination = destination.with_suffix(".html")
            if timing.is_file():
                shutil.copyfile(timing, timing_destination)
            result.update(
                outcome="passed",
                path=str(destination),
                timing=str(timing_destination) if timing_destination.exists() else None,
                build=start,
                sha256=hashlib.sha256(raw).hexdigest(),
            )
        except (OSError, ValueError, KeyError, TypeError) as error:
            result["reason"] = str(error)
        collected.append(result)
    return {
        "outcome": "passed" if all(r["outcome"] == "passed" for r in collected) else "blocked",
        "partial": any(r.get("partial", False) for r in collected),
        "sessions": collected,
    }


def delegate_wrapper(env: dict[str, str]) -> str:
    if "RUSTC_WRAPPER" in env:
        return env["RUSTC_WRAPPER"]
    if "CARGO_BUILD_RUSTC_WRAPPER" in env:
        return env["CARGO_BUILD_RUSTC_WRAPPER"]
    try:
        config = tomllib.loads((ROOT / ".cargo/config.toml").read_text())
        wrapper = config.get("build", {}).get("rustc-wrapper", "sccache")
    except OSError:
        wrapper = "sccache"
    return shutil.which(wrapper) or wrapper


class CommandSubreaper:
    """Keep orphaned descendants under this command's owner, not PID 1."""

    def __init__(self):
        self.owner = capture().process_identity(os.getpid())
        self.existing_children = set()
        for path in Path("/proc").iterdir():
            if not path.name.isdigit():
                continue
            try:
                fields = (path / "stat").read_text().rsplit(")", 1)[1].split()
                if int(fields[1]) == os.getpid():
                    self.existing_children.add(int(path.name))
            except OSError, ValueError, IndexError:
                continue
        self.libc = ctypes.CDLL(None, use_errno=True)
        previous = ctypes.c_int()
        if self.libc.prctl(37, ctypes.byref(previous), 0, 0, 0) != 0:
            raise OSError(ctypes.get_errno(), "PR_GET_CHILD_SUBREAPER")
        self.previous = previous.value
        if self.libc.prctl(36, 1, 0, 0, 0) != 0:
            raise OSError(ctypes.get_errno(), "PR_SET_CHILD_SUBREAPER")

    def close(self) -> None:
        if self.libc.prctl(36, self.previous, 0, 0, 0) != 0:
            raise OSError(ctypes.get_errno(), "restore PR_SET_CHILD_SUBREAPER")


class OwnedCommandTree:
    """Retain verified descendants while ancestry exists, including new sessions."""

    def __init__(self, pid: int, subreaper=None, directory: Path | None = None):
        self.root_pid = pid
        self.subreaper = subreaper
        self.directory = directory
        self.observer_groups: set[int] = set()
        self.shared_groups: set[int] = set()
        self.adopted: set[int] = set()
        try:
            identity = capture().process_identity(pid)
            self.processes = {pid: {**identity, "pgid": os.getpgid(pid)}}
        except OSError:
            self.processes = {}
        self.signalled: set[tuple[int, int]] = set()
        self.actions: list[dict[str, Any]] = []

    def observe(self) -> None:
        snapshots = {}
        for path in Path("/proc").iterdir():
            if not path.name.isdigit():
                continue
            try:
                fields = (path / "stat").read_text().rsplit(")", 1)[1].split()
                if fields[0] not in ("Z", "X"):
                    snapshots[int(path.name)] = fields
            except OSError, ValueError, IndexError:
                continue
        anchors = {
            pid
            for pid, identity in self.processes.items()
            if capture().identity_matches(pid, identity)
        }
        if self.subreaper is not None:
            anchors.add(os.getpid())
        while True:
            added = set()
            for pid, fields in snapshots.items():
                if pid in anchors or int(fields[1]) not in anchors:
                    continue
                if self.subreaper is not None and pid in self.subreaper.existing_children:
                    continue
                try:
                    identity = capture().process_identity(pid)
                    if identity["uid"] != os.getuid() or identity["start_time"] != fields[19]:
                        continue
                    parent = int(fields[1])
                    parent_identity = (
                        self.subreaper.owner
                        if parent == os.getpid() and self.subreaper
                        else self.processes[parent]
                    )
                    if not capture().identity_matches(parent, parent_identity):
                        continue
                    pgid = os.getpgid(pid)
                    argv = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
                    executable = Path(argv[0].decode()).name
                    # A daemon started by an owned client becomes shared infrastructure;
                    # never adopt that separate sccache session or its children.
                    if executable == "sccache" and pgid == pid:
                        self.shared_groups.add(pgid)
                        continue
                    if pgid in self.shared_groups:
                        continue
                    if not capture().identity_matches(pid, identity):
                        continue
                    self.processes[pid] = {**identity, "pgid": pgid}
                    if parent == os.getpid() and pid != self.root_pid:
                        self.adopted.add(pid)
                    if (
                        len(argv) > 3
                        and Path(argv[1].decode()).resolve() == Path(capture().__file__).resolve()
                        and argv[2] == b"_sample"
                        and self.directory is not None
                        and Path(argv[3].decode()).resolve().is_relative_to(self.directory)
                    ):
                        self.observer_groups.add(pgid)
                    added.add(pid)
                except OSError, ValueError, IndexError:
                    continue
            if not added:
                break
            anchors.update(added)
        self.observe_collectors()
        for pid in self.adopted:
            identity = self.processes[pid]
            if not capture().identity_matches(pid, identity):
                try:
                    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
                    if fields[19] == identity["start_time"] and fields[0] == "Z":
                        with suppress(ChildProcessError):
                            os.waitpid(pid, os.WNOHANG)
                except OSError, IndexError:
                    pass

    def observe_collectors(self) -> None:
        if self.directory is None:
            return
        for name in ("sampler.json", "collector.json"):
            for path in self.directory.glob(f"units/*/{name}"):
                metadata = read_json(path)
                if not isinstance(metadata, dict):
                    continue
                for key in ("identity", "recorder_identity"):
                    identity = metadata.get(key)
                    if (
                        isinstance(identity, dict)
                        and {"pid", "uid", "start_time"}.issubset(identity)
                        and isinstance(identity["pid"], int)
                        and identity["uid"] == os.getuid()
                        and capture().identity_matches(identity["pid"], identity)
                    ):
                        with suppress(ProcessLookupError):
                            self.observer_groups.add(os.getpgid(identity["pid"]))

    def escaped(self) -> dict[int, dict[str, Any]]:
        # Samplers observe target/owner death and finalize under their own 10s + 5s
        # policy. Compiler cancellation must neither signal perf nor shorten that grace.
        groups = {}
        for pid, identity in self.processes.items():
            if identity["uid"] != os.getuid() or not capture().identity_matches(pid, identity):
                continue
            try:
                pgid = os.getpgid(pid)
                if (
                    pgid != os.getpgrp()
                    and pgid not in self.observer_groups
                    and capture().identity_matches(pid, identity)
                ):
                    groups[pgid] = {**identity, "pgid": pgid}
            except ProcessLookupError:
                continue
        return groups

    def signal_escaped(self, signum: int) -> None:
        for pgid, identity in self.escaped().items():
            if (pgid, signum) in self.signalled:
                continue
            if not capture().identity_matches(identity["pid"], identity):
                continue
            try:
                if os.getpgid(identity["pid"]) != pgid:
                    continue
                os.killpg(pgid, signum)
            except ProcessLookupError:
                continue
            self.signalled.add((pgid, signum))
            self.actions.append({**identity, "signal": signum})

    def observers_pending(self) -> bool:
        for pid, identity in self.processes.items():
            if capture().identity_matches(pid, identity):
                with suppress(ProcessLookupError):
                    if os.getpgid(pid) in self.observer_groups:
                        return True
        return False


def diagnostic_settings() -> dict[str, Any]:
    settings = {}
    host = Path(
        os.environ.get(
            "LCTX_STORAGE_CONFIG",
            str(
                Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config")))
                / "library-context-storage/config.toml"
            ),
        )
    )
    for path in (ROOT / ".config/storage.toml", host):
        if path.exists():
            with path.open("rb") as stream:
                values = tomllib.load(stream).get("diagnostics", {})
            if not isinstance(values, dict):
                raise ValueError("diagnostics configuration must be a table")
            settings.update(values)
            if path == host:
                with path.open("rb") as stream:
                    selected = tomllib.load(stream).get("diagnostic_root")
                if selected is not None:
                    settings["root"] = selected
    return settings


def advisory_threshold(kind: str) -> int:
    name = (
        "LCTX_STORAGE_RECORDING_ADVISORY_BYTES"
        if kind == "recording"
        else "LCTX_STORAGE_WORK_ADVISORY_BYTES"
    )
    key = "detailed_advisory_gib" if kind == "recording" else "general_advisory_gib"
    default = diagnostic_settings().get(key, 100 if kind == "recording" else 64)
    if type(default) not in (int, float) or default < 0:
        raise ValueError(f"diagnostics.{key} must be nonnegative")
    value = int(os.environ.get(name, str(int(default * 1024**3))))
    if value < 0:
        raise ValueError(f"{name} must be nonnegative")
    return value


def diagnostic_destination(requested: str | None = None) -> Path | None:
    selected = (
        requested
        or os.environ.get("LCTX_COMPILE_PROFILE_ROOT")
        or diagnostic_settings().get("root")
    )
    if not selected:
        return None
    path = Path(selected).expanduser().absolute()
    # An explicit selection is never silently created/fallen back from during readiness.
    if not path.is_dir() or path.is_symlink() or not os.access(path, os.W_OK | os.X_OK):
        raise ValueError(f"selected diagnostic root unavailable or unwritable: {path}")
    if os.statvfs(path).f_flag & os.ST_RDONLY:
        raise ValueError(f"selected diagnostic root is on a read-only filesystem: {path}")
    return path.resolve()


def diagnostic_headroom(selected: Path | None = None) -> list[dict[str, Any]]:
    observations = []
    for destination in dict.fromkeys([runs.runs_root(), *([selected] if selected else [])]):
        existing = destination
        while not existing.exists():
            existing = existing.parent
        observations.append(
            {
                "destination": str(destination),
                "observed_path": str(existing),
                "device": existing.stat().st_dev,
                "free_bytes": shutil.disk_usage(existing).free,
            }
        )
    return observations


def capacity_observation(directory: Path) -> dict[str, Any]:
    free = shutil.disk_usage(directory).free
    allocated = 0
    files = 0
    for base, _, names in os.walk(directory):
        for name in names:
            path = Path(base) / name
            if not path.is_symlink():
                try:
                    allocated += path.stat().st_blocks * 512
                    files += 1
                except FileNotFoundError:
                    pass
    return {
        "observed_ns": time.time_ns(),
        "destination": str(directory),
        "free_bytes": free,
        "allocated_bytes": allocated,
        "files": files,
        "work_advisory_bytes": advisory_threshold("work"),
        "recording_advisory_bytes": advisory_threshold("recording"),
        "advisory": free < advisory_threshold("recording"),
        "writers": [
            "perf chunks",
            "rustc self-profile",
            "pass/mono sidecars",
            "Cargo receipts",
            "observer logs",
        ],
        "support_limit": (
            "observations do not bound peak allocation; "
            "self-profile cannot be stopped independently of rustc"
        ),
    }


def reports_root(directory: Path) -> Path:
    return directory.parent / "compile-profile-reports"


def report_location(path: Path) -> Path:
    # Separate new derived outputs from raw unit evidence. Legacy standalone readers
    # remain useful for explicit inputs without inventing a profile owner.
    if path.parent.parent.name == "units":
        return reports_root(path.parent.parent.parent) / path.parent.name
    return path.parent / "reports"


def resolve_recorded_path(directory: Path, recorded: str) -> Path:
    """Translate only the exact original profile prefix; never arbitrary suffix matching."""
    original = (read_json(directory / "record.json") or {}).get("original_profile_root") or (
        read_json(directory / "path-resolution.json") or {}
    ).get("original_profile_root")
    path = Path(recorded)
    if original and path.is_absolute() and path.is_relative_to(Path(original)):
        relative = path.relative_to(Path(original))
        if ".." in relative.parts:
            raise ValueError("unsafe recorded profile path")
        return directory / relative
    return path


def lifecycle_components(run_dir: Path) -> list[dict[str, Any]]:
    directory = profile_dir(str(run_dir))
    meta = read_json(directory / "record.json") or {}
    run = runs.view(run_dir)
    cleanup = run.get("cleanup", {}).get("status") == "confirmed"
    from storage_lifecycle import Storage

    policy = Storage().policy
    result = []
    for category, path in (("profile-raw", directory), ("profile-report", reports_root(directory))):
        days = policy["categories"][category].get("temporary_days")
        if path.exists():
            result.append(
                {
                    "category": category,
                    "path": str(path),
                    "temporary_days": days,
                    "consumer": f"profile:{run_dir.name}:{category}",
                    "owner": {
                        "kind": "profile",
                        "path": str(run_dir),
                        "root": str(ROOT),
                        "reference": run_dir.name,
                    },
                    "cleanup_confirmed": cleanup,
                    "legacy_hold": not meta.get("storage_lifecycle_version"),
                    "requires": "raw-replay" if category == "profile-raw" else "cited-report",
                }
            )
    return result


def cmd_record(args: argparse.Namespace) -> int:
    command = list(args.command)
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        raise ValueError("record requires a command after --")
    _effective, direct = cargo_command(command)
    try:
        selected = diagnostic_destination(getattr(args, "diagnostic_root", None))
    except (OSError, ValueError) as error:
        print(json.dumps({"outcome": "blocked", "reason": str(error)}))
        return 75
    readiness = doctor(check_analysis=direct, diagnostic_root=selected)
    if readiness["outcome"] != "passed":
        print(json.dumps(readiness, indent=2))
        return 75
    for observation in readiness.get("checks", {}).get("disk", {}).get("destinations", []):
        free = observation["free_bytes"]
        warning = (
            "; below detailed-recording advisory" if free < advisory_threshold("recording") else ""
        )
        print(
            f"compile-profile: {observation['destination']}: "
            f"{free / 1024**3:.1f} GiB free{warning}; "
            "sampler floor does not bound self-profile/Cargo allocation",
            file=sys.stderr,
        )
    focus_manifests(args.focus, command)
    internal = [sys.executable, str(SCRIPT), "_record", "--focus", args.focus]
    if selected:
        internal += ["--diagnostic-root", str(selected)]
    internal += ["--", *command]
    return runs.cmd_run(
        argparse.Namespace(
            command=internal, background=args.background, label=args.label, json=args.json
        )
    )


def cmd_internal(args: argparse.Namespace) -> int:
    from storage_lifecycle import admission

    run_dir = runs.current_run()
    if run_dir is None:
        raise ValueError("_record requires a live owned run")
    selected = diagnostic_destination(getattr(args, "diagnostic_root", None))
    directory = (
        selected / run_dir.name / "compile-profile" if selected else run_dir / "compile-profile"
    )
    with admission([directory, reports_root(directory)]):
        return _record_owned(args)


def _record_owned(args: argparse.Namespace) -> int:
    run_dir = runs.current_run()
    if run_dir is None:
        raise ValueError("_record requires a live owned run")
    selected = diagnostic_destination(getattr(args, "diagnostic_root", None))
    directory = (
        (selected / run_dir.name / "compile-profile") if selected else run_dir / "compile-profile"
    )
    directory.mkdir(parents=True, exist_ok=True)
    if selected:
        info = directory.stat()
        write_json_atomic(
            run_dir / "compile-profile-location.json",
            {"schema": 1, "path": str(directory), "device": info.st_dev, "inode": info.st_ino},
        )
    if (directory / "record.json").exists():
        raise ValueError("this run already has a compile profile")
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    effective, direct = cargo_command(command)
    manifests = focus_manifests(args.focus, command)
    meta = provenance(directory, command, effective, args.focus, manifests)
    meta.update(
        original_profile_root=str(directory.resolve()),
        storage_lifecycle_version=1,
        capacity=capacity_observation(directory),
    )
    env = normalized_env(dict(os.environ))
    env.update(
        LCTX_COMPILE_PROFILE_DIR=str(directory.resolve()),
        LCTX_COMPILE_PROFILE_FOCUS=json.dumps(manifests),
        LCTX_COMPILE_PROFILE_DELEGATE=delegate_wrapper(env),
        LCTX_COMPILE_PROFILE_PERF=capture().resolve_perf(env),
        RUSTC_WRAPPER=str(ROOT / "scripts/compile_profile_wrapper.py"),
    )
    meta.update(
        environment={
            k: v
            for k, v in env.items()
            if k.startswith("LCTX_COMPILE_PROFILE")
            or (
                k.startswith("CARGO_PROFILE_")
                or k.startswith("CARGO_BUILD_")
                or k
                in {
                    "CARGO_TARGET_DIR",
                    "CARGO_HOME",
                    "CARGO_INCREMENTAL",
                    "CARGO_ENCODED_RUSTFLAGS",
                }
            )
            or k == "RUSTC_WRAPPER"
        },
        cargo_analysis=direct,
        product={"outcome": "not_run"},
        telemetry={"outcome": "not_run"},
    )
    write_json_atomic(directory / "record.json", meta)
    try:
        from storage_lifecycle import Storage

        storage = Storage()
        owner = {
            "kind": "profile",
            "path": str(run_dir),
            "root": str(ROOT),
            "reference": run_dir.name,
        }
        storage.publish(
            directory,
            "profile-raw",
            owner,
            consumer=f"profile:{run_dir.name}:profile-raw",
            requires="raw-replay",
            temporary_days=storage.policy["categories"]["profile-raw"].get("temporary_days"),
            managed=True,
        )
    except (OSError, ValueError, RuntimeError) as error:
        # Failed enrollment protects this scope through unresolved run ownership. No observer
        # failure is allowed to replace the product command's exit.
        (run_dir / runs.RETAIN).touch()
        print(f"compile-profile lifecycle enrollment unresolved: {error}", file=sys.stderr)
    runs.set_progress(current_command=" ".join(command), waiting_reason=None)
    child: subprocess.Popen[bytes] | None = None
    descendants: OwnedCommandTree | None = None
    received: list[int] = []
    stopping_at: list[float] = []
    owned_record = runs.load_record(run_dir) or {}
    group_identity = (
        ProcessIdentity.from_json(owned_record["child"]) if owned_record.get("child") else None
    )
    group_id = (owned_record.get("child") or {}).get("pgid")
    subreaper = CommandSubreaper()

    def stopped(signum, _frame):
        if received:
            return
        received.append(signum)
        stopping_at.append(time.monotonic())
        # Parent-death signals reach only this leader. Forward once to its verified owned group,
        # including compiler wrappers, then remain alive long enough to retain terminal receipts.
        if (
            group_identity is not None
            and group_identity.alive()
            and group_id is not None
            and group_id == os.getpgrp()
        ):
            with suppress(ProcessLookupError):
                os.killpg(group_id, signum)
        elif child is not None:
            with suppress(ProcessLookupError):
                child.send_signal(signum)

    old = {
        signum: signal.signal(signum, stopped)
        for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP)
    }
    if received:
        for signum, handler in old.items():
            signal.signal(signum, handler)
        subreaper.close()
        return 128 + received[0]
    try:
        child = subprocess.Popen(effective, env=env, stdout=subprocess.PIPE)
    except BaseException:
        for signum, handler in old.items():
            signal.signal(signum, handler)
        subreaper.close()
        raise
    if received:
        with suppress(ProcessLookupError):
            child.send_signal(received[0])
    sessions: list[str] = []
    stream_errors: list[str] = []
    descendants = OwnedCommandTree(child.pid, subreaper, directory.resolve())
    try:
        descendants.observe()
    except (OSError, ValueError) as error:
        stream_errors.append(f"initial descendant observation: {error}")
    try:
        meta["command_identity"] = ProcessIdentity.of(child.pid).to_json()
        write_json_atomic(directory / "record.json", meta)
    except (OSError, ProcessLookupError) as error:
        stream_errors.append(f"command identity receipt: {error}")

    def forward_stdout():
        assert child is not None and child.stdout is not None
        raw = None
        try:
            raw = (directory / "command.stdout.log").open("wb")
        except OSError as error:
            stream_errors.append(f"stdout capture open: {error}")
        for line in child.stdout:
            if raw is not None:
                try:
                    raw.write(line)
                    raw.flush()
                except OSError as error:
                    stream_errors.append(f"stdout capture write: {error}")
                    with suppress(OSError):
                        raw.close()
                    raw = None
            try:
                event = json.loads(line)
                if (
                    isinstance(event, dict)
                    and event.get("reason") == "build-started"
                    and isinstance(event.get("run_id"), str)
                ):
                    sessions.append(event["run_id"])
            except ValueError, UnicodeDecodeError:
                pass
            try:
                sys.stdout.buffer.write(line)
                sys.stdout.buffer.flush()
            except (BrokenPipeError, ValueError) as error:
                if not any(item.startswith("stdout forwarding:") for item in stream_errors):
                    stream_errors.append(f"stdout forwarding: {error}")
        if raw is not None:
            with suppress(OSError):
                raw.close()

    try:
        stdout_thread = threading.Thread(target=forward_stdout, daemon=True)
        stdout_thread.start()
        retained_descendants = 0
        while True:
            if descendants is not None:
                try:
                    descendants.observe()
                except (OSError, ValueError) as error:
                    message = f"descendant observation: {error}"
                    if message not in stream_errors:
                        stream_errors.append(message)
                if len(descendants.processes) != retained_descendants:
                    retained_descendants = len(descendants.processes)
                    try:
                        write_json_atomic(
                            directory / "owned-descendants.json",
                            list(descendants.processes.values()),
                        )
                    except OSError as error:
                        stream_errors.append(f"descendant identity receipt: {error}")
                if received:
                    descendants.signal_escaped(received[0])
                    if time.monotonic() - stopping_at[0] >= 5:
                        descendants.signal_escaped(signal.SIGKILL)
            code = child.poll()
            if (
                code is not None
                and not stdout_thread.is_alive()
                and (not received or descendants is None or not descendants.escaped())
            ):
                break
            time.sleep(0.1)
        stdout_thread.join()
        try:
            meta["cargo_sessions"] = (
                retain_cargo_sessions(directory, sessions, env, effective)
                if direct
                else {
                    "outcome": "not_run",
                    "reason": "wrapped command; no Cargo arguments reinterpreted",
                }
            )
            units = unit_records(directory)
        except (OSError, ValueError) as error:
            stream_errors.append(f"capture finalization: {error}")
            units = []
        try:
            meta["capacity_final"] = capacity_observation(directory)
        except OSError as error:
            stream_errors.append(f"capacity observation: {error}")
        meta["stdout_errors"] = stream_errors
        if descendants is not None:
            meta["owned_descendants"] = list(descendants.processes.values())
            meta["descendant_signals"] = descendants.actions
        collector_errors = [
            unit.get("collector")
            for _, unit in units
            if unit.get("collector", {}).get("status") in ("failed", "blocked", "cancelled")
        ]
        telemetry = {
            "outcome": "failed"
            if collector_errors or stream_errors
            else "not_run"
            if descendants is not None and descendants.observers_pending()
            else "passed"
            if units
            else "not_run",
            "units": len(units),
            "collector_errors": collector_errors,
            "observers_pending": descendants is not None and descendants.observers_pending(),
        }
        meta.update(
            ended=runs.now(),
            product={
                "outcome": "passed" if code == 0 and not received else "failed",
                "exit_code": code,
                "signals": received,
            },
            telemetry=telemetry,
        )
        try:
            write_json_atomic(directory / "record.json", meta)
        except OSError as error:
            print(
                f"compile-profile: telemetry final receipt failed: {error}; product exit {code}",
                file=sys.stderr,
            )
        return code if code >= 0 else 128 - code
    finally:
        for signum, handler in old.items():
            signal.signal(signum, handler)
        subreaper.close()


def resolve_run(ref: str | Path) -> Path:
    path = Path(ref)
    if path.is_absolute():
        if path.is_symlink() or not path.is_dir():
            raise ValueError("invalid absolute profile run owner")
        record = runs.load_record(path)
        if not record or record.get("id", path.name) != path.name:
            raise ValueError("absolute profile run owner record differs")
        return path
    return runs.resolve(str(ref))


def profile_dir(ref: str | Path) -> Path:
    run_dir = resolve_run(ref)
    pointer = read_json(run_dir / "compile-profile-location.json")
    if not pointer:
        return run_dir / "compile-profile"
    path = Path(pointer["path"])
    if pointer.get("schema") != 1 or not path.is_absolute() or path.is_symlink():
        raise ValueError("invalid diagnostic location pointer")
    info = path.stat()
    if (info.st_dev, info.st_ino) != (pointer.get("device"), pointer.get("inode")):
        raise ValueError("diagnostic location identity changed")
    return path


def unit_records(directory: Path) -> list[tuple[Path, dict[str, Any]]]:
    return [
        (path.parent, data)
        for path in sorted(directory.glob("units/*/record.json"))
        if (data := read_json(path)) is not None
    ]


def process_metrics(pid: int) -> dict[str, Any]:
    try:
        stat = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        threads = []
        for task in sorted(Path(f"/proc/{pid}/task").iterdir()):
            try:
                thread_stat = (task / "stat").read_text().rsplit(")", 1)[1].split()
                try:
                    wait_channel = (task / "wchan").read_text().strip()
                except OSError:
                    wait_channel = None
                threads.append(
                    {
                        "tid": int(task.name),
                        "label": (task / "comm").read_text().strip(),
                        "cpu_seconds": (int(thread_stat[11]) + int(thread_stat[12]))
                        / os.sysconf("SC_CLK_TCK"),
                        "state": thread_stat[0],
                        "processor": int(thread_stat[36]),
                        "wait_channel": None if wait_channel == "0" else wait_channel,
                    }
                )
            except OSError, ValueError, IndexError:
                continue
        return {
            "sampled_at_ns": time.time_ns(),
            "cpu_seconds": (int(stat[11]) + int(stat[12])) / os.sysconf("SC_CLK_TCK"),
            "rss_bytes": int(stat[21]) * os.sysconf("SC_PAGE_SIZE"),
            "threads": threads,
        }
    except OSError, ValueError, IndexError:
        return {}


def sampled_report(chunk: Path, *, force: bool = False, readonly: bool = False) -> dict[str, Any]:
    output = report_location(chunk) / f"{chunk.name}.hotspots.txt"
    import compile_profile_tools

    perf = Path(capture().resolve_perf()).resolve()
    signature = {
        "sha256": compile_profile_tools.digest(chunk),
        "reader_sha256": compile_profile_tools.digest(perf),
        "settings": ["--children", "--percent-limit", "0.5"],
    }
    receipt = output.with_suffix(".json")
    previous = read_json(receipt)
    if not force and previous and previous.get("input") == signature and output.is_file():
        return previous
    result = execute(
        [
            capture().resolve_perf(),
            "report",
            "--stdio",
            "--children",
            "--percent-limit",
            "0.5",
            "-i",
            str(chunk),
        ]
    )
    if readonly:
        return {
            **result,
            "input": signature,
            "interpretation": "sampled stacks and symbols; no exact mapping to generic rustc items",
        }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(result.get("stdout", "") + result.get("stderr", ""))
    data = {
        **result,
        "input": signature,
        "path": str(output),
        "interpretation": "sampled stacks and symbols; no exact mapping to generic rustc items",
    }
    write_json_atomic(receipt, data)
    return data


def unit_lifecycle(unit: dict[str, Any]) -> str:
    identity = {key: unit[key] for key in ("pid", "uid", "start_time") if key in unit}
    if len(identity) == 3 and identity["start_time"] is not None:
        return "active" if capture().identity_matches(identity["pid"], identity) else "terminated"
    return "terminated" if unit.get("phase") == "completed" else "unknown"


def status_data(ref: str) -> dict[str, Any]:
    directory = profile_dir(ref)
    data = {
        "run": runs.view(resolve_run(ref)),
        "profile": read_json(directory / "record.json"),
        "units": [],
    }
    for path, unit in unit_records(directory):
        lifecycle = unit_lifecycle(unit)
        active = lifecycle == "active"
        try:
            chunks = capture().read_completed_chunks(path)
            issue = None
        except OSError as error:
            chunks, issue = [], str(error)
        recent = chunks[-1] if chunks else None
        data["units"].append(
            {
                "dir": str(path),
                "unit": unit,
                "active": active,
                "lifecycle": lifecycle,
                "phase": "unknown"
                if active
                else "interrupted"
                if lifecycle == "terminated" and unit.get("phase") != "completed"
                else unit.get("phase", "unknown"),
                "resources": process_metrics(unit["pid"]) if active else {},
                "completed_chunks": [str(p) for p in chunks],
                "chunk_issue": issue,
                "recent_hotspots": sampled_report(recent, readonly=True) if recent else None,
            }
        )
    data["capacity"] = capacity_observation(directory)
    data["capture"] = capture_completeness(data["run"], data["profile"] or {})
    return data


def capture_completeness(run: dict[str, Any], meta: dict[str, Any]) -> dict[str, Any]:
    """Completion is evidenced by capture finalization, independently of child exit."""
    reasons = []
    if run.get("state") != "completed":
        reasons.append("run not completed")
    if runs.cleanup_of(run)["status"] != "confirmed":
        reasons.append("run cleanup unresolved")
    if run.get("supervisor_error") or run.get("launch_error"):
        reasons.append("run supervisor or launch failed")
    product = meta.get("product") or {}
    if not meta.get("ended") or product.get("exit_code") is None:
        reasons.append("capture final receipt missing")
    if product.get("exit_code") != 0 or product.get("signals"):
        reasons.append("product interrupted or failed")
    telemetry = meta.get("telemetry") or {}
    if telemetry.get("outcome") != "passed" or telemetry.get("observers_pending"):
        reasons.append("telemetry not complete")
    return {"status": "partial" if reasons else "complete", "reasons": reasons}


def format_guard(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        header = handle.read(8)
    version = struct.unpack("<I", header[4:])[0] if len(header) == 8 else None
    return {
        "outcome": "passed" if header[:4] == b"MMPD" and version == FORMAT else "blocked",
        "format": version,
        "required_format": FORMAT,
        "path": str(path),
    }


def timeline_minimum_duration_us() -> int:
    name = "LCTX_COMPILE_PROFILE_TIMELINE_MIN_US"
    try:
        value = int(os.environ.get(name, "1000"))
    except ValueError as error:
        raise ValueError(f"{name} must be an integer >= 0") from error
    if value < 0:
        raise ValueError(f"{name} must be an integer >= 0")
    return value


def compiler_report(
    path: Path, *, force: bool = False, readers: dict | None = None
) -> dict[str, Any]:
    minimum_duration_us = timeline_minimum_duration_us()
    guard = format_guard(path)
    if guard["outcome"] != "passed":
        return guard
    root = tools_root()
    import compile_profile_tools

    ready: dict[str, Any]
    if readers is not None:
        binding = readers.get("readers", {})
        valid = compile_profile_tools.reader_binding_valid(readers) and all(
            name in binding for name in compile_profile_tools.NAMES
        )
        root = Path(readers.get("generation_root", "/nonexistent"))
        receipt = dict(
            readers.get("measureme_receipt", {}),
            binaries={name: binding.get(name, {}) for name in compile_profile_tools.NAMES},
        )
        ready = {
            "status": "passed" if valid else "blocked",
            "receipt": receipt,
            "errors": [] if valid else ["recorded reader generation unavailable"],
        }
    else:
        ready = compile_profile_tools.check(root)
    if ready["status"] != "passed":
        return {
            "outcome": "blocked",
            "reason": "exact measureme-12.0.3 tools missing or stale",
            "readiness": ready,
            "repair": "just compile-profile-tools",
        }
    output = report_location(path) / path.stem
    output.mkdir(parents=True, exist_ok=True)
    signature = {
        "sha256": compile_profile_tools.digest(path),
        "readers": ready["receipt"].get("binaries", {}),
    }
    receipt = output / "record.json"
    previous = read_json(receipt)
    if (
        not force
        and previous
        and previous.get("input") == signature
        and previous.get("reader_revision") == ready["receipt"]["revision"]
        and previous.get("timeline_minimum_duration_us") == minimum_duration_us
        and previous.get("outcome") == "passed"
        and previous.get("raw_capture") == str(path)
        and previous.get("dir") == str(output)
    ):
        return previous
    report_input = output / path.name
    if report_input.is_symlink() and report_input.resolve() != path.resolve():
        report_input.unlink()
    if not report_input.exists():
        report_input.symlink_to(path.resolve())
    summary = execute(
        [str(root / "bin/summarize"), "summarize", "--json", str(report_input)],
        cwd=output,
        timeout=None,
    )
    timeline = execute(
        [str(root / "bin/crox"), "--minimum-duration", str(minimum_duration_us), str(path)],
        cwd=output,
        timeout=None,
    )
    (output / "summary.log").write_text(summary.get("stdout", "") + summary.get("stderr", ""))
    (output / "timeline.log").write_text(timeline.get("stdout", "") + timeline.get("stderr", ""))
    data = {
        "outcome": "passed" if summary["outcome"] == timeline["outcome"] == "passed" else "blocked",
        "input": signature,
        "reader_revision": ready["receipt"]["revision"],
        "timeline_minimum_duration_us": minimum_duration_us,
        "timeline_filtered": minimum_duration_us > 0,
        "raw_capture": str(path),
        "format": guard,
        "summary": summary,
        "timeline": timeline,
        "dir": str(output),
        "interpretation": (
            "compiler timeline contains nested events; never sum overlapping durations"
        ),
    }
    write_json_atomic(receipt, data)
    return data


def report_data(ref: str, *, force: bool = False) -> dict[str, Any]:
    from storage_lifecycle import admission

    directory = profile_dir(ref)
    with admission([directory, reports_root(directory), tools_root()]):
        data = _report_owned(ref, force=force)
        try:
            from storage_lifecycle import Storage

            run_dir = resolve_run(ref)
            owner = {
                "kind": "profile",
                "path": str(run_dir),
                "root": str(ROOT),
                "reference": run_dir.name,
            }
            storage = Storage()
            consumer = f"profile:{run_dir.name}:profile-report"
            identity = storage.publish(
                reports_root(directory),
                "profile-report",
                owner,
                consumer=consumer,
                requires="cited-report",
                temporary_days=storage.policy["categories"]["profile-report"].get("temporary_days"),
                managed=True,
            )
            if runs.view(run_dir).get("cleanup", {}).get("status") == "confirmed":
                storage.complete(
                    identity, consumer, reason="profile report owner cleanup confirmed"
                )
        except (OSError, ValueError, RuntimeError) as error:
            data["storage_enrollment"] = {"outcome": "blocked", "reason": str(error)}
        return data


def _report_owned(ref: str, *, force: bool = False) -> dict[str, Any]:
    directory = profile_dir(ref)
    status = status_data(ref)
    reports: list[dict[str, Any]] = []
    for path, unit in unit_records(directory):
        lifecycle = unit_lifecycle(unit)
        partial = (
            unit.get("phase") != "completed"
            or unit.get("exit_code") != 0
            or unit.get("status") in ("cancelled", "interrupted", "blocked", "failed")
        )
        compiler = (
            [compiler_report(p, force=force) for p in sorted(path.glob("*.mm_profdata"))]
            if lifecycle == "terminated"
            else []
        )
        sampled = [sampled_report(p, force=force) for p in capture().read_completed_chunks(path)]
        results = compiler + sampled
        reports.append(
            {
                "unit": unit,
                "partial": partial,
                "lifecycle": lifecycle,
                "outcome": "blocked"
                if any(r["outcome"] in ("blocked", "failed") for r in results)
                else "passed"
                if results
                else "not_run",
                "compiler": compiler,
                "sampled": sampled,
                "compiler_reason": "compiler sidecars are still active or their identity is unknown"
                if lifecycle != "terminated"
                else "interrupted or failed compiler; decoders may reject incomplete sidecars"
                if partial
                else None,
                "time_passes": str(path / "time-passes.jsonl"),
                "source_arguments": [
                    arg for arg in unit.get("expanded_argv", []) if arg.endswith(".rs")
                ],
                "mono_statistics": [
                    {
                        "path": str(p),
                        "data": read_json(p),
                        "interpretation": "item counts and size estimates, not elapsed time",
                    }
                    for p in sorted(path.glob("*.mono_items.json"))
                ]
                if lifecycle == "terminated"
                else [],
                "raw_sidecars": [str(p) for p in sorted(path.iterdir()) if p.is_file()],
            }
        )
    meta = status["profile"] or {}
    data = {
        "run": status["run"],
        "product": meta.get("product"),
        "telemetry": meta.get("telemetry"),
        "cargo_sessions": meta.get("cargo_sessions"),
        "capture": status["capture"],
        "partial": status["capture"]["status"] != "complete" or any(r["partial"] for r in reports),
        "units": reports,
        "generated": runs.now(),
    }
    reports_root(directory).mkdir(parents=True, exist_ok=True)
    write_json_atomic(reports_root(directory) / "report.json", data)
    return data


def _sample_symbols(text: str) -> set[str]:
    return {
        match.group(1).strip()
        for match in re.finditer(r"\[\.\]\s+(.+)", text)
        if "[unknown]" not in match.group(1)
    }


def prepare_symbol_closure(
    directory: Path, *, source_files: tuple[Path, ...] = (), expected_symbols: tuple[str, ...] = ()
) -> dict[str, Any]:
    """Collect native build-ID cache objects, without implementing ELF resolution.

    Only a newly owned private cache is populated. Global perf/debug/toolchain caches
    are neither scanned for cleanup nor modified. Missing DSOs block the capability.
    """
    import tempfile

    import compile_profile_tools

    directory = Path(directory).resolve(strict=True)
    target = directory / "replay-dependencies"
    if target.exists():
        return {
            "outcome": "blocked",
            "reason": "dependency closure already exists; preserve its identity",
        }
    if any(unit_lifecycle(unit) != "terminated" for _, unit in unit_records(directory)):
        return {"outcome": "blocked", "reason": "capture unit cleanup is unresolved or active"}
    chunks = [
        chunk
        for unit, _ in unit_records(directory)
        for chunk in capture().read_completed_chunks(unit)
    ]
    if not chunks:
        return {"outcome": "blocked", "reason": "no closed sampled chunk"}
    perf = Path(capture().resolve_perf()).resolve(strict=True)
    with tempfile.TemporaryDirectory(
        prefix=".profile-dependencies-", dir=directory.parent
    ) as staging:
        stage = Path(staging)
        cache = stage / "native-cache"
        cache.mkdir()
        objects = {}
        for chunk in chunks:
            listed = execute(
                [str(perf), "buildid-list", "--with-hits", "-i", str(chunk)], timeout=None
            )
            if listed["outcome"] != "passed":
                return {
                    "outcome": "blocked",
                    "reason": "native build-ID discovery failed",
                    "reader": listed,
                }
            for line in listed.get("stdout", "").splitlines():
                parts = line.split(None, 1)
                if len(parts) != 2:
                    continue
                build_id, filename = parts
                path = Path(filename)
                if not path.is_absolute() or not path.is_file():
                    return {
                        "outcome": "blocked",
                        "reason": f"required sampled object unavailable: {filename}",
                    }
                added = execute(
                    [str(perf), "--buildid-dir", str(cache), "buildid-cache", "--add", str(path)],
                    timeout=None,
                )
                if added["outcome"] != "passed":
                    return {
                        "outcome": "blocked",
                        "reason": "native build-ID caching failed",
                        "reader": added,
                    }
                objects[build_id] = {
                    "original_path": str(path),
                    "sha256": compile_profile_tools.digest(path),
                }
        if not objects:
            return {"outcome": "blocked", "reason": "no native build IDs discovered"}
        bundle = stage / "bundle"
        bundle.mkdir()
        # Materialize only native-generated links whose targets remain in this private cache.
        # The published closure has regular files, so generic archive extraction accepts no links.
        for path in cache.rglob("*"):
            if path.is_symlink() and not path.resolve().is_relative_to(cache):
                return {
                    "outcome": "blocked",
                    "reason": "native cache produced an external dependency",
                }
        shutil.copytree(cache, bundle / "buildid-cache", symlinks=False)
        (bundle / "symbols").mkdir()
        for build_id, object_info in objects.items():
            cached = cache / ".build-id" / build_id[:2] / build_id[2:] / "elf"
            if not cached.is_file():
                return {
                    "outcome": "blocked",
                    "reason": "native cache did not materialize the recorded build ID",
                }
            original_path = Path(object_info["original_path"])
            symbol_path = bundle / "symbols" / original_path.relative_to(original_path.anchor)
            symbol_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(cached, symbol_path)
        copied_sources = []
        expected_source_lines = []
        for source in source_files:
            source = Path(source).resolve(strict=True)
            copied = bundle / "source" / source.relative_to(source.anchor)
            copied.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, copied)
            copied_sources.append(
                {"path": str(source), "sha256": compile_profile_tools.digest(source)}
            )
            expected_source_lines.extend(
                line.strip()
                for line in source.read_text().splitlines()
                if line.strip() and any(symbol in line for symbol in expected_symbols)
            )
        symbols = set(expected_symbols)
        expected_by_chunk = {}
        for chunk in chunks:
            result = execute(
                [
                    str(perf),
                    "--buildid-dir",
                    str(bundle / "buildid-cache"),
                    "report",
                    "--stdio",
                    "--no-children",
                    "--percent-limit",
                    "0",
                    "-i",
                    str(chunk),
                    "--symfs",
                    str(bundle / "symbols"),
                ],
                timeout=None,
            )
            found = _sample_symbols(result.get("stdout", ""))
            if (
                result["outcome"] != "passed"
                or not found
                or (expected_symbols and not set(expected_symbols) <= found)
            ):
                return {
                    "outcome": "blocked",
                    "reason": "meaningful symbolized replay unavailable",
                    "reader": result,
                }
            expected_by_chunk[chunk.relative_to(directory).as_posix()] = sorted(
                set(expected_symbols) or found
            )
            if not expected_symbols:
                symbols.update(found)
        objdump = Path(shutil.which("objdump") or "/nonexistent")
        samply = Path(shutil.which("samply") or "/nonexistent")
        manifest = {
            "schema": 1,
            "perf_sha256": compile_profile_tools.digest(perf),
            "objdump_sha256": compile_profile_tools.digest(objdump) if objdump.is_file() else None,
            "samply_sha256": compile_profile_tools.digest(samply) if samply.is_file() else None,
            "expected_source_lines": expected_source_lines,
            "objects": objects,
            "sources": copied_sources,
            "expected_symbols": sorted(symbols),
            "expected_by_chunk": expected_by_chunk,
            "annotation_symbol": sorted(symbols)[0],
        }
        write_json_atomic(bundle / "record.json", manifest)
        os.rename(bundle, target)
        return {"outcome": "passed", "path": str(target), "manifest": manifest}


def _replay_sample(
    directory: Path, chunks: list[Path], readers: dict | None = None
) -> dict[str, dict]:
    import compile_profile_tools

    blocked = {
        "outcome": "blocked",
        "reason": "symbol/debug/source closure qualification required",
        "reader_executed": False,
    }
    capabilities: dict[str, dict[str, Any]] = {
        name: dict(blocked)
        for name in (
            "sampled-symbolized-report",
            "sampled-symbolized-import",
            "disassembly",
            "source-annotation",
        )
    }
    closure = directory / "replay-dependencies"
    manifest = read_json(closure / "record.json")
    if not chunks or not manifest or manifest.get("schema") != 1:
        return capabilities
    selected = readers.get("readers", {}) if readers is not None else {}
    if readers is not None and (
        not compile_profile_tools.reader_binding_valid(readers) or "perf" not in selected
    ):
        return capabilities
    perf = Path(
        selected["perf"]["path"] if readers is not None else capture().resolve_perf()
    ).resolve()
    if compile_profile_tools.digest(perf) != manifest.get("perf_sha256"):
        return capabilities
    expected = set(manifest.get("expected_symbols", []))
    reports, annotations, sources, imports, import_details = [], [], [], [], []
    for chunk in chunks:
        common = [str(perf), "--buildid-dir", str(closure / "buildid-cache")]
        result = execute(
            [
                *common,
                "report",
                "--stdio",
                "--no-children",
                "--percent-limit",
                "0",
                "-i",
                str(chunk),
                "--symfs",
                str(closure / "symbols"),
            ],
            timeout=None,
        )
        chunk_expected = set(
            manifest.get("expected_by_chunk", {}).get(
                chunk.relative_to(directory).as_posix(), expected
            )
        )
        reports.append(
            result["outcome"] == "passed"
            and bool(chunk_expected)
            and chunk_expected <= _sample_symbols(result.get("stdout", ""))
        )
        annotation = execute(
            [
                *common,
                "annotate",
                "--stdio",
                "--no-source",
                "-i",
                str(chunk),
                "--symfs",
                str(closure / "symbols"),
                "--symbol",
                manifest["annotation_symbol"],
            ],
            timeout=None,
        )
        text = annotation.get("stdout", "")
        annotations.append(
            annotation["outcome"] == "passed" and bool(re.search(r"[0-9a-f]+:\s", text))
        )
        objdump = Path(
            selected.get("objdump", {}).get("path", "/nonexistent")
            if readers is not None
            else shutil.which("objdump") or "/nonexistent"
        )
        expected_lines = manifest.get("expected_source_lines", [])
        if (
            expected_lines
            and objdump.is_file()
            and compile_profile_tools.digest(objdump) == manifest.get("objdump_sha256")
        ):
            source_annotation = execute(
                [
                    *common,
                    "annotate",
                    "--stdio",
                    "--source",
                    "--objdump",
                    str(objdump),
                    "--prefix",
                    str(closure / "source"),
                    "--prefix-strip",
                    "0",
                    "-i",
                    str(chunk),
                    "--symfs",
                    str(closure / "symbols"),
                    "--symbol",
                    manifest["annotation_symbol"],
                ],
                timeout=None,
            )
            source_text = source_annotation.get("stdout", "")
            sources.append(
                source_annotation["outcome"] == "passed"
                and any(line in source_text for line in expected_lines)
            )
        else:
            sources.append(False)
        samply = Path(
            selected.get("samply", {}).get("path", "/nonexistent")
            if readers is not None
            else shutil.which("samply") or "/nonexistent"
        )
        if samply.is_file() and compile_profile_tools.digest(samply) == manifest.get(
            "samply_sha256"
        ):
            output = reports_root(directory) / "replay-import" / chunk.parent.name / chunk.name
            output.mkdir(parents=True, exist_ok=True)
            command = [
                str(samply),
                "import",
                "--save-only",
                "--unstable-presymbolicate",
                "--output",
                str(output / "profile.json.gz"),
            ]
            symbol_dirs = sorted(
                {
                    str((closure / "symbols" / Path(obj["original_path"]).relative_to("/")).parent)
                    for obj in manifest["objects"].values()
                }
            )
            for symbol_dir in symbol_dirs:
                command += ["--symbol-dir", symbol_dir]
            command += [str(chunk)]
            environment = dict(
                os.environ,
                HOME=str(output / "private-home"),
                XDG_CACHE_HOME=str(output / "private-cache"),
                DEBUGINFOD_URLS="",
            )
            imported = execute(command, timeout=None, env=environment)
            import_details.append(imported)
            sidecars = list(output.glob("*.syms.json"))
            symbols_text = "\n".join(sidecar.read_text() for sidecar in sidecars)
            imports.append(
                imported["outcome"] == "passed"
                and bool(sidecars)
                and all(symbol in symbols_text for symbol in expected)
            )
        else:
            imports.append(False)
    capabilities["sampled-symbolized-report"] = {
        "outcome": "passed" if all(reports) else "blocked",
        "reader_executed": True,
        "expected_symbols": sorted(expected),
        "chunks": [str(chunk) for chunk in chunks],
    }
    capabilities["disassembly"] = {
        "outcome": "passed" if all(annotations) else "blocked",
        "reader_executed": True,
    }
    capabilities["source-annotation"] = {
        "outcome": "passed" if all(sources) else "blocked",
        "reader_executed": bool(manifest.get("expected_source_lines")),
    }
    capabilities["sampled-symbolized-import"] = {
        "outcome": "passed" if all(imports) else "blocked",
        "reader_executed": bool(manifest.get("samply_sha256")),
        "readers": import_details,
        "reason": None
        if all(imports)
        else "exact samply perf import did not produce qualified meaningful symbol sidecars",
    }
    return capabilities


def replay_directory(directory: Path) -> dict[str, Any]:
    """Execute selected exact readers and report capability-specific qualification.

    This never treats an unsymbolized perf exit0 as symbolized or annotation replay.
    Native symbol closure must be explicitly bundled and independently qualified.
    """
    directory = Path(directory).resolve(strict=True)
    readers = read_json(directory / "replay-readers.json")
    capabilities = {}
    compiler = []
    sampled = []
    chunks_all = []
    partial = False
    for unit_path, unit in unit_records(directory):
        lifecycle = unit_lifecycle(unit)
        if lifecycle == "active":
            return {"outcome": "blocked", "reason": "capture still active", "capabilities": {}}
        partial |= unit.get("phase") != "completed" or unit.get("exit_code") != 0
        for raw in sorted(unit_path.glob("*.mm_profdata")):
            result = compiler_report(raw, force=True, readers=readers)
            summary = result.get("summary", {})
            timeline = result.get("timeline", {})
            try:
                summary_path = Path(result.get("dir", "/nonexistent")) / (raw.stem + ".json")
                summary_value = json.loads(summary_path.read_text())
                meaningful_summary = bool(summary_value.get("query_data"))
            except OSError, ValueError, AttributeError:
                meaningful_summary = False
            timeline_path = Path(result.get("dir", "/nonexistent")) / "chrome_profiler.json"
            try:
                timeline_value = json.loads(timeline_path.read_text())
                meaningful_timeline = (
                    bool(timeline_value.get("traceEvents"))
                    if isinstance(timeline_value, dict)
                    else bool(timeline_value)
                )
            except OSError, ValueError:
                meaningful_timeline = False
            compiler.append(
                {
                    "raw": str(raw),
                    "summary": summary.get("outcome") == "passed" and meaningful_summary,
                    "timeline": timeline.get("outcome") == "passed" and meaningful_timeline,
                    "reader_result": result,
                }
            )
        chunks = capture().read_completed_chunks(unit_path)
        chunks_all.extend(chunks)
        for chunk in chunks:
            sampled.append(
                {
                    "raw": str(chunk),
                    "outcome": "blocked",
                    "reason": (
                        "symbol/debug/source closure has not been isolated-location qualified"
                    ),
                }
            )
    for capability, key in (("compiler-summary", "summary"), ("compiler-timeline", "timeline")):
        capabilities[capability] = {
            "outcome": "passed" if compiler and all(item[key] for item in compiler) else "blocked",
            "reader_executed": bool(compiler),
            "inputs": [item["raw"] for item in compiler],
        }
    capabilities.update(_replay_sample(directory, chunks_all, readers=readers))
    return {
        "outcome": "passed"
        if any(item.get("outcome") == "passed" for item in capabilities.values())
        else "blocked",
        "partial": partial,
        "capabilities": capabilities,
        "compiler": compiler,
        "sampled": sampled,
    }


def isolated_archive_replay(directory: Path, original: Path) -> dict[str, Any]:
    """Qualify each archive with original evidence/dependency paths unavailable.

    The fixed subprocess owns a disposable mount/user/network namespace. Masking is
    never applied to the host. Original DSO routes expose only hash-verified archived
    copies, including dynamic-loader dependencies; originals remain preserved.
    """
    import tempfile

    bwrap = shutil.which("bwrap")
    if bwrap is None:
        return {
            "outcome": "blocked",
            "capabilities": {},
            "reason": "bubblewrap unavailable; isolated replay required",
        }
    directory, original = directory.resolve(strict=True), original.resolve(strict=True)
    meta = read_json(directory / "record.json") or {}
    resolution = read_json(directory / "path-resolution.json") or {}
    closure = read_json(directory / "replay-dependencies/record.json") or {}
    paths = {original}
    original_root = meta.get("original_profile_root") or resolution.get("original_profile_root")
    if original_root:
        paths.add(Path(original_root).absolute())
    bundled: dict[Path, dict[str, Any]] = {}
    for item in closure.get("objects", {}).values():
        path = Path(item["original_path"]).absolute()
        copy = directory / "replay-dependencies/symbols" / path.relative_to("/")
        if not copy.is_file():
            return {
                "outcome": "blocked",
                "capabilities": {},
                "reason": f"archived DSO closure missing or changed: {path}",
            }
        with copy.open("rb") as copied:
            matches = hashlib.file_digest(copied, "sha256").hexdigest() == item.get("sha256")
        if not matches:
            return {
                "outcome": "blocked",
                "capabilities": {},
                "reason": f"archived DSO closure missing or changed: {path}",
            }
        for route in {path, path.resolve()}:
            bundled[route] = {
                "path": str(route),
                "kind": "bundled-dso",
                "bundle_path": str(copy),
                "sha256": item["sha256"],
                "bundle_identity": [copy.stat().st_dev, copy.stat().st_ino],
            }
            paths.add(route)
    for item in closure.get("sources", []):
        paths.add(Path(item["path"]).absolute())
    for path in (
        Path.home() / ".debug",
        Path.home() / ".cache",
        Path("/usr/lib/debug"),
        Path("/lib/debug"),
        Path("/var/cache/debuginfod"),
        Path("/usr/src/debug"),
    ):
        paths.add(path)
    # Follow aliases only while constructing the namespace, so no original spelling
    # can escape a file mask through a recorded symlink target.
    paths.update(path.resolve() for path in tuple(paths) if path.exists())
    declared_paths = sorted(str(path) for path in paths)
    # A read-only host bind cannot create absent mountpoints. Hide their nearest
    # existing parent instead, so a host-side late creation cannot become visible.
    # Runtime conflicts below block rather than weakening this coverage.
    for path in tuple(paths):
        if path not in bundled and not path.exists():
            ancestor = path.parent
            while not ancestor.exists() and ancestor != ancestor.parent:
                ancestor = ancestor.parent
            paths.remove(path)
            paths.add(ancestor)
    readers = read_json(directory / "replay-readers.json") or {}
    reader_paths = tuple(Path(item["path"]) for item in readers.get("readers", {}).values())
    protected = (directory, SCRIPT, Path(sys.executable).resolve(), tools_root(), *reader_paths)
    for path in paths:
        if (
            not path.is_absolute()
            or path == Path("/")
            or any(item == path or item.is_relative_to(path) for item in protected)
        ):
            return {
                "outcome": "blocked",
                "capabilities": {},
                "reason": f"isolation conflicts with required replay runtime: {path}",
            }
    # Ancestor directory masks already hide descendants. Existing file masks use
    # /dev/null for original source files; DSO paths receive verified bundle copies.
    masks: list[dict[str, Any]] = []
    for path in sorted(paths, key=lambda value: (len(value.parts), str(value))):
        if path not in bundled and any(
            parent["kind"] == "directory" and path.is_relative_to(Path(parent["path"]))
            for parent in masks
        ):
            continue
        if path in bundled:
            masks.append(bundled[path])
        else:
            masks.append({"path": str(path), "kind": "directory" if path.is_dir() else "file"})
    with tempfile.TemporaryDirectory(
        prefix=".replay-runtime-", dir=directory.parent
    ) as runtime_name:
        runtime = Path(runtime_name)
        private_home = runtime / "home"
        private_home.mkdir()
        # The complete staging parent is the only writable host subtree; it contains
        # restored raw, newly derived reports and this private runtime.
        command: list[str] = [
            bwrap,
            "--die-with-parent",
            "--new-session",
            "--unshare-user",
            "--unshare-pid",
            "--unshare-net",
            "--unshare-ipc",
            "--unshare-uts",
            "--ro-bind",
            "/",
            "/",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--bind",
            str(directory.parent),
            str(directory.parent),
        ]
        if readers.get("generation_root"):
            command += ["--ro-bind", readers["generation_root"], readers["generation_root"]]
        for item in masks:
            if item["kind"] == "bundled-dso":
                command += ["--ro-bind", item["bundle_path"], item["path"]]
            else:
                command += (
                    ["--tmpfs", item["path"]]
                    if item["kind"] == "directory"
                    else ["--ro-bind", "/dev/null", item["path"]]
                )
        child = """import hashlib, json, os, pathlib, sys
import compile_profile
masks = json.loads(sys.argv[2])
for item in masks:
    path = pathlib.Path(item['path'])
    if item['kind'] == 'bundled-dso':
        info = path.stat()
        if [info.st_dev, info.st_ino] != item['bundle_identity']:
            raise RuntimeError('original host DSO remains accessible: ' + str(path))
        with path.open('rb') as source:
            if hashlib.file_digest(source, 'sha256').hexdigest() != item['sha256']:
                raise RuntimeError('archive DSO route differs: ' + str(path))
    elif item['kind'] == 'directory':
        if path.exists() and any(path.iterdir()):
            raise RuntimeError('original directory remains accessible: ' + str(path))
    elif path.is_file() and path.stat().st_size:
        raise RuntimeError('original file remains accessible: ' + str(path))
result = compile_profile.replay_directory(pathlib.Path(sys.argv[1]))
result['isolation'] = {'outcome': 'passed', 'method': 'bubblewrap-user-mount-network',
                       'masked_paths': masks, 'private_home': os.environ['HOME'],
                       'original_dso_routes': 'archive-only-read-only-bind',
                       'declared_paths': json.loads(sys.argv[3])}
print(json.dumps(result))
"""
        environment = dict(
            os.environ,
            HOME=str(private_home),
            XDG_CACHE_HOME=str(private_home / ".cache"),
            DEBUGINFOD_URLS="",
            DEBUGINFOD_CACHE_PATH=str(private_home / ".cache/debuginfod"),
            PERF_BUILDID_DIR=str(private_home / ".debug"),
            PYTHONPATH=str(ROOT / "scripts"),
            PYTHONDONTWRITEBYTECODE="1",
            LCTX_COMPILE_PROFILE_TOOLS_ROOT=str(tools_root()),
            LCTX_COMPILE_PROFILE_PERF=capture().resolve_perf(),
        )
        command += [
            "--chdir",
            str(directory.parent),
            "--",
            str(Path(sys.executable).resolve()),
            "-c",
            child,
            str(directory),
            json.dumps(masks),
            json.dumps(declared_paths),
        ]
        observed = subprocess.run(
            command, env=environment, capture_output=True, text=True, check=False
        )
        if observed.returncode:
            return {
                "outcome": "blocked",
                "capabilities": {},
                "reason": "isolated replay unavailable or reader/runtime could not execute",
                "stderr": observed.stderr,
                "isolation": {"outcome": "blocked", "masked_paths": masks},
            }
        try:
            result = json.loads(observed.stdout)
        except ValueError, TypeError:
            return {
                "outcome": "blocked",
                "capabilities": {},
                "reason": "isolated replay returned no valid receipt",
                "stderr": observed.stderr,
            }
        if not isinstance(result, dict) or result.get("isolation", {}).get("outcome") != "passed":
            return {
                "outcome": "blocked",
                "capabilities": {},
                "reason": "isolated replay proof missing",
            }
        return result


def archive_profile(
    directory: Path,
    destination: Path,
    required_capabilities: tuple[str, ...],
    *,
    reader_binding: dict | None = None,
) -> dict[str, Any]:
    import storage_archive

    directory = Path(directory).resolve(strict=True)
    if any(unit_lifecycle(unit) != "terminated" for _, unit in unit_records(directory)):
        return {
            "outcome": "blocked",
            "reason": "unit cleanup/identity unresolved",
            "source_preserved": True,
        }
    meta = read_json(directory / "record.json") or {}
    if not meta.get("original_profile_root") and not (directory / "path-resolution.json").exists():
        write_json_atomic(
            directory / "path-resolution.json",
            {"schema": 1, "original_profile_root": str(directory)},
        )
    if not required_capabilities:
        return {
            "outcome": "blocked",
            "reason": "explicit replay capability required",
            "source_preserved": True,
        }
    if (
        any(
            capability.startswith("sampled-") or capability == "disassembly"
            for capability in required_capabilities
        )
        and not (directory / "replay-dependencies").exists()
    ):
        prepared = prepare_symbol_closure(directory)
        if prepared["outcome"] != "passed":
            return {**prepared, "source_preserved": True}
    import compile_profile_tools

    try:
        selected_readers = (
            reader_binding
            or read_json(directory / "replay-readers.json")
            or compile_profile_tools.prepare_reader_generation(required_capabilities)
        )
    except (OSError, RuntimeError, ValueError) as error:
        return {
            "outcome": "blocked",
            "reason": "exact reader generation unavailable: " + str(error),
            "source_preserved": True,
        }
    if not compile_profile_tools.reader_binding_valid(selected_readers):
        return {
            "outcome": "blocked",
            "reason": "recorded reader generation unavailable",
            "source_preserved": True,
        }
    write_json_atomic(directory / "replay-readers.json", selected_readers)
    result = storage_archive.create_archive(
        directory,
        destination,
        replay=lambda restored: isolated_archive_replay(restored, directory),
        required_capabilities=required_capabilities,
        identity={
            "run": directory.parent.name,
            "original_root": meta.get("original_profile_root", str(directory)),
            "tools": meta.get("tools", {}),
        },
    )
    result["readers"] = selected_readers["readers"]
    result["reader_generation"] = selected_readers
    return result


def restore_profile(archive: Path, destination: Path) -> dict[str, Any]:
    """Restore immutable raw under a fresh run owner and a 14-day raw obligation."""
    import storage_archive
    from harness import hold_lock
    from storage_lifecycle import Storage, admission

    archive, destination = Path(archive).resolve(strict=True), Path(destination).absolute()
    if destination.exists() or destination.is_symlink():
        raise ValueError("restore destination already exists")
    run_dir = runs.new_run_dir()
    owner = runs.Owner(
        run_dir,
        ["storage", "restore", str(archive)],
        label="profile-restore",
        cwd=str(ROOT),
        mode="foreground",
        stream=False,
    )
    storage = Storage()
    run_consumer = f"run:{run_dir.name}"
    raw_consumer = f"profile:{run_dir.name}:profile-raw"
    with admission([run_dir, destination, reports_root(destination), archive]):
        lock = hold_lock(run_dir / runs.OWNER_LOCK)
        try:
            owner.record.update(phase="restoring", current_command="verified profile restore")
            owner.write()
            result = storage_archive.restore_archive(archive, destination)
            info = destination.stat()
            write_json_atomic(
                run_dir / "compile-profile-location.json",
                {
                    "schema": 1,
                    "path": str(destination),
                    "device": info.st_dev,
                    "inode": info.st_ino,
                },
            )
            raw_id = storage.publish(
                destination,
                "profile-raw",
                {
                    "kind": "profile",
                    "path": str(run_dir),
                    "root": str(ROOT),
                    "reference": run_dir.name,
                },
                raw_consumer,
                requires="raw-replay",
                temporary_days=storage.policy["categories"]["profile-raw"].get("temporary_days"),
                managed=True,
            )
            owner.record.update(
                phase="finished",
                ended=runs.now(),
                termination="completed",
                exit={"code": 0},
                current_command=None,
                cleanup={
                    "status": "confirmed",
                    "observed": runs.now(),
                    "reason": "synchronous verified extraction finished; no child processes",
                },
            )
            owner.write()
        except BaseException:
            owner.record.update(
                phase="finished",
                ended=runs.now(),
                termination="failed",
                exit={"code": 1},
                current_command=None,
            )
            owner.write()
            raise
        finally:
            os.close(lock)
        storage.complete(raw_id, raw_consumer, reason="verified restore completed")
        run_id = storage.publish(
            run_dir,
            "run-receipt",
            {"kind": "run", "path": str(run_dir)},
            run_consumer,
            temporary_days=storage.policy["categories"]["run-receipt"].get("temporary_days"),
            managed=True,
        )
        storage.complete(run_id, run_consumer, reason="verified restore owner completed")
        return {**result, "run": run_dir.name, "run_dir": str(run_dir), "raw_id": raw_id}


def cmd_attach(args: argparse.Namespace) -> int:
    directory = profile_dir(args.run)
    record = runs.load_record(resolve_run(args.run))
    if (
        not record
        or runs.state_of(directory.parent, record) != "running"
        or not record.get("child")
    ):
        raise ValueError("attach requires a live owned run")
    leader = ProcessIdentity.from_json(record["child"])
    if not leader.alive():
        raise ValueError("owned command identity is no longer live")
    readiness = capture().doctor()
    if readiness.get("status") not in ("ready", "passed"):
        print(json.dumps(readiness, indent=2))
        return 75
    (directory.parent / runs.RETAIN).touch()
    # Only descendants of the exact recorded command; never a shared sccache server.
    parent_map = {}
    for path in Path("/proc").iterdir():
        if path.name.isdigit():
            try:
                fields = (path / "stat").read_text().rsplit(")", 1)[1].split()
                parent_map[int(path.name)] = int(fields[1])
            except OSError, ValueError, IndexError:
                continue
    descendants = {leader.pid}
    while True:
        expanded = descendants | {
            pid for pid, parent in parent_map.items() if parent in descendants
        }
        if expanded == descendants:
            break
        descendants = expanded
    collectors = []
    for pid in sorted(descendants):
        try:
            argv = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
            executable = Path(argv[0].decode()).name
            if (
                executable not in ("rustc", "clang", "clang++", "ld", "ld.lld", "mold")
                or not leader.alive()
            ):
                continue
            identity = capture().process_identity(pid)
            path = directory / "units" / f"attached-{pid}-{identity['start_time']}"
            path.mkdir(parents=True, exist_ok=True)
            write_json_atomic(
                path / "record.json",
                {
                    **identity,
                    "version": 1,
                    "kind": "attached",
                    "phase": "running",
                    "input_argv": [p.decode(errors="replace") for p in argv if p],
                    "sidecars": {},
                    "provenance": (
                        "sample-only attachment; compiler flags and completed prefix unavailable"
                    ),
                },
            )
            collectors.append((path, identity, capture().start_sampler(path, pid, identity)))
        except (OSError, ValueError) as error:
            print(f"attach: {pid}: {error}", file=sys.stderr)
    if not collectors:
        raise ValueError(
            "no eligible owned compiler/linker descendants; shared sccache daemons are excluded"
        )
    stop = []

    def observe_signal(signum, _frame):
        stop.append(signum)

    old_handlers = {
        sig: signal.signal(sig, observe_signal)
        for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP)
    }
    try:
        while not stop and leader.alive() and any(proc.poll() is None for _, _, proc in collectors):
            if shutil.disk_usage(directory).free < MIN_FREE:
                stop.append("minimum free space reached")
                break
            for _path, identity, proc in collectors:
                if not capture().identity_matches(identity["pid"], identity):
                    capture().stop_sampler(proc)
            time.sleep(0.5)
    except KeyboardInterrupt:
        pass
    finally:
        for sig, handler in old_handlers.items():
            signal.signal(sig, handler)
        for path, _identity, proc in collectors:
            code = capture().stop_sampler(proc)
            meta = read_json(path / "record.json") or {}
            meta.update(
                phase="completed",
                collector={
                    "exit_code": code,
                    "status": "passed"
                    if capture().sampler_succeeded(path, code) and not stop
                    else "failed",
                    "stop_reasons": stop,
                },
                ended=runs.now(),
            )
            write_json_atomic(path / "record.json", meta)
    return 0


def cmd_view(args: argparse.Namespace) -> int:
    from storage_lifecycle import admission

    directory = profile_dir(args.run)
    with admission([directory, reports_root(directory), tools_root()]):
        return _view_owned(args)


def _view_owned(args: argparse.Namespace) -> int:
    directory = profile_dir(args.run)
    if args.kind == "sampled":
        chunks = [
            p for path, _ in unit_records(directory) for p in capture().read_completed_chunks(path)
        ]
        if not chunks:
            raise ValueError("no completed sampled chunk; active perf.data is never opened")
        samply = shutil.which("samply")
        if not samply:
            raise ValueError("samply 0.13.1 missing")
        # Import is local; no automatic upload or hosting outside localhost.
        return subprocess.call([samply, "import", "--address", "127.0.0.1", str(chunks[-1])])
    if args.kind == "hotspot":
        data = report_data(args.run)
        if args.function:
            chunks = [
                p
                for path, _ in unit_records(directory)
                for p in capture().read_completed_chunks(path)
            ]
            if not chunks:
                raise ValueError("no completed sampled chunk")
            return subprocess.call(
                [
                    capture().resolve_perf(),
                    "annotate",
                    "--stdio",
                    "-i",
                    str(chunks[-1]),
                    "--symbol",
                    args.function,
                ]
            )
        for unit in data["units"]:
            for result in unit.get("sampled", []):
                print(Path(result["path"]).read_text())
        return 0
    data = report_data(args.run)
    for unit in data["units"]:
        for result in unit.get("compiler", []):
            if result.get("dir"):
                print(
                    f"Perfetto: open {Path(result['dir']) / 'chrome_profiler.json'} "
                    "locally at https://ui.perfetto.dev (no automatic upload)"
                )
    return 0


def parser() -> argparse.ArgumentParser:
    top = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = top.add_subparsers(dest="action", required=True)
    sub.add_parser("doctor")
    for name in ("record", "_record"):
        cmd = sub.add_parser(name)
        cmd.add_argument("--focus", default="cpg-core")
        cmd.add_argument(
            "--diagnostic-root",
            help="existing writable diagnostic root; does not change Cargo roots",
        )
        if name == "record":
            cmd.add_argument("--background", action="store_true")
            cmd.add_argument("--label")
            cmd.add_argument("--json", action="store_true")
        cmd.add_argument("command", nargs=argparse.REMAINDER)
    dependencies = sub.add_parser(
        "dependencies", help="prepare native replay dependencies without releasing raw"
    )
    dependencies.add_argument("run")
    dependencies.add_argument("--symbol", action="append", default=[])
    dependencies.add_argument("--source-file", action="append", type=Path, default=[])
    for name in ("attach", "status", "report", "view"):
        cmd = sub.add_parser(name)
        cmd.add_argument("run")
        if name == "report":
            cmd.add_argument("--force", action="store_true")
        if name == "view":
            cmd.add_argument(
                "--kind", choices=("sampled", "compiler", "hotspot"), default="hotspot"
            )
            cmd.add_argument("--function", help="explicit symbol to annotate; hotspot only")
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        if args.action == "doctor":
            data = doctor()
            print(json.dumps(data, indent=2))
            return 0 if data["outcome"] == "passed" else 75
        if args.action == "record":
            return cmd_record(args)
        if args.action == "_record":
            return cmd_internal(args)
        if args.action == "dependencies":
            from storage_lifecycle import admission

            directory = profile_dir(args.run)
            with admission([directory, tools_root()]):
                data = prepare_symbol_closure(
                    directory,
                    source_files=tuple(args.source_file),
                    expected_symbols=tuple(args.symbol),
                )
            print(json.dumps(data, indent=2))
            return 0 if data["outcome"] == "passed" else 75
        if args.action == "attach":
            return cmd_attach(args)
        if args.action == "view":
            return cmd_view(args)
        data = (
            status_data(args.run)
            if args.action == "status"
            else report_data(args.run, force=args.force)
        )
        print(json.dumps(data, indent=2))
        return 0
    except (ValueError, OSError, runs.RunNotFound) as error:
        print(f"compile-profile: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
