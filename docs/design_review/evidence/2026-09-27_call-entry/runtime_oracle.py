"""Independent generated invocation-prefix controls; never execute analyzer fixtures."""

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

CASES = {
    "before_raise": ("compress(value, compresslevel=9, mtime=0)\n    raise None", ["compress"]),
    "after_raise": ("raise None\n    compress(value, compresslevel=9, mtime=0)", []),
    "after_opaque": ("unknown()\n    compress(value, compresslevel=9, mtime=0)", []),
    "raising_argument": ("compress(value, compresslevel=1 / 0, mtime=0)", []),
    "missing_defaults": ("compress(value)", ["compress"]),
    "selected": ("if True:\n        compress(value, compresslevel=9, mtime=0)", ["compress"]),
    "skipped": ("if False:\n        compress(value, compresslevel=9, mtime=0)", []),
    "assigned": ("result = decompress(value)\n    return result", ["decompress"]),
    "returned": ("return decompress(value)", ["decompress"]),
    "register_only": ("register(callback)", ["register"]),
    "after_total": ("cast(object, 1)\n    decompress(value)", ["decompress"]),
    "after_fallible": (
        "decompress(value)\n    compress(value, compresslevel=9, mtime=0)",
        ["decompress"],
    ),
    "nested_argument": ("cast(object, decompress(value))", ["decompress"]),
}

WORKER = r"""
import atexit, gzip, hashlib, json, resource, socket, sys, typing
resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
assert sys.version_info[:3] == (3, 14, 7)
source = open(sys.argv[1], encoding="utf-8").read()
namespace = {}
exec(compile(source, sys.argv[1], "exec"), namespace)
events = []
targets = {gzip.compress: "compress", gzip.decompress: "decompress", atexit.register: "register"}
mon = sys.monitoring
tool = mon.PROFILER_ID
mon.use_tool_id(tool, "lctx-call-entry")
def called(code, offset, callee, first_argument):
    if code.co_filename == sys.argv[1] and callee in targets:
        events.append(targets[callee])
mon.register_callback(tool, mon.events.CALL, called)
try:
    mon.set_events(tool, mon.events.CALL)
    try:
        namespace["run"](b"payload")
        outcome = "return"
    except BaseException as error:
        outcome = type(error).__name__
finally:
    mon.set_events(tool, 0)
    mon.free_tool_id(tool)
atexit.unregister(namespace["callback"])
print(json.dumps({"events": events, "outcome": outcome,
    "callback_invoked": namespace["invoked"],
    "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
    "gzip_sha256": hashlib.sha256(open(gzip.__file__, "rb").read()).hexdigest()}))
"""


def main() -> None:
    outcomes = {}
    with tempfile.TemporaryDirectory(prefix="lctx-call-entry-oracle-") as temp:
        for name, (body, expected) in CASES.items():
            source = (
                "from gzip import compress, decompress\nfrom atexit import register\n"
                "from typing import cast\ninvoked = False\n"
                "def callback():\n    global invoked\n    invoked = True\n"
                "def unknown():\n    raise RuntimeError('stop')\n"
                f"def run(value):\n    {body}\n"
            )
            path = Path(temp) / f"{name}.py"
            path.write_text(source, encoding="utf-8")
            result = subprocess.run(
                [sys.executable, "-I", "-c", WORKER, str(path)],
                cwd=temp,
                capture_output=True,
                text=True,
                timeout=5,
                env={"PATH": os.environ.get("PATH", "")},
                check=True,
            )
            observed = json.loads(result.stdout)
            assert observed["events"] == expected, (name, observed)
            assert not observed["callback_invoked"], (name, observed)
            outcomes[name] = observed
    print(
        json.dumps(
            {
                "outcome": "passed",
                "python": sys.version.split()[0],
                "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                "cases": outcomes,
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
