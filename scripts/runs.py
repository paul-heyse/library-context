"""Run handles for long commands: one record per run that later commands and sessions can find.

Standard library only (plan §5.8). `just run`/`just runs` launch it as
`uv run --no-project --offline --no-python-downloads python scripts/runs.py …`.

    run [--background] [--label L] -- <cmd…>
    list | status ID | logs ID [--follow] [--tail N] | cancel ID | retain ID [--release]
    prune [--keep N] [--older-than D] [--dry-run]

Each run owns `build/runs/<UTC-stamp>-<random>/`:

- `record.json`: written only by this script. While the launcher (the record's `owner`) lives it
  is the only writer; `cancel` finalizes a run whose owner has died.
- `output.log`: the command's stdout and stderr. The command writes it directly, so output
  survives the launcher.
- `progress.json`: consumer-written `current_command`/`waiting_reason` (`set_progress`); the
  owner folds it into the record.
- `summary.json`: consumer-owned (verify), never written here.
- `cancel.json`, `retain`: requests and markers beside the record, so nothing races its owner.
  The owner honours `cancel.json` itself, so cancellation also reaches it from another pid
  namespace.
- `owner.lock`: flocked by the owner for its lifetime; liveness across pid namespaces.

Runtime background/wait/cancel facilities come first; this adds only what crosses tool calls and
sessions. A run's state is derived, never trusted from a stale file: `running` while the owner is
alive, `interrupted` when the owner is dead without a terminal record, otherwise the record's
`termination`.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import re
import secrets
import select
import shutil
import signal
import subprocess
import sys
import time
from collections.abc import Callable, Sequence
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from harness import (
    TERMINATIONS,
    ProcessIdentity,
    group_members,
    hold_lock,
    lock_held,
    read_json,
    signal_group,
    write_json_atomic,
)

ROOT = Path(__file__).resolve().parent.parent
SCHEMA = 1
RECORD = "record.json"
OUTPUT = "output.log"
LAUNCHER = "launcher.log"
PROGRESS = "progress.json"
SUMMARY = "summary.json"
CANCEL = "cancel.json"
RETAIN = "retain"
OWNER_LOCK = "owner.lock"
PROGRESS_FIELDS = ("current_command", "waiting_reason")
DEFAULT_KEEP = 20
POLL = 0.1
# Launcher signals: SIGINT/SIGTERM are someone stopping the run; a hangup is the session going away.
LAUNCHER_SIGNALS = {
    signal.SIGINT: "cancelled",
    signal.SIGTERM: "cancelled",
    signal.SIGHUP: "interrupted",
}
_UNSET: Any = object()


def runs_root() -> Path:
    """`build/runs` under the checkout. `LCTX_RUNS_ROOT` relocates it for isolated tests."""
    override = os.environ.get("LCTX_RUNS_ROOT")
    return Path(override) if override else ROOT / "build" / "runs"


def now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def _parse_time(stamp: str | None) -> datetime | None:
    return None if stamp is None else datetime.fromisoformat(stamp)


# ---------------------------------------------------------------------------------------------
# Records and derived state


def load_record(run_dir: Path) -> dict[str, Any] | None:
    return read_json(run_dir / RECORD)


def owner_of(record: dict[str, Any]) -> ProcessIdentity:
    return ProcessIdentity.from_json(record["owner"])


def owner_alive(run_dir: Path, record: dict[str, Any]) -> bool:
    return owner_of(record).alive() or lock_held(run_dir / OWNER_LOCK)


def state_of(run_dir: Path, record: dict[str, Any] | None) -> str:
    """`running` while the owner lives, `interrupted` when it died without a terminal record,
    otherwise the record's termination; `unknown` before the first record is written."""
    if record is None:
        return "unknown"
    if record.get("termination"):
        return record["termination"]
    return "running" if owner_alive(run_dir, record) else "interrupted"


def all_runs() -> list[Path]:
    """Run directories, newest first (the UTC stamp sorts)."""
    root = runs_root()
    if not root.is_dir():
        return []
    return sorted((d for d in root.iterdir() if d.is_dir()), key=lambda d: d.name, reverse=True)


class RunNotFound(LookupError):
    pass


