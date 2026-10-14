"""Rebuildable, invocation-local selective evidence preparation.

The caller owns storage admission. Originals are never edited. A checkpoint commits
immutable metadata segments and a prefix of a separate payload stream; physical
offsets never participate in semantic evidence references.
"""

from __future__ import annotations

import base64
import fcntl
import hashlib
import importlib
import json
import os
import uuid
from collections.abc import Iterator
from dataclasses import asdict, fields
from pathlib import Path
from typing import Any

from .contracts import ADAPTER_VERSION, SCHEMA, EvidenceReference, PrefixWatermark

VERSION = SCHEMA
INDEX = "projection"
CHUNK = 65536
BINDING_FIELDS = tuple(field.name for field in fields(EvidenceReference))


class ProjectionError(ValueError):
    """An original, committed projection, or pinned continuation is unavailable."""


def _json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()


def _digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _stat(path: Path) -> dict[str, int]:
    try:
        stat = path.stat()
    except FileNotFoundError as error:
        raise ProjectionError(f"missing original: {path}") from error
    return {
        "device": stat.st_dev,
        "inode": stat.st_ino,
        "size": stat.st_size,
        "mtime_ns": stat.st_mtime_ns,
        "ctime_ns": stat.st_ctime_ns,
    }


def _same_generation(observed: dict[str, int], state: dict[str, Any]) -> bool:
    old = state["stat"]
    return observed == old or bool(
        state["live"]
        and observed["device"] == old["device"]
        and observed["inode"] == old["inode"]
        and observed["size"] > old["size"]
    )


def _hash_range(path: Path, start: int, end: int) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        source.seek(start)
        remaining = end - start
        while remaining:
            block = source.read(min(CHUNK, remaining))
            if not block:
                raise ProjectionError(f"short source range: {path}")
            digest.update(block)
            remaining -= len(block)
    return digest.hexdigest()


def _relative(directory: Path, source: str | Path) -> str:
    path = Path(source)
    if not path.is_absolute():
        path = directory / path
    try:
        return path.resolve().relative_to(directory.resolve()).as_posix()
    except ValueError as error:
        raise ProjectionError(f"original escapes invocation directory: {source}") from error


def _paths(directory: Path, invocation: dict[str, Any]) -> list[str]:
    names = {
        "junit.xml",
        "nextest-events.jsonl",
        "pytest-reportlog.jsonl",
        "pytest-hooks.jsonl",
        "native.log",
    }
    artifacts = invocation.get("artifacts", {})
    values = artifacts.values() if isinstance(artifacts, dict) else artifacts
    for value in values:
        if isinstance(value, dict):
            value = value.get("path", value.get("file"))
        if isinstance(value, str) and Path(value).suffix in {".xml", ".jsonl"}:
            names.add(_relative(directory, value))
    return sorted(name for name in names if (directory / name).is_file())


def _complete_end(path: Path, start: int, size: int) -> int:
    """Scan only the new suffix; a partial final line is not committed input."""
    end = start
    position = start
    with path.open("rb") as source:
        source.seek(start)
        while position < size:
            block = source.read(min(CHUNK, size - position))
            if not block:
                break
            newline = block.rfind(b"\n")
            if newline >= 0:
                end = position + newline + 1
            position += len(block)
    return end


def _source_state(directory: Path, name: str, previous: dict[str, Any] | None) -> dict[str, Any]:
    path = directory / name
    observed = _stat(path)
    live = path.suffix in {".jsonl", ".log"}
    end = (
        _complete_end(path, previous["end"] if previous else 0, observed["size"])
        if live
        else observed["size"]
    )
    if live:
        if previous and previous["end"]:
            identity = previous["identity"]
        else:
            with path.open("rb") as source:
                first = source.readline(end) if end else b""
            identity = _digest(first)
    else:
        identity = _hash_range(path, 0, end)
    if not _same_generation(_stat(path), {"stat": observed, "live": live}):
        raise ProjectionError(f"original changed during preparation: {name}; retry preparation")
    return {"stat": observed, "end": end, "identity": identity, "live": live}


