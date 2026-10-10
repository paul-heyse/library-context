"""Fixed storage owner adapters; runtime facts remain authoritative in their producers.

Observations contain no configuration values, command arguments or database connections.
The lifecycle manager holds admission before entering ``owner_guard`` and reobserves within it.
"""

from __future__ import annotations

import contextlib
import os
import tomllib
from collections.abc import Iterator
from contextvars import ContextVar
from pathlib import Path
from typing import Any

from harness import read_json, try_hold_lock

_GUARDED: ContextVar[frozenset[str]] = ContextVar("storage_guarded_owners", default=frozenset())


class OwnerBusy(RuntimeError):
    """An authoritative owner currently admits a writer or its facts are unavailable."""


def _result(state: str, reason: str, *, released_at: str | None = None) -> dict[str, Any]:
    return {
        "state": state,
        "active": state == "active",
        "protected": state != "released",
        "cleanup": "confirmed" if state == "released" else "unknown",
        "reason": reason,
        "reasons": [reason],
        "released_at": released_at,
    }


def owner_path(record: dict[str, Any]) -> Path:
    owner = record.get("owner") or {}
    raw = owner.get("path") or owner.get("root")
    if not isinstance(raw, str) or not Path(raw).is_absolute():
        raise ValueError("owner requires an absolute authoritative path")
    return Path(raw)


def observe(record: dict[str, Any]) -> dict[str, Any]:
    """Read only the fixed owner's release/retention and cleanup semantics; fail closed."""
    kind = (record.get("owner") or {}).get("kind")
    try:
        path = owner_path(record)
        if kind in ("run", "profile"):
            import runs

            row = runs.load_record(path)
            if row is None:
                return _result("unresolved", "run owner record unavailable")
            if (path / runs.RETAIN).exists():
                return _result("retained", "run has an explicit retention marker")
            if str(path) not in _GUARDED.get() and runs.owner_alive(path, row):
                return _result("active", "run owner is active")
            if not row.get("termination") or runs.cleanup_protected(row):
                return _result("unresolved", "run cleanup or terminality is unresolved")
            return _result(
                "released",
                "run owner confirmed cleanup",
                released_at=runs.cleanup_of(row).get("observed") or row.get("ended"),
            )
        if kind == "task":
            # Worktree source is never a generic disposable object. The explicit worktree owner
            # removes it only after checking integrations, dirty state, readers and evidence.
            if path.exists():
                selected = read_json(path / ".dev/build-dir.storage.json")
                reference = (record.get("owner") or {}).get("reference")
                if not selected or not reference or not selected.get("reference"):
                    return _result("unresolved", "task lifetime binding unavailable")
                if selected["reference"] == reference:
                    return _result("warm", "task checkout remains a current consumer")
                return _result("released", "checkout path now names a different task lifetime")
            return _result("released", "explicit task owner removed its checkout")
        if kind == "attachment":
            import surrealdb_fixture as fixture

            row = read_json(path / "record.json")
            if row is None:
                return _result("unresolved", "attachment owner record unavailable")
            if str(path) not in _GUARDED.get() and (
                fixture.lock_held(path / "owner.lock")
                or (
                    row.get("released") is not True
                    and fixture.owner_alive(row.get("owner"), path / "owner.lock")
                )
            ):
                return _result("active", "logical attachment owner is active")
            if row.get("released") is not True or fixture._cleanup_pending(row):
                return _result("unresolved", "logical attachment release or cleanup unresolved")
            if fixture._command_observation(row)["status"] != "gone":
                return _result("unresolved", "logical attachment command identity unresolved")
            return _result(
                "released",
                "local attachment released; native content unaffected",
                released_at=row.get("released_at"),
            )
        if kind == "acquisition":
            if (record.get("owner") or {}).get("reference"):
                import storage_acquisition

                return storage_acquisition.observation(
                    path,
                    record["owner"]["reference"],
                    Path(record["path"]),
                    guarded=str(path) in _GUARDED.get(),
                )
            # The Rust acquisition owner validates these inputs against installed RECORDs.
            # Its CLI permits arbitrary --envs/--sources and currently writes without storage
            # admission: a superseded pin alone cannot establish released writer/reader state.
            if not all(
                (path / name).is_file() for name in ("pyproject.toml", "uv.lock", ".python-version")
            ):
                return _result("unresolved", "acquisition project pins unavailable")
            project = tomllib.loads((path / "pyproject.toml").read_text())
            if record.get("category") == "acquired-source":
                commit = project.get("tool", {}).get("lctx", {}).get("source", {}).get("commit")
                if commit and Path(record["path"]).name == commit:
                    return _result("warm", "source is selected by the current library commit")
                return _result(
                    "unresolved",
                    "source pin superseded; acquired consumer and writer release unproven",
                )
            return _result(
                "warm",
                "library environment remains an acquisition input; "
                "deployment inventory owns readiness",
            )
        if kind == "docs":
            from harness import lock_held

            row = read_json(path / "record.json")
            if row is None:
                return _result("unresolved", "documentation owner record unavailable")
            if str(path) not in _GUARDED.get() and lock_held(path / "owner.lock"):
                return _result("active", "documentation publisher is active")
            if (
                row.get("phase") != "finished"
                or row.get("cleanup", {}).get("status") != "confirmed"
            ):
                return _result("unresolved", "documentation process cleanup unresolved")
            return _result(
                "released",
                "documentation candidate released after owned cleanup",
                released_at=row["cleanup"].get("observed"),
            )
        if kind == "environment":
            import workspace_env

            resource = workspace_env.Resource(
                (record.get("owner") or {}).get("resource_rank", 0),
                Path(record["path"]),
                (record.get("owner") or {}).get("resource_kind", "environment"),
            )
            if workspace_env.holders(resource):
                return _result("active", "environment has live managed holders")
            return _result("warm", "environment selection requires explicit consumer release")
        if kind in ("native", "external"):
            return _result("external", "retirement belongs to the native or external owner")
        return _result("unresolved", "owner release adapter not qualified")
    except OSError, ValueError, KeyError, TypeError, RuntimeError:
        return _result("unresolved", "authoritative owner observation unavailable")