def resolve(ref: str) -> Path:
    """An exact id, `last`, a unique id prefix or random suffix, or a label (its newest run)."""
    runs = all_runs()
    if ref in ("last", "latest"):
        if not runs:
            raise RunNotFound("no runs")
        return runs[0]
    exact = [d for d in runs if d.name == ref]
    if exact:
        return exact[0]
    matches = [d for d in runs if d.name.startswith(ref) or d.name.endswith(f"-{ref}")]
    if len(matches) == 1:
        return matches[0]
    if not matches:
        labelled = [d for d in runs if (load_record(d) or {}).get("label") == ref]
        if labelled:
            return labelled[0]  # the newest run with that label
        raise RunNotFound(f"no run matches {ref!r}")
    raise RunNotFound(f"{ref!r} is ambiguous: {', '.join(d.name for d in matches[:5])}")


def child_survivors(record: dict[str, Any]) -> list[int]:
    """Live members of the run's own command group; empty when it is gone or not provably ours."""
    child = record.get("child")
    if not child:
        return []
    identity = ProcessIdentity.from_json(child)
    if identity.foreign():
        return []
    members = group_members(int(child["pgid"]))
    if identity.pid in members and not identity.alive():
        return []  # the pgid now leads someone else's group
    return members


def view(run_dir: Path) -> dict[str, Any]:
    """The record as last written, the live progress overlaid, and the derived state."""
    record = load_record(run_dir)
    state = state_of(run_dir, record)
    data: dict[str, Any] = dict(record or {"id": run_dir.name})
    data["state"] = state
    data["dir"] = str(run_dir)
    if record is not None and state == "running":
        progress = read_json(run_dir / PROGRESS) or {}
        for field in PROGRESS_FIELDS:
            if field in progress:
                data[field] = progress[field]
    if record is not None and state in ("running", "interrupted"):
        data["survivors"] = child_survivors(record)
    data["retained"] = (run_dir / RETAIN).exists()
    summary = run_dir / SUMMARY
    data["summary"] = str(summary) if summary.exists() else None
    return data


# ---------------------------------------------------------------------------------------------
# Consumer API (verify and other commands running inside a run)


def current_run() -> Path | None:
    """`LCTX_RUN_DIR` when this process runs inside a live run, else None."""
    value = os.environ.get("LCTX_RUN_DIR")
    if not value:
        return None
    run_dir = Path(value)
    record = load_record(run_dir)
    if record is None or record.get("termination") or not owner_alive(run_dir, record):
        return None
    return run_dir


def set_progress(
    *, current_command: str | None = _UNSET, waiting_reason: str | None = _UNSET
) -> bool:
    """Publish what the run is doing now (None clears a field). False outside a live run."""
    run_dir = current_run()
    if run_dir is None:
        return False
    path = run_dir / PROGRESS
    progress = read_json(path) or {}
    if current_command is not _UNSET:
        progress["current_command"] = current_command
    if waiting_reason is not _UNSET:
        progress["waiting_reason"] = waiting_reason
    progress["updated"] = now()
    write_json_atomic(path, progress)
    return True


def summary_path() -> Path | None:
    """Where a consumer writes `summary.json` for the current run, if any."""
    run_dir = current_run()
    return None if run_dir is None else run_dir / SUMMARY


# ---------------------------------------------------------------------------------------------
# Owning a run

PR_SET_PDEATHSIG = 1


def _death_tie(parent: int) -> Callable[[], None]:
    """preexec_fn: SIGTERM the command when its owner dies, however the owner dies (a SIGKILLed
    foreground launcher included). Runs in the forked child of the single-threaded owner."""

    def apply() -> None:
        libc = ctypes.CDLL(None, use_errno=True)
        if libc.prctl(PR_SET_PDEATHSIG, signal.SIGTERM, 0, 0, 0) != 0:
            os._exit(126)
        if os.getppid() != parent:  # the owner died before the tie took effect
            os._exit(125)

    return apply


def spawn_tied(
    argv: Sequence[str], *, env: dict[str, str], cwd: str, stdout: Any
) -> subprocess.Popen:
    """The command as leader of its own session (pgid == pid, for group signalling), tied to
    this owner by a parent-death signal. The leader is responsible for its group on SIGTERM."""
    return subprocess.Popen(
        list(argv),
        env=env,
        cwd=cwd,
        stdin=subprocess.DEVNULL,
        stdout=stdout,
        stderr=subprocess.STDOUT,
        start_new_session=True,
        preexec_fn=_death_tie(os.getpid()),
    )


