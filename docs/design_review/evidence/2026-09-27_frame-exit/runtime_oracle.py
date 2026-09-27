"""Distinguish body return from caller continuation; generated isolated programs only."""

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

WORKER = r'''
import json, resource, socket, sys, time, typing
resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
assert sys.version_info[:3] == (3, 14, 7)
def event(name):
    print(json.dumps(name), flush=True)
def body(unused):
    return 1
target = {"body": body, "cast": typing.cast, "assert_type": typing.assert_type}[sys.argv[1]]
def returned(code, offset, result):
    if code is target.__code__:
        event("body_return")
sys.monitoring.use_tool_id(4, "frame-exit-oracle")
sys.monitoring.register_callback(4, sys.monitoring.events.PY_RETURN, returned)
sys.monitoring.set_local_events(4, target.__code__, sys.monitoring.events.PY_RETURN)
class Finalized:
    def __del__(self):
        event("finalizer_started")
        time.sleep(30)
mode = sys.argv[2]
if mode == "retained":
    retained = Finalized()
    argument = retained
elif mode == "literal":
    argument = 0
# Avoid a wrapper's extra reference: the temporary is constructed at the actual call site.
if mode == "namespace":
    class Namespace(dict):
        def __getitem__(self, key):
            if key == "argument":
                return Finalized()
            return super().__getitem__(key)
    class Meta(type):
        @classmethod
        def __prepare__(cls, name, bases):
            return Namespace()
    cast = typing.cast
    class Prepared(metaclass=Meta):
        result = cast(argument, 1)
    result = Prepared.result
elif target is body:
    result = body(Finalized()) if mode == "temporary" else body(argument)
elif target is typing.cast:
    result = typing.cast(Finalized(), 1) if mode == "temporary" else typing.cast(argument, 1)
else:
    if mode == "temporary":
        result = typing.assert_type(1, Finalized())
    else:
        result = typing.assert_type(1, argument)
assert result == 1
event("caller_continued")
# Retained controls deliberately finish before process-shutdown finalization.
import os
os._exit(0)
'''


def main() -> None:
    assert sys.version_info[:3] == (3, 14, 7)
    cases = {}
    with tempfile.TemporaryDirectory(prefix="lctx-frame-exit-") as temp:
        for target in ("body", "cast", "assert_type"):
            modes = ("temporary", "retained", "literal", "namespace") if target == "cast" else (
                "temporary", "retained", "literal",
            )
            for mode in modes:
                try:
                    result = subprocess.run(
                        [sys.executable, "-I", "-c", WORKER, target, mode],
                        capture_output=True, text=True, timeout=2, check=True, cwd=temp,
                        env={"PATH": os.environ.get("PATH", "")},
                    )
                    output, timed_out = result.stdout, False
                except subprocess.TimeoutExpired as error:
                    output, timed_out = (error.stdout or b"").decode(), True
                events = [json.loads(line) for line in output.splitlines()]
                withheld = mode in {"temporary", "namespace"}
                expected = ["body_return", "finalizer_started"] if withheld else [
                    "body_return", "caller_continued",
                ]
                assert events == expected, (target, mode, events)
                assert timed_out == withheld, (target, mode, timed_out)
                cases[f"{target}_{mode}"] = {"events": events, "timed_out": timed_out}
    print(json.dumps({"outcome": "passed", "python": sys.version.split()[0],
                      "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                      "worker_sha256": hashlib.sha256(WORKER.encode()).hexdigest(),
                      "cases": cases}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
