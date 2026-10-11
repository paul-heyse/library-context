"""Native pytest/reportlog composition over tiny, isolated Python inputs."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

import pytest

from verify_evidence import pytest_adapter as adapter
from verify_evidence.contracts import CaptureUnsupported

ROOT = Path(__file__).resolve().parents[2]


def command(tmp_path, source, *options, configuration="", environment=None):
    project = tmp_path / "project"
    project.mkdir()
    (project / "pytest.ini").write_text("[pytest]\n" + configuration)
    (project / "test_cases.py").write_text(source)
    output = tmp_path / "capture"
    output.mkdir()
    env = dict(os.environ)
    env.pop("PYTEST_ADDOPTS", None)
    env.pop("PYTEST_PLUGINS", None)
    env.pop("PYTEST_DISABLE_PLUGIN_AUTOLOAD", None)
    env.update(environment or {})
    prepared = adapter.prepare(
        [
            sys.executable,
            "-m",
            "pytest",
            "-c",
            str(project / "pytest.ini"),
            "--confcutdir",
            str(project),
            "-q",
            *options,
            str(project),
        ],
        env,
        output,
        ROOT,
    )
    return project, output, prepared


def execute(prepared, project):
    return subprocess.run(
        prepared["argv"],
        env=prepared["env"],
        cwd=project,
        capture_output=True,
        text=True,
        timeout=30,
    )


def raw_events(output, filename=adapter.REPORT):
    return [json.loads(line) for line in (output / filename).read_text().splitlines()]


def test_native_phase_failures_expected_outcomes_and_passing_capture(tmp_path):
    source = """import logging, os, pytest, warnings
@pytest.fixture
def broken_setup():
    try:
        raise KeyError("cause-key")
    except KeyError as cause:
        raise ValueError("setup-value") from cause
@pytest.fixture
def broken_teardown():
    yield
    raise RuntimeError("teardown-value")
def test_pass():
    print("passing-stdout")
    os.write(2, b"passing-fd-stderr\\n")
    logging.info("passing-info-log")
    warnings.warn("passing-warning")
def test_setup(broken_setup):
    assert False, "unreachable"
def test_call_and_teardown(broken_teardown):
    assert 1 == 2, "call-value"
