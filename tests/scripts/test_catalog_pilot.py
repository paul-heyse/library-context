"""Functional adapter controls; these do not qualify FastMCP or native persistence."""

from __future__ import annotations

import ast
import contextlib
import hashlib
import json
import os
import sys
import time
from pathlib import Path
from types import SimpleNamespace

import pytest
from blake3 import blake3

import catalog_pilot as pilot
from harness import ProcessIdentity


def test_actual_child_failure_preserves_json_and_diagnostics(tmp_path):
    commands = pilot.Commands(Path(sys.executable), tmp_path, tmp_path / "runtime")
    with pytest.raises(pilot.PilotError, match="CLI partial failed"):
        commands.run(
            [
                "-c",
                "import sys; print('{\"partial\":true}'); "
                "print('diagnostic',file=sys.stderr); sys.exit(7)",
            ],
            "partial",
        )
    assert json.loads((tmp_path / "0001-partial.json").read_text()) == {"partial": True}
    assert (tmp_path / "0001-partial.stderr.log").read_text() == "diagnostic\n"
    receipt = json.loads((tmp_path / "0001-partial.child.json").read_text())
    assert receipt["exit"] == 7
    assert receipt["cleanup"]["status"] == "confirmed"


def test_actual_deadline_cancels_and_joins_child_group(tmp_path, monkeypatch):
    identity = tmp_path / "child.json"
    # Trigger the deadline only after actual child/grandchild startup; machine contention
    # cannot turn this cancellation control into a Python startup benchmark.
    monkeypatch.setattr(
        pilot,
        "time",
        SimpleNamespace(
            monotonic=lambda: 2.0 if identity.exists() else 0.0,
            sleep=time.sleep,
        ),
    )
    commands = pilot.Commands(Path(sys.executable), tmp_path, tmp_path / "runtime", seconds=1)
    script = (
        "import json,subprocess,sys,time; "
        f"sys.path.insert(0,{str(pilot.ROOT / 'scripts')!r}); "
        "from harness import ProcessIdentity; "
        "from pathlib import Path; "
        "grandchild=subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)']); "
        f"ready=Path({str(identity)!r}); pending=ready.with_suffix('.tmp'); "
        "pending.write_text(json.dumps(ProcessIdentity.of(grandchild.pid).to_json())); "
        "pending.replace(ready); "
        "time.sleep(60)"
    )
    with pytest.raises(pilot.PilotError, match="journey deadline"):
        commands.run(["-c", script], "deadline")
    receipt = json.loads((tmp_path / "0001-deadline.child.json").read_text())
    assert receipt["cleanup"]["status"] == "confirmed"
    assert receipt["outcome"] == "failed"
    assert identity.is_file()
    assert not ProcessIdentity.from_json(json.loads(identity.read_text())).alive()


@pytest.mark.parametrize("optional_default", ["unknown", "unavailable"])
def test_source_defaults_and_effective_signature_remain_distinct(optional_default):
    node = ast.parse("def tool(self, fn=None, *, name=None): pass").body[0]
    assert isinstance(node, ast.FunctionDef)
    expected = pilot.parameters(node)
    core = {
        "signatures": [
            {
                "parameters": [
                    {"name": "fn", "required": False, "default": {"kind": "expression"}},
                    {"name": "name", "required": False, "default": {"kind": optional_default}},
                ],
                "effective_parameters": [],
            }
        ]
    }
    pilot.check_signature(core, expected, "tool")
    core["signatures"][0]["parameters"][1]["default"] = {"kind": "absent"}
    with pytest.raises(pilot.PilotError, match="requiredness or default presence"):
        pilot.check_signature(core, expected, "tool")


@pytest.mark.parametrize(
    "required,default",
    [
        (False, "absent"),
        (True, "unknown"),
        (True, "expression"),
    ],
)
def test_required_source_parameter_refuses_optional_or_invented_default(required, default):
    node = ast.parse("def resource(self, uri): pass").body[0]
    assert isinstance(node, ast.FunctionDef)
    expected = pilot.parameters(node)
    core = {
        "signatures": [
            {
                "parameters": [
                    {"name": "uri", "required": True, "default": {"kind": "absent"}},
                ]
            }
        ]
    }
    pilot.check_signature(core, expected, "resource")
    core["signatures"][0]["parameters"][0].update(
        required=required,
        default={"kind": default},
    )
    with pytest.raises(pilot.PilotError, match="requiredness or default presence"):
        pilot.check_signature(core, expected, "resource")


