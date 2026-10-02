"""Independent CPython oracle for current-model serving soundness.

The Rust serving_soundness control compiles the grouped generated source with the real
behavioral Catalog producer, then challenges named native paths against these observations.
May compatibility is never an established identity claim, and a refuted path is never
whole-operation absence. Only these generated programs execute, never analyzer fixtures.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
from dataclasses import asdict, dataclass
from pathlib import Path

from hypothesis import given, settings
from hypothesis import strategies as st


@dataclass(frozen=True)
class Function:
    name: str
    flagged: bool
    body: str
    decorated: bool = False


# Leaf shapes take `value` (and, when flagged, `flag`); callers reference earlier unflagged or
# flagged functions only, so every package is acyclic.
LEAVES = {
    "ident": (False, "    return value\n"),
    "const": (False, "    return 1\n"),
    "none": (False, "    return None\n"),
    "logged": (False, "    print(value)\n    return 1\n"),
    "listed": (False, "    return [value]\n"),
    "guard": (True, "    if flag:\n        return value\n    return None\n"),
}
# A decorator that replaces the binding: its body never runs (ADR-0064, target review F02).
DECORATED = "    return value\n"


@st.composite
def packages(draw: st.DrawFn) -> list[Function]:
    functions: list[Function] = []
    for index in range(draw(st.integers(min_value=3, max_value=8))):
        name = f"f{index}"
        plain = [f for f in functions if not f.flagged]
        flagged = [f for f in functions if f.flagged]
        shapes = ["leaf"] + (["wrap", "assign", "pre", "nest"] if plain else [])
        shapes += ["call_flag"] if flagged else []
        shape = draw(st.sampled_from(shapes))
        if shape == "leaf":
            leaf = draw(st.sampled_from([*sorted(LEAVES), "decorated"]))
            if leaf == "decorated":
                functions.append(Function(name, False, DECORATED, decorated=True))
                continue
            is_flagged, body = LEAVES[leaf]
            functions.append(Function(name, is_flagged, body))
            continue
        if shape == "call_flag":
            callee = draw(st.sampled_from(flagged)).name
            literal = draw(st.sampled_from(["True", "False"]))
            functions.append(Function(name, False, f"    return {callee}(value, {literal})\n"))
            continue
        callee = draw(st.sampled_from(plain)).name
        body = {
            "wrap": f"    return {callee}(value)\n",
            "assign": f"    result = {callee}(value)\n    return result\n",
            "pre": f"    {callee}(value)\n    return value\n",
            "nest": f"    return {callee}({draw(st.sampled_from(plain)).name}(value))\n",
        }[shape]
        functions.append(Function(name, False, body))
    return functions


WORKER = r"""
import ast, importlib, json, resource, socket, sys
resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
sys.path.insert(0, sys.argv[1])
module = importlib.import_module("demo")
filename = module.__file__
source = open(filename, encoding="utf-8").read()
tests = {node.lineno: compile(ast.Expression(node.test), filename, "eval")
         for node in ast.walk(ast.parse(source)) if isinstance(node, ast.If)}
plan = json.loads(sys.argv[2])
out = {}
for name, flagged in plan:
    runs = []
    for scalar in (False, True):
        for flag in ([True, False] if flagged else [None]):
            sentinel = "oracle-sentinel".encode().decode() if scalar else object()
            entries, last_lines, returns, guards = {}, {}, [], []
            def trace(frame, event, arg):
                if frame.f_code.co_filename != filename:
                    return trace
                key = id(frame)
                if event == "call":
                    entries[key] = frame.f_locals.get("value")
                elif event == "line":
                    last_lines[key] = frame.f_lineno
                    if frame.f_lineno in tests:
                        guards.append({"function": frame.f_code.co_name,
                                       "line": frame.f_lineno,
                                       "value": bool(eval(tests[frame.f_lineno], frame.f_globals, frame.f_locals))})
                elif event == "return":
                    returns.append({"function": frame.f_code.co_name,
                                    "line": last_lines.get(key, frame.f_lineno),
                                    "identity_entry": arg is entries.get(key),
                                    "identity_sentinel": arg is sentinel})
                    entries.pop(key, None)
                    last_lines.pop(key, None)
                return trace
            args = (sentinel, flag) if flagged else (sentinel,)
            sys.settrace(trace)
            try:
                result = getattr(module, name)(*args)
                runs.append({"flag": flag, "scalar": scalar, "identity": result is sentinel,
                             "raised": None, "returns": returns, "guards": guards})
            except Exception as exc:
                runs.append({"flag": flag, "scalar": scalar, "identity": False,
                             "raised": type(exc).__name__, "returns": returns, "guards": guards})
            finally:
                sys.settrace(None)
    out[name] = runs