def new_run_dir() -> Path:
    root = runs_root()
    root.mkdir(parents=True, exist_ok=True)
    moment = datetime.now(UTC)
    stamp = moment.strftime("%Y%m%dT%H%M%S.") + f"{moment.microsecond // 1000:03d}Z"
    while True:
        run_dir = root / f"{stamp}-{secrets.token_hex(3)}"
        try:
            run_dir.mkdir()
            return run_dir
        except FileExistsError:
            continue


def _exit_of(returncode: int) -> dict[str, Any]:
    if returncode >= 0:
        return {"code": returncode}
    try:
        name = signal.Signals(-returncode).name
    except ValueError:
        name = None
    return {"signal": -returncode, "name": name}


def _status_of(exit_: dict[str, Any] | None) -> int:
    if not exit_:
        return 1
    return exit_["code"] if "code" in exit_ else 128 + int(exit_["signal"])


class Owner:
    """The launcher side of one run: spawns the command group, folds progress, finalizes."""

    def __init__(
        self,
        run_dir: Path,
        argv: Sequence[str],
        *,
        label: str | None,
        cwd: str,
        mode: str,
        stream: bool,
    ) -> None:
        self.dir = run_dir
        self.argv = list(argv)
        self.stream = stream
        self.received: signal.Signals | None = None
        self.record: dict[str, Any] = {
            "schema": SCHEMA,
            "id": run_dir.name,
            "argv": self.argv,
            "cwd": cwd,
            "label": label,
            "mode": mode,
            "owner": ProcessIdentity.of().to_json(),
            "child": None,
            "phase": "starting",
            "current_command": None,
            "waiting_reason": None,
            "started": now(),
            "ended": None,
            "exit": None,
            "termination": None,
            "logs": {"output": str(run_dir / OUTPUT), "launcher": str(run_dir / LAUNCHER)},
        }
        self._progress_seen: Any = None

    def write(self) -> None:
        write_json_atomic(self.dir / RECORD, self.record)

    def _on_signal(self, signum: int, _frame: Any) -> None:
        if self.received is None:
            self.received = signal.Signals(signum)

    def _fold_progress(self) -> bool:
        progress = read_json(self.dir / PROGRESS)
        if progress is None or progress == self._progress_seen:
            return False
        self._progress_seen = progress
        changed = False
        for field in PROGRESS_FIELDS:
            if field in progress and self.record[field] != progress[field]:
                self.record[field] = progress[field]
                changed = True
        return changed

    def run(self, ready: Callable[[str], None] = lambda _message: None) -> int:
        for signum in LAUNCHER_SIGNALS:
            signal.signal(signum, self._on_signal)
        # Held (close-on-exec, so never by the command) until this process exits.
        self._lock_fd = hold_lock(self.dir / OWNER_LOCK)
        self.write()
        env = dict(os.environ)
        env["LCTX_RUN_ID"] = self.record["id"]
        env["LCTX_RUN_DIR"] = str(self.dir)
        output_path = self.dir / OUTPUT
        with open(output_path, "ab") as output, open(output_path, "rb") as tail:
            try:
                child = spawn_tied(
                    self.argv,
                    env=env,
                    cwd=self.record["cwd"],
                    stdout=output,
                )
            except OSError as error:
                code = 127 if isinstance(error, FileNotFoundError) else 126
                output.write(f"runs: cannot start {self.argv[0]!r}: {error}\n".encode())
                output.flush()
                self.record.update(
                    phase="ended",
                    ended=now(),
                    exit={"code": code},
                    termination="completed",
                    launch_error=str(error),
                )
                self.write()
                self._drain(tail)
                ready(f"error {code}")
                return code
            self.record["child"] = {**ProcessIdentity.of(child.pid).to_json(), "pgid": child.pid}
            self.record["phase"] = "running"
            self.write()
            ready("ok")
            returncode = self._own(child, tail)
        return self._finalize(returncode)

    def _own(self, child: subprocess.Popen, tail: Any) -> int:
        handled = False
        while True:
            returncode = child.poll()
            self._drain(tail)
            if self._fold_progress():
                self.write()
            if returncode is not None:
                break
            stop = self.received is not None or (self.dir / CANCEL).exists()
            if stop and not handled:
                handled = True
                self.record["phase"] = "stopping"
                self.write()
                signal_group(child.pid)
                continue
            time.sleep(POLL)
        # The leader is gone; anything left in its group is an owned leak.
        survivors = group_members(child.pid)
        if survivors:
            self.record["survivors_terminated"] = survivors
            signal_group(child.pid, grace=5.0)
        self._drain(tail)
        return returncode

    def _drain(self, tail: Any) -> None:
        if not self.stream:
            return
        chunk = tail.read()
        if not chunk:
            return
        try:
            sys.stdout.buffer.write(chunk)
            sys.stdout.buffer.flush()
        except BrokenPipeError, ValueError:
            self.stream = False

    def _finalize(self, returncode: int) -> int:
        self._fold_progress()
        cancel = read_json(self.dir / CANCEL)
        if cancel is not None:
            termination = "cancelled"
            self.record["cancel"] = cancel
        elif self.received is not None:
            termination = LAUNCHER_SIGNALS[self.received]
            self.record["cancel"] = {"via": f"{self.received.name} to the launcher"}
        else:
            termination = "completed"
        assert termination in TERMINATIONS
        self.record.update(
            phase="ended", ended=now(), exit=_exit_of(returncode), termination=termination
        )
        self.write()
        if self.received is not None and returncode >= 0:
            return 128 + int(self.received)
        return _status_of(self.record["exit"])


