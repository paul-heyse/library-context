"""Lightweight source observations, independent of profiling and binary provenance."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

EXCLUDED = {"fixtures", "gold", "heldout", ".venv", "target", "build", ".git"}
SENSITIVE = {"credentials", "secrets", "private", "id_rsa", "id_ed25519", ".env"}


def allowed(name: str) -> bool:
    parts = Path(name).parts
    return not EXCLUDED.intersection(parts) and not any(
        part.lower() in SENSITIVE
        or any(word in part.lower() for word in ("credential", "secret", "password", "token"))
        or part in (".ssh", ".aws", ".gnupg")
        or part.startswith((".venv", ".env."))
        or part.endswith((".pem", ".key", ".p12"))
        for part in parts
    )


def digest_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


class SourceObserver:
    """Reuse unchanged file digests; check stat before and after hashing changed bytes."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.cache: dict[str, tuple[tuple[int, ...], str]] = {}

    def git(self, *args: str) -> bytes:
        return subprocess.run(["git", *args], cwd=self.root, capture_output=True, check=True).stdout

    def observe(self) -> dict[str, Any]:
        try:
            names = self.git("ls-files", "-z", "--cached", "--others", "--exclude-standard")
            head = self.git("rev-parse", "HEAD").decode().strip()
        except (OSError, subprocess.CalledProcessError) as error:
            return {"available": False, "reason": type(error).__name__}
        hashes = {}
        unavailable = {}
        excluded = []
        for name in sorted(set(names.decode(errors="surrogateescape").split("\0")) - {""}):
            if not allowed(name):
                excluded.append(name)
                continue
            path = self.root / name
            try:
                if path.is_symlink():
                    unavailable[name] = "symlink excluded"
                    continue
                stat = path.stat()
                identity = (
                    stat.st_dev,
                    stat.st_ino,
                    stat.st_size,
                    stat.st_mtime_ns,
                    stat.st_ctime_ns,
                )
                previous = self.cache.get(name)
                digest = previous[1] if previous and previous[0] == identity else digest_file(path)
                after = path.stat()
                if identity != (
                    after.st_dev,
                    after.st_ino,
                    after.st_size,
                    after.st_mtime_ns,
                    after.st_ctime_ns,
                ):
                    unavailable[name] = "changed during observation"
                    continue
                self.cache[name] = (identity, digest)
                hashes[name] = digest
            except OSError as error:
                unavailable[name] = type(error).__name__
        content = {"head": head, "files": hashes, "unavailable": unavailable, "excluded": excluded}
        identity = hashlib.sha256(json.dumps(content, sort_keys=True).encode()).hexdigest()
        return {
            "available": True,
            "id": identity,
            **content,
            "source_to_binary": "unknown; source observations do not establish build closure",
            "concurrent_edits": "intermediate changes between observations may be unobserved",
        }
