"""Independent CPython challenge of served return claims over compiled generated packages (S7).

Each example generates a small acyclic package, compiles it with the release producer
(`lctx compile-fixture`), loads the served generation exactly as the MCP server does, and runs
every function in an isolated worker with a fresh sentinel argument. Soundness: a served
`established` identity return (`unchanged`) must return the sentinel on every normal completion,
and a `refuted_under_model` return must never return it. Only generated programs run here;
analyzer fixtures and analyzed libraries are never executed. Precision is counted, not asserted.
"""

from __future__ import annotations

import asyncio
import json
import os
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from functools import cache
from pathlib import Path

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st
from lctx_storage import open_repository

from lctx_mcp import operations as ops
from postgres_expand import write_secret
from postgres_test_support import database

ROOT = Path(__file__).resolve().parents[2]
LCTX = ROOT / "target/release/lctx"
PACKAGE = "genpkg"


@cache
def _lctx() -> Path:
    # Challenge the release producer that integrated compilation uses, in this checkout's cache.
    env = {**os.environ, "CARGO_TARGET_DIR": str(ROOT / "target")}
    subprocess.run(
        ["cargo", "build", "--release", "-p", "lctx", "--quiet"],
        cwd=ROOT,
        env=env,
        check=True,
        timeout=900,
    )
    return LCTX


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


def _source(functions: list[Function]) -> str:
    parts = [
        '"""Generated for the semantic soundness challenge; never an analyzer fixture."""\n',
        "\n\ndef _replace(function):\n    return lambda *args: None\n",
    ]
    for f in functions:
        params = "value, flag" if f.flagged else "value"
        if f.decorated:
            parts.append("\n\n@_replace")
        # A docstring gives the analysis seed a documented outcome (an undocumented,
        # analysis-free seed fails `semantic:documentation-only-has-outcome`).
        parts.append(
            ("\n" if f.decorated else "\n\n")
            + f'def {f.name}({params}):\n    """Generated."""\n{f.body}'
        )
    return "".join(parts)


WORKER = r"""
import importlib, json, resource, socket, sys

resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
sys.path.insert(0, sys.argv[1])
module = importlib.import_module(sys.argv[2])
plan = json.loads(sys.argv[3])
out = {}
for name, flagged in plan:
    runs = []
    for flag in ([True, False] if flagged else [None]):
        sentinel = object()
        args = (sentinel, flag) if flagged else (sentinel,)
        try:
            result = getattr(module, name)(*args)
            runs.append({"flag": flag, "identity": result is sentinel, "raised": None})
        except Exception as e:
            runs.append({"flag": flag, "identity": False, "raised": type(e).__name__})
    out[name] = runs
print(json.dumps(out))
"""


def _observe(tree: Path, functions: list[Function]) -> dict[str, list[dict]]:
    plan = json.dumps([[f.name, f.flagged] for f in functions])
    done = subprocess.run(
        [sys.executable, "-I", "-c", WORKER, str(tree), PACKAGE, plan],
        capture_output=True,
        text=True,
        timeout=20,
        check=True,
    )
    return json.loads(done.stdout.splitlines()[-1])


def _served(tree: Path, work: Path, functions: list[Function], db) -> dict[str, list[ops.Fate]]:
    done = subprocess.run(
        [
            str(_lctx()),
            "compile-fixture",
            str(tree),
            "--package",
            PACKAGE,
            "--seed",
            f"{PACKAGE}.{functions[-1].name}",
            "--store",
            str(work / "store"),
            "--generations",
            str(work / "generations"),
        ],
        capture_output=True,
        text=True,
        timeout=300,
        check=True,
    )
    line = next(x for x in done.stdout.splitlines() if x.startswith("generation "))
    bundle = Path(line.removeprefix("generation "))
    serving, importer, call, artifacts = db
    call(
        [
            str(_lctx()),
            "serving",
            "--importer-config",
            str(importer),
            "import-bundle",
            "--bundle",
            str(bundle),
            "--artifacts",
            str(artifacts),
        ]
    )
    manifest = json.loads((bundle / "MANIFEST.json").read_text())

    async def fetch():
        repo = await open_repository(serving)
        try:
            pin = await repo.pin(PACKAGE, manifest["projection_generation"], None)
            served = {}
            for f in functions:
                op = ops.Operation.model_validate_json(
                    await pin.get_operation(manifest["snapshot_id"], f"{PACKAGE}.{f.name}")
                )
                (value,) = [p for p in op.parameters if p.name == "value"]
                served[f.name] = [fate for fate in value.fates if fate.kind == "returns"]
            return served
        finally:
            await repo.close()

    return asyncio.run(fetch())


@pytest.fixture(scope="module")
def semantic_database(tmp_path_factory):
    tmp = tmp_path_factory.mktemp("semantic-postgres")
    with database(tmp) as (serving, _command, call, role, _port):
        importer = tmp / "importer.json"
        write_secret(
            importer,
            {
                **role,
                "role": "importer",
                "url": role["url"].replace("lctx_serving:", "lctx_importer:"),
                "statement_timeout_seconds": 30,
            },
        )
        serving = tmp / "runtime-serving.json"
        write_secret(serving, {**role, "statement_timeout_seconds": 30})
        yield serving, importer, call, tmp / "artifacts"


PRECISION = {"identity_observed": 0, "identity_established": 0}


def _check(functions: list[Function], db) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        tree = work / "tree"
        (tree / PACKAGE).mkdir(parents=True)
        source = _source(functions)
        (tree / PACKAGE / "__init__.py").write_text(source, encoding="utf-8")
        observed = _observe(tree, functions)
        served = _served(tree, work, functions, db)
    for f in functions:
        returns = served[f.name]
        normal = [r for r in observed[f.name] if r["raised"] is None]
        identity = any(r["identity"] for r in normal)
        # Identity claims: a direct `unchanged` return, or a call-transfer return discharged by
        # proved summaries (every current summary is a Value-kind identity path, ADR-0064).
        established = [
            r
            for r in returns
            if r.verdict == "established"
            and (
                (r.value or "").startswith("unchanged")
                or (
                    r.transfer == "call"
                    and bool(r.discharges)
                    and all(d.decision == "proved" for d in r.discharges)
                )
            )
        ]
        refuted = [r for r in returns if r.verdict == "refuted_under_model"]
        PRECISION["identity_observed"] += identity
        PRECISION["identity_established"] += identity and bool(established)
        assert not (established and not all(r["identity"] for r in normal)), (
            f"{f.name}: served established identity return, but CPython did not return the "
            f"argument\n{source}\nobserved {observed[f.name]}\nserved {returns}"
        )
        assert not (refuted and identity), (
            f"{f.name}: served a refuted return, but CPython returned the argument\n{source}"
        )


def test_known_shapes(semantic_database) -> None:
    fs = [
        Function("f0", False, LEAVES["ident"][1]),
        Function("f1", False, "    return f0(value)\n"),
        Function("f2", True, LEAVES["guard"][1]),
        Function("f3", False, "    return f2(value, False)\n"),
        Function("f4", False, LEAVES["logged"][1]),
        Function("f5", False, "    f4(value)\n    return value\n"),
        Function("f6", False, DECORATED, decorated=True),
        Function("f7", False, "    return f6(value)\n"),
    ]
    _check(fs, semantic_database)


@settings(max_examples=8, derandomize=True, deadline=None, database=None)
@given(packages())
def test_served_returns_admit_cpython(semantic_database, functions: list[Function]) -> None:
    _check(functions, semantic_database)
