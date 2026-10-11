"""Selective native-evidence operations; inspection never prepares a test environment."""

from __future__ import annotations

import argparse
import base64
import hashlib
import hmac
import json
import os
import shutil
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

import runs
from harness import read_json, write_json_atomic
from storage_lifecycle import Storage, admission

from . import projection, rerun
from .context import SourceObserver, digest_file

COMMANDS = {"report", "cases", "evidence", "events", "compare", "rerun-plan", "export"}


def _reader_key() -> bytes:
    """Authenticate a local validation cache; portable indexes are never authorities."""
    path = Storage().state / "evidence-reader.key"
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        return path.read_bytes()
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(os.urandom(32))
    return path.read_bytes()


def _seal(value: dict[str, Any]) -> str:
    return hmac.new(
        _reader_key(), json.dumps(value, sort_keys=True).encode(), hashlib.sha256
    ).hexdigest()


def resolve(reference: str) -> Path:
    explicit = Path(reference).expanduser()
    if explicit.is_dir():
        return explicit.resolve()
    return runs.resolve(reference).resolve()


def _inside(root: Path, relative: str) -> Path:
    path = root / relative
    if path.is_symlink() or not path.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"evidence member escapes its owner: {relative}")
    return path


def _bundle(root: Path) -> None:
    """Validate originals once per external generation; never trust imported indexes."""
    manifest = read_json(root / "bundle.json")
    if not manifest:
        return
    if manifest.get("schema") != 1:
        raise ValueError("unsupported evidence bundle schema")
    stamps = {}
    for name in manifest["members"]:
        path = _inside(root, name)
        stat = path.stat()
        stamps[name] = [stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns]
    marker = root / ".inspection.json"
    previous = read_json(marker)
    manifest_digest = digest_file(root / "bundle.json")
    checkpoints = {
        str(path.relative_to(root)): digest_file(path)
        for path in (root / "verify-evidence").glob("*/projection/checkpoint.json")
    }
    cache = {
        "manifest": manifest_digest,
        "stamps": stamps,
        "checkpoints": checkpoints,
        "location": str(root),
    }
    if (
        previous
        and previous.get("cache") == cache
        and hmac.compare_digest(previous.get("seal", ""), _seal(cache))
    ):
        return
    for name, expected in manifest["members"].items():
        if digest_file(_inside(root, name)) != expected:
            raise ValueError(f"bundle original digest mismatch: {name}")
    for invocation in (root / "verify-evidence").iterdir():
        if invocation.is_dir() and (invocation / "invocation.json").is_file():
            # Caller-provided indexes are access products, never portable authority.
            index = invocation / "projection"
            if index.exists():
                # Ignore rather than delete imported derived data. Preparing a fresh
                # checkpoint never interprets the imported checkpoint/offsets.
                checkpoint = index / "checkpoint.json"
                if checkpoint.exists():
                    checkpoint.rename(index / ("imported-" + os.urandom(8).hex() + ".json"))
            projection.prepare(invocation)
    cache["checkpoints"] = {
        str(path.relative_to(root)): digest_file(path)
        for path in (root / "verify-evidence").glob("*/projection/checkpoint.json")
    }
    write_json_atomic(marker, {"cache": cache, "seal": _seal(cache)})


def invocations(root: Path) -> list[tuple[Path, dict[str, Any]]]:
    manifest = read_json(root / "verify-evidence.json")
    if not manifest:
        return []
    if manifest.get("schema") != 1:
        raise ValueError("unsupported verification evidence manifest schema")
    result = []
    for entry in manifest.get("invocations", []):
        ident = entry["id"]
        if not isinstance(ident, str) or not ident.isalnum():
            raise ValueError("invalid invocation identity")
        directory = _inside(root, f"verify-evidence/{ident}")
        record = read_json(directory / "invocation.json")
        if record is None:
            result.append(
                (
                    directory,
                    {
                        "id": ident,
                        "capture": {"status": "missing"},
                        "boundary": entry.get("boundary"),
                        "step": entry.get("step"),
                    },
                )
            )
        elif record.get("id") != ident or record.get("schema") != 1:
            raise ValueError("invocation identity/schema mismatch")
        else:
            result.append((directory, record))
    return result


