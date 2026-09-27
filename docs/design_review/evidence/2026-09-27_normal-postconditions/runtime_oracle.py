"""Independent normal-exit implications; generated programs, never analyzer fixtures."""

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

CASES = {
    "register_normal": "result = atexit.register(callback)",
    "register_raises": "result = atexit.register(0)",
    "serialize_normal": "result = json.dump([1, 2], sink)",
    "serialize_raises": "result = json.dump([1, object()], sink)",
    "acquire_normal": "result = open('payload', 'r')",
    "acquire_raises": "result = open('missing', 'r')",
}
WORKER = r'''
import atexit, hashlib, io, json, resource, socket, sys
resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
assert sys.version_info[:3] == (3, 14, 7)
source = open(sys.argv[1], encoding="utf-8").read()
namespace = {"atexit": atexit, "json": json}
exec(compile(source, sys.argv[1], "exec"), namespace)
events, chunks = [], []
def callback():
    events.append("invoked")
class Sink:
    def write(self, chunk):
        chunks.append(chunk)
result = None
try:
    result = namespace["run"](callback, Sink())
    outcome = "normal"
except BaseException as error:
    outcome = type(error).__name__
before_shutdown = list(events)
# Oracle only: exercise the registered exit queue after the call observation.
atexit._run_exitfuncs()
resource_result = isinstance(result, io.IOBase)
resource_open = resource_result and not result.closed
if resource_result:
    result.close()
print(json.dumps({"outcome": outcome, "before_shutdown": before_shutdown,
    "after_shutdown": events, "same_callback": result is callback, "chunks": chunks,
    "resource_result": resource_result, "resource_open": resource_open,
    "source_sha256": hashlib.sha256(source.encode()).hexdigest()}))
'''


def main() -> None:
    assert sys.version_info[:3] == (3, 14, 7)
    cases = {}
    with tempfile.TemporaryDirectory(prefix="lctx-postconditions-") as temp:
        Path(temp, "payload").write_text("payload", encoding="utf-8")
        for name, body in CASES.items():
            source = f"def run(callback, sink):\n    {body}\n    return result\n"
            path = Path(temp, f"{name}.py")
            path.write_text(source, encoding="utf-8")
            run = subprocess.run(
                [sys.executable, "-I", "-c", WORKER, str(path)], cwd=temp,
                capture_output=True, text=True, timeout=5, check=True,
                env={"PATH": os.environ.get("PATH", "")},
            )
            cases[name] = json.loads(run.stdout)
    registered = cases["register_normal"]
    assert registered["outcome"] == "normal" and registered["same_callback"]
    assert registered["before_shutdown"] == [] and registered["after_shutdown"] == ["invoked"]
    assert cases["register_raises"]["outcome"] == "TypeError"
    assert cases["register_raises"]["after_shutdown"] == []
    assert cases["serialize_normal"]["outcome"] == "normal"
    assert json.loads("".join(cases["serialize_normal"]["chunks"])) == [1, 2]
    assert cases["serialize_raises"]["outcome"] == "TypeError"
    assert cases["serialize_raises"]["chunks"]
    assert cases["acquire_normal"]["outcome"] == "normal"
    assert cases["acquire_normal"]["resource_open"]
    assert cases["acquire_raises"]["outcome"] == "FileNotFoundError"
    assert not cases["acquire_raises"]["resource_result"]
    print(json.dumps({"outcome": "passed", "python": sys.version.split()[0],
                      "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                      "cases": cases}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
