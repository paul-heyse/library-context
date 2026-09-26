"""Isolated CPython observations; no fixtures or compiler output are executed."""
import importlib.metadata
import json
import subprocess
import sys
from collections import Counter

from hypothesis import given, settings, strategies as st

assert sys.version_info[:3] == (3, 14, 7), sys.version
assert importlib.metadata.version("hypothesis") == "6.168.1"

observations = Counter()
tool = next(i for i in range(6) if sys.monitoring.get_tool(i) is None)
sys.monitoring.use_tool_id(tool, "stage3-completion-oracle")
active_code = None
events = []


def returned(code, offset, value):
    if code is active_code:
        events.append("return")


def unwound(code, offset, exception):
    if code is active_code:
        events.append("unwind")


sys.monitoring.register_callback(tool, sys.monitoring.events.PY_RETURN, returned)
sys.monitoring.register_callback(tool, sys.monitoring.events.PY_UNWIND, unwound)
sys.monitoring.set_events(tool, sys.monitoring.events.PY_RETURN | sys.monitoring.events.PY_UNWIND)


@settings(max_examples=120, derandomize=True, database=None, deadline=None)
@given(st.integers(-10, 10), st.booleans(), st.sampled_from(["normal", "override", "raise", "handled"]))
def challenge(value, selected, action):
    global active_code
    finalizer = {
        "normal": "marker = cast(object, 1)",
        "override": "return None",
        "raise": "raise None",
        "handled": "try:\n            raise None\n        except:\n            pass",
    }[action]
    source = (
        "from typing import cast\n"
        "def observed(value):\n"
        "    try:\n"
        f"        return cast(object if {selected!r} else object, value)\n"
        "    finally:\n"
        f"        {finalizer}\n"
    )
    namespace = {}
    exec(compile(source, "<generated-completion-control>", "exec"), namespace)
    function = namespace["observed"]
    active_code = function.__code__
    events.clear()
    try:
        result = function(value)
    except TypeError:
        assert action == "raise"
        assert events == ["unwind"], events
        observations["raises_type_error"] += 1
    else:
        assert action != "raise"
        assert result is (None if action == "override" else value)
        assert events == ["return"], events
        observations[action] += 1
    finally:
        active_code = None


try:
    challenge()
finally:
    sys.monitoring.set_events(tool, 0)
    sys.monitoring.free_tool_id(tool)

# A started user constructor/finalizer need not complete. The bounded runtime observation is
# timeout, not a proof of divergence. Each child starts its marker inside the implicit action.
implicit = {
    "exception_class_construction": """
class Pending(Exception):
    def __new__(cls):
        print('implicit-action-started', flush=True)
        while True: pass
def observed(value):
    try: return value
    finally:
        try: raise Pending
        except: pass
observed(1)
""",
    "local_rebinding_finalization": """
class Pending:
    def __del__(self):
        print('implicit-action-started', flush=True)
        while True: pass
def observed(value, previous):
    try: return value
    finally: previous = 1
observed(1, Pending())
""",
}
timeouts = {}
for name, source in implicit.items():
    try:
        subprocess.run([sys.executable, "-I", "-c", source], capture_output=True, timeout=2, check=True)
    except subprocess.TimeoutExpired as error:
        assert b"implicit-action-started" in error.stdout, (name, error.stdout)
        timeouts[name] = "started_no_completion_before_2s_timeout"
    else:
        raise AssertionError(f"{name} unexpectedly completed")
print(json.dumps({"python": sys.version, "hypothesis": importlib.metadata.version("hypothesis"),
    "generated_cases": sum(observations.values()), "observations": dict(sorted(observations.items())),
    "implicit_execution": timeouts}, indent=2))
