"""Compact live run discovery; complete safety inventory remains explicit."""

from __future__ import annotations

import base64
import json
from pathlib import Path
from typing import Any

from harness import ProcessIdentity, ProcessSnapshot


def _token(value: dict) -> str:
    return base64.urlsafe_b64encode(json.dumps(value, sort_keys=True).encode()).decode()


def _decode(value: str) -> dict:
    try:
        result = json.loads(base64.urlsafe_b64decode(value))
        if not isinstance(result, dict):
            raise ValueError
    except (ValueError, TypeError) as error:
        raise ValueError("invalid run-list cursor") from error
    if result.get("schema") != 2:
        raise ValueError("obsolete run-list cursor schema; restart listing")
    return result


def _valid_metadata(path: Path, record) -> bool:
    if not isinstance(record, dict) or record.get("id") != path.name:
        return False
    try:
        ProcessIdentity.from_json(record["owner"])
        return True
    except KeyError, TypeError, ValueError:
        return False


def _summary(path: Path, snapshot: ProcessSnapshot) -> dict:
    import runs

    row = runs.view(path, snapshot=snapshot)
    argv = row.get("argv") or []
    command = row.get("label") or " ".join(map(str, argv)) or "unknown"
    corrupt = not _valid_metadata(path, runs.load_record(path))
    return {
        "id": path.name,
        "label": row.get("label"),
        "command": command[:160],
        "state": row["state"],
        "started": row.get("started"),
        "ended": row.get("ended"),
        "exit": row.get("exit"),
        "cleanup": {"status": row["cleanup"]["status"]},
        "host_cleanup": {"status": row["host_cleanup"]["status"]},
        "owner_live": row["owner_live"],
        "protected": row["protected"],
        "retained": row["retained"],
        "survivors": row.get("survivors", []),
        "active": row["owner_live"]
        or row["group_observation"]["status"] == "owned"
        or row["host_cleanup"].get("delegated", {}).get("status") == "active",
        "corrupt": corrupt,
    }


def page(args) -> dict:
    import runs

    limit = getattr(args, "limit", 20)
    complete = getattr(args, "all", False)
    cursor = getattr(args, "cursor", None)
    if limit < 1:
        raise ValueError("--limit must be positive")
    if complete and cursor:
        raise ValueError("--all does not accept --cursor")
    query = {"label": getattr(args, "label", None), "state": getattr(args, "state", None)}
    root = runs.runs_root()
    paths = runs.all_runs()
    names = [path.name for path in paths]
    identity = [root.stat().st_dev, root.stat().st_ino] if root.exists() else None
    bound = {
        "schema": 2,
        "root": identity,
        "query": query,
        "anchor": names[0] if names else None,
    }
    after = None
    if cursor:
        prior = _decode(cursor)
        if any(prior.get(key) != bound[key] for key in ("root", "query")):
            raise ValueError("stale run-list cursor: root or selection changed; restart listing")
        anchor, after = prior.get("anchor"), prior.get("after")
        if not isinstance(anchor, str) or not isinstance(after, str) or after > anchor:
            raise ValueError("invalid run-list cursor position")
        bound["anchor"] = anchor
        # Membership is live: deleted anchors need not survive. Newer entries
        # belong to a fresh traversal; backdated entries in the remainder may appear.
        paths = [path for path in paths if path.name <= anchor and path.name < after]
    began = runs.now()
    snapshot = None
    rows: list[dict[str, Any]] = []
    examined = 0
    last = None
    hydrated = 0
    for path in paths:
        last = path.name
        examined += 1
        # Label is authored once. Unknown/corrupt metadata remains visible because
        # a missing label cannot prove that this damaged receipt is unrelated.
        record = None if path.is_symlink() else runs.load_record(path)
        if (
            query["label"]
            and isinstance(record, dict)
            and _valid_metadata(path, record)
            and "label" in record
            and (record["label"] is None or isinstance(record["label"], str))
            and record["label"] != query["label"]
        ):
            continue
        if snapshot is None:
            snapshot = ProcessSnapshot.capture()
        row: dict[str, Any]
        hydrated += 1
        try:
            if path.is_symlink():
                raise ValueError("symlink receipt is not an owned run")
            row = _summary(path, snapshot)
        except (OSError, ValueError, KeyError, TypeError) as error:
            row = {
                "id": path.name,
                "state": "unknown",
                "label": None,
                "command": "unavailable",
                "cleanup": {"status": "unknown"},
                "host_cleanup": {"status": "unknown"},
                "owner_live": False,
                "protected": True,
                "retained": (path / runs.RETAIN).exists(),
                "active": False,
                "corrupt": True,
                "reason": type(error).__name__,
            }
        if query["state"] == "active" and not row["active"] and not row["corrupt"]:
            continue
        if (
            query["state"] == "unresolved"
            and row["host_cleanup"]["status"] == "confirmed"
            and not row["corrupt"]
        ):
            continue
        rows.append(row)
        if not complete and len(rows) >= limit:
            break
    remaining = last is not None and any(path.name < last for path in paths)
    next_cursor = _token({**bound, "after": last}) if remaining and not complete else None
    return {
        "schema": 2,
        "runs": rows,
        "next_cursor": next_cursor,
        "coverage": {
            "complete": next_cursor is None,
            "membership": "live names observed once at page start",
            "initial_upper_anchor": bound["anchor"],
            "last_examined": last,
            "hydrated_records": hydrated,
            "returned_records": len(rows),
            "examined_records": examined,
            "directory_count": len(names),
            "observation_started": began,
            "observation_ended": runs.now(),
            "live_consistency": (
                "one process snapshot per page; live filters are not "
                "a simultaneous cross-page snapshot; directory additions/deletions are live"
            ),
            "unknown_records": sum(row["corrupt"] for row in rows),
        },
    }
