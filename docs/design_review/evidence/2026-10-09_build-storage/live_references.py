#!/usr/bin/env python3
"""Read-only same-user /proc references to explicit roots; JSON to stdout.

Matching references prove use. Absence is not proof of quiescence or permission
to delete. No command arguments, environment values or database contents are read.
"""

from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import os
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roots", nargs="+", type=Path)
    args = parser.parse_args()
    roots = [str(root.absolute()) for root in args.roots]
    device = Path(roots[0]).stat().st_dev
    references = []
    denied = 0
    deleted = {}
    for process in Path("/proc").iterdir():
        if not process.name.isdigit() or process.name == str(os.getpid()):
            continue
        try:
            if process.stat().st_uid != os.getuid():
                continue
            comm = (process / "comm").read_text().strip()
        except OSError:
            continue
        hits = collections.Counter()
        examples = []
        for name in ["cwd", "exe"]:
            try:
                value = os.readlink(process / name)
                for root in roots:
                    if value == root or value.startswith(root + "/"):
                        hits[(root, name)] += 1
            except OSError:
                pass
        try:
            for descriptor in (process / "fd").iterdir():
                try:
                    value = os.readlink(descriptor)
                    for root in roots:
                        if value.startswith(root + "/"):
                            hits[(root, "fd")] += 1
                            if len(examples) < 3:
                                examples.append(value)
                    if value.endswith(" (deleted)"):
                        info = descriptor.stat()
                        if info.st_dev == device and info.st_blocks * 512 > 1024**2:
                            deleted[(info.st_dev, info.st_ino)] = {
                                "allocated_bytes": info.st_blocks * 512,
                                "path": value,
                            }
                except OSError:
                    pass
        except PermissionError:
            denied += 1
        except OSError:
            pass
        try:
            for line in (process / "maps").read_text().splitlines():
                for root in roots:
                    if root + "/" in line:
                        hits[(root, "mapping")] += 1
        except OSError:
            pass
        if hits:
            references.append(
                {
                    "pid": int(process.name),
                    "comm": comm,
                    "references": [
                        {"root": root, "kind": kind, "count": count}
                        for (root, kind), count in hits.items()
                    ],
                    "examples": examples,
                }
            )
    print(
        json.dumps(
            {
                "time_utc": dt.datetime.now(dt.UTC).isoformat(),
                "references": references,
                "fd_permission_denials": denied,
                "same_filesystem_deleted_open_bytes": sum(
                    item["allocated_bytes"] for item in deleted.values()
                ),
                "same_filesystem_deleted_open_repo_bytes": sum(
                    item["allocated_bytes"]
                    for item in deleted.values()
                    if any(item["path"].startswith(root + "/") for root in roots)
                ),
                "limits": (
                    "Same-uid visible processes; inode-deduplicated deleted files "
                    "on root filesystem only; no absence/safe-to-delete inference"
                ),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
