"""Opt-in rustc telemetry and independently owned Linux perf observers.

The wrapper uses the pinned Python 3.14.7 interpreter. No compiler flags
other than diagnostic profiling flags are introduced and no compilation is forced.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import signal
import subprocess
import sys
import threading
import time
from collections.abc import Mapping, Sequence
from contextlib import suppress
from pathlib import Path
from typing import Any

DEFAULT_MIN_FREE_BYTES = 8 * 1024**3
PRIVATE_PERF = Path("/opt/compile-profiling/perf")


class Sampler(subprocess.Popen[bytes]):
    """Collector child with the location of its own completion sidecar."""

    unit_dir: Path


def write_json(path: Path, value: Any) -> None:
    """Replace one owned sidecar atomically, never another invocation's record."""
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def resolve_perf(env: Mapping[str, str] | None = None) -> str:
    environment = os.environ if env is None else env
    requested = environment.get("LCTX_COMPILE_PROFILE_PERF")
    if requested:
        return str(Path(requested).absolute()) if "/" in requested else requested
    return str(PRIVATE_PERF if PRIVATE_PERF.is_file() else Path("/usr/bin/perf"))


def doctor(perfbin: str | None = None) -> dict[str, Any]:
    """Inspect readiness without changing permissions or recording a workload."""
    executable = perfbin or resolve_perf()
    resolved = shutil.which(executable)
    result: dict[str, Any] = {
        "perf": executable,
        "uid": os.getuid(),
        "status": "blocked",
        "attach_test": "not_run",
    }
    try:
        result["perf_event_paranoid"] = int(
            Path("/proc/sys/kernel/perf_event_paranoid").read_text().strip()
        )
    except (OSError, ValueError) as error:
        result["policy_error"] = str(error)
    if resolved is None:
        result["reason"] = "perf executable missing; install the profiling tools"
        return result
    result["perf"] = resolved
    try:
        version = subprocess.run(
            [resolved, "--version"], capture_output=True, text=True, check=False, timeout=10
        )
        result["version"] = version.stdout.strip()
        if version.returncode:
            result["reason"] = version.stderr.strip() or "perf --version failed"
            return result
    except (OSError, subprocess.TimeoutExpired) as error:
        result["reason"] = str(error)
        return result
    try:
        capabilities = subprocess.run(
            ["getcap", resolved], capture_output=True, text=True, check=False, timeout=10
        )
        result["capabilities"] = capabilities.stdout.strip()
    except (OSError, subprocess.TimeoutExpired) as error:
        # getcap is advisory: the global policy may independently permit capture.
        result["inspection_error"] = str(error)
    policy = result.get("perf_event_paranoid")
    has_capability = "cap_perfmon" in result.get("capabilities", "")
    if os.getuid() == 0 or has_capability or (isinstance(policy, int) and policy <= 2):
        result["status"] = "ready"
        result["reason"] = "user-space policy permits an attach attempt; actual attach not tested"
    else:
        result["reason"] = "perf permission setup required (private CAP_PERFMON or policy <= 2)"
    return result


def process_identity(pid: int) -> dict[str, Any]:
    """Bind a Linux process to PID, real UID and kernel start-time ticks."""
    process = Path("/proc") / str(pid)
    stat = (process / "stat").read_text()
    # comm can contain spaces and parentheses; fields following its last ')' are stable.
    fields = stat[stat.rfind(")") + 2 :].split()
    if fields[0] in ("Z", "X"):
        raise ProcessLookupError(f"process {pid} has exited")
    uid_line = next(
        line for line in (process / "status").read_text().splitlines() if line.startswith("Uid:")
    )
    return {"pid": pid, "uid": int(uid_line.split()[1]), "start_time": fields[19]}


def identity_matches(pid: int, identity: Mapping[str, Any]) -> bool:
    try:
        current = process_identity(pid)
    except OSError, ValueError, StopIteration, IndexError:
        return False
    return current == {key: identity[key] for key in ("pid", "uid", "start_time")}


