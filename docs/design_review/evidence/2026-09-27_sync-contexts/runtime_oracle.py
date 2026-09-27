"""Independent CPython challenges: execute only generated, finite, side-effect-free programs.

No fixture, analyzed library input or compiler output is executed or used to select an answer.
The parent launches one bounded worker per case. Monitoring observes the actual pinned
contextlib method bodies, including absence of entry after constructor failure.
"""
from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import textwrap


CASES = {
    "preserve": "with nullcontext():\n    return payload",
    "entry_value": "with nullcontext(payload) as chosen:\n    return chosen",
    "suppress_type_error": "with suppress(TypeError):\n    raise None\nreturn payload",
    "nonmatching": "with suppress(ValueError):\n    raise None\nreturn payload",
    "multiple": "with nullcontext(), suppress(TypeError):\n    raise None\nreturn payload",
    "assignment_failure_suppressed": (
        "with suppress(TypeError), nullcontext() as (left, right):\n"
        "    return -1\nreturn payload"
    ),
    "constructor_failure_suppressed": (
        "with suppress(TypeError), nullcontext(1, 2):\n    return -1\nreturn payload"
    ),
    "replacement_suppressed": (
        "with suppress(TypeError), suppress(1):\n    raise None\nreturn payload"
    ),
    "replacement_preserved": (
        "with nullcontext(), suppress(1):\n    raise None\nreturn payload"
    ),
    "return_preserved": "with suppress(1):\n    return payload",
    "break_preserved": "for item in (1,):\n    with suppress(1):\n        break\nreturn payload",
    "continue_preserved": "for item in (1,):\n    with suppress(1):\n        continue\nreturn payload",
    "matching_short_circuit": "with suppress(TypeError, 1):\n    raise None\nreturn payload",
    "invalid_first": "with suppress(1, TypeError):\n    raise None\nreturn payload",
}


def worker(name: str) -> dict:
    if sys.version_info[:3] != (3, 14, 7):
        raise RuntimeError("oracle requires pinned CPython 3.14.7")
    source = "def generated(payload):\n" + textwrap.indent(CASES[name], "    ")
    namespace = {"nullcontext": contextlib.nullcontext, "suppress": contextlib.suppress}
    # Only CASES above supplies source. Repository fixtures are never opened by this worker.
    exec(compile(source, "<generated-context-oracle>", "exec"), namespace)
    methods = {
        getattr(cls, method).__code__: f"{cls.__name__}.{method}"
        for cls in (contextlib.nullcontext, contextlib.suppress)
        for method in ("__init__", "__enter__", "__exit__")
    }
    monitor = sys.monitoring
    tool = next(index for index in range(6) if monitor.get_tool(index) is None)
    events = monitor.events
    trace = []

    def start(code, offset):
        trace.append([methods[code], "start"])

    def returned(code, offset, value):
        trace.append([methods[code], "return", value])

    def unwound(code, offset, error):
        if code in methods:
            trace.append([methods[code], "raise", type(error).__name__])

    monitor.use_tool_id(tool, "lctx-generated-context-oracle")
    try:
        monitor.register_callback(tool, events.PY_START, start)
        monitor.register_callback(tool, events.PY_RETURN, returned)
        monitor.register_callback(tool, events.PY_UNWIND, unwound)
        monitor.set_events(tool, events.PY_UNWIND)
        for code in methods:
            monitor.set_local_events(tool, code, events.PY_START | events.PY_RETURN)
        try:
            result = {"kind": "return", "value": namespace["generated"](42)}
        except Exception as error:
            result = {"kind": "raise", "exception": type(error).__name__}
    finally:
        monitor.free_tool_id(tool)
    # An opaque fresh object distinguishes identity from primitive equality/interning.
    sentinel = object()
    try:
        identity_result = {"kind": "return", "same_entry_value": namespace["generated"](sentinel) is sentinel}
    except Exception as error:
        identity_result = {"kind": "raise", "exception": type(error).__name__}
    return {"case": name, "source": source, "outcome": result, "identity_outcome": identity_result, "trace": trace}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", choices=CASES)
    args = parser.parse_args()
    if args.worker:
        print(json.dumps(worker(args.worker)))
        return
    outputs = []
    for name in CASES:
        completed = subprocess.run(
            [sys.executable, str(Path(__file__).resolve()), "--worker", name],
            capture_output=True, text=True, check=True, timeout=20,
        )
        outputs.append(json.loads(completed.stdout))
    # Runtime controls must actually diverge. These assertions check the oracle harness,
    # not the compiler; the source-to-native integration consumes the independent outcomes.
    by_name = {row["case"]: row for row in outputs}
    for name in ("nonmatching", "replacement_preserved", "invalid_first"):
        assert by_name[name]["outcome"] == {"kind": "raise", "exception": "TypeError"}
    for name in set(CASES) - {"nonmatching", "replacement_preserved", "invalid_first"}:
        assert by_name[name]["outcome"] == {"kind": "return", "value": 42}
        assert by_name[name]["identity_outcome"] == {"kind": "return", "same_entry_value": True}
    assert not any(row[0] == "nullcontext.__enter__" for row in by_name["constructor_failure_suppressed"]["trace"])
    exits = [row[0] for row in by_name["assignment_failure_suppressed"]["trace"] if row[1] == "start" and row[0].endswith("__exit__")]
    assert exits == ["nullcontext.__exit__", "suppress.__exit__"]
    print(json.dumps({"python": sys.version, "contextlib_sha256": hashlib.sha256(Path(contextlib.__file__).read_bytes()).hexdigest(),
                      "scope": "finite generated synchronous programs; no completeness claim", "cases": outputs}, indent=2))


if __name__ == "__main__":
    main()