def _load_checkpoint(directory: Path) -> dict[str, Any] | None:
    path = directory / INDEX / "checkpoint.json"
    if not path.exists():
        return None
    try:
        checkpoint = json.loads(path.read_bytes())
        checksum = checkpoint.pop("checksum")
        if _digest(_json(checkpoint)) != checksum or checkpoint["version"] != VERSION:
            raise ProjectionError("corrupt projection checkpoint; rebuild from originals")
        return checkpoint
    except (KeyError, TypeError, json.JSONDecodeError) as error:
        raise ProjectionError("invalid projection checkpoint; rebuild from originals") from error


def _publish(directory: Path, checkpoint: dict[str, Any]) -> None:
    data = {**checkpoint, "checksum": _digest(_json(checkpoint))}
    path = directory / INDEX / f"checkpoint-{uuid.uuid4().hex}.tmp"
    with path.open("wb") as target:
        target.write(_json(data))
        target.flush()
        os.fsync(target.fileno())
    path.replace(directory / INDEX / "checkpoint.json")
    descriptor = os.open(directory / INDEX, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _records(
    directory: Path, tool: str, watermarks: dict[str, int], ends: dict[str, int]
) -> Iterator[dict[str, Any]]:
    module = {"pytest": "pytest_adapter", "nextest": "nextest", "cargo": "cargo_adapter"}.get(tool)
    from . import runtime_adapter

    invocation = json.loads((directory / "invocation.json").read_bytes())["id"]
    if module is not None:
        adapter = importlib.import_module(f"{__package__}.{module}")
        for record in adapter.records(directory, watermarks=watermarks, ends=ends):
            yield record
            yield from runtime_adapter.from_record(record, invocation)
    yield from runtime_adapter.records(directory, watermarks=watermarks, ends=ends)


def _coverage(invocation: dict[str, Any], sources: dict[str, Any]) -> dict[str, Any]:
    return {
        "tool": invocation.get("tool"),
        "capture": invocation.get("capture", {}),
        "structured": "available" if sources else "missing",
        "supported": invocation.get("tool") in {"pytest", "nextest", "cargo"},
        "prefix": {name: state["end"] for name, state in sources.items()},
    }


def prepare(directory: Path) -> dict[str, Any]:
    """Prepare a stable generation once, or append newly complete JSONL records.

    Removing ``projection/checkpoint.json`` or the entire derived directory is an
    explicit rebuild. Abandoned payload/segment tails have no committed meaning.
    """
    directory = Path(directory)
    invocation = json.loads((directory / "invocation.json").read_bytes())
    index = directory / INDEX
    index.mkdir(exist_ok=True)
    with (index / "prepare.lock").open("ab") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        previous = _load_checkpoint(directory)
        names = _paths(directory, invocation)
        if previous:
            for name in previous["sources"]:
                _stat(directory / name)  # A surviving derived copy is not an original.
        stats = {name: _stat(directory / name) for name in names}
        if (
            previous
            and previous["invocation"] == invocation["id"]
            and previous["tool"] == invocation["tool"]
            and set(stats) == set(previous["sources"])
            and all(stats[name] == state["stat"] for name, state in previous["sources"].items())
        ):
            coverage = _coverage(invocation, previous["sources"])
            if coverage != previous["coverage"]:
                previous = {**previous, "coverage": coverage, "revision": previous["revision"] + 1}
                _publish(directory, previous)
            return previous
        incremental = (
            previous is not None
            and previous["invocation"] == invocation["id"]
            and previous["tool"] == invocation["tool"]
            and set(stats) == set(previous["sources"])
        )
        if incremental and previous is not None:
            for name, state in previous["sources"].items():
                observed = stats[name]
                old = state["stat"]
                if observed == old:
                    continue
                if (
                    not state["live"]
                    or observed["device"] != old["device"]
                    or observed["inode"] != old["inode"]
                    or observed["size"] <= old["size"]
                ):
                    incremental = False
                    break
        sources: dict[str, dict[str, Any]] = {}
        for name in names:
            old = previous["sources"].get(name) if incremental and previous is not None else None
            sources[name] = (
                old if old and old["stat"] == stats[name] else _source_state(directory, name, old)
            )
        checkpoint: dict[str, Any]
        watermarks: dict[str, int]
        if incremental and previous is not None:
            checkpoint = {
                **previous,
                "sources": sources,
                "segments": list(previous["segments"]),
                "revision": previous["revision"] + 1,
            }
            watermarks = {name: state["end"] for name, state in previous["sources"].items()}
        else:
            checkpoint = {
                "version": VERSION,
                "invocation": invocation["id"],
                "tool": invocation["tool"],
                "generation": uuid.uuid4().hex,
                "revision": 1,
                "sources": sources,
                "segments": [],
                "payload_end": 0,
                "count": 0,
            }
            watermarks = {}
        checkpoint["coverage"] = _coverage(invocation, sources)
        generation = index / checkpoint["generation"]
        generation.mkdir(exist_ok=True)
        payload_path = generation / "payload.bin"
        segment_name = f"metadata-{len(checkpoint['segments']):08d}-{uuid.uuid4().hex}.jsonl"
        segment_path = generation / segment_name
        segment_digest = hashlib.sha256()
        added = 0
        with (
            payload_path.open("r+b" if payload_path.exists() else "w+b") as payload,
            segment_path.open("wb") as segment,
        ):
            payload.truncate(checkpoint["payload_end"])
            payload.seek(checkpoint["payload_end"])
            for record in _records(
                directory,
                invocation["tool"],
                watermarks,
                {name: state["end"] for name, state in sources.items()},
            ):
                source_name = _relative(directory, record["source"])
                if source_name not in sources:
                    raise ProjectionError(f"adapter used undeclared original: {source_name}")
                state = sources[source_name]
                start, end = record.get("source_start"), record.get("source_end")
                if start is not None or end is not None:
                    if (
                        type(start) is not int
                        or type(end) is not int
                        or not 0 <= start < end <= state["end"]
                    ):
                        raise ProjectionError(
                            f"adapter used an uncommitted source range: {source_name}"
                        )
                    source_digest = _hash_range(directory / source_name, start, end)
                else:
                    if state["live"]:
                        raise ProjectionError("live adapter omitted its native byte range")
                    source_digest = state["identity"]
                text = record.get("payload", "")
                encoded = str(text).encode("utf-8")
                native = record.get("native", {})
                identity = EvidenceReference(
                    invocation=invocation["id"],
                    adapter=ADAPTER_VERSION,
                    source=source_name,
                    source_generation=state["identity"],
                    source_start=start,
                    source_end=end,
                    source_digest=source_digest,
                    locator=native.get("locator", record.get("locator")),
                    case_key=record["case_key"],
                    phase=record.get("phase", "call"),
                    attempt=record.get("attempt", 1),
                    event_kind=record.get("event_kind"),
                    payload_digest=_digest(encoded),
                )
                binding = asdict(identity)
                reference = f"ev{SCHEMA}:" + _digest(_json(binding))
                header = {**binding, "reference": reference, "length": len(encoded)}
                offset = payload.tell()
                payload.write(_json(header) + b"\n")
                payload_offset = payload.tell()
                payload.write(encoded)
                metadata = {key: value for key, value in record.items() if key != "payload"}
                metadata.update(header)
                metadata.update(offset=offset, payload_offset=payload_offset)
                line = _json(metadata) + b"\n"
                segment.write(line)
                segment_digest.update(line)
                added += 1
            for stream in (payload, segment):
                stream.flush()
                os.fsync(stream.fileno())
            checkpoint["payload_end"] = payload.tell()
        # A concurrent append can wait for the next preparation. Mutation of the
        # captured generation cannot be silently admitted with these locators.
        for name, state in sources.items():
            if not _same_generation(_stat(directory / name), state):
                raise ProjectionError(
                    f"original changed during preparation: {name}; retry preparation"
                )
        if added:
            checkpoint["segments"].append(
                {"path": segment_name, "sha256": segment_digest.hexdigest(), "count": added}
            )
        else:
            segment_path.unlink()  # Newly owned, never-published empty derived segment.
        checkpoint["count"] += added
        checkpoint["watermarks"] = {
            name: asdict(
                PrefixWatermark(
                    state["identity"],
                    state["end"],
                    checkpoint["payload_end"],
                    checkpoint["generation"],
                )
            )
            for name, state in sources.items()
        }
        _publish(directory, checkpoint)
        return checkpoint


def _rows(
    directory: Path, checkpoint: dict[str, Any], segment_count: int | None = None
) -> Iterator[dict[str, Any]]:
    generation = directory / INDEX / checkpoint["generation"]
    segments = (
        checkpoint["segments"] if segment_count is None else checkpoint["segments"][:segment_count]
    )
    for segment in segments:
        try:
            data = (generation / segment["path"]).read_bytes()
        except FileNotFoundError as error:
            raise ProjectionError(
                "missing committed metadata segment; rebuild from originals"
            ) from error
        if _digest(data) != segment["sha256"]:
            raise ProjectionError("corrupt metadata segment; rebuild from originals")
        for line in data.splitlines():
            try:
                row = json.loads(line)
                if (
                    not isinstance(row, dict)
                    or not set(BINDING_FIELDS).union(
                        {"reference", "length", "offset", "payload_offset"}
                    )
                    <= row.keys()
                ):
                    raise ProjectionError("invalid committed metadata record")
                yield row
            except json.JSONDecodeError as error:
                raise ProjectionError("invalid committed metadata record") from error


def iter_cases(
    directory: Path, *, generation: str | None = None, min_revision: int | None = None
) -> Iterator[dict[str, Any]]:
    """Iterate phase/attempt metadata without opening any captured payload."""
    directory = Path(directory)
    checkpoint = prepare(directory)
    if generation is not None and checkpoint["generation"] != generation:
        raise ProjectionError("projection changed while reading the pinned page")
    if min_revision is not None and checkpoint["revision"] < min_revision:
        raise ProjectionError("projection revision is older than the pinned page")
    return _rows(directory, checkpoint)


def _token(data: dict[str, Any]) -> str:
    return base64.urlsafe_b64encode(_json(data)).decode().rstrip("=")


def _page(
    checkpoint: dict[str, Any], cursor: str | None, filters: dict[str, Any]
) -> dict[str, Any]:
    if cursor is None:
        return {
            "generation": checkpoint["generation"],
            "revision": checkpoint["revision"],
            "segments": len(checkpoint["segments"]),
            "position": 0,
            "byte": 0,
            "filters": filters,
            "coverage": checkpoint["coverage"],
        }
    try:
        page = json.loads(base64.urlsafe_b64decode(cursor + "=" * (-len(cursor) % 4)))
        if (
            any(
                type(page[key]) is not int or page[key] < 0
                for key in ("revision", "segments", "position", "byte")
            )
            or page["generation"] != checkpoint["generation"]
            or page["revision"] > checkpoint["revision"]
            or page["segments"] > len(checkpoint["segments"])
            or page["filters"] != filters
            or not isinstance(page["coverage"], dict)
        ):
            raise ProjectionError("stale or mismatched projection cursor")
        return page
    except (ValueError, TypeError, KeyError) as error:
        raise ProjectionError("invalid or stale projection cursor") from error


def cases(
    directory: Path, outcome: str | None = None, limit: int = 50, cursor: str | None = None
) -> dict[str, Any]:
    if type(limit) is not int or limit < 1:
        raise ProjectionError("case limit must be positive")
    directory = Path(directory)
    checkpoint = prepare(directory)
    page = _page(checkpoint, cursor, {"operation": "cases", "outcome": outcome})
    rows = [
        row
        for row in _rows(directory, checkpoint, page["segments"])
        if outcome is None or row.get("outcome") == outcome
    ]
    position = page["position"]
    selected = rows[position : position + limit]
    next_position = position + len(selected)
    return {
        "cases": selected,
        "total": len(rows),
        "omitted": len(rows) - len(selected),
        "cursor": _token({**page, "position": next_position})
        if next_position < len(rows)
        else None,
        "generation": page["generation"],
        "revision": page["revision"],
        "coverage": page["coverage"],
    }


def _selected_payload(
    directory: Path, checkpoint: dict[str, Any], row: dict[str, Any], start: int, budget: int
) -> tuple[str, int]:
    try:
        identity = EvidenceReference(**{key: row[key] for key in BINDING_FIELDS})
    except ValueError as error:
        raise ProjectionError(f"invalid selected evidence identity: {error}") from error
    binding = asdict(identity)
    source = directory / row["source"]
    if row["source"] not in checkpoint["sources"] or row["invocation"] != checkpoint["invocation"]:
        raise ProjectionError("selected metadata identifies another original or invocation")
    state = checkpoint["sources"][row["source"]]
    observed = _stat(source)
    if not _same_generation(observed, state):
        raise ProjectionError(f"original changed after preparation: {row['source']}")
    if row["source_generation"] != state["identity"]:
        raise ProjectionError("selected original generation changed")
    native_start, native_end = row["source_start"], row["source_end"]
    if native_start is not None:
        if (
            type(native_start) is not int
            or type(native_end) is not int
            or not 0 <= native_start < native_end <= state["end"]
        ):
            raise ProjectionError("selected source range is outside the committed native prefix")
        if _hash_range(source, native_start, native_end) != row["source_digest"]:
            raise ProjectionError("selected original range changed")
    elif row["source_digest"] != state["identity"]:
        raise ProjectionError("selected original generation changed")
    path = directory / INDEX / checkpoint["generation"] / "payload.bin"
    try:
        with path.open("rb") as stream:
            if (
                any(
                    type(row[key]) is not int or row[key] < 0
                    for key in ("offset", "payload_offset", "length")
                )
                or row["payload_offset"] < row["offset"]
                or row["payload_offset"] + row["length"] > checkpoint["payload_end"]
            ):
                raise ProjectionError("selected range is outside the committed payload prefix")
            if not 0 <= start <= row["length"]:
                raise ProjectionError("cursor byte is outside selected payload")
            header = dict(binding)
            header.update(reference=row["reference"], length=row["length"])
            expected_header = _json(header) + b"\n"
            if row["payload_offset"] - row["offset"] != len(expected_header):
                raise ProjectionError("selected payload identity does not match metadata")
            stream.seek(row["offset"])
            if stream.read(len(expected_header)) != expected_header:
                raise ProjectionError("selected payload identity does not match metadata")
            if f"ev{SCHEMA}:" + _digest(_json(binding)) != row["reference"]:
                raise ProjectionError("selected semantic reference is corrupt")
            digest = hashlib.sha256()
            excerpt = bytearray()
            position = 0
            while position < row["length"]:
                block = stream.read(min(CHUNK, row["length"] - position))
                if not block:
                    raise ProjectionError("short selected payload range")
                digest.update(block)
                begin = max(0, start - position)
                end = min(len(block), start + budget - position)
                if end > begin:
                    excerpt.extend(block[begin:end])
                position += len(block)
            if digest.hexdigest() != row["payload_digest"]:
                raise ProjectionError("selected payload digest mismatch")
    except (FileNotFoundError, json.JSONDecodeError) as error:
        raise ProjectionError(
            "missing or corrupt selected payload; rebuild from originals"
        ) from error
    return excerpt.decode("utf-8", errors="replace"), len(excerpt)


def evidence(
    directory: Path,
    case: str,
    phase: str | None = None,
    limit_bytes: int = 65536,
    cursor: str | None = None,
) -> dict[str, Any]:
    if type(limit_bytes) is not int or limit_bytes < 1:
        raise ProjectionError("evidence byte limit must be positive")
    directory = Path(directory)
    checkpoint = prepare(directory)
    prefix = checkpoint["invocation"] + ":"
    case = case.removeprefix(prefix)
    page = _page(checkpoint, cursor, {"operation": "evidence", "case": case, "phase": phase})
    rows = [
        row
        for row in _rows(directory, checkpoint, page["segments"])
        if (row["case_key"] == case or row["reference"] == case)
        and (phase is None or row["phase"] == phase)
    ]
    if not rows:
        raise ProjectionError(f"case or evidence reference not present in committed prefix: {case}")
    position, byte = page["position"], page["byte"]
    remaining = limit_bytes
    selected = []
    while position < len(rows) and remaining:
        row = rows[position]
        text, consumed = _selected_payload(directory, checkpoint, row, byte, remaining)
        selected.append(
            {
                **row,
                "text": text,
                "byte_start": byte,
                "byte_end": byte + consumed,
                "truncated": byte > 0 or byte + consumed < row["length"],
                "original": "available",
                "excerpt_encoding": "utf-8; replacement at split boundary",
            }
        )
        remaining -= consumed
        byte += consumed
        if byte >= row["length"]:
            position, byte = position + 1, 0
        else:
            break
    return {
        "case": case,
        "evidence": selected,
        "total": len(rows),
        "omitted": len(rows) - len(selected),
        "cursor": _token({**page, "position": position, "byte": byte})
        if position < len(rows)
        else None,
        "generation": page["generation"],
        "revision": page["revision"],
        "coverage": page["coverage"],
    }
