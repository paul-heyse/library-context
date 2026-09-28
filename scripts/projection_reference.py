"""Offline evaluation inputs, never an online query backend or compiler input."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pyarrow as pa
import pyarrow.ipc as ipc
from lctx_semantics import validate_projection_ipc


class ReferenceBundle:
    def __init__(self, root: Path):
        self.root = root
        self.manifest = json.loads((root / "MANIFEST.json").read_text())
        inputs = []
        total = 0
        for name, row in self.manifest["files"].items():
            path = root / row["file"]
            if (
                row["file"] != f"{name}.arrow"
                or path.is_symlink()
                or path.stat().st_size > 128 * 1024 * 1024
            ):
                raise ValueError("invalid offline reference path/size")
            data = path.read_bytes()
            total += len(data)
            if total > 512 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != row["sha256"]:
                raise ValueError("invalid offline reference bytes")
            inputs.append((name, data))
        identity = validate_projection_ipc(
            json.dumps(self.manifest["projection"]), inputs, json.dumps(self.manifest)
        )
        if identity != self.manifest["projection_generation"]:
            raise ValueError("offline reference identity mismatch")

    def table(self, name: str) -> pa.Table:
        entry = self.manifest["files"][name]
        data = (self.root / entry["file"]).read_bytes()
        if hashlib.sha256(data).hexdigest() != entry["sha256"]:
            raise ValueError("offline reference changed")
        return ipc.open_file(pa.BufferReader(data)).read_all()
