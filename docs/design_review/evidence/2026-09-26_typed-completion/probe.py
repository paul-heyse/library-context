"""Independent generated programs; never executes compiler fixtures or compiler output."""
import importlib.metadata
import json
import sys
from collections import Counter

from hypothesis import given, settings, strategies as st

assert sys.version_info[:3] == (3, 14, 7)
assert importlib.metadata.version("hypothesis") == "6.168.1"

bodies = {
    "same_class": "try:\n    raise None\nexcept TypeError:\n    pass",
    "superclass": "try:\n    raise 1\nexcept Exception:\n    pass",
    "later": "try:\n    raise 1\nexcept ValueError:\n    raise AssertionError('wrong handler')\nexcept TypeError:\n    pass",
    "reraised": "try:\n    try:\n        raise 1\n    except TypeError:\n        raise\nexcept Exception:\n    pass",
    "nonmatch": "try:\n    raise 1\nexcept ValueError:\n    pass",
    "handled_then_bare": "try:\n    raise 1\nexcept TypeError:\n    pass\nfinally:\n    raise",
    "active_finally": "try:\n    try:\n        raise 1\n    except TypeError:\n        try:\n            pass\n        finally:\n            raise\nexcept TypeError:\n    pass",
}
observations = Counter()
tool = next(i for i in range(6) if sys.monitoring.get_tool(i) is None)
sys.monitoring.use_tool_id(tool, "typed-completion-oracle")
active = None
events = []


def returned(code, offset, value):
    if code is active:
        events.append("return")


def unwound(code, offset, error):
    if code is active:
        events.append("unwind")


sys.monitoring.register_callback(tool, sys.monitoring.events.PY_RETURN, returned)
sys.monitoring.register_callback(tool, sys.monitoring.events.PY_UNWIND, unwound)
sys.monitoring.set_events(tool, sys.monitoring.events.PY_RETURN | sys.monitoring.events.PY_UNWIND)


@settings(max_examples=100, derandomize=True, database=None, deadline=None)
@given(st.integers(-20, 20), st.sampled_from(sorted(bodies)))
def challenge(value, action):
    global active
    body = "\n".join("        " + line for line in bodies[action].splitlines())
    source = "def observed(value):\n    try:\n        return value\n    finally:\n" + body + "\n"
    namespace = {}
    exec(compile(source, "<typed-completion-oracle>", "exec"), namespace)
    function = namespace["observed"]
    active = function.__code__
    events.clear()
    expected = {"nonmatch": TypeError, "handled_then_bare": RuntimeError}.get(action)
    try:
        result = function(value)
    except Exception as error:
        assert expected is type(error), (action, type(error))
        assert events == ["unwind"], events
    else:
        assert expected is None and result is value
        assert events == ["return"], events
    finally:
        active = None
    observations[action] += 1


try:
    challenge()
finally:
    sys.monitoring.set_events(tool, 0)
    sys.monitoring.free_tool_id(tool)

print(json.dumps({"python": sys.version, "hypothesis": importlib.metadata.version("hypothesis"),
    "generated_cases": sum(observations.values()), "observations": dict(sorted(observations.items()))}, indent=2))