def _rows(root: Path, gaps: dict[str, str] | None = None, pins: dict | None = None):
    for directory, invocation in invocations(root):
        if not (directory / "invocation.json").is_file():
            continue
        try:
            pin = (pins or {}).get(invocation["id"], {})
            records = list(
                projection.iter_cases(
                    directory, generation=pin.get("generation"), min_revision=pin.get("revision")
                )
            )
            for row in records:
                yield {
                    **row,
                    "invocation": invocation["id"],
                    "case": invocation["id"] + ":" + row["case_key"],
                    "boundary": invocation["boundary"],
                    "step": invocation["step"],
                    "tool": invocation.get("tool"),
                }
        except (OSError, ValueError) as error:
            if gaps is None:
                raise
            gaps[invocation["id"]] = str(error)


def _prepared(directory: Path) -> dict[str, Any]:
    try:
        return projection.prepare(directory)
    except (OSError, ValueError) as error:
        return {
            "coverage": {"status": "failed", "reason": str(error)},
            "count": 0,
            "generation": None,
            "revision": 0,
        }


def retention(root: Path, consumer: str | None = None) -> dict[str, Any]:
    manifest = read_json(root / "verify-evidence.json") or {}
    ids = [ident for ident in manifest.get("storage", {}).values() if ident]
    if (root / "bundle.json").exists():
        ids = [row["id"] for row in Storage().records() if Path(row["path"]) == root]
    if consumer:
        if not ids:
            raise ValueError("evidence has no enrolled lifetime; cannot establish a citation hold")
        for ident in ids:
            Storage().retain(ident, consumer, "cited-report")
    return {
        "storage_ids": ids,
        "consumer": consumer,
        "citation": "held"
        if consumer
        else "temporary availability; retain before cross-session citation",
        "retain": [
            f"just storage retain {ident} --consumer NAME --requires cited-report" for ident in ids
        ],
        "release": [f"just storage release {ident} --consumer NAME --reason TEXT" for ident in ids],
    }


def report(root: Path) -> dict[str, Any]:
    gaps: dict[str, str] = {}
    rows = list(_rows(root, gaps))
    groups = defaultdict(list)
    for row in rows:
        if row.get("outcome") in ("failed", "failure", "error", "xpass"):
            signature = (
                row.get("tool"),
                row.get("phase"),
                row.get("outcome"),
                row.get("event_kind"),
            )
            groups[json.dumps(signature)].append(row["case"])
    observations = []
    for directory, row in invocations(root):
        entry = {
            key: row.get(key)
            for key in (
                "id",
                "boundary",
                "step",
                "tool",
                "tool_version",
                "command",
                "capture",
                "finalization",
                "postprocessing",
                "boundary_final",
                "context_before",
                "context_after",
                "binaries",
            )
        }
        entry["coverage"] = (
            _prepared(directory)["coverage"]
            if (directory / "invocation.json").exists()
            else {"status": "missing"}
        )
        if row["id"] in gaps:
            entry["coverage"] = {"status": "failed", "reason": gaps[row["id"]]}
        recording = directory / "nextest-recording.zip"
        if recording.exists():
            entry["replay"] = {
                "argv": ["cargo", "nextest", "--no-pager", "replay", "-R", str(recording)],
                "environment": {"NEXTEST_EXPERIMENTAL_RECORD": "1"},
                "exit_meaning": (
                    "inspection exit is not a test pass; --exit-code preserves native result"
                ),
            }
        observations.append(entry)
    return {
        "schema": 1,
        "run": root.name,
        "summary": read_json(root / runs.SUMMARY),
        "run_record": read_json(root / runs.RECORD),
        "invocations": observations,
        "observation_counts": dict(Counter(row.get("outcome", "unknown") for row in rows)),
        "failure_groups": [
            {
                "signature": signature,
                "count": len(members),
                "members": members[:50],
                "omitted": max(0, len(members) - 50),
            }
            for signature, members in groups.items()
        ],
        "group_meaning": (
            "transparent observation groups, not causal conclusions; "
            "cases returns complete membership"
        ),
        "coverage": "legacy boundary/log only"
        if not observations
        else "per-invocation native channel coverage",
        "case_count_meaning": (
            "phase/attempt and supplemental observations; not unique test executions"
        ),
    }


def _encode(value: Any) -> str:
    return base64.urlsafe_b64encode(json.dumps(value, sort_keys=True).encode()).decode()


