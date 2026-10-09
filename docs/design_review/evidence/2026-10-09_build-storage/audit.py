#!/usr/bin/env python3
"""Read-only, single-filesystem storage census for the 2026-10-09 review.

Writes JSON only to stdout. Does not follow symlinks, execute workloads, hash large
files, or infer age/use/relevance from mtime. Arguments are explicit scan roots.
"""

from __future__ import annotations

import argparse
import collections
import datetime as dt
import heapq
import json
import os
import stat
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roots", nargs="+", type=Path)
    args = parser.parse_args()
    started = dt.datetime.now(dt.UTC).isoformat()
    roots = [p.absolute() for p in args.roots]
    # One record per physical inode, including counts of observed names by bucket.
    inodes: dict[tuple[int, int], dict] = {}
    buckets: dict[str, dict] = {}
    errors: list[dict] = []
    symlinks: list[dict] = []

    def bucket_for(index: int, relative: Path) -> str:
        first = relative.parts[0] if relative.parts else "."
        # Separate profiles and managed attempts for actionable attribution.
        depth = 2 if index == 2 and first == "runs" and len(relative.parts) > 1 else 1
        return f"{index}:" + "/".join(relative.parts[:depth] or (".",))

    for index, root in enumerate(roots):
        device = root.lstat().st_dev
        stack = [root]
        while stack:
            path = stack.pop()
            try:
                info = path.lstat()
                relative = path.relative_to(root)
                bucket = bucket_for(index, relative)
                result = buckets.setdefault(
                    bucket,
                    {
                        "path": str(root / relative.parts[0]) if relative.parts else str(root),
                        "root_index": index,
                        "entries": 0,
                        "files": 0,
                        "directory_entries_allocated_bytes": 0,
                        "types": {},
                        "largest_files": [],
                        "min_file_mtime": None,
                        "max_file_mtime": None,
                    },
                )
                if index == 2 and len(relative.parts) > 1 and relative.parts[0] == "runs":
                    result["path"] = str(root / "runs" / relative.parts[1])
                result["entries"] += 1
                if info.st_dev != device:
                    errors.append({"path": str(path), "reason": "different device skipped"})
                    continue
                if stat.S_ISLNK(info.st_mode):
                    result["symlink_allocated_bytes"] = (
                        result.get("symlink_allocated_bytes", 0) + info.st_blocks * 512
                    )
                    symlinks.append(
                        {
                            "path": str(path),
                            "target": os.readlink(path),
                            "allocated_bytes": info.st_blocks * 512,
                        }
                    )
                    continue
                allocated = info.st_blocks * 512
                if stat.S_ISDIR(info.st_mode):
                    result["directory_entries_allocated_bytes"] += allocated
                    with os.scandir(path) as entries:
                        stack.extend(Path(entry.path) for entry in entries)
                    continue
                if not stat.S_ISREG(info.st_mode):
                    continue
                result["files"] += 1
                result["min_file_mtime"] = min(
                    result["min_file_mtime"] or info.st_mtime, info.st_mtime
                )
                result["max_file_mtime"] = max(
                    result["max_file_mtime"] or info.st_mtime, info.st_mtime
                )
                if "perf.data" in path.name:
                    kind = "perf_data"
                elif path.suffix in {".mm_profdata", ".events", ".string_data", ".string_index"}:
                    kind = "rustc_self_profile"
                elif "incremental" in relative.parts:
                    kind = "incremental"
                else:
                    kind = "other"
                inode = inodes.setdefault(
                    (info.st_dev, info.st_ino),
                    {
                        "allocated": allocated,
                        "apparent": info.st_size,
                        "nlink": info.st_nlink,
                        "names": collections.Counter(),
                        "kind": kind,
                    },
                )
                inode["names"][bucket] += 1
                if allocated >= 100 * 1024**2:
                    item = (allocated, info.st_size, str(path))
                    if len(result["largest_files"]) < 12:
                        heapq.heappush(result["largest_files"], item)
                    elif item > result["largest_files"][0]:
                        heapq.heapreplace(result["largest_files"], item)
            except OSError as error:
                errors.append({"path": str(path), "reason": type(error).__name__})

    total_allocated = total_apparent = outside_links_allocated = shared_bucket_allocated = 0
    for inode in inodes.values():
        total_allocated += inode["allocated"]
        total_apparent += inode["apparent"]
        observed = sum(inode["names"].values())
        if observed < inode["nlink"]:
            outside_links_allocated += inode["allocated"]
        if len(inode["names"]) > 1:
            shared_bucket_allocated += inode["allocated"]
        for bucket, count in inode["names"].items():
            result = buckets[bucket]
            for field, value in [
                ("unique_file_allocated_bytes", inode["allocated"]),
                ("unique_file_apparent_bytes", inode["apparent"]),
            ]:
                result[field] = result.get(field, 0) + value
            by_kind = result["types"].setdefault(inode["kind"], {"allocated_bytes": 0, "inodes": 0})
            by_kind["allocated_bytes"] += inode["allocated"]
            by_kind["inodes"] += 1
            if count >= inode["nlink"]:
                result["exclusive_file_allocated_bytes"] = (
                    result.get("exclusive_file_allocated_bytes", 0) + inode["allocated"]
                )
            elif observed < inode["nlink"]:
                result["files_with_unobserved_links_bytes"] = (
                    result.get("files_with_unobserved_links_bytes", 0) + inode["allocated"]
                )
    for result in buckets.values():
        result["largest_files"] = [
            {"allocated_bytes": allocated, "apparent_bytes": apparent, "path": path}
            for allocated, apparent, path in sorted(result["largest_files"], reverse=True)
        ]
        for field in [
            "unique_file_allocated_bytes",
            "unique_file_apparent_bytes",
            "exclusive_file_allocated_bytes",
            "files_with_unobserved_links_bytes",
        ]:
            result.setdefault(field, 0)
        if result["root_index"] == 2 and "/runs/" in result["path"]:
            directory = Path(result["path"])
            result["retained"] = (directory / "retain").exists()
            try:
                record = json.loads((directory / "record.json").read_text())
                result["run"] = {
                    key: record[key]
                    for key in ["schema", "label", "kind", "started", "ended", "termination", "cwd"]
                    if key in record
                }
            except (OSError, ValueError):
                result["run"] = None

    fs = os.statvfs(roots[0])
    print(
        json.dumps(
            {
                "started_utc": started,
                "ended_utc": dt.datetime.now(dt.UTC).isoformat(),
                "roots": [str(root) for root in roots],
                "filesystem_available_bytes_at_end": fs.f_bavail * fs.f_frsize,
                "unique_regular_file_inodes": len(inodes),
                "unique_regular_file_allocated_bytes": total_allocated,
                "unique_regular_file_apparent_bytes": total_apparent,
                "files_with_unobserved_links_allocated_bytes": outside_links_allocated,
                "files_shared_across_buckets_allocated_bytes": shared_bucket_allocated,
                "buckets": dict(sorted(buckets.items())),
                "symlinks": symlinks,
                "errors": errors,
                "limits": [
                    "Live census, not an atomic snapshot",
                    "No symlink traversal",
                    "Allocated bytes are st_blocks * 512, excluding filesystem metadata",
                    "Reflink/shared-extents accounting not available from stat",
                    "Exclusive file bytes are an unlink upper bound, not a deletion recommendation",
                    "No creation-date, last-use, workload-liveness or relevance inference",
                ],
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
