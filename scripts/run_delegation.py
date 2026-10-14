"""Prospective delegated host ownership for a managed run.

The run owns admission and recovery; each registered command/writer owns its receipt.
Native effects are deliberately outside this module. No observed numeric PID alone
authorizes signalling, and old run records acquire no invented child registrations.
"""

from __future__ import annotations

import contextlib
import functools
import json
import os
import signal
import time
import uuid
from collections.abc import Mapping
from pathlib import Path
from typing import Any

from harness import (
    ProcessIdentity,
    cleanup_group,
    hold_lock,
    lock_held,
    read_json,
    recorded_group,
    try_hold_lock,
    write_json_atomic,
)

SCOPE = "delegation.json"
ENTRIES = "delegations"


def _now() -> str:
    from datetime import UTC, datetime

    return datetime.now(UTC).isoformat(timespec="seconds")


def initialize(run_dir: Path, owner: Mapping[str, Any]) -> dict[str, Any]:
    """Called by the run owner before its first command can launch."""
    value = {"schema": 1, "run": run_dir.name, "owner": dict(owner), "accepting": True}
    (run_dir / ENTRIES).mkdir(exist_ok=True)
    with _gate(run_dir):
        if (run_dir / SCOPE).exists():
            raise ValueError("run delegation scope already exists")
        write_json_atomic(run_dir / SCOPE, value)
    return {"schema": 1, "path": SCOPE}


@contextlib.contextmanager
def _gate(run_dir: Path):
    descriptor = hold_lock(run_dir / "delegation.lock")
    try:
        yield
    finally:
        os.close(descriptor)


def _scope(run_dir: Path, record: Mapping[str, Any]) -> dict[str, Any]:
    if record.get("delegation") != {"schema": 1, "path": SCOPE}:
        raise ValueError("delegation binding is unsupported")
    if run_dir.is_symlink() or (run_dir / SCOPE).is_symlink():
        raise ValueError("delegation scope is redirected")
    value = read_json(run_dir / SCOPE)
    if (
        not isinstance(value, dict)
        or value.get("schema") != 1
        or value.get("run") != run_dir.name
        or value.get("owner") != record.get("owner")
        or type(value.get("accepting")) is not bool
    ):
        raise ValueError("delegation scope differs from run owner")
    return value


class Registration:
    def __init__(self, run_dir: Path, directory: Path, row: dict, admission, gate: int | None):
        self.run_dir, self.directory, self.row = run_dir, directory, row
        self.admission, self.gate = admission, gate
        self.lock = hold_lock(directory / "owner.lock")
        self.closed = False

    def _unlock_gate(self) -> None:
        if self.gate is not None:
            os.close(self.gate)
            self.gate = None

    def started(self, child) -> None:
        # The admission gate remains held across Popen and publication. Cancellation
        # cannot close the registered set between an accepted intent and its spawn.
        self.row["child"] = {**ProcessIdentity.of(child.pid).to_json(), "pgid": child.pid}
        write_json_atomic(self.directory / "record.json", self.row)
        self._unlock_gate()

    def finish(self, cleanup: Mapping[str, Any]) -> None:
        if self.closed:
            return
        try:
            self.row.update(cleanup=dict(cleanup), ended=_now())
            write_json_atomic(self.directory / "record.json", self.row)
        finally:
            self.closed = True
            self._unlock_gate()
            os.close(self.lock)
            self.admission.__exit__(None, None, None)


def register(
    kind: str,
    *,
    context: Mapping[str, Any] | None = None,
    spawning: bool = False,
    env: Mapping[str, str] | None = None,
) -> Registration | None:
    """Register under the live caller's run, or do nothing for a bare invocation.

    A spawning registration holds admission until started()/finish(). A writer
    registration has no process group and is held until its writing has stopped.
    """
    import runs
    from storage_lifecycle import admission

    source = os.environ if env is None else env
    raw = source.get("LCTX_RUN_DIR")
    if not raw:
        return None
    run_dir = Path(raw)
    if not run_dir.is_absolute() or run_dir.is_symlink():
        raise ValueError("enclosing run path must be absolute and unredirected")
    record = runs.load_record(run_dir)
    if not isinstance(record, dict) or record.get("id") != run_dir.name:
        raise ValueError(
            "enclosing run receipt unavailable; clear LCTX_RUN_DIR for an explicit bare invocation"
        )
    try:
        parent = runs.owner_of(record)
    except (OSError, KeyError, TypeError, ValueError) as error:
        raise ValueError("enclosing run owner identity unavailable") from error
    # Legacy runs remain directly scoped; do not synthesize historical containment.
    if not record.get("delegation"):
        if (run_dir / SCOPE).exists():
            raise ValueError("enclosing run delegation binding unavailable")
        return None
    if record.get("termination") or not runs.owner_alive(run_dir, record) or not parent.alive():
        raise InterruptedError("enclosing run no longer admits delegated work")
    owned = admission([run_dir])
    owned.__enter__()
    gate = None
    registration = None
    try:
        gate = hold_lock(run_dir / "delegation.lock")
        scope = _scope(run_dir, record)
        current = runs.load_record(run_dir)
        if (
            not isinstance(current, dict)
            or current.get("owner") != record.get("owner")
            or current.get("termination")
            or not runs.owner_alive(run_dir, current)
            or not runs.owner_of(current).alive()
            or not scope["accepting"]
            or (run_dir / runs.CANCEL).exists()
        ):
            raise InterruptedError("enclosing run no longer admits delegated work")
        ident = uuid.uuid4().hex
        directory = run_dir / ENTRIES / ident
        directory.mkdir()
        row = {
            "schema": 1,
            "id": ident,
            "run": run_dir.name,
            "run_owner": record["owner"],
            "owner": ProcessIdentity.of().to_json(),
            "kind": kind,
            "context": dict(context or {}),
            "child": None,
            "spawning": spawning,
            "started": _now(),
            "ended": None,
            "cleanup": {
                "status": "unknown",
                "reason": "launch pending" if spawning else "writer active",
            },
        }
        registration = Registration(run_dir, directory, row, owned, gate)
        gate = None
        write_json_atomic(directory / "record.json", row)
        if not spawning:
            registration._unlock_gate()
        return registration
    except BaseException:
        if registration is not None:
            registration.finish({"status": "unknown", "reason": "registration publication failed"})
        else:
            if gate is not None:
                os.close(gate)
            owned.__exit__(None, None, None)
        raise