class Evidence:
    def __init__(self, pages):
        self.responses = iter(pages)
        self.requests = []

    def tool(self, name, request):
        assert name == "get_evidence"
        self.requests.append(request)
        return {"evidence": next(self.responses)}


def original_range():
    return {
        "source": {"kind": "artifact", "artifact": [1] * 16},
        "artifact": [1] * 16,
        "digest": list(blake3(b"XXabcdefYY").digest()),
        "encoding": "raw_bytes",
        "release": [3] * 16,
        "context": [4] * 16,
        "start": 2,
        "end": 8,
    }


def body_page(original, start, raw, continuation=None):
    body = {
        "start": start,
        "end": start + len(raw),
        "bytes": list(raw),
        "omitted": 3 if continuation else 0,
        "truncated": continuation is not None,
    }
    if continuation is not None:
        body["continuation"] = continuation
    return {"original": original, "body": body}


def test_original_continuations_compare_absolute_offsets_and_independent_bytes():
    original = original_range()
    evidence = Evidence([body_page(original, 0, b"abc", "next"), body_page(original, 3, b"def")])
    result = pilot.check_original(evidence, original, [b"XXabcdefYY"], (3, 7))
    assert result["sha256"] == hashlib.sha256(b"abcdef").hexdigest()
    assert result["native_digest"] == original["digest"]
    assert result["native_digest"] != list(blake3(b"abcdef").digest())
    assert result["continuations"] == 1
    assert evidence.requests[1]["page"]["cursor"] == "next"


@pytest.mark.parametrize("fault", ["gap", "digest", "bytes", "span"])
def test_original_continuation_refuses_corruption_and_foreign_source(fault):
    original = original_range()
    second = dict(original)
    if fault == "digest":
        second["digest"] = [9] * 32
    evidence = Evidence(
        [
            body_page(original, 0, b"abc", "next"),
            body_page(second, 4 if fault == "gap" else 3, b"xyz" if fault == "bytes" else b"def"),
        ]
    )
    with pytest.raises(pilot.PilotError):
        pilot.check_original(
            evidence,
            original,
            [b"XXabcdefYY"],
            (1, 7) if fault == "span" else None,
        )


@pytest.mark.parametrize("digest", [list(bytes(32)), list(blake3(b"abcdef").digest())])
def test_consistent_but_false_whole_artifact_digest_is_refused(digest):
    original = {**original_range(), "digest": digest}
    evidence = Evidence([body_page(original, 0, b"abc", "next"), body_page(original, 3, b"def")])
    with pytest.raises(pilot.PilotError, match="independent BLAKE3"):
        pilot.check_original(evidence, original, [b"XXabcdefYY"])


def test_interpreted_navigation_hit_does_not_hide_later_raw_source():
    original = original_range()
    prose = {**original, "encoding": "native_literal_utf8_slice"}

    class Navigation(Evidence):
        def pages(self, name, request, section):
            assert name == "search_evidence" and section == "results"
            yield {
                "channels": {"vector": {"status": "disabled"}},
                "results": {
                    "items": [
                        {"associated_members": ["tool"], "originals": [prose, original]},
                    ]
                },
            }

    evidence = Navigation(
        [
            body_page(original, 0, b"abc", "next"),
            body_page(original, 3, b"def"),
        ]
    )
    result = pilot.check_example_original(
        evidence,
        {"name": "example", "start": 3, "end": 7},
        b"XXabcdefYY",
        "tool",
    )
    assert result["native_digest"] == original["digest"]
    assert len(evidence.requests) == 2


