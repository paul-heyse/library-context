"""Closed, verified evidence bundles; retirement remains the lifecycle owner's operation.

Python 3.14's zstandard codec writes .tar.zst without acquiring a compressor. Archives
contain only manifest-bounded regular files/directories, never links or special files.
"""

from __future__ import annotations

import ctypes
import hashlib
import io
import json
import os
import shutil
import stat
import tarfile
import tempfile
import time
from collections.abc import Callable
from compression import zstd
from pathlib import Path, PurePosixPath

SCHEMA = 1
MANIFEST = "storage-manifest.json"
MAX_MANIFEST = 16 * 1024**2
MAX_ENTRIES = 200_000


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024**2), b""):
            h.update(block)
    return h.hexdigest()


def _name(name: str) -> str:
    path = PurePosixPath(name)
    if not name or name == "." or path.is_absolute() or ".." in path.parts or str(path) != name:
        raise ValueError(f"unsafe archive entry: {name!r}")
    if name == MANIFEST:
        raise ValueError("reserved manifest name")
    return name


def _sync_dir(path: Path) -> None:
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def inventory(source: Path) -> list[dict]:
    source = source.resolve(strict=True)
    from storage_lifecycle import _mountpoints

    device = source.stat().st_dev
    mounts = _mountpoints()
    if source in mounts:
        raise ValueError("filesystem mount boundary at archive source")
    entries = []
    for base, dirs, files in os.walk(source, followlinks=False):
        for name in sorted(dirs + files):
            path = Path(base) / name
            relative = _name(path.relative_to(source).as_posix())
            info = path.lstat()
            if info.st_dev != device or path in mounts:
                raise ValueError(f"filesystem boundary: {relative}")
            if stat.S_ISLNK(info.st_mode):
                # Reader-created links are derived and are rebuilt after restore. No other
                # links are silently omitted, including links in cited raw evidence.
                if "reports" in path.relative_to(source).parts:
                    continue
                raise ValueError(f"link is not archiveable: {relative}")
            if not (stat.S_ISDIR(info.st_mode) or stat.S_ISREG(info.st_mode)):
                raise ValueError(f"special file is not archiveable: {relative}")
            entry = {
                "path": relative,
                "kind": "directory" if path.is_dir() else "file",
                "mode": stat.S_IMODE(info.st_mode),
                "mtime_ns": info.st_mtime_ns,
                "size": 0 if path.is_dir() else info.st_size,
            }
            if entry["kind"] == "file":
                entry["sha256"] = digest(path)
            entries.append(entry)
            if len(entries) > MAX_ENTRIES:
                raise ValueError("archive manifest entry bound exceeded")
    return sorted(entries, key=lambda entry: entry["path"])


def _extract(archive: Path, stage: Path) -> dict:
    try:
        return _extract_checked(archive, stage)
    except (
        tarfile.TarError,
        zstd.ZstdError,
        EOFError,
        KeyError,
        TypeError,
        AttributeError,
    ) as error:
        raise ValueError(f"invalid evidence archive: {error}") from error


def _extract_checked(archive: Path, stage: Path) -> dict:
    """Validate stream membership and every byte before publishing any restored root."""
    with zstd.open(archive, "rb") as compressed, tarfile.open(fileobj=compressed, mode="r|") as tar:
        first = tar.next()
        if not first or first.name != MANIFEST or not first.isfile() or first.size > MAX_MANIFEST:
            raise ValueError("missing or oversized archive manifest")
        manifest_stream = tar.extractfile(first)
        if manifest_stream is None:
            raise ValueError("archive manifest body missing")
        with manifest_stream:
            manifest = json.loads(manifest_stream.read())
        if manifest.get("schema") != SCHEMA:
            raise ValueError("unknown archive schema")
        entries = manifest.get("entries")
        if not isinstance(entries, list) or len(entries) > MAX_ENTRIES:
            raise ValueError("invalid archive entries")
        expected = {}
        for entry in entries:
            name = _name(entry["path"])
            if name in expected or entry.get("kind") not in ("file", "directory"):
                raise ValueError("duplicate or invalid manifest entry")
            if not isinstance(entry.get("size"), int) or entry["size"] < 0:
                raise ValueError("invalid manifest size")
            expected[name] = entry
        seen = set()
        seen_manifest = False
        device = stage.stat().st_dev
        for member in tar:
            if member.name == MANIFEST:
                if seen_manifest:
                    raise ValueError("duplicate archive manifest")
                seen_manifest = True
                continue  # tar iteration includes the already-read first header
            name = _name(member.name.rstrip("/") if member.isdir() else member.name)
            if name in seen or name not in expected:
                raise ValueError("duplicate or undeclared archive entry")
            entry = expected[name]
            if not (
                (member.isfile() and entry["kind"] == "file")
                or (member.isdir() and entry["kind"] == "directory")
            ):
                raise ValueError("archive link/type mismatch")
            if member.size != entry["size"]:
                raise ValueError("archive size differs from manifest")
            target = stage / name
            target.parent.mkdir(parents=True, exist_ok=True)
            if target.parent.stat().st_dev != device:
                raise ValueError("restore crosses filesystem")
            if member.isdir():
                target.mkdir(exist_ok=True)
            else:
                h = hashlib.sha256()
                source = tar.extractfile(member)
                if source is None:
                    raise ValueError(f"archive member body missing: {name}")
                with source as src, target.open("xb") as dst:
                    for block in iter(lambda: src.read(1024**2), b""):
                        dst.write(block)
                        h.update(block)
                    dst.flush()
                    os.fsync(dst.fileno())
                if h.hexdigest() != entry.get("sha256"):
                    raise ValueError(f"archive digest differs: {name}")
            if entry["kind"] == "file":
                os.chmod(target, entry["mode"] & 0o777)
                os.utime(target, ns=(entry["mtime_ns"], entry["mtime_ns"]))
            seen.add(name)
        if seen != set(expected):
            raise ValueError("archive is truncated or missing declared entries")
        # Consume the compressed tail too; zstd checks frame integrity beyond tar's end marker.
        while compressed.read(1024**2):
            pass
    return manifest


