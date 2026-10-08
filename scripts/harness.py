"""Process identity, process groups, outcomes and atomic records shared by harness scripts.

Standard library only (plan §5.8). Liveness is judged from an identity record: pid, kernel start
ticks, boot id and pid namespace. A reused pid, a reboot or another pid namespace never looks
alive. A foreign identity is never signalled.
"""

from __future__ import annotations

import fcntl
import json
import os
import signal
import subprocess
import tempfile
import time
from collections.abc import Mapping, Sequence
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

OUTCOMES = ("passed", "failed", "blocked", "not_run")
TERMINATIONS = ("completed", "interrupted", "cancelled")


def boot_id() -> str:
    return Path("/proc/sys/kernel/random/boot_id").read_text().strip()


def pid_namespace(pid: int | str = "self") -> int:
    return os.stat(f"/proc/{pid}/ns/pid").st_ino


def _stat_fields(pid: int) -> list[str] | None:
    """Fields after the parenthesised command name: [state, ppid, pgrp, …]."""
    try:
        raw = Path(f"/proc/{pid}/stat").read_text()
    except OSError:
        return None
    return raw[raw.rindex(")") + 2 :].split()


@dataclass(frozen=True)
class ProcessIdentity:
    pid: int
    start_ticks: int
    boot_id: str
    pid_namespace: int

    @classmethod
    def of(cls, pid: int | None = None) -> ProcessIdentity:
        pid = os.getpid() if pid is None else pid
        fields = _stat_fields(pid)
        if fields is None:
            raise ProcessLookupError(pid)
        return cls(pid, int(fields[19]), boot_id(), pid_namespace(pid))

    def foreign(self) -> bool:
        """Recorded under another boot or pid namespace: unknowable here, never signal it."""
        return self.boot_id != boot_id() or self.pid_namespace != pid_namespace()

    def alive(self) -> bool:
        if self.foreign():
            return False
        fields = _stat_fields(self.pid)
        return fields is not None and fields[0] != "Z" and int(fields[19]) == self.start_ticks

    def to_json(self) -> dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> ProcessIdentity:
        return cls(
            int(data["pid"]),
            int(data["start_ticks"]),
            str(data["boot_id"]),
            int(data["pid_namespace"]),
        )


def spawn_group(
    argv: Sequence[str],
    *,
    env: Mapping[str, str] | None = None,
    cwd: Path | str | None = None,
    stdout: Any = None,
    stderr: Any = None,
    stdin: Any = None,
) -> subprocess.Popen:
    """Start argv as the leader of a new session, so its process group id is its pid."""
    return subprocess.Popen(
        list(argv),
        env=None if env is None else dict(env),
        cwd=cwd,
        stdin=stdin,
        stdout=stdout,
        stderr=stderr,
        start_new_session=True,
    )


def group_members(pgid: int) -> list[int]:
    """Live (non-zombie) processes in the group: the survivors of an owned command tree."""
    members = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        fields = _stat_fields(int(entry.name))
        if fields is not None and fields[0] != "Z" and int(fields[2]) == pgid:
            members.append(int(entry.name))
    return sorted(members)


def signal_group(pgid: int, *, grace: float = 10.0) -> bool:
    """SIGTERM the group, then SIGKILL survivors after one grace period. True when none remain."""
    for sig, wait in ((signal.SIGTERM, grace), (signal.SIGKILL, 5.0)):
        try:
            os.killpg(pgid, sig)
        except ProcessLookupError:
            return True
        deadline = time.monotonic() + wait
        while time.monotonic() < deadline:
            if not group_members(pgid):
                return True
            time.sleep(0.05)
    return not group_members(pgid)


def hold_lock(path: Path) -> int:
    """An exclusive flock held until this process exits (close-on-exec, never inherited)."""
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_CLOEXEC, 0o600)
    fcntl.flock(descriptor, fcntl.LOCK_EX)
    return descriptor


def lock_held(path: Path) -> bool:
    """Whether a live process holds the flock on ``path``. Locks are kernel open-file state, so
    this answers across pid namespaces where an identity cannot (plan §5.8.3), and the kernel
    drops the lock however the holder dies."""
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC)
    except OSError:
        return False
    try:
        fcntl.flock(descriptor, fcntl.LOCK_SH | fcntl.LOCK_NB)
    except BlockingIOError:
        return True
    except OSError:
        return False
    finally:
        os.close(descriptor)
    return False


def write_json_atomic(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "w") as handle:
            json.dump(data, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except BaseException:
        Path(temporary).unlink(missing_ok=True)
        raise


def read_json(path: Path) -> Any | None:
    try:
        return json.loads(path.read_text())
    except OSError, ValueError:
        return None