# ---------------------------------------------------------------------------------------------
# Commands


def _nested_target() -> Path | None:
    """Reuse the enclosing run only while its owner is alive; otherwise forget stale variables."""
    run_dir = current_run()
    if run_dir is None:
        os.environ.pop("LCTX_RUN_ID", None)
        os.environ.pop("LCTX_RUN_DIR", None)
    return run_dir


def cmd_run(args: argparse.Namespace) -> int:
    argv = list(args.command)
    if argv and argv[0] == "--":
        argv = argv[1:]
    if not argv:
        print("runs run: give a command after --", file=sys.stderr)
        return 2
    enclosing = _nested_target()
    if enclosing is not None:
        note = " (--background ignored)" if args.background else ""
        print(f"runs: inside run {enclosing.name}; running there{note}", file=sys.stderr)
        sys.stderr.flush()
        try:
            os.execvp(argv[0], argv)
        except FileNotFoundError as error:
            print(f"runs: cannot start {argv[0]!r}: {error}", file=sys.stderr)
            return 127
        except OSError as error:
            print(f"runs: cannot start {argv[0]!r}: {error}", file=sys.stderr)
            return 126
    run_dir = new_run_dir()
    cwd = os.getcwd()
    if not args.background:
        owner = Owner(run_dir, argv, label=args.label, cwd=cwd, mode="foreground", stream=True)
        print(f"runs: {run_dir.name}  ({run_dir / OUTPUT})", file=sys.stderr)
        sys.stderr.flush()
        return owner.run()
    return _launch_background(run_dir, argv, args)


def _launch_background(run_dir: Path, argv: list[str], args: argparse.Namespace) -> int:
    read_end, write_end = os.pipe()
    command = [sys.executable, str(Path(__file__).resolve()), "_own", "--dir", str(run_dir)]
    command += ["--ready-fd", str(write_end)]
    if args.label is not None:
        command += ["--label", args.label]
    command += ["--", *argv]
    with open(run_dir / LAUNCHER, "ab") as launcher_log:
        subprocess.Popen(
            command,
            cwd=os.getcwd(),
            stdin=subprocess.DEVNULL,
            stdout=launcher_log,
            stderr=subprocess.STDOUT,
            pass_fds=(write_end,),
            start_new_session=True,
        )
    os.close(write_end)
    message = b""
    deadline = time.monotonic() + 30.0
    with os.fdopen(read_end, "rb", buffering=0) as ready:
        while time.monotonic() < deadline:
            readable, _, _ = select.select([ready], [], [], deadline - time.monotonic())
            if not readable:
                break
            chunk = ready.read(256)
            if not chunk:
                break
            message += chunk
    text = message.decode().strip()
    info = {
        "id": run_dir.name,
        "dir": str(run_dir),
        "output": str(run_dir / OUTPUT),
        "launcher": str(run_dir / LAUNCHER),
    }
    if args.json:
        print(json.dumps({**info, "started": text == "ok", "launch": text or "no answer"}))
    else:
        print(f"run {run_dir.name} (background)")
        print(f"  dir     {run_dir}")
        print(f"  output  {run_dir / OUTPUT}")
        print(f"  status  just runs status {run_dir.name}")
    if text == "ok":
        return 0
    if text.startswith("error "):
        print(f"runs: launch failed ({text}); see {run_dir / OUTPUT}", file=sys.stderr)
        return int(text.split()[1])
    print(f"runs: the launcher did not confirm a start; see {run_dir / LAUNCHER}", file=sys.stderr)
    return 1


