"""Small owner-directed storage policy; Python 3.14.7, standard library only.

Descriptors and journals survive output deletion. Paths, age and inventory caches never
authorize deletion. Native content is protected unless its own qualified owner retires it.
"""

from __future__ import annotations

import contextlib
import datetime as dt
import fcntl
import hashlib
import json
import os
import re
import stat
import threading
import tomllib
import uuid
from collections.abc import Iterator, Mapping, Sequence
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
CAPABILITIES = frozenset(
    {
        "immediate-use",
        "raw-replay",
        "restorable-replay",
        "cited-report",
        "receipt",
        "external-owner",
    }
)
OPERATIONS = frozenset(
    {
        "compiler-summary",
        "compiler-timeline",
        "sampled-symbolized-report",
        "sampled-symbolized-import",
        "disassembly",
        "source-annotation",
    }
)
OWNER_KINDS = frozenset(
    {
        "task",
        "run",
        "profile",
        "attachment",
        "environment",
        "acquisition",
        "docs",
        "native",
        "external",
    }
)
_local = threading.local()
ADMISSION_ENV = "LCTX_STORAGE_ADMISSION"
NONCE_ATTRIBUTE = "user.library-context-lifetime"


class Invalid(ValueError):
    """Invalid policy or descriptor; never an eligible object."""


class Blocked(RuntimeError):
    """An explicitly requested effect cannot safely proceed."""


def now() -> str:
    return dt.datetime.now(dt.UTC).isoformat()


def instant(value: str) -> dt.datetime:
    stamp = dt.datetime.fromisoformat(value)
    if stamp.tzinfo is None:
        raise Invalid("timestamps require a timezone")
    return stamp


def absolute(path: str | Path) -> Path:
    return Path(os.path.abspath(Path(path).expanduser()))


def state_root(env: Mapping[str, str] | None = None) -> Path:
    source = os.environ if env is None else env
    return absolute(
        source.get(
            "LCTX_STORAGE_STATE",
            str(
                Path(source.get("XDG_STATE_HOME", str(Path.home() / ".local/state")))
                / "library-context-storage"
            ),
        )
    )


def host_config_path() -> Path:
    return absolute(
        os.environ.get(
            "LCTX_STORAGE_CONFIG",
            str(
                Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config")))
                / "library-context-storage/config.toml"
            ),
        )
    )


def private_directory(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True, mode=0o700)
    info = path.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise Invalid(f"state directory must be private, owned and not a symlink: {path}")


def durable_json(path: Path, value: Any) -> None:
    private_directory(path.parent)
    temporary = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    try:
        fd = os.open(temporary, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "w") as stream:
            json.dump(value, stream, sort_keys=True, indent=2)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        fsync_directory(path.parent)
    finally:
        temporary.unlink(missing_ok=True)


def fsync_directory(path: Path) -> None:
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def read_json(path: Path) -> dict:
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd) as stream:
        row = json.load(stream)
    if not isinstance(row, dict):
        raise Invalid(f"expected object: {path}")
    return row


def physical(path: Path) -> dict:
    # Check every ancestor without resolving symlinks into another scope.
    for item in reversed((path, *path.parents)):
        if stat.S_ISLNK(item.lstat().st_mode):
            raise Blocked(f"symlink in managed scope: {item}")
    info = path.lstat()
    if not stat.S_ISDIR(info.st_mode) and not stat.S_ISREG(info.st_mode):
        raise Blocked(f"unsupported managed object: {path}")
    return {"device": info.st_dev, "inode": info.st_ino, "type": stat.S_IFMT(info.st_mode)}


def _nonce(path: Path, *, create: bool = False) -> str:
    try:
        return os.getxattr(path, NONCE_ATTRIBUTE, follow_symlinks=False).decode("ascii")
    except OSError:
        if not create:
            raise Blocked(f"physical lifetime nonce unavailable: {path}") from None
    value = uuid.uuid4().hex
    os.setxattr(path, NONCE_ATTRIBUTE, value.encode(), flags=os.XATTR_CREATE, follow_symlinks=False)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)
    return value


def _start(pid: int) -> str:
    return Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[19]


def _inherited() -> dict[str, dict]:
    raw = os.environ.get(ADMISSION_ENV)
    if not raw:
        return {}
    try:
        rows = json.loads(raw)
        ancestors = {os.getpid()}
        pid = os.getppid()
        while pid > 1 and pid not in ancestors:
            ancestors.add(pid)
            pid = int(Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[1])
        result: dict[str, dict] = {}
        for supplied in rows:
            if not isinstance(supplied, dict) or not isinstance(supplied.get("key"), str):
                raise Blocked("invalid inherited storage admission entry")
            row: dict[str, Any] = supplied
            if row["pid"] == os.getpid():
                # Live same-process ownership is authoritative in the context's actual map.
                # Lifetimes may close in non-LIFO order (two logical attachments).
                continue
            try:
                live = row["pid"] in ancestors and _start(row["pid"]) == row["start"]
            except OSError:
                live = False
            if not live:
                if row["exclusive"]:
                    raise Blocked("inherited exclusive admission owner identity is no longer live")
                # A detached shared child acquires its own admission for the recorded
                # scopes; it must not depend on the launcher remaining alive.
                row = dict(row, reacquire=True)
                result[row["key"]] = row
                continue
            handle = Path(f"/proc/{row['pid']}/fd/{row['fd']}")
            info = handle.stat()
            current = Path(row["key"]).stat()
            if (info.st_dev, info.st_ino) != (current.st_dev, current.st_ino) or [
                info.st_dev,
                info.st_ino,
            ] != row["identity"]:
                raise Blocked("inherited admission descriptor identity changed")
            # fdinfo proves this descriptor actually owns a flock, not merely an open file.
            fdinfo = Path(f"/proc/{row['pid']}/fdinfo/{row['fd']}").read_text()
            expected = "WRITE" if row["exclusive"] else "READ"
            if not any(
                "FLOCK" in line and expected in line
                for line in fdinfo.splitlines()
                if line.startswith("lock:")
            ):
                raise Blocked("inherited admission no longer holds its declared lock")
            result[row["key"]] = row
        return result
    except (OSError, ValueError, KeyError, TypeError) as exc:
        raise Blocked("invalid inherited storage admission") from exc


