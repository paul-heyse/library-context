"""Selective preparation controls over hand-authored native record expectations."""

from __future__ import annotations

import hashlib
import json
import shutil
from pathlib import Path

import pytest

from verify_evidence import projection


@pytest.fixture
def invocation(tmp_path, monkeypatch):
    directory = tmp_path / "invocation"
    directory.mkdir()
    (directory / "invocation.json").write_text(
        json.dumps(
            {
                "id": "inv-1",
                "tool": "pytest",
                "artifacts": {"reports": "pytest-reportlog.jsonl"},
                "capture": {"state": "available"},
            }
        )
    )
    (directory / "pytest-reportlog.jsonl").touch()
    calls = []

    def records(root, tool, watermarks, ends):
        calls.append(dict(watermarks))
        source = root / "pytest-reportlog.jsonl"
        with source.open("rb") as stream:
            stream.seek(watermarks.get(source.name, 0))
            while True:
                start = stream.tell()
                if start >= ends[source.name]:
                    break
                line = stream.readline()
                if not line.endswith(b"\n"):
                    break
                native = json.loads(line)
                yield {
                    "case_key": native["case"],
                    "nodeid": native["case"],
                    "phase": native.get("phase", "call"),
                    "attempt": native.get("attempt", 1),
                    "outcome": native.get("outcome", "passed"),
                    "payload": native["output"],
                    "source": str(source),
                    "source_start": start,
                    "source_end": stream.tell(),
                    "native": {"locator": {"nodeid": native["case"]}, "coverage": "native"},
                }

    monkeypatch.setattr(projection, "_records", records)
    return directory, calls


def append(directory, case, output="output", **fields):
    with (directory / "pytest-reportlog.jsonl").open("ab") as stream:
        stream.write(json.dumps({"case": case, "output": output, **fields}).encode() + b"\n")


def rewrite_metadata(directory, transform):
    checkpoint = projection._load_checkpoint(directory)
    assert checkpoint is not None
    segment = checkpoint["segments"][0]
    path = directory / projection.INDEX / checkpoint["generation"] / segment["path"]
    rows = [json.loads(line) for line in path.read_bytes().splitlines()]
    transform(rows)
    data = b"".join(projection._json(row) + b"\n" for row in rows)
    path.write_bytes(data)
    segment["sha256"] = hashlib.sha256(data).hexdigest()
    projection._publish(directory, checkpoint)


def test_many_large_unrelated_outputs_are_not_read_for_selected_evidence(invocation, monkeypatch):
    directory, calls = invocation
    for index in range(35):
        append(directory, f"test::{index}", str(index) + "x" * 100_000)
    checkpoint = projection.prepare(directory)
    metadata = list(projection.iter_cases(directory))
    assert all("payload" not in row for row in metadata)
    selected = metadata[17]
    original_open = Path.open
    reads = []

    class AccessSpy:
        def __init__(self, path, stream):
            self.path, self.stream = path, stream

        def __enter__(self):
            return self

        def __exit__(self, *args):
            return self.stream.__exit__(*args)

        def read(self, size=-1):
            start = self.stream.tell()
            value = self.stream.read(size)
            reads.append((self.path.name, start, self.stream.tell()))
            return value

        def __getattr__(self, name):
            return getattr(self.stream, name)

    def spy(path, *args, **kwargs):
        stream = original_open(path, *args, **kwargs)
        return (
            AccessSpy(path, stream)
            if path.name in {"payload.bin", "pytest-reportlog.jsonl"}
            else stream
        )

    monkeypatch.setattr(Path, "open", spy)
    result = projection.evidence(directory, "inv-1:test::17", limit_bytes=29)
    assert result["evidence"][0]["text"] == "17" + "x" * 27
    assert len(calls) == 1
    assert checkpoint["count"] == 35
    assert reads
    for name, start, end in reads:
        if name == "payload.bin":
            assert (
                selected["offset"]
                <= start
                <= end
                <= selected["payload_offset"] + selected["length"]
            )
        else:
            assert selected["source_start"] <= start <= end <= selected["source_end"]


def test_append_commits_only_complete_lines_and_preserves_pinned_prefix(invocation):
    directory, calls = invocation
    append(directory, "first", "first text")
    append(directory, "second", "second text")
    first = projection.cases(directory, limit=1)
    first_reference = first["cases"][0]["reference"]
    source = directory / "pytest-reportlog.jsonl"
    with source.open("ab") as stream:
        stream.write(b'{"case":"third","output":"third text"}')
    partial = projection.prepare(directory)
    assert partial["count"] == 2
    assert partial["sources"][source.name]["end"] < source.stat().st_size
    with source.open("ab") as stream:
        stream.write(b"\n")
    complete = projection.prepare(directory)
    assert complete["count"] == 3
    assert complete["generation"] == first["generation"]
    assert len(complete["segments"]) == 2
    assert calls[1][source.name] > 0
    assert calls[2][source.name] == calls[1][source.name]
    continuation = projection.cases(directory, limit=1, cursor=first["cursor"])
    assert continuation["total"] == 2
    assert continuation["cases"][0]["case_key"] == "second"
    assert projection.evidence(directory, first_reference)["evidence"][0]["text"] == "first text"


