"""Pinned-library generation owner and explicit reader lease (Python 3.14.7).

The Rust CLI owns semantic library validation and frozen capture. This helper owns only
uv/git writers, generation selection and cooperating original-input reader lifetimes.
"""

from __future__ import annotations

import contextlib
import datetime as dt
import fcntl
import hashlib
import json
import os
import re
import signal
import sys
import tempfile
import time
import tomllib
import uuid
from pathlib import Path

from harness import ProcessIdentity, SpawnGuard, hold_lock, observe_exit, read_json, spawn_group
from storage_lifecycle import (
    Blocked,
    Storage,
    absolute,
    admission,
    durable_json,
    instant,
    now,
    private_directory,
)


class WriterFailed(RuntimeError):
    """The operation failed after actual owned writer cleanup was confirmed."""


def validate_request(request: dict, storage: Storage) -> dict:
    if not isinstance(request, dict) or set(request) - {
        "library",
        "environment_root",
        "source_root",
        "synchronize",
        "reinstall",
        "source",
    }:
        raise Blocked("invalid acquisition request fields")
    result = dict(request)
    for key in ("library", "environment_root", "source_root"):
        if not isinstance(request.get(key), str) or not request[key]:
            raise Blocked("acquisition paths must be explicit strings")
        path = absolute(request[key])
        if path == Path(path.anchor) or any(
            parent.is_symlink() for parent in (path, *path.parents)
        ):
            raise Blocked("acquisition paths must be physical non-root scopes")
        if path == storage.state or path in storage.state.parents or storage.state in path.parents:
            raise Blocked("acquisition scopes must not contain lifecycle authority")
        result[key] = str(path)
    env, source, library = (
        Path(result[key]) for key in ("environment_root", "source_root", "library")
    )
    if any(
        left == right or left in right.parents or right in left.parents
        for left, right in ((env, source), (env, library), (source, library))
    ):
        raise Blocked("library and acquisition output scopes must be independent")
    for key in ("synchronize", "reinstall"):
        if key in request and not isinstance(request[key], bool):
            raise Blocked("acquisition operation flags must be booleans")
    if request.get("source") is not None:
        source = request["source"]
        if (
            not isinstance(source, dict)
            or set(source) != {"repository", "commit"}
            or not isinstance(source["repository"], str)
            or not source["repository"]
            or source["repository"].startswith("-")
            or not isinstance(source["commit"], str)
            or not re.fullmatch("[a-f0-9]{40}", source["commit"])
        ):
            raise Blocked("source requires an exact repository and commit")
    return result


def prune_finished_leases(owner: Path, days: float) -> None:
    cutoff = dt.datetime.now(dt.UTC) - dt.timedelta(days=days)
    for path in (owner / "leases").glob("*.json"):
        row = read_json(path)
        if (
            row
            and row.get("phase") == "finished"
            and row.get("finished_at")
            and instant(row["finished_at"]) < cutoff
        ):
            path.unlink()


def required_record(path: Path) -> dict:
    """Missing owner authority cannot establish cleanup or release."""
    value = read_json(path)
    if value is None:
        raise Blocked(f"acquisition owner record unavailable: {path}")
    return value


def configuration(library: Path) -> str:
    digest = hashlib.sha256()
    for name in ("pyproject.toml", "uv.lock", ".python-version"):
        digest.update(name.encode())
        digest.update((library / name).read_bytes())
    return digest.hexdigest()


def owner_root(storage: Storage, request: dict) -> Path:
    identity = json.dumps(
        {key: request[key] for key in ("library", "environment_root", "source_root")},
        sort_keys=True,
    )
    return storage.state / "owners/acquisition" / hashlib.sha256(identity.encode()).hexdigest()


def group_path(owner: Path, reference: str) -> Path:
    uuid.UUID(reference)
    return owner / "groups" / f"{reference}.json"