def cases(root: Path, outcome: str | None, limit: int, cursor: str | None) -> dict[str, Any]:
    if limit < 1:
        raise ValueError("limit must be positive")
    checkpoints = {row["id"]: _prepared(directory) for directory, row in invocations(root)}
    page: dict[str, Any]
    if cursor:
        page = json.loads(base64.urlsafe_b64decode(cursor))
        if page["outcome"] != outcome or page["position"] < 0:
            raise ValueError("mismatched case cursor")
        for ident, pinned in page["pins"].items():
            current = checkpoints.get(ident)
            if (
                current is None
                or current["generation"] != pinned["generation"]
                or current["revision"] < pinned["revision"]
            ):
                raise ValueError("stale case cursor")
    else:
        page = {
            "position": 0,
            "outcome": outcome,
            "pins": {
                ident: {
                    "generation": cp["generation"],
                    "revision": cp["revision"],
                    "count": cp["count"],
                    "coverage": cp["coverage"],
                }
                for ident, cp in checkpoints.items()
            },
        }
    seen = Counter()
    rows = []
    gaps: dict[str, str] = {}
    for row in _rows(root, gaps, page["pins"]):
        ident = row["invocation"]
        seen[ident] += 1
        if ident not in page["pins"] or seen[ident] > page["pins"][ident]["count"]:
            continue
        if outcome is None or row.get("outcome") == outcome:
            rows.append(row)
    start = page["position"]
    selected = rows[start : start + limit]
    next_position = start + len(selected)
    return {
        "schema": 1,
        "cases": selected,
        "total": len(rows),
        "omitted": len(rows) - len(selected),
        "pins": page["pins"],
        "cursor": _encode({**page, "position": next_position})
        if next_position < len(rows)
        else None,
        "coverage": {
            ident: {"status": "failed", "reason": gaps[ident]}
            if ident in gaps
            else pinned["coverage"]
            for ident, pinned in page["pins"].items()
        },
        "membership": "phase/attempt observations; supplemental starts are not executions",
    }


def selected(root: Path, selector: str) -> tuple[Path, dict[str, Any], dict[str, Any]]:
    matches = []
    for directory, invocation in invocations(root):
        if _prepared(directory)["generation"] is None:
            continue
        for row in projection.iter_cases(directory):
            if selector in (
                row["case_key"],
                row["reference"],
                invocation["id"] + ":" + row["case_key"],
            ):
                matches.append((directory, invocation, row))
    identities = {(row[1]["id"], row[2]["case_key"]) for row in matches}
    if len(identities) != 1:
        raise ValueError(
            f"case selector matches {len(identities)} identities; use invocation-qualified case"
        )
    # Favor an actual native result carrying exact identity over supplemental membership.
    matches.sort(key=lambda row: (not bool(row[2].get("binary_id")), row[2].get("phase") != "call"))
    return matches[0]


def events(root: Path, step: str | None, limit: int, cursor: str | None) -> dict[str, Any]:
    if limit < 1:
        raise ValueError("limit must be positive")
    milestones = []
    checkpoints = {row["id"]: _prepared(directory) for directory, row in invocations(root)}
    for _, invocation in invocations(root):
        if step and step not in (invocation.get("step"), invocation.get("id")):
            continue
        for kind in ("spawn", "command", "postprocessing", "boundary_final", "finalization"):
            if kind in invocation:
                milestones.append(
                    {
                        "invocation": invocation["id"],
                        "step": invocation["step"],
                        "event": kind,
                        "observation": invocation[kind],
                    }
                )
    page: dict[str, Any]
    if cursor:
        page = json.loads(base64.urlsafe_b64decode(cursor))
        if page["step"] != step or type(page["position"]) is not int or page["position"] < 0:
            raise ValueError("mismatched event cursor")
        for ident, pinned in page["pins"].items():
            current = checkpoints.get(ident)
            if (
                current is None
                or current["generation"] != pinned["generation"]
                or current["revision"] < pinned["revision"]
            ):
                raise ValueError("stale event cursor")
    else:
        page = {
            "step": step,
            "position": 0,
            "milestones": milestones,
            "pins": {
                ident: {
                    "generation": cp["generation"],
                    "revision": cp["revision"],
                    "count": cp["count"],
                }
                for ident, cp in checkpoints.items()
            },
        }
        for ident, pinned in page["pins"].items():
            pinned["coverage"] = checkpoints[ident]["coverage"]
    rows = list(page["milestones"])
    seen = Counter()
    gaps: dict[str, str] = {}
    for row in _rows(root, gaps, page["pins"]):
        ident = row["invocation"]
        seen[ident] += 1
        if ident not in page["pins"] or seen[ident] > page["pins"][ident]["count"]:
            continue
        if step and step not in (row["step"], row["invocation"]):
            continue
        rows.append(
            {
                "invocation": row["invocation"],
                "step": row["step"],
                "event": row.get("event_kind", "native-phase"),
                "case": row["case"],
                "phase": row.get("phase"),
                "attempt": row.get("attempt"),
                "outcome": row.get("outcome"),
                "duration": row.get("duration"),
                "reference": row["reference"],
                "source": row["source"],
            }
        )
    start = page["position"]
    return {
        "schema": 1,
        "events": rows[start : start + limit],
        "total": len(rows),
        "cursor": _encode({**page, "position": start + limit})
        if start + limit < len(rows)
        else None,
        "pins": page["pins"],
        "coverage": (
            "durable milestones and prepared native observations; "
            "channels are not a total temporal order"
        ),
        "native_coverage": {
            ident: {"status": "failed", "reason": gaps[ident]}
            if ident in gaps
            else pinned["coverage"]
            for ident, pinned in page["pins"].items()
        },
    }