@pytest.mark.skip(reason="skip-reason")
def test_skip(): pass
@pytest.mark.xfail(reason="xfail-reason")
def test_xfail(): assert False
@pytest.mark.xfail(reason="xpass-reason")
def test_xpass(): pass
@pytest.mark.xfail(strict=True, reason="strict-reason")
def test_strict_xpass(): pass
"""
    project, output, prepared = command(tmp_path, source)
    result = execute(prepared, project)
    assert result.returncode == 1, result.stdout + result.stderr
    rows = list(adapter.records(output))
    phases = {
        (r["nodeid"].split("::")[-1], r["phase"]): r
        for r in rows
        if r["event_kind"] == "TestReport"
    }
    assert phases["test_setup", "setup"]["outcome"] == "failed"
    assert ("test_setup", "call") not in phases
    assert phases["test_call_and_teardown", "call"]["outcome"] == "failed"
    assert phases["test_call_and_teardown", "teardown"]["outcome"] == "failed"
    assert phases["test_skip", "setup"]["outcome"] == "skipped"
    assert phases["test_xfail", "call"]["outcome"] == "xfail"
    assert phases["test_xpass", "call"]["outcome"] == "xpass"
    assert phases["test_strict_xpass", "call"]["outcome"] == "failed"
    passing = phases["test_pass", "call"]["payload"]
    assert all(
        text in passing for text in ("passing-stdout", "passing-fd-stderr", "passing-info-log")
    )
    assert all(
        text in phases["test_setup", "setup"]["payload"] for text in ("cause-key", "setup-value")
    )
    assert any("passing-warning" in row["payload"] for row in rows)
    assert all(row["attempt"] == 1 for row in rows)
    assert adapter.finalize(output)["channels"][adapter.REPORT]["session_finished"]
    # Locators point to the exact native originals, not a lossy derived reporter.
    for row in rows:
        with (output / row["source"]).open("rb") as stream:
            stream.seek(row["source_start"])
            original = json.loads(stream.read(row["source_end"] - row["source_start"]))
        assert original["$report_type"] == row["event_kind"]
        if row["event_kind"] == "TestReport":
            for field in ("duration", "start", "stop"):
                assert row[field] == original[field]
            assert row["duration"] >= 0 and row["stop"] >= row["start"]


def test_collection_error_has_native_identity_without_completed_test(tmp_path):
    project, output, prepared = command(tmp_path, "def test_invalid(: pass\n")
    assert execute(prepared, project).returncode == 2
    rows = list(adapter.records(output))
    errors = [
        row for row in rows if row["event_kind"] == "CollectReport" and row["outcome"] == "failed"
    ]
    assert len(errors) == 1 and errors[0]["phase"] == "collection"
    assert errors[0]["nodeid"] == "test_cases.py"
    assert "SyntaxError" in errors[0]["payload"]
    assert not any(row["event_kind"] in {"CaseStart", "TestReport"} for row in rows)


@pytest.mark.parametrize("configuration", ["", "addopts = -p pytest_reportlog.plugin\n"])
def test_plugin_composition_membership_deselection_and_owned_destination(tmp_path, configuration):
    project, output, prepared = command(
        tmp_path,
        "def test_keep(): pass\ndef test_drop(): pass\n",
        "-p",
        "pytest_reportlog.plugin",
        "-k",
        "keep",
        environment={"PYTEST_PLUGINS": "pytest_reportlog.plugin"},
        configuration=configuration,
    )
    # Ordinary project plugin discovery remains enabled alongside explicit loading.
    (project / "conftest.py").write_text("""def pytest_configure(config):
    assert config.pluginmanager.hasplugin("pytest_reportlog")
    config.addinivalue_line("markers", "composed: discovered project plugin")
""")
    result = execute(prepared, project)
    assert result.returncode == 0, result.stdout + result.stderr
    assert "PYTEST_DISABLE_PLUGIN_AUTOLOAD" not in prepared["env"]
    assert "-p" in prepared["argv"]
    events = raw_events(output)
    assert sum(event["$report_type"] == "SessionStart" for event in events) == 1
    hooks = raw_events(output, adapter.HOOKS)
    assert [e["nodeid"] for e in hooks if e["$report_type"] == "CollectionMember"] == [
        "test_cases.py::test_keep"
    ]
    assert [e["nodeid"] for e in hooks if e["$report_type"] == "Deselected"] == [
        "test_cases.py::test_drop"
    ]
    assert [e["nodeid"] for e in hooks if e["$report_type"] == "CaseStart"] == [
        "test_cases.py::test_keep"
    ]
    assert hooks[0]["traceback"] == "long" and hooks[0]["capture"] == "fd"
    assert hooks[0]["passing_logs"] and hooks[0]["warnings_plugin"]
    collection = next(e for e in hooks if e["$report_type"] == "CollectionFinished")
    assert (collection["selected"], collection["deselected"]) == (1, 1)


@pytest.mark.parametrize(
    "configuration,options,environment",
    [
        ("addopts = --tb=short --capture=sys\nlog_level = ERROR\n", (), {}),
        ("", ("--tb=short", "--capture=sys", "--log-level=ERROR"), {}),
        ("", (), {"PYTEST_ADDOPTS": "--tb=short --capture=sys --log-level=ERROR"}),
    ],
)
def test_explicit_diagnostics_preserved(tmp_path, configuration, options, environment):
    project, output, prepared = command(
        tmp_path,
        "def test_pass(): pass\n",
        *options,
        configuration=configuration,
        environment=environment,
    )
    result = execute(prepared, project)
    assert result.returncode == 0, result.stdout + result.stderr
    config = raw_events(output, adapter.HOOKS)[0]
    assert (config["traceback"], config["capture"], config["log_level"]) == (
        "short",
        "sys",
        "ERROR",
    )
    assert adapter.finalize(output)["effective"]["capture"] == "sys"


def test_live_prefix_interruption_and_partial_tail(tmp_path):
    marker = tmp_path / "ready"
    source = """import os, pathlib, time
