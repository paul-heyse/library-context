"""Process identity, process groups, outcomes and atomic records shared by harness scripts.

Standard library only (plan §5.8). Liveness is judged from an identity record: pid, kernel start
ticks, boot id and pid namespace. A reused pid, a reboot or another pid namespace never looks
alive. A foreign identity is never signalled.
"""

from __future__ import annotations

import contextlib
import ctypes
import fcntl
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable, Mapping, Sequence
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

    def previous_boot(self) -> bool:
        """Recorded before the current boot: certainly dead, so its records may be swept."""
        return self.boot_id != boot_id()

    def foreign(self) -> bool:
        """Same boot, another pid namespace (e.g. a Codex sandbox call): its liveness is unknowable
        from here, so never signal or sweep it by identity; an owner lock decides instead."""
        return not self.previous_boot() and self.pid_namespace != pid_namespace()

    def alive(self) -> bool:
        if self.previous_boot() or self.foreign():
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


PR_SET_PDEATHSIG = 1


def _death_signal(signum: int, parent: int) -> Callable[[], None]:
    """preexec_fn: deliver `signum` to the child when its spawning thread exits, however it exits
    (SIGKILL included). Runs in the forked child; spawn from the owner's main thread."""

    def apply() -> None:
        libc = ctypes.CDLL(None, use_errno=True)
        if libc.prctl(PR_SET_PDEATHSIG, signum, 0, 0, 0) != 0:
            os._exit(126)
        if os.getppid() != parent:  # the owner died before the tie took effect
            os._exit(125)

    return apply


