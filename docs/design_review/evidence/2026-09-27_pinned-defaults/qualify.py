"""Pinned stdlib default qualification and isolated generated entry controls."""

import ast
import gzip
import hashlib
import inspect
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

EXPECTED = {
    "json.dump": ["skipkeys", "ensure_ascii", "check_circular", "allow_nan", "cls",
                  "indent", "separators", "default", "sort_keys"],
    "json.dumps": ["skipkeys", "ensure_ascii", "check_circular", "allow_nan", "cls",
                   "indent", "separators", "default", "sort_keys"],
    "json.loads": ["cls", "object_hook", "parse_float", "parse_int", "parse_constant",
                   "object_pairs_hook"],
    "gzip.compress": ["compresslevel", "mtime"],
}
CASES = {
    "dump_defaults": ("dump([1, 2], sink)", ["dump"], "return"),
    "dump_fallible": ("dump([1, value], sink)", ["dump"], "TypeError"),
    "dumps_fallible": ("dumps(value)", ["dumps"], "TypeError"),
    "loads_fallible": ("loads('!')", ["loads"], "JSONDecodeError"),
    "compress_defaults": ("compress(b'payload')", ["compress"], "return"),
    "missing_required": ("dump([1, 2])", [], "TypeError"),
    "raising_explicit": ("dump([1, 2], sink, indent=1 / 0)", [], "ZeroDivisionError"),
}
WORKER = r'''
import gzip, hashlib, json, resource, socket, sys
resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
assert sys.version_info[:3] == (3, 14, 7)
source = open(sys.argv[1], encoding="utf-8").read()
namespace = {}
exec(compile(source, sys.argv[1], "exec"), namespace)
starts, chunks = [], []
class Sink:
    def write(self, chunk):
        chunks.append(chunk)
targets = {f.__code__: f.__name__ for f in [json.dump, json.dumps, json.loads, gzip.compress]}
mon = sys.monitoring
tool = mon.PROFILER_ID
mon.use_tool_id(tool, "lctx-pinned-defaults")
def started(code, offset):
    if code in targets:
        starts.append(targets[code])
mon.register_callback(tool, mon.events.PY_START, started)
try:
    mon.set_events(tool, mon.events.PY_START)
    try:
        namespace["run"](object(), Sink())
        outcome = "return"
    except BaseException as error:
        outcome = type(error).__name__
finally:
    mon.set_events(tool, 0)
    mon.free_tool_id(tool)
print(json.dumps({"starts": starts, "chunks": chunks, "outcome": outcome,
    "source_sha256": hashlib.sha256(source.encode()).hexdigest()}))
'''


def main() -> None:
    assert sys.version_info[:3] == (3, 14, 7)
    qualified = {}
    for function in [json.dump, json.dumps, json.loads, gzip.compress]:
        name = f"{function.__module__}.{function.__name__}"
        source = inspect.getsource(function)
        node = ast.parse(source).body[0]
        assert isinstance(node, ast.FunctionDef)
        args = node.args
        positional = [a.arg for a in args.posonlyargs + args.args]
        declared = (positional[-len(args.defaults):] if args.defaults else []) + [
            a.arg for a, default in zip(args.kwonlyargs, args.kw_defaults, strict=True)
            if default is not None
        ]
        runtime = [p.name for p in inspect.signature(function).parameters.values()
                   if p.default is not inspect.Parameter.empty]
        assert declared == runtime == EXPECTED[name], (name, declared, runtime)
        qualified[name] = {
            "available_formals": runtime,
            "function_source_sha256": hashlib.sha256(source.encode()).hexdigest(),
        }
    outcomes = {}
    with tempfile.TemporaryDirectory(prefix="lctx-pinned-defaults-") as temp:
        for name, (body, expected, outcome) in CASES.items():
            source = (
                "from json import dump, dumps, loads\nfrom gzip import compress\n"
                f"def run(value, sink):\n    {body}\n"
            )
            path = Path(temp) / f"{name}.py"
            path.write_text(source, encoding="utf-8")
            result = subprocess.run(
                [sys.executable, "-I", "-c", WORKER, str(path)], cwd=temp,
                capture_output=True, text=True, timeout=5,
                env={"PATH": os.environ.get("PATH", "")}, check=True,
            )
            observed = json.loads(result.stdout)
            assert observed["starts"] == expected, (name, observed)
            assert observed["outcome"] == outcome, (name, observed)
            outcomes[name] = observed
    assert outcomes["dump_fallible"]["chunks"]
    print(json.dumps({
        "outcome": "passed", "python": sys.version.split()[0], "qualified": qualified,
        "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "cases": outcomes,
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