print(json.dumps(out))
"""


def _source(functions: list[Function]) -> str:
    parts = ['"""Independent generated-program soundness challenge."""\n',
             "__all__ = " + repr([f.name for f in functions]) + "\n",
             "\ndef _replace(function):\n    return lambda *args: None\n"]
    for f in functions:
        params = "value, flag" if f.flagged else "value"
        parts.append(("\n@_replace\n" if f.decorated else "\n")
                     + f'def {f.name}({params}):\n    """Generated."""\n{f.body}')
    return "".join(parts)


def _observe(source: str, functions: list[Function]) -> dict:
    with tempfile.TemporaryDirectory() as tmp:
        package = Path(tmp) / "demo"
        package.mkdir()
        (package / "__init__.py").write_text(source, encoding="utf-8")
        plan = json.dumps([[f.name, f.flagged] for f in functions])
        done = subprocess.run([sys.executable, "-I", "-c", WORKER, tmp, plan],
                              capture_output=True, text=True, timeout=20, check=True)
    return json.loads(done.stdout.splitlines()[-1])


def known_shapes() -> list[Function]:
    return [Function("f0", False, LEAVES["ident"][1]),
            Function("f1", False, "    return f0(value)\n"),
            Function("f2", True, LEAVES["guard"][1]),
            Function("f3", False, "    return f2(value, False)\n"),
            Function("f4", False, LEAVES["logged"][1]),
            Function("f5", False, "    f4(value)\n    return value\n"),
            Function("f6", False, DECORATED, decorated=True),
            Function("f7", False, "    return f6(value)\n"),
            Function("f8", False, "    if value is None:\n        return value\n    return value\n")]


def challenge_bundle() -> dict:
    """Group the known program and eight generated acyclic programs into one actual package."""
    groups = [known_shapes()]

    @settings(max_examples=8, derandomize=True, deadline=None, database=None)
    @given(packages())
    def collect(functions: list[Function]) -> None:
        groups.append(functions)

    collect()
    functions = []
    for index, group in enumerate(groups):
        names = {f.name: f"p{index}_{f.name}" for f in group}
        for f in group:
            body = re.sub(r"\bf\d+\b", lambda match, names=names: names[match.group()], f.body)
            functions.append(Function(names[f.name], f.flagged, body, f.decorated))
    source = _source(functions)
    return {"source": source, "functions": [asdict(f) for f in functions],
            "observed": _observe(source, functions), "groups": len(groups)}


def test_known_shapes_preserve_independent_identity_and_decorator_observations() -> None:
    functions = known_shapes()
    observed = _observe(_source(functions), functions)
    for name in ["f0", "f1", "f5", "f8"]:
        assert all(run["identity"] for run in observed[name])
    for name in ["f3", "f4", "f6", "f7"]:
        assert all(not run["identity"] for run in observed[name])
    assert all(run["identity"] == run["flag"] for run in observed["f2"])
    assert all(run["raised"] is None for runs in observed.values() for run in runs)
    assert any(run["guards"] for run in observed["f8"])


@settings(max_examples=8, derandomize=True, deadline=None, database=None)
@given(packages())
def test_generated_acyclic_programs_complete_with_fresh_sentinel_observations(functions: list[Function]) -> None:
    observed = _observe(_source(functions), functions)
    assert set(observed) == {f.name for f in functions}
    assert all(run["raised"] is None for runs in observed.values() for run in runs)
    # CPython's object sentinel and the fresh exact-scalar string agree on these generated shapes.
    for f in functions:
        object_runs = [run for run in observed[f.name] if not run["scalar"]]
        scalar_runs = [run for run in observed[f.name] if run["scalar"]]
        assert [run["identity"] for run in object_runs] == [run["identity"] for run in scalar_runs]


if __name__ == "__main__":
    if sys.argv[1:] != ["--bundle"]:
        raise SystemExit("expected --bundle")
    print(json.dumps(challenge_bundle()))