def spawn_group(
    argv: Sequence[str],
    *,
    env: Mapping[str, str] | None = None,
    cwd: Path | str | None = None,
    stdout: Any = None,
    stderr: Any = None,
    stdin: Any = None,
    death_signal: int | None = None,
) -> subprocess.Popen:
    """Start argv as the leader of a new session, so its process group id is its pid. With
    `death_signal`, the leader also receives that signal when its owner dies."""
    return subprocess.Popen(
        list(argv),
        env=None if env is None else dict(env),
        cwd=cwd,
        stdin=stdin,
        stdout=stdout,
        stderr=stderr,
        start_new_session=True,
        preexec_fn=None if death_signal is None else _death_signal(death_signal, os.getpid()),
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


def cleanup_group(pgid: int, *, grace: float = 10.0) -> dict[str, Any]:
    """Observe cleanup of a directly owned group; an attempt alone is never confirmation."""
    identities = []
    try:
        survivors = group_members(pgid)
        for pid in survivors:
            # A disappearing member must not prevent the cleanup attempt.
            with contextlib.suppress(OSError):
                identities.append(ProcessIdentity.of(pid).to_json())
        if survivors and not signal_group(pgid, grace=grace):
            return {"status": "failed", "survivors": group_members(pgid), "members": identities}
        remaining = group_members(pgid)
        return {"status": "failed" if remaining else "confirmed", "survivors": remaining}
    except PermissionError as error:
        return {"status": "failed", "error": type(error).__name__, "members": identities}
    except Exception as error:
        return {"status": "unknown", "error": type(error).__name__, "members": identities}


def observe_exit(child: subprocess.Popen) -> int | None:
    """Observe actual child exit without reaping its leader or releasing its numeric identity.

    A SpawnGuard consumer must use this instead of Popen.poll/wait/communicate. The guard is
    the sole reaper: a waitable leader, including a zombie, pins the PID/process-group ID until
    owned-group cleanup finishes. Cached Popen.returncode is still truthful after guard exit,
    but it cannot authorize any later numeric-group cleanup.
    """
    if child.returncode is not None:
        return child.returncode
    status = os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    if status is None:
        return None
    if status.si_code == os.CLD_EXITED:
        return status.si_status
    if status.si_code in (os.CLD_KILLED, os.CLD_DUMPED):
        return -status.si_status
    raise ChildProcessError("child terminal status unavailable")


class SpawnGuard:
    """Install immediately after spawn, before identity/receipt work can fail.

    Consumers observe_exit without reaping. The guard keeps the original leader waitable
    throughout cleanup, then reaps after confirmation. Losing that pin refuses numeric cleanup;
    discovering current members alone cannot establish that a reused group is still ours.
    Historical recovery must establish identity separately before calling cleanup_group.
    """

    def __init__(self, child: subprocess.Popen, *, grace: float = 10.0) -> None:
        self.child = child
        self.pgid = child.pid
        self.grace = grace
        self.cleanup: dict[str, Any] = {"status": "unknown"}
        self.returncode: int | None = None

    def __enter__(self) -> SpawnGuard:
        return self

    def __exit__(self, *_exc: Any) -> None:
        if self.child.returncode is not None:
            self.returncode = self.child.returncode
            self.cleanup = {"status": "unknown", "reason": "leader reaped before cleanup"}
            return
        try:
            self.returncode = observe_exit(self.child)
        except OSError:
            self.cleanup = {"status": "unknown", "reason": "leader no longer waitable"}
            return
        self.cleanup = cleanup_group(self.pgid, grace=self.grace)
        if self.cleanup["status"] == "confirmed":
            self.returncode = self.child.wait()
        elif self.returncode is None:
            # Preserve the actual leader result even when descendant cleanup is unresolved;
            # leave it unreaped so no later direct-owner cleanup can follow a reused group ID.
            with contextlib.suppress(OSError):
                self.returncode = observe_exit(self.child)


def recorded_group(
    child: Mapping[str, Any] | None, *, members: Sequence[Mapping[str, Any]] = ()
) -> dict[str, Any]:
    """Observe an old group's identity without treating an unknowable group as absent.

    An extant leader (including a zombie) with matching kernel start time identifies the
    group. With an absent leader, an empty group is gone, but surviving members alone cannot
    exclude a subsequently reused group id. Foreign namespaces never authorize signalling.
    """
    if not child:
        return {"status": "unknown", "reason": "no child identity"}
    try:
        identity = ProcessIdentity.from_json(child)
        if identity.previous_boot():
            return {"status": "gone", "members": [], "reason": "previous boot"}
        if identity.foreign():
            return {"status": "unknown", "reason": "foreign pid namespace"}
        pgid = int(child["pgid"])
        if pgid != identity.pid:
            return {"status": "unknown", "reason": "group identity mismatch"}
        fields = _stat_fields(identity.pid)
        if fields is not None and int(fields[19]) != identity.start_ticks:
            return {"status": "unknown", "reason": "reused pid"}
        current_members = group_members(pgid)
        if not current_members:
            return {"status": "gone", "members": []}
        if fields is None or int(fields[2]) != pgid:
            # A surviving member recorded by the direct owner establishes uninterrupted group
            # identity after leader exit; a group id cannot be reused while that member lives.
            for member in members:
                known = ProcessIdentity.from_json(member)
                if known.pid in current_members and known.alive():
                    return {"status": "owned", "members": current_members}
            return {"status": "unknown", "reason": "group leader identity unavailable"}
        return {"status": "owned", "members": current_members}
    except OSError, ValueError, KeyError, TypeError:
        return {"status": "unknown", "reason": "child identity unavailable"}


def try_hold_lock(path: Path) -> int | None:
    """Acquire the same exclusive owner lock without waiting on a live supervisor."""
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_CLOEXEC, 0o600)
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        os.close(descriptor)
        return None
    except BaseException:
        os.close(descriptor)
        raise
    return descriptor


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


LOG_BLOCK = 64 * 1024


def tail_bytes(
    path: Path, lines: int, *, block_size: int = LOG_BLOCK, end_offset: int | None = None
) -> bytes:
    """Read only the requested LF/CRLF suffix, preserving bytes and an unfinished last line."""
    if lines < 0:
        raise ValueError("tail lines must be nonnegative")
    if block_size <= 0:
        raise ValueError("block_size must be positive")
    if lines == 0:
        return b""
    with path.open("rb") as handle:
        size = handle.seek(0, os.SEEK_END)
        position = size if end_offset is None else min(size, end_offset)
        if not position:
            return b""
        handle.seek(position - 1)
        needed = lines + (handle.read(1) == b"\n")
        blocks = []
        found = 0
        while position and found < needed:
            size = min(position, block_size)
            position -= size
            handle.seek(position)
            block = handle.read(size)
            blocks.append(block)
            found += block.count(b"\n")
        data = b"".join(reversed(blocks))
    cursor = len(data) - (data.endswith(b"\n"))
    for _ in range(lines):
        cursor = data.rfind(b"\n", 0, cursor)
        if cursor < 0:
            return data
    return data[cursor + 1 :]