def restore_archive(archive: Path, destination: Path) -> dict:
    archive, destination = Path(archive).resolve(strict=True), Path(destination).absolute()
    if destination.exists() or destination.is_symlink():
        raise ValueError("restore destination already exists")
    destination.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=".storage-restore-", dir=destination.parent))
    try:
        manifest = _extract(archive, stage)
        # Directory modes/times may have changed while children were extracted.
        for entry in reversed(manifest["entries"]):
            path = stage / entry["path"]
            os.chmod(path, entry["mode"] & 0o777)
            os.utime(path, ns=(entry["mtime_ns"], entry["mtime_ns"]))
        _sync_dir(stage)
        # Linux renameat2 no-replace closes the check/rename race without publishing
        # into a concurrent empty directory with somebody else's identity.
        libc = ctypes.CDLL(None, use_errno=True)
        result = libc.renameat2(-100, os.fsencode(stage), -100, os.fsencode(destination), 1)
        if result:
            error = ctypes.get_errno()
            raise OSError(error, os.strerror(error), str(destination))
        _sync_dir(destination.parent)
        return {
            "outcome": "passed",
            "destination": str(destination),
            "manifest": manifest,
            "temporary_raw_days": 14,
            "archive_sha256": digest(archive),
        }
    finally:
        if stage.exists():
            shutil.rmtree(stage)


def create_archive(
    source: Path,
    destination: Path,
    *,
    replay: Callable[[Path], dict] | None = None,
    required_capabilities: tuple[str, ...] = (),
    identity: dict | None = None,
) -> dict:
    """Publish only closed/verified archives; never release or delete source evidence.

    The profile owner supplies replay, not a configurable executable or policy callback.
    Absence of actual reader qualification leaves all replay capabilities unqualified.
    """
    source, destination = Path(source).resolve(strict=True), Path(destination).absolute()
    if not source.is_dir() or not destination.name.endswith(".tar.zst"):
        raise ValueError("directory source and .tar.zst destination required")
    if destination.exists() or destination.is_symlink() or destination.is_relative_to(source):
        raise ValueError("conflicting or nested archive destination")
    destination.parent.mkdir(parents=True, exist_ok=True)
    entries = inventory(source)
    manifest = {
        "schema": SCHEMA,
        "original_root": str(source),
        "identity": identity or {},
        "created_ns": time.time_ns(),
        "entries": entries,
    }
    raw_manifest = json.dumps(manifest, sort_keys=True).encode()
    if len(raw_manifest) > MAX_MANIFEST:
        raise ValueError("archive manifest exceeds bound")
    fd, temporary_name = tempfile.mkstemp(prefix=".storage-archive-", dir=destination.parent)
    os.close(fd)
    temporary = Path(temporary_name)
    try:
        with (
            zstd.open(temporary, "wb") as compressed,
            tarfile.open(fileobj=compressed, mode="w|") as tar,
        ):
            header = tarfile.TarInfo(MANIFEST)
            header.size, header.mode = len(raw_manifest), 0o600
            tar.addfile(header, io.BytesIO(raw_manifest))
            for entry in entries:
                path = source / entry["path"]
                # Validate again at the read boundary; owner admission prevents legitimate writers.
                if path.is_symlink() or path.stat().st_dev != source.stat().st_dev:
                    raise ValueError("source topology changed during archive")
                info = tar.gettarinfo(str(path), arcname=entry["path"])
                if info.islnk():
                    info.type = (
                        tarfile.REGTYPE
                    )  # preserve bytes, not cross-member hardlink semantics
                    info.linkname, info.size = "", entry["size"]
                with path.open("rb") if entry["kind"] == "file" else io.BytesIO() as stream:
                    tar.addfile(info, stream if entry["kind"] == "file" else None)
        with temporary.open("rb") as stream:
            os.fsync(stream.fileno())
        with tempfile.TemporaryDirectory(
            prefix=".storage-verify-", dir=destination.parent
        ) as staging:
            restored = Path(staging) / "restored"
            result = restore_archive(temporary, restored)
            qualification = (
                replay(restored) if replay else {"capabilities": {}, "outcome": "not_run"}
            )
            capabilities = qualification.get("capabilities", {})
            qualified = qualification.get("isolation", {}).get("outcome") == "passed" and all(
                capabilities.get(name, {}).get("outcome") == "passed"
                for name in required_capabilities
            )
            if required_capabilities and not qualified:
                return {
                    "outcome": "blocked",
                    "reason": "required replay capability unqualified",
                    "replay": qualification,
                    "source_preserved": True,
                }
            # Hash comparison catches source changes even when no reader was requested.
            if inventory(source) != entries:
                raise ValueError("source changed during archive")
            if destination.exists():
                raise ValueError("archive destination appeared during verification")
            os.link(
                temporary, destination
            )  # exclusive publication, cannot overwrite a concurrent file
            _sync_dir(destination.parent)
            return {
                "outcome": "passed",
                "path": str(destination),
                "sha256": result["archive_sha256"],
                "manifest": manifest,
                "replay": qualification,
                "source_preserved": True,
                "replay_qualified": bool(required_capabilities) and qualified,
            }
    finally:
        temporary.unlink(missing_ok=True)
