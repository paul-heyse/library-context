"""Service-owned executable generations, independent of checkout and database payloads.

Only surrealdb_service calls effectful operations. The lifecycle manager consumes sanitized
dependencies; it never reads credentials or removes service generations.
"""

from __future__ import annotations

import hashlib
import os
import shutil
import uuid
from pathlib import Path

from storage_lifecycle import Blocked, durable_json, fsync_directory, private_directory, read_json


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def tools_root(directory: Path) -> Path:
    return directory.with_name(directory.name + ".tools")


def prepare(directory: Path, source: Path) -> dict:
    source = Path(source)
    if any(parent.is_symlink() for parent in (source, *source.parents)):
        raise Blocked("installer must have a physical regular path")
    if source.is_symlink() or not source.is_file() or not os.access(source, os.X_OK):
        raise Blocked("installer must be an executable regular file")
    info = source.stat()
    identity = (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns)
    sha = digest(source)
    root = tools_root(directory)
    if any(parent.is_symlink() for parent in (root, *root.parents)):
        raise Blocked("service tools must have a physical path")
    for path in (root, root / "lctx", root / "lctx" / sha):
        private_directory(path)
    target = root / "lctx" / sha / "lctx"
    if target.exists() or target.is_symlink():
        if (
            target.is_symlink()
            or not target.is_file()
            or digest(target) != sha
            or not os.access(target, os.X_OK)
        ):
            raise Blocked("service executable generation identity conflicts")
    else:
        temporary = target.with_name(".lctx-" + uuid.uuid4().hex)
        fd = os.open(temporary, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o500)
        try:
            with os.fdopen(fd, "wb") as output, source.open("rb") as input_stream:
                shutil.copyfileobj(input_stream, output, length=1024**2)
                output.flush()
                os.fsync(output.fileno())
            after = source.stat()
            if (
                identity != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
                or digest(temporary) != sha
            ):
                raise Blocked("installer changed during owned transfer")
            os.link(temporary, target)
            fsync_directory(target.parent)
        finally:
            temporary.unlink(missing_ok=True)
    # The marker may durably name this generation immediately after return. Persist each
    # newly linked ancestor, as well as the binary link, before exposing that reference.
    for path in (target.parent, target.parent.parent, root, root.parent):
        fsync_directory(path)
    return {"path": str(target), "sha256": sha, "size": info.st_size}