@contextlib.contextmanager
def writer_owner(
    kind: str, context: Mapping[str, Any] | None = None, env: Mapping[str, str] | None = None
):
    """A capture/profiler writer stops writing before releasing this host obligation."""
    registration = register(kind, context=context, env=env)
    try:
        yield registration
    finally:
        if registration is not None:
            registration.finish({"status": "confirmed", "survivors": []})


def delegated_writer(kind: str):
    """Adapt an existing independent writer without changing its terminal policy."""

    def decorate(function):
        @functools.wraps(function)
        def owned(*args, **kwargs):
            with writer_owner(kind):
                return function(*args, **kwargs)

        return owned

    return decorate


def close_admission(run_dir: Path, record: Mapping[str, Any]) -> None:
    if not record.get("delegation"):
        return
    with _gate(run_dir):
        scope = _scope(run_dir, record)
        if scope["accepting"]:
            write_json_atomic(run_dir / SCOPE, {**scope, "accepting": False, "closed": _now()})


def _entry(path: Path, run_dir: Path, record: Mapping[str, Any]) -> dict:
    if path.is_symlink() or (path / "record.json").is_symlink():
        raise ValueError("delegated receipt is redirected")
    row = read_json(path / "record.json")
    if (
        not isinstance(row, dict)
        or row.get("schema") != 1
        or row.get("id") != path.name
        or row.get("run") != run_dir.name
        or row.get("run_owner") != record.get("owner")
        or type(row.get("spawning")) is not bool
    ):
        raise ValueError("delegated receipt identity differs")
    ProcessIdentity.from_json(row["owner"])
    return row


def _signal_owner(owner: ProcessIdentity, lock: Path, signum: int) -> bool:
    """Pin the kernel process before revalidating its recorded identity and lease."""
    if not hasattr(os, "pidfd_open") or not hasattr(signal, "pidfd_send_signal"):
        raise OSError("kernel-pinned owner signals unavailable")
    descriptor = os.pidfd_open(owner.pid)
    try:
        if not owner.alive() or not lock_held(lock):
            return False
        signal.pidfd_send_signal(descriptor, signum)
        return True
    finally:
        os.close(descriptor)


def _observation(path: Path, row: dict, snapshot=None) -> dict:
    identity = ProcessIdentity.from_json(row["owner"])
    if identity.foreign():
        return {"status": "unknown", "reason": "foreign owner namespace"}
    if lock_held(path / "owner.lock") or (identity.alive() and not row.get("ended")):
        return {"status": "active", "owner": row["owner"]}
    if row.get("child"):
        recovery = read_json(path / "recovery.json") or {}
        members = recovery.get("members", []) or (row.get("cleanup") or {}).get("members", [])
        group = recorded_group(row["child"], members=members, snapshot=snapshot)
        if group["status"] != "gone":
            if row["kind"] == "profile-observer" and group["status"] == "owned":
                return {
                    "status": "active",
                    "reason": "independent observer finalizing",
                    "group": group,
                }
            return {"status": "unknown", "group": group}
        if (row.get("cleanup") or {}).get("status") != "confirmed" and recovery.get(
            "cleanup", {}
        ).get("status") != "confirmed":
            return {"status": "unknown", "reason": "delegated cleanup needs selected recovery"}
    elif row["spawning"] and not row.get("ended"):
        return {"status": "unknown", "reason": "launch exposure unobserved"}
    return {"status": "confirmed", "survivors": []}