def cmd_own(args: argparse.Namespace) -> int:
    """Internal: the detached launcher of a background run."""
    argv = list(args.command)
    if argv and argv[0] == "--":
        argv = argv[1:]
    ready_fd = args.ready_fd

    def ready(message: str) -> None:
        nonlocal ready_fd
        if ready_fd is None:
            return
        try:
            os.write(ready_fd, message.encode())
            os.close(ready_fd)
        except OSError:
            pass
        ready_fd = None

    owner = Owner(
        Path(args.dir), argv, label=args.label, cwd=os.getcwd(), mode="background", stream=False
    )
    return owner.run(ready)


def _age(stamp: str | None, until: str | None = None) -> str:
    start = _parse_time(stamp)
    if start is None:
        return "-"
    end = _parse_time(until) or datetime.now(UTC)
    seconds = int((end - start).total_seconds())
    if seconds < 60:
        return f"{seconds}s"
    if seconds < 3600:
        return f"{seconds // 60}m{seconds % 60:02d}s"
    if seconds < 86400:
        return f"{seconds // 3600}h{seconds % 3600 // 60:02d}m"
    return f"{seconds // 86400}d{seconds % 86400 // 3600:02d}h"


def _exit_text(exit_: dict[str, Any] | None) -> str:
    if not exit_:
        return "-"
    if "code" in exit_:
        return f"exit {exit_['code']}"
    return f"signal {exit_.get('name') or exit_['signal']}"


def _describe(data: dict[str, Any]) -> str:
    if data.get("label"):
        return str(data["label"])
    return " ".join(data.get("argv") or [])


def cmd_list(args: argparse.Namespace) -> int:
    views = [view(d) for d in all_runs()]
    if args.json:
        print(json.dumps(views, indent=2))
        return 0
    if not views:
        print("no runs")
        return 0
    for data in views:
        duration = _age(data.get("started"), data.get("ended"))
        text = _describe(data)
        if len(text) > 60:
            text = text[:57] + "..."
        print(
            f"{data['id']}  {data['state']:<11} {_exit_text(data.get('exit')):<14} "
            f"{duration:>7}  {text}"
        )
    return 0


def cmd_status(args: argparse.Namespace) -> int:
    data = view(resolve(args.id))
    if args.json:
        print(json.dumps(data, indent=2))
        return 0
    print(f"{data['id']}  {data['state']}" + ("  (retained)" if data["retained"] else ""))
    rows = [
        ("label", data.get("label")),
        ("argv", " ".join(data.get("argv") or [])),
        ("cwd", data.get("cwd")),
        ("started", data.get("started")),
        ("ended", data.get("ended")),
        ("duration", _age(data.get("started"), data.get("ended"))),
        ("phase", data.get("phase")),
        ("command", data.get("current_command")),
        ("waiting", data.get("waiting_reason")),
        ("exit", _exit_text(data.get("exit")) if data.get("exit") else None),
    ]
    owner = data.get("owner")
    if owner:
        rows.append(("owner", f"pid {owner['pid']}"))
    child = data.get("child")
    if child:
        rows.append(("child", f"pid {child['pid']} pgid {child['pgid']}"))
    if "survivors" in data:
        rows.append(("members", " ".join(map(str, data["survivors"])) or "none"))
    cancel_ = data.get("cancel")
    if cancel_:
        by = (cancel_.get("by") or {}).get("pid")
        rows.append(("cancel", cancel_.get("via") or f"{cancel_.get('requested')} by pid {by}"))
    rows.append(("output", (data.get("logs") or {}).get("output")))
    rows.append(("summary", data.get("summary")))
    for key, value in rows:
        if value not in (None, ""):
            print(f"  {key:<9}{value}")
    if data["state"] == "interrupted":
        print("  the launcher died without finishing the record; `runs cancel` stops survivors")
        print("  and finalizes it")
    return 0