def test_a_pass(): print("completed-passing-output")
def test_b_inflight():
    print("unfinished-call-output", flush=True)
    pathlib.Path(os.environ["READY_MARKER"]).write_text("ready")
    time.sleep(60)
"""
    project, output, prepared = command(tmp_path, source, environment={"READY_MARKER": str(marker)})
    with (tmp_path / "console.log").open("wb") as log:
        child = subprocess.Popen(
            prepared["argv"], env=prepared["env"], cwd=project, stdout=log, stderr=subprocess.STDOUT
        )
        try:
            deadline = time.monotonic() + 20
            while not marker.exists() and child.poll() is None and time.monotonic() < deadline:
                time.sleep(0.01)
            assert marker.exists(), (tmp_path / "console.log").read_text()
            live = list(adapter.records(output))
            assert any(
                r["nodeid"].endswith("::test_b_inflight") and r["phase"] == "start" for r in live
            )
            assert any("completed-passing-output" in r["payload"] for r in live)
            assert not any("unfinished-call-output" in r["payload"] for r in live)
            assert not adapter.finalize(output)["channels"][adapter.REPORT]["session_finished"]
            child.terminate()
            assert child.wait(timeout=10) != 0
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=10)
    before = list(adapter.records(output))
    report = output / adapter.REPORT
    with report.open("ab") as stream:
        stream.write(b'{"$report_type":"SessionFinish", "exitstatus":15}')
    assert list(adapter.records(output)) == before
    assert adapter.finalize(output)["channels"][adapter.REPORT]["partial_tail"]
    watermarks = {
        name: max((r["source_end"] for r in before if r["source"] == name), default=0)
        for name in (adapter.REPORT, adapter.HOOKS)
    }
    assert list(adapter.records(output, watermarks=watermarks)) == []
    with report.open("ab") as stream:
        stream.write(b"\n")
    appended = list(adapter.records(output, watermarks=watermarks))
    assert len(appended) == 1 and appended[0]["event_kind"] == "SessionFinish"
    assert list(adapter.records(output, ends=watermarks)) == before


@pytest.mark.parametrize(
    "flags",
    [
        ["--report-log", "/outside.jsonl"],
        ["-p", "no:pytest_reportlog"],
        ["-n", "2"],
        ["--reruns=2"],
    ],
)
def test_preparation_rejects_conflicts_without_mutation(tmp_path, flags):
    argv = [sys.executable, "-m", "pytest", *flags]
    env = {"PYTEST_ADDOPTS": "-q"}
    original = list(argv), dict(env)
    with pytest.raises(CaptureUnsupported):
        adapter.prepare(argv, env, tmp_path / "absent", ROOT)
    assert (argv, env) == original
    assert not (tmp_path / "absent").exists()


def test_config_conflict_is_refused_before_preparation_writes(tmp_path):
    config = tmp_path / "pytest.ini"
    config.write_text("[pytest]\naddopts = --report-log=/outside.jsonl\n")
    output = tmp_path / "absent"
    with pytest.raises(CaptureUnsupported, match="destination"):
        adapter.prepare(["pytest", "-c", str(config)], {}, output, ROOT)
    assert not output.exists()


def test_preparation_preserves_existing_original_capture(tmp_path):
    original = tmp_path / adapter.REPORT
    original.write_bytes(b"original native bytes\n")
    with pytest.raises(CaptureUnsupported, match="already been used"):
        adapter.prepare(["pytest"], {}, tmp_path, ROOT)
    assert original.read_bytes() == b"original native bytes\n"


@pytest.mark.parametrize("config_option", ["split", "compact", "environment"])
def test_effective_configuration_identity_and_hidden_selection_options(
    tmp_path, monkeypatch, config_option
):
    config = tmp_path / "external.ini"
    config.write_text("[pytest]\naddopts = -k selected extra_case.py\n")
    monkeypatch.setattr(
        adapter,
        "_tool_versions",
        lambda root, env: {"pytest": "9.1.1", "pytest-reportlog": "1.0.0"},
    )
    option = ["-c", str(config)] if config_option == "split" else ["-c" + str(config)]
    argv = ["pytest", *option] if config_option != "environment" else ["pytest"]
    env = (
        {"PYTEST_ADDOPTS": "-q"}
        if config_option != "environment"
        else {"PYTEST_ADDOPTS": "-q -c " + str(config)}
    )
    prepared = adapter.prepare(argv, env, tmp_path / "capture", ROOT)
    assert prepared["configuration"] == {
        "path": str(config),
        "sha256": hashlib.sha256(config.read_bytes()).hexdigest(),
    }
    assert prepared["selection_options"]["configured_addopts"] == [
        "-k",
        "selected",
        "extra_case.py",
    ]
    assert prepared["selection_options"]["environment_addopts"] == (
        ["-q"] if config_option != "environment" else ["-q", "-c", str(config)]
    )


def test_late_unsupported_capture_does_not_change_native_passing_result(tmp_path):
    project, output, prepared = command(tmp_path, "def test_pass(): pass\n")
    (project / "conftest.py").write_text("""import pytest