def observe(
    run_dir: Path, record: Mapping[str, Any], *, ignore_self: bool = False, snapshot=None
) -> dict:
    if not record.get("delegation"):
        return {"status": "confirmed", "scope": "direct-only", "children": []}
    children = []
    stamps = []
    try:
        scope = _scope(run_dir, record)
        root = run_dir / ENTRIES
        if root.is_symlink() or not root.is_dir():
            raise ValueError("delegated receipt directory unavailable")
        for path in sorted(root.iterdir()):
            row = _entry(path, run_dir, record)
            if ignore_self and row["owner"] == ProcessIdentity.of().to_json():
                continue
            children.append(
                {"id": path.name, "kind": row["kind"], **_observation(path, row, snapshot)}
            )
            recovery = read_json(path / "recovery.json") or {}
            stamps.extend(value for value in (row.get("ended"), recovery.get("observed")) if value)
        status = (
            "active"
            if any(c["status"] == "active" for c in children)
            else "unknown"
            if any(c["status"] != "confirmed" for c in children)
            else "confirmed"
        )
        return {
            "status": status,
            "scope": "registered-host",
            "accepting": scope["accepting"],
            "children": children,
            "observed": max(stamps) if stamps else scope.get("closed"),
        }
    except (OSError, ValueError, KeyError, TypeError) as error:
        return {"status": "unknown", "reason": str(error), "children": children}


def request_stop(run_dir: Path, record: Mapping[str, Any], *, grace: float = 5.0) -> dict:
    """Close admission, request live owners, recover only authenticated dead owners.

    Recovery is a separate parent-written receipt. It never rewrites a child's
    original result. Member identities are retained before signalling the owner.
    """
    if not record.get("delegation"):
        return observe(run_dir, record)
    try:
        close_admission(run_dir, record)
        paths = sorted((run_dir / ENTRIES).iterdir())
        observers_finalizing = False
        owner_requests: dict[str, dict[str, Any]] = {}
        for path in paths:
            row = _entry(path, run_dir, record)
            prior = read_json(path / "recovery.json") or {}
            owner_key = json.dumps(row["owner"], sort_keys=True)
            if prior.get("requested_monotonic") is not None:
                old = owner_requests.get(owner_key)
                if old is None or prior["requested_monotonic"] < old["requested_monotonic"]:
                    owner_requests[owner_key] = prior
            if (
                row["kind"] == "profile-observer"
                and row.get("child")
                and recorded_group(row["child"], members=prior.get("members", []))["status"]
                == "owned"
            ):
                observers_finalizing = True
        for path in paths:
            row = _entry(path, run_dir, record)
            owner = ProcessIdentity.from_json(row["owner"])
            if owner.foreign() or row["owner"] == ProcessIdentity.of().to_json():
                continue
            recovery_lock = try_hold_lock(path / "recovery.lock")
            if recovery_lock is None:
                continue
            try:
                previous = read_json(path / "recovery.json") or {}
                owner_key = json.dumps(row["owner"], sort_keys=True)
                prior_request = owner_requests.get(owner_key)
                members = previous.get("members", [])
                group = (
                    recorded_group(row.get("child"), members=members) if row.get("child") else None
                )
                if group and group["status"] == "owned":
                    for pid in group["members"]:
                        with contextlib.suppress(OSError):
                            member = ProcessIdentity.of(pid).to_json()
                            if member not in members:
                                members.append(member)
                state: dict[str, Any] = {
                    **previous,
                    "members": members,
                    "requested": (prior_request or {}).get("requested") or _now(),
                    "requested_monotonic": (prior_request or {}).get(
                        "requested_monotonic", time.monotonic()
                    ),
                }
                owner_requests[owner_key] = state
                if prior_request is None and owner.alive() and lock_held(path / "owner.lock"):
                    write_json_atomic(path / "recovery.json", state)
                    _signal_owner(owner, path / "owner.lock", signal.SIGTERM)
                elif (
                    owner.alive()
                    and lock_held(path / "owner.lock")
                    and row["kind"] != "profile-observer"
                    and not observers_finalizing
                    and time.monotonic() - state["requested_monotonic"] >= grace
                ):
                    # A child owner that refuses its graceful request cannot keep
                    # cancellation pending forever. Its child groups remain separate
                    # obligations, recovered below only after this owner has stopped.
                    if _signal_owner(owner, path / "owner.lock", signal.SIGKILL):
                        state["owner_escalated"] = _now()
                    write_json_atomic(path / "recovery.json", state)
                elif not lock_held(path / "owner.lock") and not owner.alive():
                    group = (
                        recorded_group(row["child"], members=members) if row.get("child") else None
                    )
                    if group and group["status"] == "owned" and row["kind"] != "profile-observer":
                        state["cleanup"] = cleanup_group(row["child"]["pgid"], grace=grace)
                    elif (group and group["status"] == "gone") or (
                        not group and not row["spawning"]
                    ):
                        state["cleanup"] = {"status": "confirmed", "survivors": []}
                    else:
                        state["cleanup"] = {
                            "status": "unknown",
                            "reason": "delegated launch/group identity unavailable",
                        }
                    state["observed"] = _now()
                    write_json_atomic(path / "recovery.json", state)
            finally:
                os.close(recovery_lock)
        return observe(run_dir, record, ignore_self=True)
    except (OSError, ValueError, KeyError, TypeError) as error:
        return {"status": "unknown", "reason": str(error), "children": []}