def cmd_logs(args: argparse.Namespace) -> int:
    run_dir = resolve(args.id)
    path = run_dir / OUTPUT
    out = sys.stdout.buffer
    try:
        with open(path, "rb") as handle:
            if args.tail is not None:
                lines = handle.read().splitlines(keepends=True)
                out.write(b"".join(lines[-args.tail :]))
            else:
                shutil.copyfileobj(handle, out)
            out.flush()
            while args.follow:
                chunk = handle.read()
                if chunk:
                    out.write(chunk)
                    out.flush()
                    continue
                if state_of(run_dir, load_record(run_dir)) != "running":
                    out.write(handle.read())
                    out.flush()
                    break
                time.sleep(0.2)
    except FileNotFoundError:
        print(f"runs: {run_dir.name} has no output yet", file=sys.stderr)
        return 1
    except BrokenPipeError:
        pass
    return 0


def _wait_terminal(run_dir: Path, timeout: float) -> dict[str, Any] | None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        record = load_record(run_dir)
        if record is not None and record.get("termination"):
            return record
        if record is not None and not owner_alive(run_dir, record):
            return record
        time.sleep(0.05)
    return load_record(run_dir)


def cancel(run_dir: Path, *, grace: float = 10.0) -> dict[str, Any]:
    """Stop one run's own command group; other runs are untouched. Returns the resulting view."""
    record = load_record(run_dir)
    state = state_of(run_dir, record)
    if record is None or record.get("termination") or state not in ("running", "interrupted"):
        return {**view(run_dir), "cancel_result": f"not cancelled: already {state}"}
    survivors = child_survivors(record)
    if state == "running":
        write_json_atomic(
            run_dir / CANCEL, {"requested": now(), "by": ProcessIdentity.of().to_json()}
        )
        if survivors:
            signal_group(int(record["child"]["pgid"]), grace=grace)
        final = _wait_terminal(run_dir, grace + 15.0)
        if final is not None and final.get("termination"):
            return {**view(run_dir), "cancel_result": "cancelled"}
        if final is not None and owner_alive(run_dir, final):
            return {**view(run_dir), "cancel_result": "signalled; the owner has not finalized"}
        record = load_record(run_dir) or record  # owner died meanwhile: finalize below
        survivors = child_survivors(record)
    # Interrupted: the owner is dead, so this command finalizes the record.
    stopped = signal_group(int(record["child"]["pgid"]), grace=grace) if survivors else True
    record.update(
        phase="ended",
        ended=now(),
        termination="cancelled" if survivors else "interrupted",
        cancel={
            "requested": now(),
            "by": ProcessIdentity.of().to_json(),
            "owner_dead": True,
            "survivors_signalled": survivors,
            "stopped": stopped,
        },
    )
    write_json_atomic(run_dir / RECORD, record)
    return {**view(run_dir), "cancel_result": record["termination"]}


def cmd_cancel(args: argparse.Namespace) -> int:
    data = cancel(resolve(args.id), grace=args.grace)
    if args.json:
        print(json.dumps(data, indent=2))
    else:
        exit_ = f", {_exit_text(data.get('exit'))}" if data.get("exit") else ""
        print(f"{data['id']}  {data['cancel_result']}  (state {data['state']}{exit_})")
    return 0 if data.get("termination") else 1


def cmd_retain(args: argparse.Namespace) -> int:
    run_dir = resolve(args.id)
    marker = run_dir / RETAIN
    if args.release:
        marker.unlink(missing_ok=True)
    else:
        marker.write_text(now() + "\n")
    print(f"{run_dir.name}  {'released' if args.release else 'retained'}")
    return 0


_DURATION = re.compile(r"(\d+)([smhdw])")
_UNITS = {"s": 1, "m": 60, "h": 3600, "d": 86400, "w": 604800}


