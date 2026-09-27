"""Independent generated fresh-call controls; no analyzer fixture or fact is executed."""

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

CASES = {
    "literal": ("return (1, 'closed')", "inner()", "return", True),
    "fallthrough": ('"Definition metadata only."', "inner()", "return", True),
    "finally_return": ("try:\n    return 2\nfinally:\n    return 3", "inner()", "return", True),
    "modeled": ("return cast(int, 4)", "inner()", "return", True),
    "primitive_raise": ("raise None", "inner()", "TypeError", False),
    "local_read": ("local = 5\nreturn local", "inner()", "return", False),
    "captured": ("if False:\n    return value\nreturn 6", "inner()", "return", False),
    "intervening": ("return 7", "pass\ninner()", "return", False),
    "alias": ("return 8", "alias = inner\nalias()", "return", False),
    "generator": ("yield 11", "inner()", "generator", False),
    "unreachable_yield": ("if False:\n    yield 12\nreturn 13", "inner()", "generator", False),
    "unknown_body": ("unresolved()", "inner()", "NameError", False),
    "failed_header": ("return 10", "inner()", "ValueError", False),
    "proof_limit": ("pass\n" * 62 + "return 9", "inner()", "return", False),
}

WORKER = r"""
import json, os, resource, socket, sys
resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
assert sys.version_info[:3] == (3, 14, 7)
def event(name):
    print(json.dumps(name), flush=True)
def started(code, offset):
    if code.co_filename == sys.argv[1] and code.co_name in ("inner", "caller"):
        event(code.co_name + ":start")
def returned(code, offset, value):
    if code.co_filename == sys.argv[1] and code.co_name in ("inner", "caller"):
        event(code.co_name + ":return")
def unwound(code, offset, error):
    if code.co_filename == sys.argv[1] and code.co_name in ("inner", "caller"):
        event(code.co_name + ":raise:" + type(error).__name__)
mon = sys.monitoring
mon.use_tool_id(4, "fresh-source-call-oracle")
mon.register_callback(4, mon.events.PY_START, started)
mon.register_callback(4, mon.events.PY_RETURN, returned)
mon.register_callback(4, mon.events.PY_UNWIND, unwound)
mon.set_events(4, mon.events.PY_START | mon.events.PY_RETURN | mon.events.PY_UNWIND)
namespace = {}
exec(compile(open(sys.argv[1], encoding="utf-8").read(), sys.argv[1], "exec"), namespace)
sentinel = object()
try:
    result = namespace["caller"](sentinel)
    event("identity:" + str(result is sentinel))
except BaseException as error:
    event("caught:" + type(error).__name__)
os._exit(0)
"""


def main() -> None:
    results = {}
    with tempfile.TemporaryDirectory(prefix="lctx-source-call-") as temp:
        for name, (body, invocation, outcome, supported) in CASES.items():
            body = "\n".join(f"        {line}" for line in body.splitlines())
            invocation = "\n".join(f"    {line}" for line in invocation.splitlines())
            source = (
                f"from typing import cast\ndef caller(value):\n    def inner():\n{body}\n"
                f"{invocation}\n    return value\n"
            )
            if name == "failed_header":
                source = "def broken(fn):\n    raise ValueError()\n" + source.replace(
                    "    def inner():",
                    "    @broken\n    def inner():",
                )
            path = Path(temp) / f"{name}.py"
            path.write_text(source)
            result = subprocess.run(
                [sys.executable, "-I", "-c", WORKER, str(path)],
                cwd=temp,
                capture_output=True,
                text=True,
                timeout=2,
                check=True,
                env={"PATH": os.environ.get("PATH", "")},
            )
            events = [json.loads(line) for line in result.stdout.splitlines()]
            expected = (
                ["inner:return", "caller:return", "identity:True"]
                if outcome in ("return", "generator")
                else [
                    f"inner:raise:{outcome}",
                    f"caller:raise:{outcome}",
                    f"caught:{outcome}",
                ]
            )
            expected.insert(0, "caller:start")
            if name == "failed_header":
                expected.remove("inner:raise:ValueError")
            elif outcome == "generator":
                expected.remove("inner:return")
            else:
                expected.insert(1, "inner:start")
            assert events == expected, (name, events)
            results[name] = {
                "events": events,
                "in_initial_normal_domain": supported,
                "body_entered": "inner:start" in events,
                "in_source_invocation_domain": name
                not in ("failed_header", "alias", "intervening", "generator", "unreachable_yield"),
                "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
            }
    print(
        json.dumps(
            {
                "outcome": "passed",
                "python": sys.version.split()[0],
                "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                "worker_sha256": hashlib.sha256(WORKER.encode()).hexdigest(),
                "cases": results,
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