def test_rebuild_changes_offsets_generation_but_not_semantic_reference(invocation):
    directory, _ = invocation
    append(directory, "case", "retained output")
    before = projection.cases(directory)
    reference = before["cases"][0]["reference"]
    shutil.rmtree(directory / projection.INDEX)
    after = projection.cases(directory)
    assert before["generation"] != after["generation"]
    assert after["cases"][0]["reference"] == reference
    assert projection.evidence(directory, reference)["evidence"][0]["text"] == "retained output"


def test_partial_payload_and_uncommitted_metadata_tail_are_ignored(invocation):
    directory, _ = invocation
    append(directory, "first")
    checkpoint = projection.prepare(directory)
    generation = directory / projection.INDEX / checkpoint["generation"]
    with (generation / "payload.bin").open("ab") as stream:
        stream.write(b"abandoned payload" * 100)
    (generation / "abandoned.jsonl").write_text('{"case_key":"not committed"}\n')
    assert projection.cases(directory)["total"] == 1
    append(directory, "second")
    new = projection.prepare(directory)
    assert projection.cases(directory)["total"] == 2
    assert (generation / "payload.bin").stat().st_size == new["payload_end"]
    assert b"abandoned payload" not in (generation / "payload.bin").read_bytes()


def test_selected_payload_corruption_is_rejected(invocation):
    directory, _ = invocation
    append(directory, "first", "abcdefgh")
    checkpoint = projection.prepare(directory)
    row = next(projection.iter_cases(directory))
    payload = directory / projection.INDEX / checkpoint["generation"] / "payload.bin"
    with payload.open("r+b") as stream:
        stream.seek(row["payload_offset"] + 2)
        stream.write(b"!")
    with pytest.raises(projection.ProjectionError, match="payload digest"):
        projection.evidence(directory, "first")


def test_metadata_redirect_to_valid_other_payload_is_independently_rejected(invocation):
    directory, _ = invocation
    append(directory, "first", "first output")
    append(directory, "second", "other output")
    projection.prepare(directory)

    def redirect(rows):
        rows[0]["offset"] = rows[1]["offset"]
        rows[0]["payload_offset"] = rows[1]["payload_offset"]
        rows[0]["length"] = rows[1]["length"]

    rewrite_metadata(directory, redirect)
    with pytest.raises(projection.ProjectionError, match="identity does not match"):
        projection.evidence(directory, "first")


def test_metadata_source_range_redirect_is_independently_rejected(invocation):
    directory, _ = invocation
    append(directory, "first", "same length")
    append(directory, "other", "same length")
    projection.prepare(directory)

    def redirect(rows):
        for key in ("source_start", "source_end", "source_digest"):
            rows[0][key] = rows[1][key]

    rewrite_metadata(directory, redirect)
    with pytest.raises(projection.ProjectionError, match="identity does not match"):
        projection.evidence(directory, "first")


def test_metadata_corruption_is_detected_before_listing(invocation):
    directory, _ = invocation
    append(directory, "case")
    checkpoint = projection.prepare(directory)
    segment = (
        directory / projection.INDEX / checkpoint["generation"] / checkpoint["segments"][0]["path"]
    )
    segment.write_bytes(segment.read_bytes().replace(b'"case_key":"case"', b'"case_key":"fake"'))
    with pytest.raises(projection.ProjectionError, match="metadata segment"):
        projection.cases(directory)


def test_missing_original_is_explicit_even_when_payload_survives(invocation):
    directory, _ = invocation
    append(directory, "case")
    projection.prepare(directory)
    (directory / "pytest-reportlog.jsonl").unlink()
    with pytest.raises(projection.ProjectionError, match="missing original"):
        projection.evidence(directory, "case")


def test_same_path_replacement_requires_fresh_generation(invocation):
    directory, _ = invocation
    append(directory, "case", "original")
    before = projection.cases(directory)
    old_reference = before["cases"][0]["reference"]
    replacement = directory / "replacement"
    replacement.write_text(json.dumps({"case": "case", "output": "replaced"}) + "\n")
    replacement.replace(directory / "pytest-reportlog.jsonl")
    after = projection.cases(directory)
    assert after["generation"] != before["generation"]
    assert projection.evidence(directory, "case")["evidence"][0]["text"] == "replaced"
    with pytest.raises(projection.ProjectionError, match="not present"):
        projection.evidence(directory, old_reference)


def test_phase_attempt_outcome_and_evidence_continuation_are_distinct(invocation):
    directory, _ = invocation
    append(directory, "case", "setup", phase="setup", outcome="passed")
    append(directory, "case", "failed", phase="call", attempt=1, outcome="failed")
    append(directory, "case", "retry", phase="call", attempt=2, outcome="passed")
    failed = projection.cases(directory, outcome="failed")
    assert failed["total"] == 1
    assert failed["cases"][0]["attempt"] == 1
    first = projection.evidence(directory, "case", phase="call", limit_bytes=3)
    assert first["evidence"][0]["text"] == "fai"
    second = projection.evidence(
        directory, "case", phase="call", limit_bytes=20, cursor=first["cursor"]
    )
    assert [row["text"] for row in second["evidence"]] == ["led", "retry"]
    with pytest.raises(projection.ProjectionError, match="cursor"):
        projection.evidence(directory, "case", phase="setup", cursor=first["cursor"])


