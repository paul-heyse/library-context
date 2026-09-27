"""Generated source-body and function-object retention controls, independent of compiler facts."""

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

CASES = {
    "fallthrough": ("pass", "return"),
    "docstring_only": ('"Definition metadata only."', "return"),
    "literal_return": ("return (1, 'closed')", "return"),
    "closed_local": ("unused = (2, 3)\nreturn 4", "return"),
    "selected_branch": ("if True:\n    return 5\nelse:\n    unknown()", "return"),
    "pass_finally": ("try:\n    return 6\nfinally:\n    pass", "return"),
    "return_finally": ("try:\n    return 7\nfinally:\n    return 8", "return"),
    "raise_finally": ("try:\n    return 9\nfinally:\n    raise None", "TypeError"),
    "primitive_raise": ("raise None", "TypeError"),
    "local_read": ("value = 10\nreturn value", "return"),
}

WORKER = r'''
import json, os, resource, socket, sys, time
resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
assert sys.version_info[:3] == (3, 14, 7)
def event(name):
    print(json.dumps(name), flush=True)
def returned(code, offset, value):
    if code.co_name == "inner":
        event("body_return")
def unwound(code, offset, error):
    if code.co_name == "inner":
        event("body_raise:" + type(error).__name__)
mon = sys.monitoring
mon.use_tool_id(4, "source-body-oracle")
mon.register_callback(4, mon.events.PY_RETURN, returned)
mon.register_callback(4, mon.events.PY_UNWIND, unwound)
mon.set_events(4, mon.events.PY_RETURN | mon.events.PY_UNWIND)
if sys.argv[1] == "body":
    source = open(sys.argv[2], encoding="utf-8").read()
    namespace = {}
    exec(compile(source, sys.argv[2], "exec"), namespace)
    try:
        namespace["inner"]()
        event("caller_continued")
    except BaseException as error:
        event("caller_caught:" + type(error).__name__)
else:
    class Finalized:
        def __del__(self):
            event("function_attribute_finalizer")
            time.sleep(30)
    def make():
        def inner():
            return 11
        inner.observer = Finalized()
        return inner
    if sys.argv[1] == "retained_function":
        retained = make()
        retained()
    else:
        make()()
    event("caller_continued")
os._exit(0)
'''


def main() -> None:
    results = {}
    with tempfile.TemporaryDirectory(prefix="lctx-source-body-") as temp:
        specs = []
        for name, (body, outcome) in CASES.items():
            indented = "\n".join(f"    {line}" for line in body.splitlines())
            source = f"def inner():\n{indented}\n"
            path = Path(temp) / f"{name}.py"
            path.write_text(source)
            expected = ["body_return", "caller_continued"] if outcome == "return" else [
                f"body_raise:{outcome}", f"caller_caught:{outcome}",
            ]
            specs.append((name, ["body", str(path)], expected, False, source))
        specs += [
            ("retained_function", ["retained_function"],
             ["body_return", "caller_continued"], False, ""),
            ("temporary_function", ["temporary_function"],
             ["body_return", "function_attribute_finalizer"], True, ""),
        ]
        for name, args, expected, timeout_expected, source in specs:
            try:
                result = subprocess.run(
                    [sys.executable, "-I", "-c", WORKER, *args], cwd=temp,
                    capture_output=True, text=True, timeout=2, check=True,
                    env={"PATH": os.environ.get("PATH", "")},
                )
                output, timed_out = result.stdout, False
            except subprocess.TimeoutExpired as error:
                output, timed_out = (error.stdout or b"").decode(), True
            events = [json.loads(line) for line in output.splitlines()]
            assert events == expected and timed_out == timeout_expected, (name, events, timed_out)
            results[name] = {"events": events, "timed_out": timed_out,
                             "source_sha256": hashlib.sha256(source.encode()).hexdigest()}
    print(json.dumps({"outcome": "passed", "python": sys.version.split()[0],
                      "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                      "worker_sha256": hashlib.sha256(WORKER.encode()).hexdigest(),
                      "cases": results}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