@pytest.hookimpl(tryfirst=True)
def pytest_configure(config):
    config.option.reruns = 2
""")
    result = execute(prepared, project)
    assert result.returncode == 0, result.stdout + result.stderr
    rows = list(adapter.records(output))
    assert any(
        row["event_kind"] == "TestReport" and row["phase"] == "call" and row["outcome"] == "passed"
        for row in rows
    )
    assert any(row["event_kind"] == "CaptureUnsupported" for row in rows)
    assert adapter.finalize(output)["channels"][adapter.HOOKS]["status"] == "unsupported"


def test_stdlib_coordinator_probes_selected_tools_environment(tmp_path):
    # The verify launcher deliberately uses stdlib Python, not the tools venv.
    env = dict(os.environ, PYTHONPATH=str(ROOT / "scripts"))
    code = """import json,sys,os
from pathlib import Path
from verify_evidence import pytest_adapter
result = pytest_adapter.prepare(["pytest"], dict(os.environ), Path(sys.argv[2]), Path(sys.argv[1]))
print(json.dumps(result["tool_version"]))
"""
    result = subprocess.run(
        [
            getattr(sys, "_base_executable", sys.executable),
            "-c",
            code,
            str(ROOT),
            str(tmp_path / "absent"),
        ],
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert json.loads(result.stdout) == {"pytest": "9.1.1", "pytest-reportlog": "1.0.0"}
    assert not (tmp_path / "absent").exists()


def test_preparation_missing_dependency_and_availability_are_separate(tmp_path, monkeypatch):
    def missing(root, env):
        raise CaptureUnsupported("pytest evidence metadata unavailable; repair: just sync tools")

    monkeypatch.setattr(adapter, "_tool_versions", missing)
    with pytest.raises(CaptureUnsupported, match="just sync tools"):
        adapter.prepare(["pytest"], {}, tmp_path, ROOT)
    assert adapter.finalize(tmp_path)["channels"][adapter.REPORT]["status"] == "unavailable"
    (tmp_path / adapter.REPORT).write_bytes(b"not-json\n")
    assert adapter.finalize(tmp_path)["channels"][adapter.REPORT]["status"] == "failed"
    with pytest.raises(ValueError, match="malformed pytest evidence"):
        list(adapter.records(tmp_path))