def group_lock(owner: Path, reference: str) -> Path:
    uuid.UUID(reference)
    return owner / "groups" / f"{reference}.lock"


def generation_busy(owner: Path, reference: str) -> bool:
    fd = os.open(group_lock(owner, reference), os.O_RDONLY | os.O_NOFOLLOW)
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return True
        return False
    finally:
        os.close(fd)


def observation(owner: Path, reference: str, path: Path, *, guarded: bool = False) -> dict:
    group = read_json(group_path(owner, reference))
    selected = read_json(owner / "record.json")
    if not group or not selected or group.get("path") != str(path):
        return {
            "state": "unresolved",
            "cleanup": "unknown",
            "reason": "acquisition generation binding unavailable",
        }
    if not guarded and generation_busy(owner, reference):
        return {
            "state": "active",
            "cleanup": "unknown",
            "reason": "acquisition generation has an admitted reader or writer",
        }
    leases = [read_json(p) for p in (owner / "leases").glob("*.json")]
    if any(
        not lease or (reference in lease.get("groups", []) and lease.get("phase") != "finished")
        for lease in leases
    ):
        return {
            "state": "unresolved",
            "cleanup": "unknown",
            "reason": "acquisition reader acknowledgement unresolved",
        }
    if group.get("cleanup") != "confirmed":
        return {
            "state": "unresolved",
            "cleanup": "unknown",
            "reason": "acquisition writer cleanup unresolved",
        }
    if reference in selected.get("selected", {}).values():
        return {
            "state": "warm",
            "cleanup": "confirmed",
            "reason": "acquisition owner selects this generation",
        }
    if not group.get("released_at"):
        return {
            "state": "unresolved",
            "cleanup": "unknown",
            "reason": "acquisition owner has not released this generation",
        }
    return {
        "state": "released",
        "cleanup": "confirmed",
        "reason": "acquisition owner released generation and readers acknowledged cleanup",
        "released_at": group["released_at"],
    }


@contextlib.contextmanager
def retirement_guard(owner: Path, reference: str):
    from storage_lifecycle import Blocked

    descriptors = []
    try:
        for path in (owner / "owner.lock", group_lock(owner, reference)):
            fd = os.open(path, os.O_RDWR | os.O_NOFOLLOW)
            descriptors.append(fd)
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as error:
                raise Blocked("acquisition owner or generation is busy") from error
        yield
    finally:
        for fd in reversed(descriptors):
            os.close(fd)


def execute(
    argv: list[str], *, cwd: Path | None = None, env: dict | None = None, capture: bool = False
) -> str:
    """Keep actual writer children waitable until their process groups are drained."""
    with tempfile.TemporaryFile() as output:
        child = spawn_group(
            argv,
            cwd=cwd,
            env=env,
            stdout=output if capture else sys.stderr,
            stderr=sys.stderr,
            death_signal=signal.SIGTERM,
        )
        guard = SpawnGuard(child)
        try:
            with guard:
                while observe_exit(child) is None:
                    time.sleep(0.01)
        except Exception as error:
            if guard.cleanup.get("status") == "confirmed":
                raise WriterFailed("acquisition writer failed with confirmed cleanup") from error
            raise
        if guard.cleanup.get("status") != "confirmed":
            raise RuntimeError("acquisition writer process cleanup unresolved")
        if guard.returncode:
            raise WriterFailed(f"acquisition writer failed ({guard.returncode}): {argv[0]}")
        if capture:
            output.seek(0)
            return output.read().decode().strip()
        return ""


def sync(library: Path, environment: Path, reinstall: bool) -> None:
    env = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith("UV_") and key != "VIRTUAL_ENV"
    }
    env["UV_PROJECT_ENVIRONMENT"] = str(environment)
    command = [
        "uv",
        "sync",
        "--project",
        str(library),
        "--frozen",
        "--no-install-project",
        "--no-config",
        "--python",
        (library / ".python-version").read_text().strip(),
        "--link-mode",
        "copy",
    ]
    if reinstall:
        command.append("--reinstall")
    execute(command, env=env)