def parse_duration(text: str) -> float:
    parts = _DURATION.findall(text)
    if not parts or "".join(n + u for n, u in parts) != text:
        raise argparse.ArgumentTypeError(f"duration like 30m, 12h, 7d or 1d12h: {text!r}")
    return float(sum(int(n) * _UNITS[u] for n, u in parts))


def prune_candidates(keep: int | None, older_than: float | None) -> list[Path]:
    """Completed (terminal-record), unretained runs beyond the newest `keep` and older than the
    cutoff. Running, interrupted, unknown and retained runs are never candidates."""
    if keep is None and older_than is None:
        keep = DEFAULT_KEEP
    completed = []
    for run_dir in all_runs():
        record = load_record(run_dir)
        if record is None or not record.get("termination") or (run_dir / RETAIN).exists():
            continue
        completed.append((run_dir, record))
    selected = completed[keep or 0 :] if keep is not None else completed
    if older_than is not None:
        cutoff = datetime.now(UTC).timestamp() - older_than
        selected = [
            (d, r)
            for d, r in selected
            if (_parse_time(r.get("ended") or r.get("started")) or datetime.now(UTC)).timestamp()
            < cutoff
        ]
    return [d for d, _ in selected]


def cmd_prune(args: argparse.Namespace) -> int:
    doomed = prune_candidates(args.keep, args.older_than)
    if not args.dry_run:
        for run_dir in doomed:
            shutil.rmtree(run_dir)
    if args.json:
        print(json.dumps({"removed": [d.name for d in doomed], "dry_run": args.dry_run}))
    else:
        verb = "would remove" if args.dry_run else "removed"
        print(f"{verb} {len(doomed)} completed run(s)")
        for run_dir in doomed:
            print(f"  {run_dir.name}")
    return 0


def parser() -> argparse.ArgumentParser:
    top = argparse.ArgumentParser(prog="runs", description=__doc__.split("\n\n")[0])
    sub = top.add_subparsers(dest="action")

    run = sub.add_parser("run", help="run a command under a run handle")
    run.add_argument("--background", action="store_true", help="return once it has started")
    run.add_argument("--label")
    run.add_argument("--json", action="store_true")
    run.add_argument("command", nargs=argparse.REMAINDER)
    run.set_defaults(handler=cmd_run)

    own = sub.add_parser("_own")
    own.add_argument("--dir", required=True)
    own.add_argument("--ready-fd", type=int)
    own.add_argument("--label")
    own.add_argument("command", nargs=argparse.REMAINDER)
    own.set_defaults(handler=cmd_own)

    listing = sub.add_parser("list", help="runs, newest first")
    listing.add_argument("--json", action="store_true")
    listing.set_defaults(handler=cmd_list)

    status = sub.add_parser("status", help="one run's derived state and record")
    status.add_argument("id", help="id, unique prefix or suffix, or `last`")
    status.add_argument("--json", action="store_true")
    status.set_defaults(handler=cmd_status)

    logs = sub.add_parser("logs", help="print a run's output")
    logs.add_argument("id")
    logs.add_argument("--follow", "-f", action="store_true", help="until the run stops")
    logs.add_argument("--tail", type=int, metavar="N")
    logs.set_defaults(handler=cmd_logs)

    cancel_ = sub.add_parser("cancel", help="stop a run's command group")
    cancel_.add_argument("id")
    cancel_.add_argument("--grace", type=float, default=10.0, help="seconds before SIGKILL")
    cancel_.add_argument("--json", action="store_true")
    cancel_.set_defaults(handler=cmd_cancel)

    retain = sub.add_parser("retain", help="keep a run from pruning")
    retain.add_argument("id")
    retain.add_argument("--release", action="store_true")
    retain.set_defaults(handler=cmd_retain)

    prune = sub.add_parser("prune", help="remove completed, unretained runs")
    prune.add_argument(
        "--keep", type=int, metavar="N", help=f"newest completed kept (default {DEFAULT_KEEP})"
    )
    prune.add_argument("--older-than", type=parse_duration, metavar="D", help="e.g. 7d, 12h")
    prune.add_argument("--dry-run", action="store_true")
    prune.add_argument("--json", action="store_true")
    prune.set_defaults(handler=cmd_prune)
    return top


def main(argv: Sequence[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if args.action is None:
        args = parser().parse_args(["list"])
    try:
        return args.handler(args)
    except RunNotFound as error:
        print(f"runs: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