@contextlib.contextmanager
def owner_guard(record: dict[str, Any]) -> Iterator[None]:
    """Exclude actual owner writes without waiting; callers reobserve before an effect."""
    kind = (record.get("owner") or {}).get("kind")
    descriptors: list[int] = []
    token = None
    acquisition = None
    try:
        path = owner_path(record)
        if kind in ("run", "profile", "attachment", "docs"):
            if not path.is_dir():
                raise OwnerBusy("authoritative owner directory unavailable")
            fd = try_hold_lock(path / "owner.lock")
            if fd is None:
                raise OwnerBusy("authoritative owner is busy")
            descriptors.append(fd)
        elif kind == "task":
            if observe(record)["state"] != "released":
                raise OwnerBusy("task checkout remains selected or unresolved")
        elif kind == "environment":
            import workspace_env

            resource = workspace_env.Resource(
                (record.get("owner") or {}).get("resource_rank", 0),
                Path(record["path"]),
                (record.get("owner") or {}).get("resource_kind", "environment"),
            )
            # Owner sync takes the writer gate before its resource lock. Acquire in that order.
            for gate in (True, False):
                lock = workspace_env.lock_path(resource, gate=gate)
                lock.parent.mkdir(parents=True, exist_ok=True)
                fd = try_hold_lock(lock)
                if fd is None:
                    raise OwnerBusy("environment owner is busy")
                descriptors.append(fd)
        elif kind == "acquisition" and record.get("owner", {}).get("reference"):
            import storage_acquisition

            acquisition = storage_acquisition.retirement_guard(path, record["owner"]["reference"])
            acquisition.__enter__()
        else:
            raise OwnerBusy("owner effect adapter not qualified")
        token = _GUARDED.set(_GUARDED.get() | {str(path)})
        yield
    finally:
        if token is not None:
            _GUARDED.reset(token)
        if acquisition is not None:
            acquisition.__exit__(None, None, None)
        for fd in reversed(descriptors):
            os.close(fd)


def enroll(
    path: Path,
    category: str,
    owner: dict[str, Any],
    consumer: str,
    *,
    temporary_days: int | None = None,
) -> str | None:
    """Publish a small protected descriptor; bookkeeping cannot change command outcomes."""
    try:
        from storage_lifecycle import Storage

        storage = Storage()
        if temporary_days is not None:
            temporary_days = storage.policy["categories"][category].get(
                "temporary_days", temporary_days
            )
        return storage.publish(
            path, category, owner, consumer, temporary_days=temporary_days, managed=True
        )
    except Exception as error:
        import sys

        print(
            f"storage: enrollment unresolved ({type(error).__name__}); content kept: {path}",
            file=sys.stderr,
        )
        return None


def complete(
    object_id: str | None, consumer: str, *, reason: str = "confirmed owner cleanup"
) -> bool:
    if object_id is None:
        return False
    try:
        from storage_lifecycle import Storage

        Storage().complete(object_id, consumer, reason=reason)
        return True
    except Exception as error:
        import sys

        print(
            f"storage: completion unresolved ({type(error).__name__}); content kept: {object_id}",
            file=sys.stderr,
        )
        return False