def iter_file_chunks(
    path: Path,
    *,
    offset: int = 0,
    follow: bool = False,
    alive: Callable[[], bool] = lambda: True,
    block_size: int = LOG_BLOCK,
):
    """Yield bytes once from an offset; follow replacement/truncation from their new start.

    Bytes (including incomplete UTF-8 characters/lines) are forwarded intact without a text
    decoder losing a trailing fragment. A final drain occurs after the producer ends.
    """
    if offset < 0 or block_size <= 0:
        raise ValueError("offset must be nonnegative and block_size positive")
    with path.open("rb") as handle:
        handle.seek(offset)
        while True:
            chunk = handle.read(block_size)
            if chunk:
                yield chunk
                continue
            if not follow:
                return
            try:
                current = path.stat()
            except FileNotFoundError:
                current = None
            if current is not None:
                opened = os.fstat(handle.fileno())
                if (opened.st_dev, opened.st_ino) != (current.st_dev, current.st_ino):
                    # Keep the context manager's descriptor lifetime while replacing its file.
                    replacement = os.open(path, os.O_RDONLY | os.O_CLOEXEC)
                    try:
                        os.dup2(replacement, handle.fileno())
                    finally:
                        os.close(replacement)
                    handle.seek(0)
                    continue
                if current.st_size < handle.tell():
                    handle.seek(0)
                    continue
            if not alive():
                final = handle.read(block_size)
                if final:
                    yield final
                    continue
                return
            time.sleep(0.05)


class FileDisplay:
    """Best-effort file observer in its own process, never part of the product child group.

    start()/close() never raise; construction and failed display cannot determine command
    outcome. close asks for a final drain, then detaches a blocked sink by stopping the observer.
    Only the observer can block writing to a terminal; its one-block buffer cannot grow a queue.
    """

    def __init__(self, path: Path, *, sink_fd: int = 1, offset: int = 0, banner: str = "") -> None:
        self.path, self.sink_fd, self.offset, self.banner = path, sink_fd, offset, banner
        self.process: subprocess.Popen | None = None
        self.control: int | None = None

    def start(self) -> FileDisplay:
        read_end = write_end = None
        try:
            read_end, write_end = os.pipe()
            self.process = subprocess.Popen(
                [
                    sys.executable,
                    str(Path(__file__).resolve()),
                    "_display",
                    str(self.path),
                    str(read_end),
                    str(self.offset),
                    self.banner,
                ],
                stdin=subprocess.DEVNULL,
                stdout=self.sink_fd,
                stderr=subprocess.DEVNULL,
                pass_fds=(read_end,),
                start_new_session=True,
                preexec_fn=_death_signal(signal.SIGTERM, os.getpid()),
            )
            self.control = write_end
            write_end = None
        except Exception:
            pass
        finally:
            for descriptor in (read_end, write_end):
                if descriptor is not None:
                    os.close(descriptor)
        return self

    def close(self) -> None:
        try:
            if self.control is not None:
                os.close(self.control)
                self.control = None
            if self.process is not None:
                try:
                    self.process.wait(timeout=0.5)
                except subprocess.TimeoutExpired:
                    self.process.terminate()
                    try:
                        self.process.wait(timeout=0.5)
                    except subprocess.TimeoutExpired:
                        self.process.kill()
                        self.process.wait()
        except Exception:
            pass

    def __enter__(self) -> FileDisplay:
        return self.start()

    def __exit__(self, *_exc: Any) -> None:
        self.close()


def _display(path: Path, control: int, offset: int, banner: str) -> None:
    os.set_blocking(control, False)

    def alive() -> bool:
        try:
            return os.read(control, 1) != b""
        except BlockingIOError:
            return True

    def write(chunk: bytes) -> None:
        while chunk:
            written = os.write(1, chunk)
            chunk = chunk[written:]

    try:
        if banner:
            write((banner + "\n").encode())
        for chunk in iter_file_chunks(path, offset=offset, follow=True, alive=alive):
            write(chunk)
    except OSError, ValueError:
        pass


if __name__ == "__main__" and len(sys.argv) == 6 and sys.argv[1] == "_display":
    _display(Path(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), sys.argv[5])
