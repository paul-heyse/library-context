"""Controlled current production DTOs, with separately authored evaluator expectations.

Only lctx_semantics.wire_tool_result produces captured MCP bytes; this module does
not normalize them into finite packets. This is a renderer lane, never native readiness.
"""
import json
import hashlib
from typing import Any


def source_case() -> tuple[dict[str, Any], dict[str, Any]]:
    body = "timeout = 10 if timeout is None else timeout\n"
    source = {"kind": "artifact", "artifact": [1] * 16}
    section = {"availability": {"status": "not_requested"}, "items": [], "omitted": 0, "truncated": False}
    response = {"snapshot": {"semantic": [5] * 32, "realization": [6] * 32,
                             "database": {"namespace": "control", "database": "renderer"}},
                "evidence": {"original": {"source": source, "artifact": [1] * 16,
                    "start": 0, "end": len(body.encode()), "digest": [0] * 32,
                    "encoding": "utf-8", "release": [2] * 16, "context": [3] * 16},
                    "body": {"start": 0, "end": len(body.encode()), "bytes": list(body.encode()), "omitted": 0, "truncated": False},
                    "flow_inventory": section, "source_characterization": section,
                    "status": 0, "derivation": section}}
    task = {"id": "actual-rendered-original", "split": "development", "family": "current-mcp-original",
            "public_call": {"tool": "get_evidence", "arguments": {"source": source}},
            "request": {"question": "Read this exact original source page", "context": {}, "allowed_followups": ["get_evidence"]},
            "envelope": {"max_calls": 1, "max_bytes": 32768}, "intent": "positive",
            "oracle": {"kind": "independent_generated_source", "input_digest": hashlib.sha256(body.encode()).hexdigest(),
                       "qualification": "exact source bytes only, no runtime claim", "supported_domain": "controlled UTF-8 original page",
                       "revision": "1", "completeness": "complete", "unknown_limits": ["no configuration/variant interpretation"]},
            "predicates": [{"name": "original", "role": "original_source", "accepted_text": [body],
                            "anchors": [json.dumps(source, sort_keys=True, separators=(",", ":"))],
                            "context": {}, "qualifications": [], "candidate_status": "supported"}],
            "witness": {"kind": "leaf", "predicate": "original"}, "model": None}
    return task, response