def compare(first: Path, second: Path) -> dict[str, Any]:
    def contexts(root):
        result = {}
        for _, row in invocations(root):
            ident = row.get("context_before")
            context = (
                read_json(_inside(root, f"verify-evidence/contexts/{ident}.json"))
                if ident
                else None
            )
            result[(row["boundary"], row["step"])] = (row, context)
        return result

    left, right = contexts(first), contexts(second)
    differences = []
    for key in sorted(left.keys() | right.keys()):
        a, ca = left.get(key, ({}, None))
        b, cb = right.get(key, ({}, None))
        names = set((ca or {}).get("files", {})) | set((cb or {}).get("files", {}))
        changed = sorted(
            name
            for name in names
            if (ca or {}).get("files", {}).get(name) != (cb or {}).get("files", {}).get(name)
        )
        differences.append(
            {
                "boundary": key[0],
                "step": key[1],
                "source_changed": changed[:50],
                "source_changed_count": len(changed),
                "source_omitted": max(0, len(changed) - 50),
                "argv_changed": a.get("argv") != b.get("argv"),
                "settings_changed": a.get("settings") != b.get("settings"),
                "fixture_changed": a.get("fixture") != b.get("fixture"),
                "tool_version_changed": a.get("tool_version") != b.get("tool_version"),
                "effective_config_changed": a.get("effective_config") != b.get("effective_config"),
                "effective_config_available": a.get("effective_config") is not None
                and b.get("effective_config") is not None,
                "context_available": bool(ca and cb),
            }
        )
    return {
        "schema": 1,
        "runs": [first.name, second.name],
        "differences": differences,
        "source_to_binary": "unknown unless independently established in native evidence",
        "causality": "observed differences do not establish the cause of a result",
    }


