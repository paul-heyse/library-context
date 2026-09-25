"""Independent CPython admission checks for the flow translator (ADR-0022, X6).

Only small generated programs in temporary files are executed. Analyzer fixtures and analyzed
libraries are never run. CPython's monitoring API supplies observed lines, local writes/reads and
identity-preserving returns; the Rust developer command supplies modeled regions, reaching
definitions and value sources.
"""

import json
import os
import subprocess
import sys
import tempfile
from functools import cache
from pathlib import Path

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

ROOT = Path(__file__).resolve().parents[2]
FLOW_BIN = ROOT / "target/debug/lctx"


@cache
def _flow_bin() -> Path:
    subprocess.run(["cargo", "build", "-p", "lctx", "--quiet"], cwd=ROOT, check=True, timeout=180)
    return FLOW_BIN


WORKER = r"""
import dis, inspect, json, resource, socket, sys

resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
path = sys.argv[1]
source = open(path, encoding="utf-8").read()
code = compile(source, path, "exec")
lines = set()
accesses = []
stores = {}
loads = {}
returns = []
fates = []
instructions = {}
mon = sys.monitoring
tool = mon.PROFILER_ID
mon.use_tool_id(tool, "lctx-flow-oracle")

def on_line(c, line):
    if c.co_filename == path:
        lines.add(line)

def on_instruction(c, offset):
    if c.co_filename != path:
        return
    if c not in instructions:
        instructions[c] = {i.offset: i for i in dis.get_instructions(c)}
    i = instructions[c].get(offset)
    if i is None or not (i.opname.startswith("STORE_FAST") or i.opname.startswith("LOAD_FAST")):
        return
    p = i.positions
    if p.lineno is None or p.col_offset is None or p.end_lineno is None or p.end_col_offset is None:
        return
    frame = inspect.currentframe().f_back
    key = (id(frame), i.argval)
    position = [p.lineno, p.col_offset, p.end_lineno, p.end_col_offset]
    if i.opname.startswith("STORE_FAST"):
        stores[key] = position
    else:
        if key in stores:
            accesses.append({"name": i.argval, "load": position, "store": stores[key]})
        if i.argval in frame.f_locals:
            loads.setdefault(id(frame), []).append(
                {"name": i.argval, "load": position, "value_id": id(frame.f_locals[i.argval])}
            )

def on_return(c, offset, value):
    if c.co_filename != path:
        return
    frame = inspect.currentframe().f_back
    candidates = loads.get(id(frame), [])
    if c not in instructions:
        instructions[c] = {i.offset: i for i in dis.get_instructions(c)}
    returning = instructions[c].get(offset)
    return_line = returning.positions.lineno if returning is not None else None
    # The return value is a distinct object in the focused oracle case. An identity match is
    # runtime evidence only; require that the load occurred in the return expression as well.
    returns.extend(
        {"name": item["name"], "load": item["load"]}
        for item in candidates
        if item["value_id"] == id(value) and item["load"][0] == return_line
    )
    if return_line is not None:
        fates.append({"event": "return", "line": return_line})

def on_fate(event):
    def record(c, offset, _exception):
        if c.co_filename != path:
            return
        if c not in instructions:
            instructions[c] = {i.offset: i for i in dis.get_instructions(c)}
        instruction = instructions[c].get(offset)
        if instruction is not None and instruction.positions.lineno is not None:
            fates.append({"event": event, "line": instruction.positions.lineno})
    return record

try:
    mon.register_callback(tool, mon.events.LINE, on_line)
    mon.register_callback(tool, mon.events.INSTRUCTION, on_instruction)
    mon.register_callback(tool, mon.events.PY_RETURN, on_return)
    mon.register_callback(tool, mon.events.RAISE, on_fate("raise"))
    mon.register_callback(tool, mon.events.RERAISE, on_fate("reraise"))
    mon.register_callback(tool, mon.events.EXCEPTION_HANDLED, on_fate("handled"))
    mon.register_callback(tool, mon.events.PY_UNWIND, on_fate("unwind"))
    mon.set_events(tool, mon.events.LINE | mon.events.INSTRUCTION | mon.events.PY_RETURN
                   | mon.events.RAISE | mon.events.RERAISE
                   | mon.events.EXCEPTION_HANDLED | mon.events.PY_UNWIND)
    exec(code, {"INPUT": json.loads(sys.argv[2])})
finally:
    mon.set_events(tool, 0)
    mon.free_tool_id(tool)
print(json.dumps({"lines": sorted(lines), "accesses": accesses, "returns": returns,
                  "fates": fates}))
"""


