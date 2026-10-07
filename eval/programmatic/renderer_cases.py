"""Controlled current production DTOs, with separately authored evaluator expectations.

Only lctx_semantics.wire_tool_result produces captured MCP bytes; this module does
not normalize them into finite packets. This is a renderer lane, never native readiness.
"""
import json
import hashlib
import copy
from typing import Any


def source_case() -> tuple[dict[str, Any], dict[str, Any]]:
    body = "timeout = 10 if timeout is None else timeout\n"
    source = {"kind": "artifact", "artifact": [1] * 16}
    section = {"availability": {"status": "not_requested"}, "items": [], "omitted": 0, "truncated": False}
    response = {"snapshot": {"semantic": [5] * 32, "realization": [6] * 32,
                             "database": {"namespace": "control", "database": "renderer"}},
                "evidence": {"release":{"input":[2]*16,"release":[2]*16,"distribution":"controlled-source","version":"1"},
                    "interpretation":{"contexts":[],"defaults":[],"qualifications":[],"availability":{"status":"unavailable","reason":"controlled_original_only"}},
                    "original": {"source": source, "artifact": [1] * 16,
                    "start": 0, "end": len(body.encode()), "digest": [0] * 32,
                    "encoding": "raw_bytes", "release": [2] * 16, "context": [3] * 16},
                    "body": {"start": 0, "end": len(body.encode()), "bytes": list(body.encode()), "omitted": 0, "truncated": False},
                    "flow_inventory": section, "source_characterization": section,
                    "status": 0, "derivation": section}}
    # Hand-authored expected layout for this one tiny controlled DTO. The actual
    # Rust formatter still owns captured bytes; these maps never supply oracle truth.
    empty_binding = {"member": None, "signature": None, "variant": None,
                     "analysis": None, "parameter": None, "field": None}
    response["delivery"] = {
        "fields": [
            {"field": "/content/0/text", "role": "synthetic", "original": None,
             "binding": empty_binding, "qualifications": [], "dependencies": [],
             "availability": {"status": "available"}},
            {"field": "/structuredContent/evidence/body/bytes", "role": "primary",
             "original": copy.deepcopy(response["evidence"]["original"]),
             "binding": {**empty_binding, "analysis": [3] * 16}, "qualifications": [],
             "dependencies": ["/structuredContent/evidence/release", "/structuredContent/evidence/interpretation"],
             "availability": {"status": "available"}},
        ],
        "omissions": [
            {"field": "/structuredContent/evidence/flow_inventory", "availability": {"status": "not_requested"}, "expand": None},
            {"field": "/structuredContent/evidence/source_characterization", "availability": {"status": "not_requested"}, "expand": None},
            {"field": "/structuredContent/evidence/derivation", "availability": {"status": "not_requested"}, "expand": None},
        ], "ranked_continuation": None,
        "packing_policy": "controlled_exact_original_page",
    }
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