def fetch(tree: Path, source: dict) -> None:
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(
        GIT_CONFIG_NOSYSTEM="1",
        GIT_CONFIG_GLOBAL="/dev/null",
        GIT_ATTR_NOSYSTEM="1",
        GIT_TERMINAL_PROMPT="0",
    )
    tree.mkdir(parents=True, exist_ok=True)
    execute(["git", "init", "-q", "--template="], cwd=tree, env=env)
    execute(
        ["git", "fetch", "-q", "--depth", "1", source["repository"], source["commit"]],
        cwd=tree,
        env=env,
    )
    execute(
        [
            "git",
            "-c",
            "advice.detachedHead=false",
            "-c",
            "core.attributesFile=/dev/null",
            "-c",
            "core.autocrlf=false",
            "checkout",
            "-q",
            "FETCH_HEAD",
        ],
        cwd=tree,
        env=env,
    )
    check_source(tree, source)


def check_source(tree: Path, source: dict) -> None:
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(
        GIT_CONFIG_NOSYSTEM="1",
        GIT_CONFIG_GLOBAL="/dev/null",
        GIT_ATTR_NOSYSTEM="1",
        GIT_TERMINAL_PROMPT="0",
    )
    if execute(["git", "rev-parse", "HEAD"], cwd=tree, env=env, capture=True) != source["commit"]:
        raise WriterFailed("acquired source no longer matches its exact commit")


def _complete(storage: Storage, group: dict) -> None:
    import storage_owners

    if group.get("object_id"):
        storage_owners.complete(
            group["object_id"],
            group["consumer"],
            reason="acquisition owner confirmed generation release",
        )