def export(root: Path, output: Path) -> dict[str, Any]:
    owner = runs.current_run()
    if owner is None:
        raise ValueError(
            "export requires a run owner; use just run -- just verify export RUN --output PATH"
        )
    output = output.absolute()
    if output.exists():
        raise ValueError("export destination already exists")
    with admission([output]):
        output.mkdir(parents=True)
        consumer = f"evidence-export:{owner.name}"
        ident = Storage().publish(
            output,
            "run-log",
            {"kind": "run", "path": str(owner.resolve())},
            consumer,
            requires="cited-report",
            managed=True,
        )
        members = {}
        candidates = [root / "verify-evidence.json", root / runs.RECORD, root / runs.SUMMARY]
        candidates.extend(
            _inside(root, str(Path(row["log"]).relative_to(root)))
            for _, row in invocations(root)
            if row.get("log") and Path(row["log"]).is_relative_to(root)
        )
        raw = root / "verify-evidence"
        (output / "verify-evidence").mkdir()
        if raw.exists():
            candidates.extend(
                path
                for path in raw.rglob("*")
                if path.is_file()
                and "projection" not in path.relative_to(raw).parts
                and "nextest-state" not in path.relative_to(raw).parts
            )
        for source in candidates:
            if not source.exists():
                continue
            relative = source.relative_to(root)
            if source.is_symlink() and relative != Path("verify-evidence.json"):
                raise ValueError(f"cannot export evidence symlink: {relative}")
            if relative == Path("verify-evidence.json") and source.is_symlink():
                _inside(root, str(source.resolve().relative_to(root)))
            target = output / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            before = source.stat()
            shutil.copyfile(source, target)
            after = source.stat()
            if (before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (
                after.st_ino,
                after.st_size,
                after.st_mtime_ns,
                after.st_ctime_ns,
            ):
                raise ValueError(
                    f"source changed during export: {relative}; partial export protected"
                )
            members[str(relative)] = digest_file(target)
        # Absolute original locations are preserved as context, not used by portable readers.
        manifest = read_json(output / "verify-evidence.json") or {"schema": 1, "invocations": []}
        manifest["storage"] = {"bundle": ident}
        write_json_atomic(output / "verify-evidence.json", manifest)
        members["verify-evidence.json"] = digest_file(output / "verify-evidence.json")
        for directory, row in invocations(output):
            artifacts = row.get("artifacts", {})
            if isinstance(artifacts, dict):
                row["artifacts"] = {
                    key: Path(value).name if isinstance(value, str) else value
                    for key, value in artifacts.items()
                }
            write_json_atomic(directory / "invocation.json", row)
            members[str((directory / "invocation.json").relative_to(output))] = digest_file(
                directory / "invocation.json"
            )
        write_json_atomic(
            output / "bundle.json",
            {
                "schema": 1,
                "source_run": root.name,
                "members": members,
                "storage_id": ident,
                "native_replay": "nextest-recording.zip where available",
                "secrets": "native output is not sanitized",
            },
        )
        _bundle(output)
        return {
            "schema": 1,
            "output": str(output),
            "storage_id": ident,
            "consumer": consumer,
            "members": len(members),
            "release": f"just storage release {ident} --consumer {consumer} --reason TEXT",
        }


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("command", choices=sorted(COMMANDS))
    result.add_argument("run")
    result.add_argument("other", nargs="?")
    result.add_argument("--json", action="store_true")
    result.add_argument("--case")
    result.add_argument("--phase")
    result.add_argument("--step")
    result.add_argument("--outcome")
    result.add_argument("--limit", type=int, default=50)
    result.add_argument("--limit-bytes", type=int, default=65536)
    result.add_argument("--cursor")
    result.add_argument("--consumer")
    result.add_argument("--print", action="store_true")
    result.add_argument("--output", type=Path)
    return result


def reader_paths(root: Path) -> list[Path]:
    # Independent components can retire without their parent. Admit their actual
    # physical scopes explicitly, not merely the enclosing run directory.
    return [root, root / "verify-evidence", root / "verify-evidence-receipt"]


def main(argv: list[str]) -> int:
    args = parser().parse_args(argv)
    if args.command == "export" and runs.current_run() is None:
        command = [
            sys.executable,
            str(Path(runs.__file__).resolve()),
            "run",
            "--label",
            "verify evidence export",
            "--",
            sys.executable,
            str(Path(__file__).parents[1] / "verify.py"),
            *argv,
        ]
        os.execv(sys.executable, command)
    try:
        root = resolve(args.run)
        other = resolve(args.other) if args.command == "compare" and args.other else None
        with admission([*reader_paths(root), *(reader_paths(other) if other else [])]):
            _bundle(root)
            if other:
                _bundle(other)
            if args.command == "report":
                response = report(root)
            elif args.command == "cases":
                response = cases(root, args.outcome, args.limit, args.cursor)
            elif args.command in ("evidence", "rerun-plan"):
                if not args.case:
                    raise ValueError("--case is required")
                directory, invocation, case = selected(root, args.case)
                if args.command == "evidence":
                    response = projection.evidence(
                        directory, args.case, args.phase, args.limit_bytes, args.cursor
                    )
                else:
                    response = rerun.plan(
                        invocation,
                        case,
                        read_json(root / runs.SUMMARY) or {},
                        [row for _, row in invocations(root)],
                    )
                    current = SourceObserver(Path(invocation["cwd"])).observe()
                    response["current_context"] = current.get("id")
                    response["context_changed"] = current.get("id") != invocation.get(
                        "context_before"
                    )
                    response["current_context_available"] = current.get("available", False)
            elif args.command == "events":
                response = events(root, args.step, args.limit, args.cursor)
            elif args.command == "compare":
                if other is None:
                    raise ValueError("compare requires two run handles")
                response = compare(root, other)
            else:
                if args.output is None:
                    raise ValueError("export requires --output")
                response = export(root, args.output)
            response["retention"] = retention(root, args.consumer)
        print(json.dumps(response, indent=2))
        return 0
    except (OSError, ValueError, KeyError, runs.RunNotFound) as error:
        print(f"verify evidence: {error}", file=sys.stderr)
        return 2
