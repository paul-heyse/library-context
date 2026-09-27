"""Generated partial-I/O timing challenges; never imports analyzer fixtures."""

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

CALL = (
    "dump(value, sink, skipkeys=False, ensure_ascii=True, check_circular=True, "
    "allow_nan=True, cls=None, indent=None, separators=None, default=None, sort_keys=False)"
)
CASES = {
    "normal": (CALL, "simple", True, True, "return"),
    "before_raise": (CALL + "\n    raise None", "simple", True, True, "TypeError"),
    "after_raise": ("raise None\n    " + CALL, "simple", False, False, "TypeError"),
    "partial_encoding": (CALL, "partial", True, True, "TypeError"),
    "no_write_encoding": (CALL, "opaque", True, False, "TypeError"),
    "raising_argument": (
        CALL.replace("skipkeys=False", "skipkeys=1 / 0"),
        "simple", False, False, "ZeroDivisionError",
    ),
    "after_opaque": ("unknown()\n    " + CALL, "simple", False, False, "RuntimeError"),
    "missing_defaults": ("dump(value, sink)", "simple", True, True, "return"),
    "skipped": ("if False:\n        " + CALL, "simple", False, False, "return"),
}

WORKER = r'''
import hashlib, json, resource, socket, sys
resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
assert sys.version_info[:3] == (3, 14, 7)
source = open(sys.argv[1], encoding="utf-8").read()
namespace = {}
exec(compile(source, sys.argv[1], "exec"), namespace)
chunks, calls = [], []
class Sink:
    def write(self, chunk):
        chunks.append(chunk)
value = {"simple": [1, 2], "partial": [1, object()], "opaque": object()}[sys.argv[2]]
mon = sys.monitoring
tool = mon.PROFILER_ID
mon.use_tool_id(tool, "lctx-action-triggers")
def called(code, offset, callee, first_argument):
    if code.co_filename == sys.argv[1] and callee is json.dump:
        calls.append("dump")
mon.register_callback(tool, mon.events.CALL, called)
try:
    mon.set_events(tool, mon.events.CALL)
    try:
        namespace["run"](value, Sink())
        outcome = "return"
    except BaseException as error:
        outcome = type(error).__name__
finally:
    mon.set_events(tool, 0)
    mon.free_tool_id(tool)
print(json.dumps({"calls": calls, "chunks": chunks, "outcome": outcome,
    "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
    "json_sha256": hashlib.sha256(open(json.__file__, "rb").read()).hexdigest()}))
'''


def main() -> None:
    outcomes = {}
    with tempfile.TemporaryDirectory(prefix="lctx-action-trigger-oracle-") as temp:
        for name, (body, value, called, wrote, outcome) in CASES.items():
            source = (
                "from json import dump\n"
                "def unknown():\n    raise RuntimeError('stop')\n"
                f"def run(value, sink):\n    {body}\n"
            )
            path = Path(temp) / f"{name}.py"
            path.write_text(source, encoding="utf-8")
            result = subprocess.run(
                [sys.executable, "-I", "-c", WORKER, str(path), value],
                cwd=temp,
                capture_output=True,
                text=True,
                timeout=5,
                env={"PATH": os.environ.get("PATH", "")},
                check=True,
            )
            observed = json.loads(result.stdout)
            assert bool(observed["calls"]) == called, (name, observed)
            assert bool(observed["chunks"]) == wrote, (name, observed)
            assert observed["outcome"] == outcome, (name, observed)
            outcomes[name] = observed
    print(json.dumps({
        "outcome": "passed", "python": sys.version.split()[0],
        "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "cases": outcomes,
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
