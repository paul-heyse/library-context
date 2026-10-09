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


def execute(argv: list[str], *, cwd: Path = ROOT, timeout: float | None = 30) -> dict[str, Any]:
    try:
        proc = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, timeout=timeout)
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


def doctor(*, check_analysis: bool = True) -> dict[str, Any]:
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
    destination = runs.runs_root()
    while not destination.exists():
        destination = destination.parent
    free = shutil.disk_usage(destination).free
    checks["disk"] = {
        "outcome": "passed" if free >= MIN_FREE else "blocked",
        "free_bytes": free,
        "minimum_bytes": MIN_FREE,
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
                        self.subreaper.owner if parent == os.getpid() and self.subreaper
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


def cmd_record(args: argparse.Namespace) -> int:
    command = list(args.command)
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        raise ValueError("record requires a command after --")
    _effective, direct = cargo_command(command)
    readiness = doctor(check_analysis=direct)
    if readiness["outcome"] != "passed":
        print(json.dumps(readiness, indent=2))
        return 75
    focus_manifests(args.focus, command)
    internal = [sys.executable, str(SCRIPT), "_record", "--focus", args.focus, "--", *command]
    return runs.cmd_run(
        argparse.Namespace(
            command=internal, background=args.background, label=args.label, json=args.json
        )
    )


def cmd_internal(args: argparse.Namespace) -> int:
    run_dir = runs.current_run()
    if run_dir is None:
        raise ValueError("_record requires a live owned run")
    directory = run_dir / "compile-profile"
    directory.mkdir(parents=True, exist_ok=True)
    (run_dir / runs.RETAIN).touch()
    if (directory / "record.json").exists():
        raise ValueError("this run already has a compile profile")
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    effective, direct = cargo_command(command)
    manifests = focus_manifests(args.focus, command)
    meta = provenance(directory, command, effective, args.focus, manifests)
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
                        write_json_atomic(directory / "owned-descendants.json", list(descendants.processes.values()))
                    except OSError as error:
                        stream_errors.append(f"descendant identity receipt: {error}")
                if received:
                    descendants.signal_escaped(received[0])
                    if time.monotonic() - stopping_at[0] >= 5:
                        descendants.signal_escaped(signal.SIGKILL)
            code = child.poll()
            if code is not None and not stdout_thread.is_alive() and (
                not received or descendants is None or not descendants.escaped()
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


def profile_dir(ref: str) -> Path:
    return runs.resolve(ref) / "compile-profile"


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
    output = chunk.parent / "reports" / f"{chunk.name}.hotspots.txt"
    signature = {"size": chunk.stat().st_size, "mtime_ns": chunk.stat().st_mtime_ns}
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
        "run": runs.view(directory.parent),
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
    return data


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


def compiler_report(path: Path, *, force: bool = False) -> dict[str, Any]:
    minimum_duration_us = timeline_minimum_duration_us()
    guard = format_guard(path)
    if guard["outcome"] != "passed":
        return guard
    root = tools_root()
    import compile_profile_tools

    ready = compile_profile_tools.check(root)
    if ready["status"] != "passed":
        return {
            "outcome": "blocked",
            "reason": "exact measureme-12.0.3 tools missing or stale",
            "readiness": ready,
            "repair": "just compile-profile-tools",
        }
    output = path.parent / "reports" / path.stem
    output.mkdir(parents=True, exist_ok=True)
    signature = {"size": path.stat().st_size, "mtime_ns": path.stat().st_mtime_ns}
    receipt = output / "record.json"
    previous = read_json(receipt)
    if (
        not force
        and previous
        and previous.get("input") == signature
        and previous.get("reader_revision") == ready["receipt"]["revision"]
        and previous.get("timeline_minimum_duration_us") == minimum_duration_us
        and previous.get("outcome") == "passed"
    ):
        return previous
    report_input = output / path.name
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
        "partial": status["run"].get("state") in ("cancelled", "interrupted", "running")
        or (meta.get("product") or {}).get("exit_code", 0) != 0
        or bool((meta.get("product") or {}).get("signals"))
        or any(r["partial"] for r in reports),
        "units": reports,
        "generated": runs.now(),
    }
    write_json_atomic(directory / "report.json", data)
    return data


def cmd_attach(args: argparse.Namespace) -> int:
    directory = profile_dir(args.run)
    record = runs.load_record(directory.parent)
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
        if name == "record":
            cmd.add_argument("--background", action="store_true")
            cmd.add_argument("--label")
            cmd.add_argument("--json", action="store_true")
        cmd.add_argument("command", nargs=argparse.REMAINDER)
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