class Lease:
    def __init__(self, request: dict):
        self.storage = Storage()
        request = validate_request(request, self.storage)
        self.library = Path(request["library"])
        self.owner = owner_root(self.storage, request)
        self.stack = contextlib.ExitStack()
        self.locks: list[int] = []
        self.groups: list[dict] = []
        self.id = str(uuid.uuid4())
        self.request = request
        self.acknowledged = False

    def prepare(self) -> dict:
        from storage_lifecycle import Blocked

        library = self.library
        # Root admission is knowable before selector observation. It precedes every native
        # metadata/generation lock and protects a containing checkout from owner removal.
        self.stack.enter_context(
            admission(
                [
                    library,
                    Path(self.request["environment_root"]),
                    Path(self.request["source_root"]),
                ],
                state=self.storage.state,
            )
        )
        revision = configuration(library)
        source = self.request.get("source")
        if source:
            declared = (
                tomllib.loads((library / "pyproject.toml").read_text())
                .get("tool", {})
                .get("lctx", {})
                .get("source", {})
            )
            if any(source.get(key) != declared.get(key) for key in ("repository", "commit")):
                raise Blocked("source selection changed before acquisition admission")
        private_directory(self.storage.state)
        for directory in (self.owner, self.owner / "groups", self.owner / "leases"):
            private_directory(directory)
        metadata = hold_lock(self.owner / "owner.lock")
        try:
            prune_finished_leases(
                self.owner, self.storage.policy["maintenance"]["action_receipt_days"]
            )
            record = read_json(self.owner / "record.json") or {
                "schema": 1,
                "library": str(library),
                "selected": {},
            }
            if not (self.owner / "record.json").exists():
                durable_json(self.owner / "record.json", record)
            selected = dict(record["selected"])
            paths = {}
            kinds = ["environment"] + (["source"] if self.request.get("source") else [])
            for kind in kinds:
                created = False
                current_id = selected.get(kind)
                group: dict | None = (
                    read_json(group_path(self.owner, current_id)) if current_id else None
                )
                key = revision if kind == "environment" else self.request["source"]["commit"]
                if (
                    group
                    and group.get("key") == key
                    and Path(group["path"]).is_dir()
                    and group.get("cleanup") == "confirmed"
                    and not (kind == "environment" and self.request.get("reinstall"))
                ):
                    pass
                elif not self.request.get("synchronize", True):
                    # Existing pre-integration environments remain usable without gaining
                    # deletion authority.
                    legacy = Path(self.request["environment_root"])
                    if kind == "environment" and legacy.joinpath("pyvenv.cfg").is_file():
                        self.stack.enter_context(admission([legacy], state=self.storage.state))
                        paths[kind] = str(legacy)
                        continue
                    raise Blocked("selected acquisition generation unavailable; run lctx acquire")
                else:
                    created = True
                    reference = str(uuid.uuid4())
                    base = Path(self.request[f"{kind}_root"])
                    path = base / ".lctx-generations" / f"{key[:16]}-{reference}"
                    self.stack.enter_context(admission([path], state=self.storage.state))
                    path.mkdir(parents=True)
                    group = {
                        "schema": 1,
                        "id": reference,
                        "kind": kind,
                        "path": str(path),
                        "key": key,
                        "cleanup": "unknown",
                        "consumer": f"acquisition:{self.owner.name}:{reference}",
                    }
                    durable_json(group_path(self.owner, reference), group)
                    fd = hold_lock(group_lock(self.owner, reference))
                    import storage_owners

                    group["object_id"] = storage_owners.enroll(
                        path,
                        f"acquired-{kind}",
                        {"kind": "acquisition", "path": str(self.owner), "reference": reference},
                        group["consumer"],
                        temporary_days=0,
                    )
                    durable_json(group_path(self.owner, reference), group)
                    failed = None
                    try:
                        if kind == "environment":
                            sync(library, path, self.request.get("reinstall", False))
                        else:
                            fetch(path, self.request["source"])
                        if configuration(library) != revision:
                            raise WriterFailed(
                                "library pins changed during acquisition; "
                                "generation released unselected"
                            )
                        group["cleanup"] = "confirmed"
                        durable_json(group_path(self.owner, reference), group)
                    except WriterFailed as error:
                        group.update(cleanup="confirmed", released_at=now())
                        durable_json(group_path(self.owner, reference), group)
                        failed = error
                    finally:
                        os.close(fd)
                    if failed is not None:
                        _complete(self.storage, group)
                        raise failed
                if group is None:
                    raise Blocked("acquisition generation owner unavailable")
                path = Path(group["path"])
                self.stack.enter_context(admission([path], state=self.storage.state))
                fd = os.open(
                    group_lock(self.owner, group["id"]),
                    os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW,
                    0o600,
                )
                fcntl.flock(fd, fcntl.LOCK_SH)
                self.locks.append(fd)
                self.groups.append(group)
                paths[kind] = str(path)
                if kind == "source" and not created:
                    check_source(path, self.request["source"])
                if self.request.get("synchronize", True):
                    selected[kind] = group["id"]
            if self.request.get("synchronize", True):
                if not self.request.get("source"):
                    selected.pop("source", None)
                durable_json(
                    self.owner / "record.json",
                    {**record, "configuration": revision, "selected": selected},
                )
                for old in set(record["selected"].values()) - set(selected.values()):
                    group = read_json(group_path(self.owner, old))
                    if group:
                        group["released_at"] = now()
                        durable_json(group_path(self.owner, old), group)
                        _complete(self.storage, group)
            self.receipt = {
                "schema": 1,
                "id": self.id,
                "parent": ProcessIdentity.of(os.getppid()).to_json(),
                "groups": [group["id"] for group in self.groups],
                "phase": "active",
                "created_at": now(),
            }
            durable_json(self.owner / "leases" / f"{self.id}.json", self.receipt)
            return {"outcome": "passed", "lease": self.id, "owner": str(self.owner), **paths}
        finally:
            os.close(metadata)

    def close(self, *, acknowledged: bool) -> None:
        for fd in reversed(self.locks):
            os.close(fd)
        self.locks.clear()
        if hasattr(self, "receipt"):
            metadata = hold_lock(self.owner / "owner.lock")
            try:
                self.receipt.update(
                    phase="finished" if acknowledged else "uncertain", finished_at=now()
                )
                durable_json(self.owner / "leases" / f"{self.id}.json", self.receipt)
                if acknowledged:
                    selected = required_record(self.owner / "record.json")["selected"]
                    for item in self.groups:
                        group = required_record(group_path(self.owner, item["id"]))
                        if group["id"] not in selected.values() and not generation_busy(
                            self.owner, group["id"]
                        ):
                            group["released_at"] = now()
                            durable_json(group_path(self.owner, group["id"]), group)
                            _complete(self.storage, group)
            finally:
                os.close(metadata)
        elif self.groups:
            # Startup failed before any original-input reader was exposed to Rust. Known
            # completed writers can release their unselected generations independently.
            metadata = hold_lock(self.owner / "owner.lock")
            try:
                selected = required_record(self.owner / "record.json")["selected"]
                for item in self.groups:
                    group = required_record(group_path(self.owner, item["id"]))
                    if (
                        group["id"] not in selected.values()
                        and group.get("cleanup") == "confirmed"
                        and not generation_busy(self.owner, group["id"])
                    ):
                        group["released_at"] = now()
                        durable_json(group_path(self.owner, group["id"]), group)
                        _complete(self.storage, group)
            finally:
                os.close(metadata)
        self.stack.close()