def test_relocated_originals_rebuild_the_same_semantic_reference(invocation, tmp_path):
    directory, _ = invocation
    append(directory, "case", "portable")
    reference = projection.cases(directory)["cases"][0]["reference"]
    copied = tmp_path / "imported"
    shutil.copytree(directory, copied, ignore=shutil.ignore_patterns(projection.INDEX))
    assert projection.evidence(copied, reference)["evidence"][0]["text"] == "portable"


def test_concurrent_append_commits_the_captured_prefix_and_leaves_new_suffix(
    invocation, monkeypatch
):
    directory, _ = invocation
    append(directory, "first")
    original_records = projection._records

    def append_during_records(root, tool, watermarks, ends):
        for record in original_records(root, tool, watermarks, ends):
            append(directory, "concurrent")
            yield record

    monkeypatch.setattr(projection, "_records", append_during_records)
    first = projection.prepare(directory)
    assert first["count"] == 1
    assert (
        first["sources"]["pytest-reportlog.jsonl"]["end"]
        < (directory / "pytest-reportlog.jsonl").stat().st_size
    )
    monkeypatch.setattr(projection, "_records", original_records)
    second = projection.prepare(directory)
    assert second["count"] == 2
    assert second["generation"] == first["generation"]


def test_coverage_updates_without_reinterpreting_unchanged_originals(invocation):
    directory, calls = invocation
    append(directory, "case")
    first = projection.prepare(directory)
    invocation_path = directory / "invocation.json"
    data = json.loads(invocation_path.read_bytes())
    data["capture"] = {"state": "failed", "reason": "native export failed"}
    invocation_path.write_text(json.dumps(data))
    second = projection.prepare(directory)
    assert second["coverage"]["capture"]["state"] == "failed"
    assert second["count"] == first["count"]
    assert len(calls) == 1


@pytest.mark.parametrize(
    "field,value", [("offset", -1), ("length", -1), ("payload_offset", True), ("source_start", -2)]
)
def test_invalid_selected_range_values_are_rejected_before_seek(invocation, field, value):
    directory, _ = invocation
    append(directory, "case")
    projection.prepare(directory)
    rewrite_metadata(directory, lambda rows: rows[0].update({field: value}))
    with pytest.raises(projection.ProjectionError, match=r"range|identity"):
        projection.evidence(directory, "case")


def test_real_pytest_adapter_and_projection_keep_native_phases_and_prefix(tmp_path):
    (tmp_path / "invocation.json").write_text(json.dumps({"id": "native-pytest", "tool": "pytest"}))
    source = tmp_path / "pytest-reportlog.jsonl"
    events = [
        {
            "$report_type": "TestReport",
            "nodeid": "tests/test_one.py::test_case",
            "when": "setup",
            "outcome": "passed",
        },
        {
            "$report_type": "TestReport",
            "nodeid": "tests/test_one.py::test_case",
            "when": "call",
            "outcome": "failed",
            "longrepr": "native assertion",
            "sections": [["Captured stdout call", "native stdout"]],
        },
    ]
    source.write_text("".join(json.dumps(event) + "\n" for event in events))
    first = projection.evidence(tmp_path, "tests/test_one.py::test_case", phase="call")
    assert "native assertion" in first["evidence"][0]["text"]
    assert "Captured stdout call\nnative stdout" in first["evidence"][0]["text"]
    assert first["evidence"][0]["coverage"]["authority"] == "native-report"
    with source.open("a") as stream:
        stream.write(
            json.dumps(
                {
                    "$report_type": "TestReport",
                    "nodeid": "tests/test_one.py::test_case",
                    "when": "teardown",
                    "outcome": "failed",
                    "longrepr": "native cleanup",
                }
            )
            + "\n"
        )
    assert projection.cases(tmp_path, outcome="failed")["total"] == 2


def test_real_nextest_junit_projection_binds_identical_names_to_binaries(tmp_path):
    (tmp_path / "invocation.json").write_text(
        json.dumps({"id": "native-nextest", "tool": "nextest"})
    )
    (tmp_path / "junit.xml").write_text("""<testsuites uuid="4b4ec130-d5d7-4db4-9237-4dd6b7d6303c">
<testsuite name="crate::one"><testcase name="same">
<system-out>first output</system-out></testcase></testsuite>
<testsuite name="crate::two"><testcase name="same">
<failure>second failure</failure></testcase></testsuite>
</testsuites>""")
    metadata = projection.cases(tmp_path)["cases"]
    assert len(metadata) == 2
    assert metadata[0]["case_key"] != metadata[1]["case_key"]
    selected = projection.evidence(tmp_path, metadata[1]["reference"])
    assert "second failure" in selected["evidence"][0]["text"]
    assert "first output" not in selected["evidence"][0]["text"]