def _flow(source: str, value: object, runtime_bindings: dict | None = None) -> tuple[dict, dict]:
    binary = _flow_bin()
    with tempfile.TemporaryDirectory(prefix="lctx-flow-oracle-") as temp:
        path = Path(temp) / "generated.py"
        path.write_text(source, encoding="utf-8")
        command = [str(binary), "flow", str(path)]
        if runtime_bindings is not None:
            bindings_path = Path(temp) / "runtime-bindings.json"
            bindings_path.write_text(json.dumps(runtime_bindings), encoding="utf-8")
            command += ["--runtime-bindings", str(bindings_path)]
        model = subprocess.run(
            command,
            cwd=temp,
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
        observed = subprocess.run(
            [sys.executable, "-I", "-c", WORKER, str(path), json.dumps(value)],
            cwd=temp,
            check=True,
            capture_output=True,
            text=True,
            timeout=5,
            env={"PATH": os.environ.get("PATH", "")},
        )
    return json.loads(model.stdout), json.loads(observed.stdout)


def _byte(source: str, line: int, column: int) -> int:
    return sum(len(part.encode()) for part in source.splitlines(keepends=True)[: line - 1]) + column


def _span(source: str, position: list[int]) -> list[int]:
    return [_byte(source, position[0], position[1]), _byte(source, position[2], position[3])]


def _assert_admitted(source: str, model: dict, observed: dict) -> None:
    assert observed["lines"], (source, observed)
    for line in observed["lines"]:
        regions = [r for r in model["regions"] if source.count("\n", 0, r["span"][0]) + 1 == line]
        assert regions, ("executed line has no modeled region", line, source)
        for region in regions:
            assert region["condition"] != "false", (line, region, source)


def _assert_reaching(source: str, model: dict, observed: dict) -> int:
    matched = 0
    for access in observed["accesses"]:
        use_span = _span(source, access["load"])
        def_span = _span(source, access["store"])
        uses = [
            i
            for i, row in enumerate(model["uses"])
            if row["place"] == access["name"] and row["span"] == use_span
        ]
        defs = [
            i
            for i, row in enumerate(model["definitions"])
            if row["place"] == access["name"]
            and def_span[0] <= row["target"][0]
            and row["target"][1] <= def_span[1]
        ]
        assert uses and defs, ("observed access has no modeled use or definition", access, model)
        assert any(row["use_ix"] in uses and row["def_ix"] in defs for row in model["reaching"]), (
            "observed definition absent from reaching relation",
            access,
        )
        matched += 1
    return matched


def _assert_return_value_flow(source: str, model: dict, observed: dict) -> int:
    matched = 0
    for returned in observed["returns"]:
        use_span = _span(source, returned["load"])
        uses = [
            i
            for i, row in enumerate(model["uses"])
            if row["place"] == returned["name"] and row["span"] == use_span
        ]
        assert uses, ("returned local has no modeled use", returned, model)
        assert any(
            row["sink"] == "Return" and row["use_ix"] in uses and row["condition"] != "false"
            for row in model["values"]
        ), ("observed local-to-return flow absent", returned, model)
        matched += 1
    return matched


def _assert_exit_admitted(source: str, model: dict, observed: dict) -> int:
    matched = 0
    for fate in observed["fates"]:
        regions = [
            row for row in model["regions"]
            if source.count("\n", 0, row["span"][0]) + 1 == fate["line"]
        ]
        assert regions and any(row["condition"] != "false" for row in regions), (
            "observed exit or exception has no admitted region", fate, source, regions
        )
        matched += 1
    return matched


PROGRAMS = {
    "calls": """state = [True, False]
def probe():
    return state.pop(0)
if probe():
    if not probe():
        reached = 1
""",
    "ranges": """def run(n):
    found = -1
    for i in range(n + 1):
        found = i
    for j in range(0):
        found = j
    return found
result = run(INPUT)
""",
    "mutation": """class Box:
    def __init__(self): self.x = None
    def reset(self): self.x = 1
box = Box()
if box.x is None:
    box.reset()
    if box.x is not None:
        reached = 1
""",
    "truth": """items = [INPUT]
if items:
    items.clear()
    if not items:
        reached = 1
""",
    "equal": """x = 1
if x == True:
    if x is not True:
        reached = 1
""",
    "try_with": """import contextlib
def run():
    try:
        with contextlib.nullcontext():
            value = INPUT
    finally:
        observed = value
    return observed
result = run()
""",
    "rebound_class": """def run(x):
    C = int
    if isinstance(x, C):
        C = str
        if not isinstance(x, C):
            return 1
    return 0
result = run(INPUT)
""",
    "version": """import sys
if sys.version_info > (3, 14, 7):
    reached = 1
else:
    reached = 0
if sys.version_info <= (3, 14, 7):
    reached = 2
""",
    "typing_alias": """from typing import TYPE_CHECKING as TC
if TC:
    reached = 0
else:
    reached = 1
""",
    "nonlocal_change": """def run():
    x = None
    def change():
        nonlocal x
        x = 1
    if x is None:
        change()
        if x is not None:
            return 1
    return 0
result = run()
""",
}


def _assert_case(shape: str, n: int) -> None:
    source = PROGRAMS[shape]
    runtime = None
    if shape == "version":
        runtime = {
            "sys_modules": [
                {"start": at, "end": at + 3}
                for at in (source.index("sys.version_info"), source.rindex("sys.version_info"))
            ]
        }
    elif shape == "typing_alias":
        at = source.index("TC:")
        runtime = {"checking_names": [{"start": at, "end": at + 2}]}
    model, observed = _flow(source, n, runtime)
    _assert_admitted(source, model, observed)
    _assert_reaching(source, model, observed)


@pytest.mark.parametrize("shape", sorted(PROGRAMS))
def test_every_seed_shape_is_admitted(shape: str) -> None:
    _assert_case(shape, 1)


@settings(max_examples=18, deadline=None, derandomize=True)
@given(st.sampled_from(sorted(PROGRAMS)), st.integers(min_value=0, max_value=3))
def test_generated_inputs_are_admitted(shape: str, n: int) -> None:
    _assert_case(shape, n)


def test_observed_local_definition_is_among_reaching_rows() -> None:
    source = """def run(n):
    found = -1
    for i in range(n):
        found = i
    return found
result = run(INPUT)
"""
    model, observed = _flow(source, 2)
    _assert_admitted(source, model, observed)
    assert _assert_reaching(source, model, observed) > 0


def test_observed_identity_return_is_admitted_by_value_sources() -> None:
    source = """def run(flag):
    left = object()
    right = object()
    selected = left if flag else right
    return selected
result = run(INPUT)
"""
    for flag in (False, True):
        model, observed = _flow(source, flag)
        _assert_admitted(source, model, observed)
        assert _assert_return_value_flow(source, model, observed) > 0
        missing = {**model, "values": [v for v in model["values"] if v["sink"] != "Return"]}
        with pytest.raises(AssertionError, match="observed local-to-return flow absent"):
            _assert_return_value_flow(source, missing, observed)


def test_observed_exception_and_exit_regions_are_admitted() -> None:
    source = """def run(flag):
    try:
        if flag:
            raise ValueError("expected")
        return 1
    except ValueError:
        return 2
    finally:
        marker = 3
result = run(INPUT)
"""
    for flag in (False, True):
        model, observed = _flow(source, flag)
        assert _assert_exit_admitted(source, model, observed) > 0
        if flag:
            assert any(row["event"] == "raise" for row in observed["fates"])
        else:
            assert any(row["event"] == "return" for row in observed["fates"])
    missing = {**model, "regions": []}
    with pytest.raises(AssertionError, match="observed exit or exception has no admitted region"):
        _assert_exit_admitted(source, missing, observed)


def test_runtime_binding_spans_are_checked_before_the_oracle_runs() -> None:
    source = "import sys\nif sys.version_info > (3, 14, 7):\n    reached = 1\n"
    with tempfile.TemporaryDirectory(prefix="lctx-flow-oracle-") as temp:
        path = Path(temp) / "generated.py"
        path.write_text(source, encoding="utf-8")
        bad = Path(temp) / "bad.json"
        bad.write_text('{"sys_modules":[{"start":999,"end":1002}]}', encoding="utf-8")
        result = subprocess.run(
            [str(_flow_bin()), "flow", str(path), "--runtime-bindings", str(bad)],
            capture_output=True,
            text=True,
            timeout=10,
        )
    assert result.returncode != 0
    assert "runtime binding span lies outside" in result.stderr
