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
    return {"path": str(target), "sha256": sha, "size": info.st_size}


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
