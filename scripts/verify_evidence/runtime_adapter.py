"""Attributed native phase/wait/resource observations, never test outcomes or diagnoses."""

from __future__ import annotations

import json
import re
from collections.abc import Iterator
from pathlib import Path
from typing import Any

ANSI = re.compile(r"\x1b\[[0-9;]*m")
FIELD = re.compile(r'\b([a-z_]+)=("(?:[^"\\]|\\.)*"|[^\s{}]+)')
TERMINAL = {"passed", "failed", "returned", "cancelled"}


def observation(text: str, invocation: str) -> dict | None:
    text = ANSI.sub("", text)
    try:
        event = json.loads(text)
    except ValueError:
        event = None
    if isinstance(event, dict):
        if event.get("target") != "lctx_phase":
            return None
        raw_fields = event.get("fields", {})
        if not isinstance(raw_fields, dict):
            return None
        fields = dict(raw_fields)
        # Tracing JSON declares owner context in current/root span records. Keep
        # their order and original attribution, without promoting a span field
        # into an event identity or guessing the originating producer.
        spans = event.get("spans", [])
        context = (
            [span for span in spans if isinstance(span, dict)] if isinstance(spans, list) else []
        )
        current = event.get("span")
        if isinstance(current, dict) and (not context or context[-1] != current):
            context.append(current)
        if context:
            fields["span_context"] = context
        fields["span_parentage"] = "structured producer span order"
        fields["current_span"] = current
    else:
        if "lctx_phase" not in text:
            return None
        ordered = []
        values = {}
        for name, token in FIELD.findall(text):
            try:
                value = json.loads(token)
            except ValueError:
                value = token
            ordered.append({"name": name, "value": value})
            values.setdefault(name, []).append(value)
        ambiguous = [
            name
            for name, items in values.items()
            if any(type(item) is not type(items[0]) or item != items[0] for item in items[1:])
        ]
        fields = {name: items[0] for name, items in values.items() if name not in ambiguous}
        fields["ordered_fields"] = ordered
        fields["ambiguous_fields"] = ambiguous
        fields["span_parentage"] = "unknown; flat text does not establish parentage"
        if set(ambiguous) & {
            "invocation_id",
            "process_id",
            "process_start_ticks",
            "phase_id",
            "phase",
            "status",
            "observation",
            "requested",
            "used",
            "limit",
        }:
            return {
                "event_kind": "native-observation-unattributed",
                "phase": "unknown",
                "outcome": "observed",
                "observation": fields,
                "identity_meaning": (
                    "conflicting flat attribution or semantics; identity not established"
                ),
            }
    if fields.get("invocation_id") != invocation or type(fields.get("process_id")) is not int:
        return None
    if fields.get("observation") == "resource_refusal":
        if not all(type(fields.get(name)) is int for name in ("requested", "used", "limit")):
            return None
        return {
            "event_kind": "native-resource-refusal",
            "phase": "resource",
            "outcome": "observed",
            "observation": fields,
            "resource_meaning": "admitted reservations; not RSS",
            "causal_meaning": "refusal context, not dominant-cause attribution",
        }
    if not isinstance(fields.get("phase"), str) or type(fields.get("phase_id")) is not int:
        return None
    status = fields.get("status")
    if status not in TERMINAL | {"begin"}:
        return None
    return {
        "event_kind": "native-phase",
        "phase": fields["phase"],
        "outcome": "observed",
        "status": status,
        "duration": fields.get("elapsed_ms"),
        "duration_unit": "milliseconds",
        "phase_identity": {
            name: fields.get(name)
            for name in ("invocation_id", "process_id", "process_start_ticks", "phase_id")
        },
        "observation": fields,
        "completion_meaning": "owner-emitted observation; unmatched begin remains incomplete",
    }


def from_record(record: dict, invocation: str) -> Iterator[dict[str, Any]]:
    for position, line in enumerate(str(record.get("payload", "")).splitlines()):
        parsed = observation(line, invocation)
        if parsed is None:
            continue
        fields = parsed["observation"]
        key = [
            fields.get("process_id"),
            fields.get("process_start_ticks"),
            fields.get("phase_id"),
            fields.get("observation"),
            position,
        ]
        locator = {
            "envelope": record.get("locator", record.get("native", {}).get("locator")),
            "line": position,
            "source": record.get("source"),
            "source_start": record.get("source_start"),
            "source_end": record.get("source_end"),
        }
        yield {
            **record,
            **parsed,
            "case_key": "runtime:" + json.dumps(key, separators=(",", ":")),
            "payload": line,
            "attempt": 1,
            "locator": locator,
            "native": {**record.get("native", {}), "locator": locator},
            "coverage": {
                "authority": (
                    "unattributed-native-observation"
                    if parsed["event_kind"] == "native-observation-unattributed"
                    else "attributed-native-observation"
                ),
                "overlap": "supplemental source observation; no cross-channel event deduplication",
                "payload_loss": record.get("native", {}).get("truncation", "unknown"),
                "case_outcome": "not established",
                "unfinished": "no terminal inferred",
                "source_to_binary": "not established",
            },
        }


def records(directory: Path, *, watermarks: dict, ends: dict):
    invocation = json.loads((directory / "invocation.json").read_bytes())["id"]
    name = "native.log"
    path = directory / name
    if not path.exists():
        return
    with path.open("rb") as stream:
        stream.seek(watermarks.get(name, 0))
        bound = ends.get(name, path.stat().st_size)
        while stream.tell() < bound:
            start = stream.tell()
            line = stream.readline()
            end = stream.tell()
            if not line.endswith(b"\n") or end > bound:
                return
            yield from from_record(
                {
                    "payload": line.decode(errors="replace"),
                    "source": name,
                    "source_start": start,
                    "source_end": end,
                },
                invocation,
            )