def start_sampler(
    unit_dir: Path, pid: int, identity: Mapping[str, Any], perfbin: str | None = None
) -> subprocess.Popen[bytes]:
    """Attach only to the same user's verified process; own a separate collector group."""
    if identity.get("uid") != os.getuid() or not identity_matches(pid, identity):
        raise ValueError("sampling target PID/UID/start time does not match")
    unit_dir.mkdir(parents=True, exist_ok=True)
    arguments = [
        perfbin or resolve_perf(),
        "record",
        "-e",
        "cpu-clock:u",
        "-F",
        "99",
        "--call-graph",
        "dwarf,16384",
        "--switch-output=45s",
        "-o",
        str(unit_dir / "perf.data"),
        "-p",
        str(pid),
    ]
    owner = process_identity(os.getpid())
    supervisor_arguments = [
        sys.executable,
        str(Path(__file__).resolve()),
        "_sample",
        str(unit_dir),
        json.dumps(owner),
        json.dumps(dict(identity)),
        *arguments,
    ]
    with (unit_dir / "collector.log").open("ab") as log:
        collector = Sampler(
            supervisor_arguments,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=log,
            start_new_session=True,
            close_fds=True,
        )
    collector.unit_dir = unit_dir
    try:
        write_json(
            unit_dir / "sampler.json",
            {
                "pid": collector.pid,
                "identity": process_identity(collector.pid),
                "target": dict(identity),
                "owner": owner,
                "argv": arguments,
                "phase": "running",
            },
        )
    except BaseException:
        stop_sampler(collector)
        raise
    return collector


def supervise_sampler(
    unit_dir: Path, owner: dict[str, Any], target: dict[str, Any], arguments: list[str]
) -> int:
    """Finalize perf if its observer owner dies, including SIGKILL of that owner.

    The private perf's file capabilities can clear PR_SET_PDEATHSIG at exec.
    The ordinary Python supervisor therefore monitors the exact owner identity;
    perf stays in its independently owned group and never owns the compiler.
    """
    stopped: list[int] = []

    def request_stop(signum: int, _frame: Any) -> None:
        if not stopped:
            stopped.append(signum)

    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, request_stop)
    if not identity_matches(owner["pid"], owner) or not identity_matches(target["pid"], target):
        return 1
    try:
        recorder = subprocess.Popen(arguments, close_fds=True)
    except OSError as error:
        print(f"compile profiling collector: {error}", file=sys.stderr)
        return 1
    record: dict[str, Any] = {
        "pid": os.getpid(),
        "identity": process_identity(os.getpid()),
        "recorder_pid": recorder.pid,
        "owner": owner,
        "target": target,
        "argv": arguments,
        "phase": "running",
    }
    reason = "target_exit"
    try:
        record["recorder_identity"] = process_identity(recorder.pid)
        write_json(unit_dir / "collector.json", record)
        while recorder.poll() is None:
            if stopped:
                reason = "cancelled"
                break
            if not identity_matches(owner["pid"], owner):
                reason = "owner_exit"
                break
            if not identity_matches(target["pid"], target):
                break
            time.sleep(0.1)
    except (OSError, ValueError, StopIteration, IndexError) as error:
        reason = f"observer_error: {error}"
    finally:
        if recorder.poll() is None:
            with suppress(ProcessLookupError):
                recorder.send_signal(signal.SIGINT)
            try:
                recorder.wait(timeout=10)
            except subprocess.TimeoutExpired:
                recorder.terminate()
                try:
                    recorder.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    recorder.kill()
        code = recorder.wait()
        record.update({"phase": "completed", "exit_code": code, "reason": reason})
        with suppress(OSError):
            write_json(unit_dir / "collector.json", record)
    return code