def release_generation(object_id: str) -> dict:
    """Explicit owner release, including a previously selected override generation."""
    from storage_lifecycle import Blocked

    storage = Storage()
    row = storage.get(object_id)
    if row.get("owner", {}).get("kind") != "acquisition" or not row["owner"].get("reference"):
        raise Blocked("object has no managed acquisition owner")
    owner = Path(row["owner"]["path"])
    if owner.parent != storage.state / "owners/acquisition":
        raise Blocked("acquisition owner is outside the durable owner store")
    reference = row["owner"]["reference"]
    with admission([Path(row["path"])], state=storage.state), retirement_guard(owner, reference):
        group = read_json(group_path(owner, reference))
        if not group or group.get("object_id") != object_id or group.get("cleanup") != "confirmed":
            raise Blocked("generation identity or writer cleanup unresolved")
        leases = [read_json(path) for path in (owner / "leases").glob("*.json")]
        if any(
            not lease or (reference in lease.get("groups", []) and lease.get("phase") != "finished")
            for lease in leases
        ):
            raise Blocked("generation reader acknowledgement unresolved")
        record = required_record(owner / "record.json")
        record["selected"] = {
            key: value for key, value in record["selected"].items() if value != reference
        }
        durable_json(owner / "record.json", record)
        group["released_at"] = now()
        durable_json(group_path(owner, reference), group)
    _complete(storage, group)
    return {"outcome": "passed", "released": object_id}


def main() -> int:
    lease = None
    acknowledged = False
    try:
        if len(sys.argv) == 3 and sys.argv[1] == "release":
            print(json.dumps(release_generation(sys.argv[2])))
            return 0
        request = json.loads(sys.stdin.readline())
        lease = Lease(request)
        print(json.dumps(lease.prepare()), flush=True)
        response = sys.stdin.readline()
        acknowledged = bool(response) and json.loads(response) == {"action": "finish"}
        lease.close(acknowledged=acknowledged)
        lease = None
        if acknowledged:
            print(json.dumps({"outcome": "passed", "finished": True}), flush=True)
            return 0
        return 75
    except Exception as error:
        print(json.dumps({"outcome": "blocked", "reason": str(error)}), flush=True)
        return 75
    finally:
        if lease is not None:
            lease.close(acknowledged=False)


if __name__ == "__main__":
    raise SystemExit(main())