@pytest.mark.parametrize("expected_on_later_page", [True, False])
def test_operation_search_requires_known_member_and_follows_continuation(expected_on_later_page):
    expected = [7] * 16

    class Search(pilot.Commands):
        def __init__(self):
            self.requests = []
            self.responses = iter(
                [
                    {
                        "channels": {"vector": {"status": "disabled"}},
                        "results": {
                            "items": [{"member": [1] * 16}],
                            "truncated": True,
                            "continuation": "later-page",
                        },
                    },
                    {
                        "channels": {"vector": {"status": "disabled"}},
                        "results": {
                            "items": [{"member": expected if expected_on_later_page else [2] * 16}],
                            "truncated": False,
                        },
                    },
                ]
            )

        def tool(self, name, request):
            assert name == "search_operations"
            self.requests.append(request)
            return next(self.responses)

    search = Search()
    if expected_on_later_page:
        result = pilot.check_operation_search(search, expected)
        assert result == {"member": expected, "pages": 2}
    else:
        with pytest.raises(pilot.PilotError, match=r"independently selected FastMCP\.tool"):
            pilot.check_operation_search(search, expected)
    assert len(search.requests) == 2
    assert "cursor" not in search.requests[0]["page"]
    assert search.requests[1]["page"]["cursor"] == "later-page"
    assert all(request["query"] == "tool" for request in search.requests)


def test_partial_publication_failure_keeps_export_and_does_not_retain_viewer(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_COMPILER_RUNTIME_CONFIG", str(tmp_path / "runtime.json"))
    viewer = tmp_path / "viewer.json"
    monkeypatch.setenv("LCTX_RETAIN_NATIVE_FIXTURE_CONFIG", str(viewer))
    monkeypatch.setattr(pilot, "current_run", lambda: tmp_path)
    monkeypatch.setattr(pilot, "admission", lambda *_: contextlib.nullcontext())
    monkeypatch.setattr(pilot, "set_progress", lambda **_: None)

    class FakeStorage:
        def publish(self, *args, **kwargs):
            return "owned-output"

    class PartialCommands:
        def __init__(self, *args):
            pass

        def run(self, arguments, label):
            if label == "compile":
                artifact = Path(arguments[arguments.index("--output") + 1])
                artifact.mkdir()
                (artifact / "original").write_bytes(b"export")
                return {
                    "published": False,
                    "profile": "catalog",
                    "frontier": "catalog",
                    "receipt": {"export": "retained"},
                }
            raise pilot.PilotError("publish refused")

    monkeypatch.setattr(pilot, "Storage", FakeStorage)
    monkeypatch.setattr(pilot, "Commands", PartialCommands)
    output = tmp_path / "output"
    with pytest.raises(pilot.PilotError, match="publish refused"):
        pilot.execute("produce", Path(sys.executable), output)
    assert (output / "artifact/original").read_bytes() == b"export"
    assert json.loads((output / "receipt.json").read_text()) == {"export": "retained"}
    assert json.loads((output / "failure.json").read_text())["outcome"] == "failed"
    assert not (output / "handle.json").exists()
    assert not viewer.exists()
    assert not viewer.with_suffix(".selected.json").exists()


def test_viewer_is_private_and_does_not_modify_runtime_selection(tmp_path, monkeypatch):
    runtime = tmp_path / "runtime.json"
    selected = tmp_path / "operator.selected.json"
    selected.write_text("unchanged")
    runtime.write_text(
        json.dumps(
            {
                "endpoint": "grpc://127.0.0.1:99",
                "username": "writer",
                "password": "private",
                "selection": str(selected),
            }
        )
    )
    viewer = tmp_path / "viewer.json"
    monkeypatch.setenv("LCTX_RETAIN_NATIVE_FIXTURE_CONFIG", str(viewer))
    monkeypatch.setattr(pilot, "admission", lambda *_: contextlib.nullcontext())
    handle = {"publication": [4] * 32}
    pilot.retain_viewer(runtime, handle)
    assert selected.read_text() == "unchanged"
    assert os.stat(viewer).st_mode & 0o777 == 0o600
    assert os.stat(viewer.with_suffix(".selected.json")).st_mode & 0o777 == 0o600
    assert json.loads(viewer.with_suffix(".selected.json").read_text()) == handle
    assert json.loads(viewer.read_text())["selection"] == str(viewer.with_suffix(".selected.json"))