def prepare_server(directory: Path, source: Path, provenance: dict) -> dict:
    """Publish an immutable, recipe-bound server without changing the installed service."""
    import json
    import stat

    import surrealdb_server
    from storage_lifecycle import Storage, admission

    surrealdb_server.validate_provenance(provenance, directory=directory)
    source = Path(source)
    if any(parent.is_symlink() for parent in (source, *source.parents)):
        raise Blocked("server executable must have a physical regular path")
    if not source.is_file() or not os.access(source, os.X_OK):
        raise Blocked("server must be an executable regular file")
    with admission([source]):
        info = source.stat()
        before = (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns)
        source_digest = digest(source)
    body = {
        "schema": 1,
        "kind": "library-context-surrealdb-server",
        "sha256": source_digest,
        "size": info.st_size,
        "version": provenance["recipe"]["pin"]["cli_version"],
        "http_version": provenance["recipe"]["pin"]["http_version"],
        "provenance": provenance,
    }
    identity = hashlib.sha256(json.dumps(body, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    root = tools_root(directory) / "surreal" / identity
    descriptor = {**body, "identity": identity, "path": str(root / "surreal"),
                  "descriptor_path": str(root / "record.json")}
    if any(parent.is_symlink() for parent in (root, *root.parents)):
        raise Blocked("server generation must have a physical path")
    with admission([root, source], exclusive_paths=[root]):
        for path in (tools_root(directory), root.parent, root):
            private_directory(path)
        target = root / "surreal"
        if target.exists() or target.is_symlink():
            if (target.is_symlink() or not target.is_file() or digest(target) != body["sha256"]
                    or target.stat().st_mode & 0o777 != 0o500):
                raise Blocked("server executable generation identity conflicts")
        else:
            temporary = root / (".surreal-" + uuid.uuid4().hex)
            fd = os.open(temporary, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o500)
            try:
                with os.fdopen(fd, "wb") as output, source.open("rb") as input_stream:
                    shutil.copyfileobj(input_stream, output, length=1024**2)
                    output.flush()
                    os.fsync(output.fileno())
                after = source.stat()
                if (before != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
                        or digest(temporary) != body["sha256"]):
                    raise Blocked("server changed during owned transfer")
                os.link(temporary, target)
            finally:
                temporary.unlink(missing_ok=True)
        marker = root / "record.json"
        if not marker.exists() and not marker.is_symlink():
            durable_json(marker, descriptor)
        try:
            fd = os.open(marker, os.O_RDONLY | os.O_NOFOLLOW)
        except OSError as error:
            raise Blocked("server provenance marker must be an owned regular file") from error
        with os.fdopen(fd) as stream:
            marker_info = os.fstat(stream.fileno())
            if not stat.S_ISREG(marker_info.st_mode) or marker_info.st_uid != os.getuid():
                raise Blocked("server provenance marker must be an owned regular file")
            if json.load(stream) != descriptor:
                raise Blocked("server provenance generation identity conflicts")
            # A crash after durable_json but before chmod leaves an equal marker at 0600.
            # Repair only that exact owned generation, on the same validated descriptor.
            os.fchmod(stream.fileno(), 0o400)
            os.fsync(stream.fileno())
        for path in (root, root.parent, root.parent.parent, root.parent.parent.parent):
            fsync_directory(path)
        Storage().publish(root, "native-content", {"kind": "native", "path": str(root)},
                          "service-server-generation:" + identity, requires="raw-replay")
    return descriptor


def dependencies(installation) -> dict:
    row = installation.record
    selected = Path(row["installer"])
    generation = row.get("installer_generation", {})
    stable = (
        generation.get("path") == str(selected)
        and selected
        == tools_root(installation.directory) / "lctx" / generation.get("sha256", "") / "lctx"
    )
    result = [
        {
            "path": str(selected),
            "sha256": generation.get("sha256"),
            "role": "maintenance-executable",
        }
    ]
    marker = installation.directory / "installer-transfer.json"
    if marker.exists():
        transfer = read_json(marker)
        for key in ("previous", "replacement"):
            if isinstance(transfer.get(key), str):
                result.append({"path": transfer[key], "role": "unresolved-installer-transfer"})
    current_upgrade = row.get("native_upgrade", {})
    for key in ("previous_installer", "journal"):
        if isinstance(current_upgrade.get(key), str):
            result.append({"path": current_upgrade[key], "role": "native-upgrade-evidence"})
    def predecessors(descriptors):
        from surrealdb_service import _upgrade_private_plan
        if not isinstance(descriptors, list):
            raise Blocked("native upgrade predecessor inventory is unavailable")
        seen = set()
        for descriptor in descriptors:
            if not isinstance(descriptor, dict) or descriptor.get("operation") in seen:
                raise Blocked("native upgrade predecessor identity is unavailable")
            seen.add(descriptor.get("operation"))
            journal, _ = _upgrade_private_plan(installation, descriptor)
            for key in ("candidate", "previous_installer", "journal"):
                result.append({"path": descriptor[key], "role": "native-upgrade-predecessor"})
            result.append({"path": str(journal.parent), "role": "native-upgrade-private-assets"})
    def plan_predecessors(upgrade, *, candidate=None):
        from surrealdb_service import _upgrade_private_plan
        if not upgrade.get("journal"):
            return
        descriptor = {**upgrade, "candidate": candidate or upgrade.get("candidate")}
        journal, plan = _upgrade_private_plan(installation, descriptor)
        if upgrade.get("predecessors", []) != plan.get("predecessors", []):
            raise Blocked("native upgrade predecessor inventory differs from journal")
        result.append({"path": str(journal.parent), "role": "native-upgrade-private-assets"})
        predecessors(plan.get("predecessors", []))
    plan_predecessors(current_upgrade, candidate=str(selected))
    try:
        maintenance = read_json(installation.directory / "maintenance.json")
    except FileNotFoundError:
        maintenance = None
    if isinstance(maintenance, dict) and maintenance.get("operation") == "native-schema-upgrade":
        upgrade = maintenance.get("upgrade", {})
        for key in ("previous_installer", "candidate", "journal"):
            if isinstance(upgrade.get(key), str):
                result.append({"path": upgrade[key], "role": "unresolved-native-upgrade"})
        plan_predecessors(upgrade)
    from surrealdb_service import _server_handoff_dependencies

    result.extend(_server_handoff_dependencies(installation))
    return {"installation_id": installation.id, "stable_installer": stable, "dependencies": result}


def stabilize(installation) -> dict:
    import surrealdb_service as service

    previous = str(installation.record["installer"])
    marker = installation.directory / "installer-transfer.json"
    # One existing service maintenance boundary drains borrowers/native work and preserves
    # its failure marker. Never nest maintenance or alter executable/schema identity.
    with service.maintenance(installation):
        generation = prepare(installation.directory, Path(previous))
        durable_json(
            marker,
            {
                "schema": 1,
                "previous": previous,
                "replacement": generation["path"],
                "sha256": generation["sha256"],
            },
        )
        installation.record["installer"] = generation["path"]
        installation.record["installer_generation"] = generation
        service._run_installer(installation, Path(generation["path"]), "check")
        durable_json(installation.directory / "installation.json", installation.record)
    marker.unlink(missing_ok=True)
    fsync_directory(installation.directory)
    return {"outcome": "passed", **dependencies(installation)}


def rebind_restored(record: dict, current: dict, archive_digest: str) -> dict:
    """Old archives may name removed checkouts; only same-digest rebinding is allowed."""
    generation = current.get("installer_generation")
    if not generation:
        return record
    path = Path(generation["path"])
    if (
        generation.get("sha256") != archive_digest
        or path.is_symlink()
        or digest(path) != archive_digest
    ):
        raise Blocked("restored installer differs from current qualified generation")
    return {**record, "installer": str(path), "installer_generation": generation}