def _lock_path(root: Path, path: Path) -> Path:
    return root / "admission" / (hashlib.sha256(os.fsencode(path)).hexdigest() + ".lock")


@contextlib.contextmanager
def admission(
    paths: Sequence[Path],
    *,
    exclusive: bool = False,
    blocking: bool = True,
    state: Path | None = None,
    exclusive_paths: Sequence[Path] = (),
) -> Iterator[None]:
    """All lexical ancestors participate, including children not yet registered.

    Acquire outer-to-inner. Nested ownership reuses descriptors; never upgrade shared
    ownership. The calling owner keeps admission through subprocess cleanup.
    """
    root = absolute(state) if state is not None else state_root()
    private_directory(root)
    private_directory(root / "admission")
    requested: dict[Path, bool] = {}
    writes = {absolute(path) for path in exclusive_paths}
    for supplied in paths:
        path = absolute(supplied)
        for ancestor in (*reversed(path.parents), path):
            requested[ancestor] = requested.get(ancestor, False) or (
                ancestor == path and (exclusive or path in writes)
            )
    held = getattr(_local, "held", None)
    if held is None:
        held = _local.held = {}
    acquired: list[tuple[str, bool]] = []
    inherited = _inherited()
    for entry in inherited.values():
        if not entry["exclusive"]:
            scope = absolute(entry["path"])
            if str(_lock_path(root, scope)) != entry["key"]:
                raise Blocked("inherited scope/state binding differs")
            for ancestor in (*reversed(scope.parents), scope):
                requested.setdefault(ancestor, False)
    try:
        for path, write in sorted(
            requested.items(), key=lambda entry: (len(entry[0].parts), str(entry[0]))
        ):
            key = str(_lock_path(root, path))
            if write and key in inherited and not inherited[key]["exclusive"]:
                raise Blocked("inherited shared-to-exclusive admission upgrade refused")
            if key in inherited and inherited[key]["exclusive"] and key not in held:
                if write and not inherited[key]["exclusive"]:
                    raise Blocked("inherited shared-to-exclusive admission upgrade refused")
                continue
            if key in held:
                fd, previous, count = held[key]
                if write and not previous:
                    raise Blocked("shared-to-exclusive storage admission upgrade refused")
                held[key] = (fd, previous, count + 1)
                acquired.append((key, False))
                continue
            # Refuse a nested acquisition that would reverse global lock order.
            active_paths = [entry[3] for entry in getattr(_local, "order", [])]
            shared_only = (
                not any(requested.values())
                and not any(value[1] for value in held.values())
                and not any(value["exclusive"] for value in inherited.values())
            )
            if (
                not shared_only
                and active_paths
                and (len(path.parts), str(path)) < max((len(p.parts), str(p)) for p in active_paths)
            ):
                raise Blocked("nested storage admission would reverse lock order")
            fd = os.open(key, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
            try:
                fcntl.flock(
                    fd,
                    (fcntl.LOCK_EX if write else fcntl.LOCK_SH)
                    | (0 if blocking else fcntl.LOCK_NB),
                )
            except BlockingIOError as exc:
                os.close(fd)
                raise Blocked(f"storage scope busy: {path}") from exc
            held[key] = (fd, write, 1)
            if not hasattr(_local, "order"):
                _local.order = []
            _local.order.append((key, fd, write, path))
            acquired.append((key, True))
        entries = {key: row for key, row in inherited.items() if row["exclusive"]}
        for key, (fd, write, _) in held.items():
            info = os.fstat(fd)
            entries[key] = {
                "key": key,
                "pid": os.getpid(),
                "start": _start(os.getpid()),
                "fd": fd,
                "exclusive": write,
                "identity": [info.st_dev, info.st_ino],
                "path": next(entry[3] for entry in _local.order if entry[0] == key).as_posix(),
            }
        os.environ[ADMISSION_ENV] = json.dumps(list(entries.values()), separators=(",", ":"))
        yield
    finally:
        for key, _opened in reversed(acquired):
            fd, write, count = held[key]
            if count > 1:
                held[key] = (fd, write, count - 1)
            else:
                os.close(fd)
                del held[key]
                _local.order = [entry for entry in _local.order if entry[0] != key]
        entries = {key: row for key, row in inherited.items() if row["exclusive"]}
        for key, (fd, write, _) in held.items():
            info = os.fstat(fd)
            entries[key] = {
                "key": key,
                "pid": os.getpid(),
                "start": _start(os.getpid()),
                "fd": fd,
                "exclusive": write,
                "identity": [info.st_dev, info.st_ino],
                "path": next(entry[3] for entry in _local.order if entry[0] == key).as_posix(),
            }
        if entries:
            os.environ[ADMISSION_ENV] = json.dumps(list(entries.values()), separators=(",", ":"))
        else:
            os.environ.pop(ADMISSION_ENV, None)


def observe(record: dict, host: dict | None = None) -> dict:
    kind = record["owner"]["kind"]
    if kind in {"native", "external"}:
        return {
            "state": "external",
            "cleanup": "unknown",
            "reason": "native/external owner; no generic retirement",
        }
    try:
        import storage_owners

        return storage_owners.observe(record)
    except (ImportError, OSError, ValueError, RuntimeError) as exc:
        return {"state": "unresolved", "cleanup": "unknown", "reason": str(exc)}


@contextlib.contextmanager
def owner_guard(record: dict, host: dict) -> Iterator[None]:
    if record["owner"]["kind"] in {"native", "external"}:
        raise Blocked("no generic native/external retirement")
    import storage_owners

    try:
        with storage_owners.owner_guard(record):
            yield
    except storage_owners.OwnerBusy as error:
        raise Blocked(str(error)) from error


class Storage:
    def __init__(self, root: Path = ROOT, *, config: Path | None = None, state: Path | None = None):
        self.root = absolute(root)
        self.state = absolute(state) if state is not None else state_root()
        self.config = absolute(config) if config is not None else self.root / ".config/storage.toml"
        raw = self.config.read_bytes()
        self.policy = tomllib.loads(raw.decode())
        if self.policy.get("schema") != 1 or not isinstance(self.policy.get("categories"), dict):
            raise Invalid("unknown or incomplete storage policy")
        import math

        for name, rule in self.policy["categories"].items():
            if not isinstance(rule, dict):
                raise Invalid(f"invalid category {name}")
            if rule.get("owner") not in OWNER_KINDS:
                raise Invalid(f"unknown owner in category {name}")
            for key in ("grace_days", "temporary_days"):
                if key in rule and (
                    type(rule[key]) not in (float, int)
                    or not math.isfinite(rule[key])
                    or rule[key] < 0
                ):
                    raise Invalid(f"invalid duration in category {name}")
        for key, value in self.policy.get("maintenance", {}).items():
            if (
                key not in {"tombstone_days", "action_receipt_days"}
                or type(value) not in (float, int)
                or not math.isfinite(value)
                or value < 0
            ):
                raise Invalid("invalid metadata maintenance duration")
        self.host_path = host_config_path()
        self.host = (
            tomllib.loads(self.host_path.read_text())
            if self.host_path.exists()
            else {"schema": 1, "repositories": []}
        )
        if self.host.get("schema") != 1:
            raise Invalid("unknown host policy schema")
        for key in ("repositories",):
            if not isinstance(self.host.get(key, []), list) or any(
                not isinstance(p, str) or not Path(p).is_absolute() for p in self.host.get(key, [])
            ):
                raise Invalid(f"host {key} must be explicit absolute paths")
        self.revision = hashlib.sha256(
            raw + json.dumps(self.host, sort_keys=True).encode()
        ).hexdigest()

    @contextlib.contextmanager
    def metadata(self) -> Iterator[None]:
        private_directory(self.state)
        fd = os.open(
            self.state / "metadata.lock",
            os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW | os.O_CLOEXEC,
            0o600,
        )
        try:
            fcntl.flock(fd, fcntl.LOCK_EX)
            yield
        finally:
            os.close(fd)

    def records(self) -> list[dict]:
        result = []
        for path in sorted((self.state / "objects").glob("*.json")):
            row = read_json(path)
            self.validate(row)
            result.append(row)
        return result

    def validate(self, row: dict) -> None:
        import math

        if row.get("schema") != 1 or row.get("category") not in self.policy["categories"]:
            raise Invalid("unknown descriptor schema/category")
        try:
            uuid.UUID(row["id"])
            if not Path(row["path"]).is_absolute() or row["owner"]["kind"] not in OWNER_KINDS:
                raise Invalid("invalid owner or physical scope")
            if row["owner"]["kind"] != self.policy["categories"][row["category"]]["owner"]:
                raise Invalid("category/owner mismatch")
            if (
                type(row.get("grace_days")) not in (float, int)
                or not math.isfinite(row["grace_days"])
                or row["grace_days"] < 0
            ):
                raise Invalid("invalid descriptor grace")
            if type(row.get("managed")) is not bool or row.get("exposure") not in {
                "managed",
                "unresolved",
            }:
                raise Invalid("invalid managed exposure")
            for item in row["obligations"].values():
                if item["requires"] not in CAPABILITIES:
                    raise Invalid("unknown availability capability")
                for key in ("until", "released_at"):
                    if item.get(key):
                        instant(item[key])
                duration = item.get("temporary_days")
                if duration is not None and (
                    type(duration) not in (float, int)
                    or not math.isfinite(duration)
                    or duration < 0
                ):
                    raise Invalid("invalid obligation duration")
        except (KeyError, TypeError, AttributeError) as exc:
            raise Invalid("incomplete descriptor") from exc

    def get(self, object_id: str) -> dict:
        try:
            uuid.UUID(object_id)
        except ValueError as exc:
            raise Invalid("object ID must be a UUID") from exc
        row = read_json(self.state / "objects" / f"{object_id}.json")
        self.validate(row)
        return row

    def save(self, row: dict) -> None:
        self.validate(row)
        key = hashlib.sha256(os.fsencode(row["path"])).hexdigest()
        index = self.state / "paths" / f"{key}.json"
        binding = read_json(index) if index.exists() else None
        if not row.get("retired_at") and binding and binding.get("id") != row["id"]:
            previous = self.get(binding["id"])
            if not previous.get("retired_at"):
                raise Invalid("physical scope already has another live lifetime")
        durable_json(self.state / "objects" / f"{row['id']}.json", row)
        if row.get("retired_at"):
            if binding and binding.get("id") == row["id"]:
                index.unlink()
                fsync_directory(index.parent)
        else:
            durable_json(index, {"schema": 1, "id": row["id"], "path": row["path"]})

    def _index_paths(self) -> None:
        """One-time upgrade under metadata ownership; new producers use bounded lookups."""
        marker = self.state / "paths-ready.json"
        if marker.exists():
            if read_json(marker).get("schema") != 1:
                raise Invalid("unknown physical path index schema")
            return
        for row in self.records():
            key = hashlib.sha256(os.fsencode(row["path"])).hexdigest()
            target = self.state / "paths" / f"{key}.json"
            if not row.get("retired_at"):
                durable_json(target, {"schema": 1, "id": row["id"], "path": row["path"]})
        durable_json(marker, {"schema": 1})

    def _path_row(self, path: Path) -> dict | None:
        key = hashlib.sha256(os.fsencode(path)).hexdigest()
        target = self.state / "paths" / f"{key}.json"
        if not target.exists():
            return None
        index = read_json(target)
        if index.get("schema") != 1 or index.get("path") != str(path):
            raise Invalid("physical path index differs")
        row = self.get(index["id"])
        if row["path"] != str(path):
            raise Invalid("physical path index points to another scope")
        return None if row.get("retired_at") else row

    def publish(
        self,
        path: Path,
        category: str,
        owner: dict,
        consumer: str | None,
        requires: str = "immediate-use",
        temporary_days: float | None = None,
        managed: bool = False,
        parent: str | None = None,
    ) -> str:
        path = absolute(path)
        if category not in self.policy["categories"] or requires not in CAPABILITIES:
            raise Invalid("unknown category or capability")
        if path == self.state or path in self.state.parents or self.state in path.parents:
            raise Invalid("management state must be outside disposable scopes")
        with admission([path], state=self.state), self.metadata():
            self._index_paths()
            identity = physical(path)
            existing = self._path_row(path)
            row: dict[str, Any]
            if existing is not None:
                row = existing
                if (
                    row["identity"] != identity
                    or row["owner"] != owner
                    or row["category"] != category
                ):
                    raise Blocked("path already has a different lifetime or owner")
                if row.get("retirement"):
                    raise Blocked("retirement already selected")
            else:
                row = {
                    "schema": 1,
                    "id": str(uuid.uuid4()),
                    "category": category,
                    "path": str(path),
                    "owner": owner,
                    "identity": identity,
                    "nonce": _nonce(path, create=True) if managed else None,
                    "managed": managed,
                    "created_at": now(),
                    "policy_revision": self.revision,
                    "grace_days": self.policy["categories"][category].get("grace_days", 0),
                    "obligations": {},
                    "exposure": "managed" if managed else "unresolved",
                }
                containers = [
                    r
                    for parent_path in path.parents
                    if (r := self._path_row(parent_path)) is not None
                ]
                if parent is not None:
                    container = self.get(parent)
                    if Path(container["path"]) not in path.parents:
                        raise Invalid("declared parent is not a physical ancestor")
                    row["parent"] = parent
                elif containers:
                    container = max(containers, key=lambda r: len(Path(r["path"]).parts))
                    if owner.get("path") and container["owner"].get("path") == owner["path"]:
                        row["parent"] = container["id"]
            if consumer:
                if temporary_days is not None and (
                    type(temporary_days) not in (int, float) or temporary_days < 0
                ):
                    raise Invalid("invalid temporary-use duration")
                previous = row["obligations"].get(consumer)
                if not previous or previous.get("released_at"):
                    row["obligations"][consumer] = {
                        "requires": requires,
                        "temporary_days": temporary_days,
                        "until": None,
                        "released_at": None,
                    }
            self.save(row)
            if row.get("managed"):
                key = hashlib.sha256(os.fsencode(path)).hexdigest()
                durable_json(
                    self.state / "scopes" / (key + ".json"), {"id": row["id"], "path": str(path)}
                )
            return row["id"]

    def manage_existing(self, object_id: str) -> dict:
        """Qualify a released producer's exact legacy scope without releasing its holds.

        Missing task checkouts and arbitrary paths are not evidence of historical writer
        quiescence. Only owners with actual cleanup and fixed physical scope participate.
        """
        initial = self.get(object_id)
        path = absolute(initial["path"])
        owner_path = absolute(initial["owner"].get("path", "/"))
        kind = initial["owner"]["kind"]
        if kind not in {"run", "attachment", "profile", "docs"}:
            raise Blocked("legacy owner has no qualified quiescent adoption operation")
        with (
            admission([path, owner_path], exclusive_paths=[path], blocking=False, state=self.state),
            owner_guard(initial, self.host),
            self.metadata(),
        ):
            row = self.get(object_id)
            if row.get("retired_at") or row.get("retirement"):
                raise Blocked("cannot manage a selected or retired lifetime")
            if row["path"] != initial["path"] or row["owner"] != initial["owner"]:
                raise Blocked("legacy lifetime binding changed")
            if physical(path) != row["identity"]:
                raise Blocked("legacy physical identity changed")
            physical(owner_path)
            if kind in {"run", "attachment"}:
                expected = [owner_path]
            elif kind == "profile":
                import compile_profile

                raw = absolute(compile_profile.profile_dir(owner_path))
                expected = [raw, absolute(compile_profile.reports_root(raw))]
            else:
                owner_record = read_json(owner_path / "record.json")
                candidate = owner_record.get("path")
                if not isinstance(candidate, str) or not Path(candidate).is_absolute():
                    raise Blocked("documentation owner has no exact candidate binding")
                expected = [absolute(candidate)]
            if path not in expected:
                raise Blocked("legacy path is outside the owner's exact disposable scope")
            facts = observe(row, self.host)
            cleanup = facts.get("cleanup", "unknown")
            if isinstance(cleanup, dict):
                cleanup = cleanup.get("status")
            if facts.get("state") != "released" or cleanup != "confirmed":
                raise Blocked(
                    "legacy adoption requires authoritative release and confirmed cleanup"
                )
            if row.get("managed"):
                if _nonce(path) != row["nonce"]:
                    raise Blocked("managed physical lifetime nonce changed")
                return row
            row.update(
                nonce=_nonce(path, create=True), managed=True, exposure="managed", adopted_at=now()
            )
            self.save(row)
            key = hashlib.sha256(os.fsencode(path)).hexdigest()
            durable_json(
                self.state / "scopes" / (key + ".json"), {"id": row["id"], "path": str(path)}
            )
            return row

    def retain(
        self,
        object_id: str,
        consumer: str,
        requires: str = "immediate-use",
        operations: Sequence[str] = (),
    ) -> dict:
        initial = self.get(object_id)
        if requires not in CAPABILITIES or not consumer:
            raise Invalid("named consumer and known capability required")
        if set(operations) - OPERATIONS or (requires == "restorable-replay" and not operations):
            raise Invalid("restorable replay requires explicit supported operations")
        with admission([Path(initial["path"])], state=self.state), self.metadata():
            row = self.get(object_id)
            if row.get("retired_at") or row.get("retirement"):
                raise Blocked("cannot retain an object selected for retirement")
            if physical(Path(row["path"])) != row["identity"]:
                raise Blocked("physical identity changed")
            if row.get("managed") and _nonce(Path(row["path"])) != row["nonce"]:
                raise Blocked("physical lifetime nonce changed")
            row["obligations"][consumer] = {
                "requires": requires,
                "until": None,
                "released_at": None,
                "temporary_days": None,
                "operations": list(operations),
            }
            self.save(row)
            return row

    def complete(
        self, object_id: str, consumer: str, *, reason: str = "confirmed owner cleanup"
    ) -> dict:
        initial = self.get(object_id)
        with admission([Path(initial["path"])], state=self.state), self.metadata():
            row = self.get(object_id)
            facts = observe(row, self.host)
            cleanup = facts.get("cleanup", "unknown")
            if isinstance(cleanup, dict):
                cleanup = cleanup.get("status")
            item = row["obligations"].get(consumer)
            if cleanup != "confirmed" or not item or item.get("temporary_days") is None:
                raise Blocked("confirmed cleanup and declared temporary obligation required")
            if not item.get("until"):
                origin = facts.get("released_at") or facts.get("cleanup_at") or now()
                item["until"] = (
                    instant(origin) + dt.timedelta(days=item["temporary_days"])
                ).isoformat()
                item["reason"] = reason
            self.save(row)
            self.enqueue(row["id"])
            return row

    def release(self, object_id: str, consumer: str, reason: str) -> dict:
        if not reason.strip():
            raise Invalid("release reason required")
        initial = self.get(object_id)
        with admission([Path(initial["path"])], state=self.state), self.metadata():
            row = self.get(object_id)
            if consumer not in row["obligations"]:
                raise Invalid("unknown consumer")
            item = row["obligations"][consumer]
            if not item.get("released_at"):
                item.update(released_at=now(), reason=reason)
            self.save(row)
            self.enqueue(row["id"])
            return row

    def enqueue(self, object_id: str) -> None:
        if not (self.state / "hooks-enabled.json").is_file():
            return
        try:
            durable_json(
                self.state / "queue" / f"{object_id}.json",
                {"schema": 1, "id": object_id, "observed_at": now()},
            )
        except OSError:
            # Catch-up enumerates known objects; enqueue failure is never release authority.
            import sys

            print(
                f"storage: enqueue failed; content kept for catch-up: {object_id}", file=sys.stderr
            )

    def references(self) -> list[tuple[Path, str]]:
        """Current tracked references once per operation; no raw outputs or private configs."""
        import subprocess

        references = []
        repos = {str(self.root), *self.host.get("repositories", [])}
        for name in sorted(repos):
            repo = Path(name)
            if not repo.exists():
                references.append((repo, "__missing_participant__"))
                continue
            result = subprocess.run(
                ["git", "-C", str(repo), "ls-files", "-z", "--", "*.md", "*.toml"],
                capture_output=True,
                check=False,
            )
            if result.returncode:
                references.append((repo, "__unreadable_participant__"))
                continue
            for relative in result.stdout.split(b"\0"):
                if not relative:
                    continue
                path = repo / os.fsdecode(relative)
                try:
                    references.append((path, path.read_text()))
                except OSError, UnicodeError:
                    references.append((repo, "__unreadable_participant__"))
        return references

    def disposition(
        self,
        row: dict,
        *,
        records: list[dict] | None = None,
        references: list[tuple[Path, str]] | None = None,
        clock: dt.datetime | None = None,
        check_children: bool = True,
    ) -> dict:
        clock = clock or dt.datetime.now(dt.UTC)
        path = Path(row["path"])
        reasons: list[str] = []
        result = {
            "id": row["id"],
            "category": row["category"],
            "path": str(path),
            "owner": row["owner"],
            "disposition": "unresolved",
            "reasons": reasons,
        }
        if row.get("retired_at"):
            result["disposition"] = "archived" if row.get("archive") else "retired"
            if row.get("archive") and not self.archive_ready(row):
                result["disposition"] = "unresolved"
                reasons.append("archive identity or required replay capability unavailable")
            return result
        if row.get("retirement"):
            reasons.append("interrupted retirement requires exact journal recovery")
            return result
        try:
            if physical(path) != row["identity"]:
                reasons.append("physical identity changed")
                return result
            if row.get("managed") and _nonce(path) != row["nonce"]:
                reasons.append("physical lifetime nonce changed")
                return result
        except (OSError, Blocked) as exc:
            reasons.append(str(exc))
            return result
        facts = observe(row, self.host)
        result["observation"] = facts
        state = facts.get("state", "unresolved")
        cleanup = facts.get("cleanup", "unknown")
        if isinstance(cleanup, dict):
            cleanup = cleanup.get("status")
        if self.policy["categories"][row["category"]].get("protected") or state in {
            "warm",
            "external",
            "active",
            "unresolved",
        }:
            result["disposition"] = (
                state if state in {"warm", "external", "active"} else "unresolved"
            )
            reasons.append(facts.get("reason", "category protected"))
            return result
        if not row.get("managed") or row.get("exposure") != "managed":
            reasons.append("unmanaged exposure has not been quiescently adopted")
        if cleanup != "confirmed":
            reasons.append("owner cleanup is unresolved")
        deadlines = []
        if facts.get("released_at"):
            deadlines.append(instant(facts["released_at"]))
        for consumer, item in row["obligations"].items():
            if item.get("released_at"):
                deadlines.append(instant(item["released_at"]))
            elif item.get("until") and instant(item["until"]) <= clock:
                deadlines.append(instant(item["until"]))
            elif item["requires"] == "restorable-replay" and self.archive_ready(
                row, item.get("operations", [])
            ):
                continue
            else:
                reasons.append(f"consumer {consumer} requires {item['requires']}")
        if not row["obligations"]:
            reasons.append("no authoritative consumer/release history")
        if deadlines and max(deadlines) + dt.timedelta(days=row["grace_days"]) > clock:
            reasons.append("post-release grace period has not expired")
        if references is not None:
            known_scopes = {
                Path(record["path"])
                for record in (self.records() if records is None else records)
                if not record.get("retired_at")
            }
            for source, text in references:
                if text.startswith("__"):
                    reasons.append(f"consumer coverage unavailable: {source}")
                elif (
                    row["id"] in text
                    or str(path) in text
                    or (path.name in text and len(path.name) >= 12)
                ):
                    reasons.append(f"current tracked reference: {source}")
                else:
                    from urllib.parse import unquote, urlsplit

                    targets = re.findall(r"\]\(([^)]+)\)|`([^`\n]+)`", text)
                    for markdown, code in targets:
                        candidate = (markdown or code).strip().strip("<>")
                        if not candidate or any(char in candidate for char in "\n\r"):
                            continue
                        link = urlsplit(candidate)
                        if link.scheme or link.netloc:
                            continue
                        name = unquote(link.path)
                        if not name or name in {".", ".."} or (" " in name and not markdown):
                            continue
                        roots = [
                            source.parent,
                            self.root,
                            *(Path(p) for p in self.host.get("repositories", [])),
                        ]
                        resolved = [absolute(base / name) for base in roots]
                        resolved = [target for target in resolved if target != Path(target.anchor)]
                        # A broad documentation mention of /tmp or build/ is not an
                        # evidence consumer. Ancestor citations protect descendants only
                        # when that ancestor is itself a declared lifetime.
                        if any(
                            target == path
                            or path in target.parents
                            or (target in path.parents and target in known_scopes)
                            for target in resolved
                        ):
                            reasons.append(f"current tracked reference: {source}")
                            break
        if check_children:
            for child in self.records() if records is None else records:
                if child["id"] == row["id"] or child.get("retired_at"):
                    continue
                other = Path(child["path"])
                if other == path:
                    reasons.append(f"overlapping lifetime {child['id']}")
                elif path in other.parents:
                    reasons.append(f"descendant must retire independently first: {child['id']}")
                elif other in path.parents:
                    # Sharing a root requires a declared ancestor owner, never accidental overlaps.
                    by_id = {
                        record["id"]: record
                        for record in (self.records() if records is None else records)
                    }
                    ancestors = set()
                    parent = row.get("parent")
                    while parent and parent not in ancestors and parent in by_id:
                        ancestors.add(parent)
                        parent = by_id[parent].get("parent")
                    if child["id"] not in ancestors:
                        reasons.append(f"undeclared containing scope {child['id']}")
        if reasons:
            result["disposition"] = (
                "retained" if any(r.startswith("consumer ") for r in reasons) else "unresolved"
            )
        else:
            result["disposition"] = "eligible"
        return result

    def archive_ready(self, row: dict, operations: Sequence[str] = ()) -> bool:
        archive = row.get("archive") or {}
        if (
            archive.get("outcome") != "passed"
            or not archive.get("replay_qualified")
            or archive.get("replay", {}).get("isolation", {}).get("outcome") != "passed"
        ):
            return False
        try:
            import compile_profile_tools

            if not compile_profile_tools.reader_binding_valid(archive.get("reader_generation", {})):
                return False
            path = Path(archive["path"])
            if physical(path) != archive["physical_identity"]:
                return False
            info = path.stat()
            if [info.st_size, info.st_mtime_ns] != archive["file_signature"]:
                return False
            capabilities = archive["replay"]["capabilities"]
            required = operations or [
                op
                for item in row["obligations"].values()
                if item["requires"] == "restorable-replay" and not item.get("released_at")
                for op in item.get("operations", [])
            ]
            return bool(required) and all(
                capabilities.get(op, {}).get("outcome") == "passed" for op in required
            )
        except OSError, Blocked, KeyError, TypeError:
            return False

    def plan(self, ids: Sequence[str] = ()) -> dict:
        rows = [self.get(i) for i in ids] if ids else self.records()
        refs = self.references()
        all_rows = self.records()
        return self.envelope([self.disposition(r, records=all_rows, references=refs) for r in rows])

    def envelope(
        self,
        dispositions: list[dict],
        *,
        outcome: str = "passed",
        actions: list[dict] | None = None,
    ) -> dict:
        return {
            "schema": 1,
            "outcome": outcome,
            "observed_at": now(),
            "policy_revision": self.revision,
            "scope": {"repository": str(self.root), "state": str(self.state)},
            "dispositions": dispositions,
            "actions": actions or [],
            "accounting_limits": [
                "native/shared stores are not exclusively attributed",
                "allocated bytes may include shared extents",
            ],
        }

    def retire(self, object_id: str, *, references: list[tuple[Path, str]] | None = None) -> dict:
        initial = self.get(object_id)
        path = Path(initial["path"])
        if initial.get("retired_at"):
            return {"id": object_id, "action": "already-retired"}
        guarded = initial
        finalizing = False
        if initial.get("retirement"):
            journal = read_json(self.state / "journals" / f"{object_id}.json")
            quarantine = path.parent / (".lctx-retired-" + object_id)
            if journal.get("nonce") != initial["nonce"] or journal.get("quarantine") != str(
                quarantine
            ):
                raise Blocked("recovery journal scope changed")
            owner_path = Path(initial["owner"].get("path", path))
            if quarantine.exists() and (owner_path == path or path in owner_path.parents):
                guarded = {
                    **initial,
                    "owner": {
                        **initial["owner"],
                        "path": str(quarantine / owner_path.relative_to(path)),
                    },
                }
            finalizing = not quarantine.exists() and journal.get("phase") in {"renamed", "deleted"}
        with (
            admission([path], exclusive=True, blocking=False, state=self.state),
            contextlib.nullcontext() if finalizing else owner_guard(guarded, self.host),
            self.metadata(),
        ):
            row = self.get(object_id)
            if row.get("retired_at"):
                return {"id": object_id, "action": "already-retired"}
            if row.get("retirement"):
                return self._recover(row)
            decision = self.disposition(
                row, references=references if references is not None else self.references()
            )
            if decision["disposition"] != "eligible":
                raise Blocked("; ".join(decision["reasons"]))
            if any(
                item["requires"] == "restorable-replay" and not item.get("released_at")
                for item in row["obligations"].values()
            ):
                archive = row.get("archive") or {}
                with Path(archive["path"]).open("rb") as source:
                    if hashlib.file_digest(source, "sha256").hexdigest() != archive["sha256"]:
                        raise Blocked("archive bytes changed; original raw retained")
            # Refuse mount boundaries/symlinks before moving anything. Leaf links are removed
            # as links by fd-relative cleanup, never followed; directory mounts block.
            _preflight(path, row["identity"]["device"])
            quarantine = path.parent / (".lctx-retired-" + object_id)
            if quarantine.exists() or quarantine.is_symlink():
                raise Blocked("retirement destination already exists")
            journal = {
                "schema": 1,
                "id": object_id,
                "original": str(path),
                "quarantine": str(quarantine),
                "identity": row["identity"],
                "nonce": row["nonce"],
                "policy_revision": self.revision,
                "phase": "prepared",
                "observed_at": now(),
            }
            durable_json(self.state / "journals" / f"{object_id}.json", journal)
            row["retirement"] = True
            self.save(row)
            os.rename(path, quarantine)
            fsync_directory(path.parent)
            journal["phase"] = "renamed"
            durable_json(self.state / "journals" / f"{object_id}.json", journal)
            return self._recover(row)

    def _recover(self, row: dict) -> dict:
        journal_path = self.state / "journals" / f"{row['id']}.json"
        journal = read_json(journal_path)
        if (
            journal.get("schema") != 1
            or journal.get("nonce") != row["nonce"]
            or journal.get("identity") != row["identity"]
        ):
            raise Blocked("journal identity mismatch")
        original, quarantine = Path(journal["original"]), Path(journal["quarantine"])
        if str(original) != row["path"] or quarantine != original.parent / (
            ".lctx-retired-" + row["id"]
        ):
            raise Blocked("journal boundary mismatch")
        if not quarantine.exists():
            if journal["phase"] == "prepared" and original.exists():
                if physical(original) != row["identity"]:
                    raise Blocked("prepared original identity changed")
                if _nonce(original) != row["nonce"]:
                    raise Blocked("prepared original lifetime nonce changed")
                row.pop("retirement", None)
                self.save(row)
                journal_path.unlink()
                return {"id": row["id"], "action": "prepared-retirement-cancelled"}
            if journal["phase"] not in {"renamed", "deleted"}:
                raise Blocked("missing retirement target")
        else:
            if physical(quarantine) != row["identity"] or _nonce(quarantine) != row["nonce"]:
                raise Blocked("quarantine identity changed")
            _delete(quarantine, row["identity"]["device"])
        journal["phase"] = "deleted"
        durable_json(journal_path, journal)
        row.pop("retirement", None)
        row["retired_at"] = now()
        self.save(row)
        journal_path.unlink()
        fsync_directory(journal_path.parent)
        return {"id": row["id"], "action": "retired", "path": row["path"]}

    def sweep(self, ids: Sequence[str] = ()) -> dict:
        refs = self.references()
        decisions = sorted(
            self.plan(ids)["dispositions"],
            key=lambda row: len(Path(row["path"]).parts),
            reverse=True,
        )
        actions = []
        for decision in decisions:
            row = self.get(decision["id"])
            if self.disposition(row, references=refs)["disposition"] != "eligible" and not row.get(
                "retirement"
            ):
                continue
            try:
                actions.append(self.retire(row["id"], references=refs))
            except (Blocked, OSError, Invalid) as exc:
                actions.append({"id": row["id"], "action": "skipped", "reason": str(exc)})
        result = self.envelope(decisions, actions=actions)
        durable_json(self.state / "last-sweep.json", result)
        durable_json(self.state / "actions" / f"{uuid.uuid4()}.json", result)
        self.trim_metadata(refs)
        return result

    def trim_metadata(self, references: list[tuple[Path, str]]) -> None:
        """Only manager-owned closed receipts expire; unknown journals never do."""
        clock = dt.datetime.now(dt.UTC)
        with self.metadata():
            for path in (self.state / "actions").glob("*.json"):
                try:
                    uuid.UUID(path.stem)
                    row = read_json(path)
                    if clock - instant(row["observed_at"]) > dt.timedelta(
                        days=self.policy.get("maintenance", {}).get("action_receipt_days", 30)
                    ):
                        path.unlink()
                except OSError, Invalid, ValueError, KeyError:
                    continue
            rows = self.records()
            for row in rows:
                if (
                    not row.get("retired_at")
                    or row.get("archive")
                    or (self.state / "journals" / f"{row['id']}.json").exists()
                ):
                    continue
                if clock - instant(row["retired_at"]) <= dt.timedelta(
                    days=self.policy.get("maintenance", {}).get("tombstone_days", 90)
                ):
                    continue
                if any(
                    text.startswith("__") or row["id"] in text or row["path"] in text
                    for _, text in references
                ):
                    continue
                if any(child.get("parent") == row["id"] for child in rows):
                    continue
                (self.state / "objects" / f"{row['id']}.json").unlink()
                (self.state / "queue" / f"{row['id']}.json").unlink(missing_ok=True)
                key = hashlib.sha256(os.fsencode(row["path"])).hexdigest()
                index = self.state / "scopes" / f"{key}.json"
                if index.exists() and read_json(index).get("id") == row["id"]:
                    index.unlink()
                path_index = self.state / "paths" / f"{key}.json"
                if path_index.exists() and read_json(path_index).get("id") == row["id"]:
                    path_index.unlink()
                owner = Path(row["owner"].get("path", "/"))
                if (
                    row["owner"]["kind"] == "docs"
                    and owner.parent == self.state / "owners/docs"
                    and owner.is_dir()
                    and not any(
                        other["id"] != row["id"] and other["owner"].get("path") == str(owner)
                        for other in rows
                    )
                ):
                    _delete(owner, owner.stat().st_dev)


def _mountpoints() -> set[Path]:
    def unescape(text: str) -> str:
        return re.sub(r"\\([0-7]{3})", lambda match: chr(int(match[1], 8)), text)

    return {
        Path(unescape(line.split()[4]))
        for line in Path("/proc/self/mountinfo").read_text().splitlines()
    }


def _preflight(path: Path, device: int, mounts: set[Path] | None = None) -> None:
    mounts = _mountpoints() if mounts is None else mounts
    info = path.lstat()
    if info.st_dev != device or path in mounts or os.path.ismount(path):
        raise Blocked("mount boundary in retirement scope")
    if stat.S_ISDIR(info.st_mode):
        for child in path.iterdir():
            _preflight(child, device, mounts)


def _delete(path: Path, device: int) -> None:
    """fd-relative recursion; refuse mounts and never follow directory links."""
    parent = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:

        def remove(fd: int, name: str) -> None:
            info = os.stat(name, dir_fd=fd, follow_symlinks=False)
            if info.st_dev != device:
                raise Blocked("mount boundary encountered during deletion")
            if stat.S_ISDIR(info.st_mode):
                child = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
                try:
                    opened = os.fstat(child)
                    if (opened.st_dev, opened.st_ino) != (info.st_dev, info.st_ino):
                        raise Blocked("directory identity changed")

                    def mount_id(descriptor: int) -> str:
                        return next(
                            line.split()[1]
                            for line in Path(f"/proc/self/fdinfo/{descriptor}")
                            .read_text()
                            .splitlines()
                            if line.startswith("mnt_id:")
                        )

                    if mount_id(child) != mount_id(fd):
                        raise Blocked("mount boundary encountered during deletion")
                    for entry in os.listdir(child):
                        remove(child, entry)
                    os.fsync(child)
                finally:
                    os.close(child)
                os.rmdir(name, dir_fd=fd)
            else:
                os.unlink(name, dir_fd=fd)

        remove(parent, path.name)
        os.fsync(parent)
    finally:
        os.close(parent)


def publish(
    path: Path,
    category: str,
    owner: dict,
    consumer: str | None,
    requires: str = "immediate-use",
    temporary_days: float | None = None,
    managed: bool = False,
    parent: str | None = None,
) -> str:
    return Storage(root=Path(owner.get("root", ROOT))).publish(
        path, category, owner, consumer, requires, temporary_days, managed, parent
    )


def complete(object_id: str, consumer: str, *, reason: str = "confirmed owner cleanup") -> dict:
    return Storage().complete(object_id, consumer, reason=reason)


def release(object_id: str, consumer: str, reason: str) -> dict:
    return Storage().release(object_id, consumer, reason)


@contextlib.contextmanager
def managed(
    path: Path,
    category: str,
    owner: dict,
    consumer: str,
    requires: str = "immediate-use",
    temporary_days: float | None = None,
) -> Iterator[str]:
    with admission([path]):
        path.mkdir(parents=True, exist_ok=True)
        yield publish(path, category, owner, consumer, requires, temporary_days, managed=True)