def stop_sampler(collector: subprocess.Popen[bytes]) -> int:
    """Finalize only this collector's independent process group, never its target."""
    if collector.poll() is None:
        with suppress(ProcessLookupError):
            os.killpg(collector.pid, signal.SIGINT)
        try:
            collector.wait(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(collector.pid, signal.SIGTERM)
            try:
                collector.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(collector.pid, signal.SIGKILL)
    code = collector.wait()
    unit_dir = collector.unit_dir if isinstance(collector, Sampler) else None
    if unit_dir is not None:
        with suppress(OSError, ValueError):
            metadata_path = unit_dir / "collector.json"
            metadata = json.loads(metadata_path.read_text())
            metadata.update({"phase": "completed", "exit_code": code})
            write_json(metadata_path, metadata)
    return code


def read_completed_chunks(unit_dir: Path) -> list[Path]:
    """Exclude active/open perf files; lack of proof never makes a chunk complete."""
    metadata_path = unit_dir / "collector.json"
    if not metadata_path.is_file():
        return []
    metadata = json.loads(metadata_path.read_text())
    opened: set[Path] = set()
    announced: set[Path] = set()
    # perf 7.0 emits this line after finish_output and switching/closing the old
    # file. It remains usable when CAP_PERFMON makes /proc/<collector>/fd private.
    with suppress(FileNotFoundError):
        for line in (unit_dir / "collector.log").read_text(errors="replace").splitlines():
            if line.startswith("[ perf record: Dump ") and line.endswith(" ]"):
                path = Path(line[len("[ perf record: Dump ") : -2])
                profile_root = unit_dir.parent.parent
                try:
                    meta = json.loads((profile_root / "record.json").read_text())
                except OSError, ValueError:
                    meta = {}
                original = meta.get("original_profile_root")
                if not original:
                    with suppress(OSError, ValueError):
                        original = json.loads(
                            (profile_root / "path-resolution.json").read_text()
                        ).get("original_profile_root")
                if original and path.is_absolute() and path.is_relative_to(Path(original)):
                    path = profile_root / path.relative_to(Path(original))
                if path.resolve().parent == unit_dir.resolve():
                    announced.add(path.resolve())
    pid = int(metadata.get("recorder_pid", metadata["pid"]))
    recorder_identity = metadata.get("recorder_identity", metadata["identity"])
    alive = identity_matches(pid, recorder_identity)
    fd_readable = False
    if alive:
        try:
            for descriptor in (Path("/proc") / str(pid) / "fd").iterdir():
                try:
                    opened.add(Path(os.readlink(descriptor)).resolve())
                except FileNotFoundError:
                    continue
            fd_readable = True
        except FileNotFoundError, PermissionError:
            pass
    finalized = metadata.get("phase") == "completed" and metadata.get("exit_code") == 0
    return sorted(
        path
        for path in unit_dir.glob("perf.data*")
        if path.is_file()
        and (path.name == "perf.data" or path.name.removeprefix("perf.data.").isdigit())
        and path.resolve() not in opened
        and (
            path.resolve() in announced
            or (fd_readable and path.name != "perf.data")
            or (not alive and finalized)
        )
    )


def sampler_succeeded(unit_dir: Path, exit_code: int) -> bool:
    """perf re-raises SIGINT after finalization; completed output qualifies that exit."""
    return exit_code == 0 or (exit_code == -signal.SIGINT and bool(read_completed_chunks(unit_dir)))


def expand_response_files(arguments: Sequence[str]) -> list[str]:
    """Match pinned rustc's single-pass newline argfiles; refuse enabled shell files.

    rustc_driver_impl/src/args.rs at c1070d693: argfile contents are pushed,
    not recursively expanded, and -Z shell-argfiles takes effect in argument order.
    Shell parsing is intentionally refused rather than approximated with Python shlex.
    """
    expanded: list[str] = []
    shell_enabled = False
    next_unstable = False

    def push(argument: str) -> None:
        nonlocal shell_enabled, next_unstable
        if next_unstable:
            shell_enabled |= argument == "shell-argfiles"
            next_unstable = False
        elif argument.startswith("-Z"):
            option = argument[2:]
            next_unstable = not option
            shell_enabled |= option == "shell-argfiles"
        expanded.append(argument)

    for argument in arguments:
        if not argument.startswith("@"):
            push(argument)
            continue
        name = argument[1:]
        if shell_enabled and name.startswith("shell:"):
            raise ValueError("selected rustc @shell: response files are unsupported")
        # newline='' avoids Python's universal-newline conversion of lone CR bytes.
        with Path(name).open(encoding="utf-8", newline="") as source:
            content = source.read()
        lines = content.split("\n")
        terminated = content.endswith("\n")
        if terminated:
            lines.pop()
        for index, line in enumerate(lines):
            if index < len(lines) - 1 or terminated:
                line = line.removesuffix("\r")
            if content:
                push(line)
    return expanded


def option_values(arguments: Sequence[str], option: str) -> list[str]:
    values = []
    for index, argument in enumerate(arguments):
        if argument == option and index + 1 < len(arguments):
            values.append(arguments[index + 1])
        elif argument.startswith(option + "="):
            values.append(argument[len(option) + 1 :])
    return values


def selected_invocation(arguments: Sequence[str], env: Mapping[str, str]) -> list[str] | None:
    focus = json.loads(env.get("LCTX_COMPILE_PROFILE_FOCUS", "[]"))
    if not isinstance(focus, list) or not all(isinstance(path, str) for path in focus):
        raise ValueError("LCTX_COMPILE_PROFILE_FOCUS must be a JSON list of manifest directories")
    manifest = env.get("CARGO_MANIFEST_DIR")
    if not manifest or Path(manifest).resolve() not in {Path(path).resolve() for path in focus}:
        return None
    expanded = expand_response_files(arguments)
    if any(
        arg in ("-vV", "-V", "--version", "--help", "-h", "--print") or arg.startswith("--print=")
        for arg in expanded
    ):
        return None
    crate = option_values(expanded, "--crate-name")
    if not crate or not option_values(expanded, "--out-dir"):
        return None
    if crate[-1] == "build_script_build":
        return None
    return expanded


def profile_flags(unit_dir: Path) -> list[str]:
    return [
        f"-Zself-profile={unit_dir}",
        "-Zself-profile-events=default,args,llvm",
        "-Ztime-passes",
        "-Ztime-passes-format=json",
        f"-Zdump-mono-stats={unit_dir}",
        "-Zdump-mono-stats-format=json",
    ]


def invocation_kind(arguments: Sequence[str], env: Mapping[str, str]) -> str:
    if "--test" not in arguments:
        return "normal"
    manifest = Path(env["CARGO_MANIFEST_DIR"]).resolve()
    for argument in arguments:
        if argument.startswith("-") or not argument.endswith(".rs"):
            continue
        source = Path(argument)
        if source.is_absolute():
            try:
                source = source.relative_to(manifest)
            except ValueError:
                continue
        if source.parts and source.parts[0] == "tests":
            return "integration_test"
        if source.parts and source.parts[0] == "src":
            return "unit_test"
    # Cargo uses src/ and tests/ by convention, but custom target paths remain
    # legitimate. Name correspondence identifies the ordinary library test.
    crate = option_values(arguments, "--crate-name")[-1]
    package = env.get("CARGO_PKG_NAME", "cpg-core").replace("-", "_")
    return "unit_test" if crate == package else "integration_test"


def _tee_stderr(stream: Any, sidecar: Path, timing: Path, errors: list[str]) -> None:
    try:
        with sidecar.open("wb") as log, timing.open("wb") as times:
            while data := stream.readline():
                try:
                    sys.stderr.buffer.write(data)
                    sys.stderr.buffer.flush()
                except (OSError, BrokenPipeError) as error:
                    errors.append(str(error))
                try:
                    log.write(data)
                    log.flush()
                    if not data.startswith(b"time: "):
                        continue
                    payload = data[len(b"time: ") :]
                    parsed = json.loads(payload)
                    if isinstance(parsed, dict):
                        times.write(payload)
                        times.flush()
                except ValueError:
                    pass
                except OSError as error:
                    errors.append(str(error))
    except OSError as error:
        errors.append(str(error))
        # Always drain diagnostics, even when observation storage fails.
        while data := stream.readline():
            try:
                sys.stderr.buffer.write(data)
                sys.stderr.buffer.flush()
            except OSError:
                pass
    finally:
        stream.close()


def storage_observation(unit_dir: Path) -> dict[str, Any]:
    """Bounded unit-local allocation observation; never a writer quota."""
    allocated = {}
    for path in unit_dir.iterdir():
        if path.is_file() and not path.is_symlink():
            with suppress(FileNotFoundError):
                allocated[path.name] = path.stat().st_blocks * 512
    return {
        "observed_ns": time.time_ns(),
        "free_bytes": shutil.disk_usage(unit_dir).free,
        "allocated_bytes": sum(allocated.values()),
        "writers": allocated,
        "support_limit": (
            "sampler stopping does not stop rustc self-profile, Cargo, or other writers"
        ),
    }


def capture_rustc(arguments: Sequence[str], expanded: list[str], env: dict[str, str]) -> int:
    root = Path(env["LCTX_COMPILE_PROFILE_DIR"])
    if not root.is_absolute():
        raise ValueError("LCTX_COMPILE_PROFILE_DIR must be absolute")
    floor = int(env.get("LCTX_COMPILE_PROFILE_MIN_FREE_BYTES", str(DEFAULT_MIN_FREE_BYTES)))
    if floor < 0:
        raise ValueError("LCTX_COMPILE_PROFILE_MIN_FREE_BYTES must be nonnegative")
    crate_name = option_values(expanded, "--crate-name")[-1]
    kind = invocation_kind(expanded, env)
    started = time.time_ns()
    digest = hashlib.sha256(json.dumps(list(arguments)).encode()).hexdigest()[:12]
    unit_dir = root / "units" / f"{digest}-{kind}-{os.getpid()}-{started}"
    flags = profile_flags(unit_dir)
    effective = [*arguments, *flags]
    record: dict[str, Any] = {
        "version": 1,
        "input_argv": list(arguments),
        "expanded_argv": expanded,
        "effective_argv": effective,
        "profile_flags": flags,
        "crate_name": crate_name,
        "kind": kind,
        "features": [
            value for value in option_values(expanded, "--cfg") if value.startswith("feature=")
        ],
        "out_dir": option_values(expanded, "--out-dir")[-1],
        "manifest_dir": env["CARGO_MANIFEST_DIR"],
        "wrapper_pid": os.getpid(),
        "started_at_ns": started,
        "started_monotonic_ns": time.monotonic_ns(),
        "phase": "starting",
        "status": "running",
        "exit_code": None,
        "collector": {"status": "not_run"},
        "sidecars": {
            "stderr": str(unit_dir / "rustc.stderr.log"),
            "time_passes": str(unit_dir / "time-passes.jsonl"),
            "self_profile": str(unit_dir),
            "mono_stats": str(unit_dir),
        },
    }
    try:
        unit_dir.mkdir(parents=True, exist_ok=False)
        write_json(unit_dir / "record.json", record)
    except OSError as error:
        # Observation storage must never prevent the originally requested compile.
        # Inherit ordinary streams and the original jobserver descriptors unchanged.
        print(f"compile profiling observer storage unavailable: {error}", file=sys.stderr)
        code = subprocess.call(list(arguments), env=env, close_fds=False)
        return code if code >= 0 else 128 - code
    # Cargo jobserver descriptors are already inheritable. Keep their exact numbers,
    # environment and ownership; the independently spawned perf receives no such FDs.
    compiler = subprocess.Popen(effective, env=env, stderr=subprocess.PIPE, close_fds=False)
    record.update({"pid": compiler.pid, "uid": os.getuid(), "start_time": None})
    errors: list[str] = []
    reader = threading.Thread(
        target=_tee_stderr,
        args=(
            compiler.stderr,
            unit_dir / "rustc.stderr.log",
            unit_dir / "time-passes.jsonl",
            errors,
        ),
        daemon=True,
    )
    reader.start()
    sampler: subprocess.Popen[bytes] | None = None
    cancelled: list[int] = []
    old_handlers: dict[signal.Signals, Any] = {}
    if threading.current_thread() is threading.main_thread():

        def cancel_observer(signum: int, _frame: Any) -> None:
            if not cancelled:
                cancelled.append(signum)

        for signum in (signal.SIGINT, signal.SIGTERM):
            old_handlers[signum] = signal.signal(signum, cancel_observer)
    try:
        identity = process_identity(compiler.pid)
        record.update(identity)
        record["phase"] = "running"
        if shutil.disk_usage(unit_dir).free < floor:
            record["collector"] = {"status": "blocked", "reason": "low_disk"}
        else:
            sampler = start_sampler(unit_dir, compiler.pid, identity, resolve_perf(env))
            record["collector"] = {"pid": sampler.pid, "status": "running"}
    except (OSError, ValueError, StopIteration, IndexError) as error:
        record["collector"] = {"status": "failed", "reason": str(error)}
    try:
        write_json(unit_dir / "record.json", record)
    except OSError as error:
        errors.append(str(error))
    try:
        while compiler.poll() is None:
            try:
                record["capacity_latest"] = storage_observation(unit_dir)
            except OSError as error:
                if str(error) not in errors:
                    errors.append(str(error))
            if sampler is not None and sampler.poll() is None:
                try:
                    if cancelled or errors or shutil.disk_usage(unit_dir).free < floor:
                        reason = (
                            "cancelled" if cancelled else "observer_error" if errors else "low_disk"
                        )
                        record["collector"]["reason"] = reason
                        stop_sampler(sampler)
                except OSError as error:
                    record["collector"]["reason"] = f"observer_error: {error}"
                    stop_sampler(sampler)
            time.sleep(0.1)
    finally:
        if sampler is not None:
            sampler_exit = stop_sampler(sampler)
            collector_state: dict[str, Any] = record["collector"]
            reason = collector_state.get("reason")
            try:
                finalized = sampler_succeeded(unit_dir, sampler_exit)
            except (OSError, ValueError) as error:
                finalized = False
                errors.append(str(error))
            sampler_status = (
                "blocked"
                if reason == "low_disk"
                else "cancelled"
                if reason == "cancelled"
                else "failed"
                if reason == "observer_error" or not finalized
                else "passed"
            )
            collector_state.update(
                {
                    "exit_code": sampler_exit,
                    "status": sampler_status,
                }
            )
            try:
                collector_record = json.loads((unit_dir / "collector.json").read_text())
                collector_record.update({"phase": "completed", "exit_code": sampler_exit})
                write_json(unit_dir / "collector.json", collector_record)
            except (OSError, ValueError) as error:
                errors.append(str(error))
        for signum, handler in old_handlers.items():
            signal.signal(signum, handler)
    exit_code = compiler.wait()
    reader.join()
    record.update(
        {
            "phase": "completed",
            "exit_code": exit_code,
            "status": "passed" if exit_code == 0 else "failed",
            "finished_at_ns": time.time_ns(),
            "observer_errors": errors,
        }
    )
    try:
        write_json(unit_dir / "record.json", record)
    except OSError as error:
        print(f"compile profiling observer: {error}", file=sys.stderr)
    return exit_code if exit_code >= 0 else 128 - exit_code


def wrapper_main(argv: Sequence[str] | None = None, env: Mapping[str, str] | None = None) -> int:
    arguments = list(sys.argv[1:] if argv is None else argv)
    environment = dict(os.environ if env is None else env)
    if not arguments:
        print("compile profiling wrapper: missing rustc argument", file=sys.stderr)
        return 2
    try:
        expanded = selected_invocation(arguments[1:], environment)
        if expanded is not None:
            return capture_rustc(arguments, expanded, environment)
    except (OSError, ValueError) as error:
        print(f"compile profiling wrapper: {error}", file=sys.stderr)
        return 2
    delegate = environment.get("LCTX_COMPILE_PROFILE_DELEGATE") or shutil.which("sccache")
    if not delegate:
        print("compile profiling wrapper: original sccache delegate missing", file=sys.stderr)
        return 2
    os.execvpe(delegate, [delegate, *arguments], environment)
    return 2


if __name__ == "__main__":
    if len(sys.argv) >= 7 and sys.argv[1] == "_sample":
        result = supervise_sampler(
            Path(sys.argv[2]), json.loads(sys.argv[3]), json.loads(sys.argv[4]), sys.argv[5:]
        )
        if result < 0:
            if -result not in (signal.SIGKILL, signal.SIGSTOP):
                signal.signal(-result, signal.SIG_DFL)
            os.kill(os.getpid(), -result)
        raise SystemExit(result)
    raise SystemExit("compile_profile_capture is invoked by the profiling wrapper")
