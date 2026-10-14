"""Actual launch capture, shared registration and attributed runtime observations."""

from __future__ import annotations

import concurrent.futures
import json
import os
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

import pytest
import test_verify_evidence_nextest as native_controls

import verify
from harness import ProcessIdentity, write_json_atomic
from verify_evidence import cli, projection, rerun
from verify_evidence.capture import CaptureOwner, capture_launch
from verify_evidence.contracts import CaptureUnsupported

ROOT = Path(__file__).resolve().parents[2]
native_fixture = native_controls.native_project


def managed_capture(tmp_path, argv, cwd):
    env = dict(os.environ, PYTHONPATH=str(ROOT / "scripts"), LCTX_RUNS_ROOT=str(tmp_path / "runs"))
    result = subprocess.run(
        [
            getattr(sys, "_base_executable", sys.executable),
            str(ROOT / "scripts/runs.py"),
            "run",
            "--capture-native",
            "--",
            *argv,
        ],
        cwd=cwd,
        env=env,
        capture_output=True,
        timeout=60,
    )
    directories = list((tmp_path / "runs").iterdir())
    assert len(directories) == 1, result.stderr.decode()
    return result, directories[0]


def run_directory(tmp_path):
    directory = tmp_path / "run"
    directory.mkdir()
    write_json_atomic(
        directory / "record.json",
        {
            "schema": 2,
            "id": directory.name,
            "owner": ProcessIdentity.of().to_json(),
            "phase": "ended",
            "termination": "completed",
            "exit": {"code": 0},
            "cleanup": {"status": "confirmed", "survivors": []},
        },
    )
    return directory


def test_concurrent_repeated_create_or_attach_never_overwrites_registrations(tmp_path):
    directory = run_directory(tmp_path)

    def capture(index):
        owner = CaptureOwner(directory, tmp_path)
        step = SimpleNamespace(
            name=f"native-{index}", tool="other", consumes=(), produces=(), restart_after=False
        )
        invocation = owner.start(
            None,
            step,
            ["opaque", str(index)],
            {},
            None,
            None,
            directory / "output.log",
            cwd=tmp_path,
        )
        invocation.save(command={"exit": index})
        return invocation.row["id"]

    with concurrent.futures.ThreadPoolExecutor() as pool:
        identities = list(pool.map(capture, range(8)))
    owner = CaptureOwner(directory, tmp_path)
    manifest = owner.manifest
    assert {row["id"] for row in manifest["invocations"]} == set(identities)
    assert len(identities) == len(set(identities))
    for ident in identities:
        path = directory / "verify-evidence" / ident / "invocation.json"
        row = json.loads(path.read_text())
        assert row["capture"]["status"] == "unsupported"
        assert row["argv"] == row["effective_argv"]
        assert row["cwd"] == str(tmp_path)
    original = manifest["invocations"][0]
    owner.register(original)
    with pytest.raises(ValueError, match="changed"):
        owner.register({**original, "step": "substituted"})
    assert len(owner.manifest["invocations"]) == 8


def test_native_hook_claim_once_but_child_can_claim_nested_same_argv(tmp_path, monkeypatch):
    directory = run_directory(tmp_path)
    monkeypatch.setenv("LCTX_RUN_DIR", str(directory))
    env = dict(os.environ, LCTX_CAPTURE_NATIVE="1")
    argv = [sys.executable, "-c", "print('original command')"]
    with capture_launch(argv, env, tmp_path, directory / "output.log") as first:
        assert first.invocation is not None
        assert first.argv == argv  # unsupported capture never edits the command
        with capture_launch(first.argv, first.env, first.cwd, first.log) as duplicate:
            assert duplicate.invocation is None
        monkeypatch.setattr("verify_evidence.capture._marker", lambda argv, cwd: "other-process")
        with capture_launch(first.argv, first.env, first.cwd, first.log) as nested:
            assert nested.invocation is not None
            assert nested.invocation.row["id"] != first.invocation.row["id"]
            nested.finish(7)
        first.finish(0)
    rows = cli.report(directory)["invocations"]
    assert {row["command"]["exit"] for row in rows} == {0, 7}
    assert all(row["prerequisites"].startswith("unsupported") for row in rows)


def test_actual_pytest_launch_uses_cwd_and_ad_hoc_report_not_catalog_prerequisites(
    tmp_path, monkeypatch
):
    directory = run_directory(tmp_path)
    project = tmp_path / "project"
    project.mkdir()
    (project / "pytest.ini").write_text("[pytest]\naddopts = -k selected\n")
    (project / "test_actual.py").write_text(
        "def test_selected(): print('actual captured case')\n"
        "def test_other(): raise AssertionError('not selected')\n"
    )
    env = dict(os.environ, LCTX_RUN_DIR=str(directory), LCTX_CAPTURE_NATIVE="1")
    monkeypatch.setenv("LCTX_RUN_DIR", str(directory))
    argv = [str(ROOT / ".venv/bin/python"), "-m", "pytest", "-q", "test_actual.py"]
    with capture_launch(
        argv, env, project, directory / "output.log", {"attachment": "actual-context"}
    ) as captured:
        assert captured.invocation is not None
        request = captured.invocation.request(captured.log, False)
        assert request.cwd == project
        result = verify.launch(
            request.argv,
            request.env,
            request.log,
            False,
            cwd=request.cwd,
            stdout=request.stdout,
            observer=request.observer,
        )
        captured.finish(result.exit, result.launch_error, {"status": "confirmed"})
    report = cli.report(directory)
    row = report["invocations"][0]
    assert row["command"]["exit"] == 0
    assert row["cwd"] == str(project)
    assert row["fixture"] == {"attachment": "actual-context"}
    assert row["capture"]["status"] == "effective"
    assert Path(captured.invocation.row["log"]).is_file()
    cases = cli.cases(directory, "passed", 20, None)["cases"]
    assert any("test_selected" in case["case_key"] for case in cases)
    assert not any("test_other" in case["case_key"] for case in cases)
    with pytest.raises(CaptureUnsupported, match="no declared prerequisites"):
        rerun.plan(captured.invocation.row, cases[0], {}, [])


def test_attributed_runtime_events_keep_process_identity_and_incomplete_phase(tmp_path):
    directory = run_directory(tmp_path)
    owner = CaptureOwner(directory, tmp_path)
    step = SimpleNamespace(
        name="native", tool="other", consumes=(), produces=(), restart_after=False
    )
    invocation = owner.start(None, step, ["unqualified"], {}, None, None, directory / "output.log")
    ident = invocation.row["id"]
    lines = [
        f'INFO lctx_phase: phase="wait" phase_id=1 status="begin" invocation_id="{ident}" '
        "process_id=17 process_start_ticks=100",
        f'INFO lctx_phase: phase="wait" phase_id=1 status="begin" invocation_id="{ident}" '
        "process_id=18 process_start_ticks=200",
        'INFO lctx_phase: phase="wait" phase_id=1 status="returned" elapsed_ms=2.5 '
        f'invocation_id="{ident}" process_id=18 process_start_ticks=200',
        json.dumps(
            {
                "target": "lctx_phase",
                "fields": {
                    "observation": "resource_refusal",
                    "invocation_id": ident,
                    "process_id": 17,
                    "owner": "typed backing",
                    "requested": 32,
                    "used": 100,
                    "limit": 128,
                    "cause": "native consumer diagnostic",
                },
            }
        ),
        'INFO lctx_phase: phase="foreign" phase_id=1 status="passed" '
        'invocation_id="foreign" process_id=17',
    ]
    (invocation.directory / "native.log").write_text("\n".join(lines) + "\npartial")
    checkpoint = projection.prepare(invocation.directory)
    assert checkpoint["count"] == 4
    events = cli.events(directory, ident, 20, None)["events"]
    phases = [event for event in events if event["event"] == "native-phase"]
    assert len(phases) == 3
    assert len({json.dumps(event["phase_identity"], sort_keys=True) for event in phases}) == 2
    assert not any(
        event["status"] != "begin" and event["phase_identity"]["process_id"] == 17
        for event in phases
    )
    resources = [event for event in events if event["event"] == "native-resource-refusal"]
    assert resources[0]["observation"]["used"] == 100
    assert all(event.get("outcome") in (None, "observed") for event in events)


def test_session_summaries_are_local_with_honest_parent_aggregate(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_RUN_ID", "combined")
    root = tmp_path / "run" / "verify-evidence-receipt" / "sessions"
    first = verify.Summary(root / "first" / "summary.json", [], verify.Options())
    second = verify.Summary(root / "second" / "summary.json", [], verify.Options())
    first.finish([], "completed")
    parent = json.loads((root.parent.parent / "summary.json").read_text())
    assert len(parent["sessions"]) == 2
    assert sum(row["termination"] == "completed" for row in parent["sessions"]) == 1
    assert second.path is not None
    assert json.loads(second.path.read_text())["termination"] is None


def test_managed_actual_nextest_capture_and_readonly_ad_hoc_operations(tmp_path, native_fixture):
    project, repo, user = native_fixture
    argv = [
        "cargo",
        "nextest",
        "run",
        "--config-file",
        str(repo),
        "--user-config-file",
        str(user),
        "-E",
        "test(passes)",
    ]
    result, directory = managed_capture(tmp_path, argv, project)
    assert result.returncode == 0, (result.stdout + result.stderr).decode()
    report = cli.report(directory)
    invocation = report["invocations"][0]
    assert invocation["capture"]["status"] == "effective"
    assert invocation["argv"] == argv and invocation["cwd"] == str(project)
    assert invocation["command"]["exit"] == 0
    assert report["host_cleanup"]["status"] == "confirmed"
    cases = cli.cases(directory, "passed", 20, None)["cases"]
    assert any(case.get("test") == "passes" for case in cases)
    assert cli.events(directory, invocation["id"], 20, None)["events"]
    selected = cases[0]
    source, row, case = cli.selected(directory, selected["case"])
    assert projection.evidence(source, case["case_key"], phase=None, limit_bytes=10000)["evidence"]
    with pytest.raises(CaptureUnsupported, match="no declared prerequisites"):
        rerun.plan(row, case, {}, [])
    bundle = tmp_path / "portable-bundle"
    exported, _ = managed_capture(
        tmp_path / "export-owner",
        [
            getattr(sys, "_base_executable", sys.executable),
            str(ROOT / "scripts/verify.py"),
            "export",
            str(directory),
            "--output",
            str(bundle),
        ],
        ROOT,
    )
    assert exported.returncode == 0, (exported.stdout + exported.stderr).decode()
    assert cli.report(bundle)["host_cleanup"]["status"] == "confirmed"
    assert cli.cases(bundle, "passed", 20, None)["cases"]


def test_managed_unsupported_capture_preserves_actual_native_failure(tmp_path):
    project = tmp_path / "project"
    project.mkdir()
    (project / "pytest.ini").write_text("[pytest]\n")
    (project / "test_one.py").write_text("def test_one(): assert False, 'original failure'\n")
    argv = [
        str(ROOT / ".venv/bin/python"),
        "-m",
        "pytest",
        "-q",
        "--report-log",
        "original.jsonl",
        "test_one.py",
    ]
    result, directory = managed_capture(tmp_path, argv, project)
    assert result.returncode == 1, (result.stdout + result.stderr).decode()
    row = cli.report(directory)["invocations"][0]
    assert row["argv"] == row["effective_argv"] == argv
    assert row["capture"]["status"] == "unsupported"
    assert row["command"]["exit"] == 1
    assert (project / "original.jsonl").is_file()


def test_opaque_parent_request_reaches_nested_actual_launch(tmp_path):
    project = tmp_path / "project"
    project.mkdir()
    (project / "pytest.ini").write_text("[pytest]\n")
    (project / "test_one.py").write_text("def test_one(): pass\n")
    native = [str(ROOT / ".venv/bin/python"), "-m", "pytest", "-q", "test_one.py"]
    wrapper = (
        "import subprocess,sys; raise SystemExit(subprocess.call("
        "[sys.executable,sys.argv[1],'run','--',*sys.argv[2:]]))"
    )
    argv = [
        getattr(sys, "_base_executable", sys.executable),
        "-c",
        wrapper,
        str(ROOT / "scripts/runs.py"),
        *native,
    ]
    result, directory = managed_capture(tmp_path, argv, project)
    assert result.returncode == 0, (result.stdout + result.stderr).decode()
    rows = cli.report(directory)["invocations"]
    assert len(rows) == 2
    assert [row["capture"]["status"] for row in rows] == ["unsupported", "effective"]
    assert rows[1]["argv"] == native and rows[1]["cwd"] == str(project)
    assert rows[1]["command"]["exit"] == 0


def test_concurrent_actual_nested_native_writers_preserve_all_receipts(tmp_path):
    project = tmp_path / "project"
    project.mkdir()
    (project / "pytest.ini").write_text("[pytest]\n")
    (project / "test_one.py").write_text("def test_one(): pass\n")
    native = [str(ROOT / ".venv/bin/python"), "-m", "pytest", "-q", "test_one.py"]
    wrapper = (
        "import subprocess,sys; jobs=[subprocess.Popen("
        "[sys.executable,sys.argv[1],'run','--',*sys.argv[2:]]) for _ in range(2)]; "
        "raise SystemExit(max(job.wait() for job in jobs))"
    )
    result, directory = managed_capture(
        tmp_path,
        [
            getattr(sys, "_base_executable", sys.executable),
            "-c",
            wrapper,
            str(ROOT / "scripts/runs.py"),
            *native,
        ],
        project,
    )
    assert result.returncode == 0, (result.stdout + result.stderr).decode()
    rows = cli.report(directory)["invocations"]
    assert len(rows) == 3 and len({row["id"] for row in rows}) == 3
    native_rows = [row for row in rows if row["capture"]["status"] == "effective"]
    assert len(native_rows) == 2
    assert all(row["command"]["exit"] == 0 and row["argv"] == native for row in native_rows)
    assert (
        len(
            [
                case
                for case in cli.cases(directory, "passed", 20, None)["cases"]
                if case["phase"] == "call" and case.get("event_kind") == "TestReport"
            ]
        )
        == 2
    )
    assert cli.report(directory)["host_cleanup"]["status"] == "confirmed"


def test_native_pytest_discovery_follows_target_directory_before_capture(tmp_path):
    project = tmp_path / "project"
    project.mkdir()
    (project / "pytest.ini").write_text("[pytest]\naddopts = --report-log=original-target.jsonl\n")
    target = project / "test_one.py"
    target.write_text("def test_one(): pass\n")
    argv = [str(ROOT / ".venv/bin/python"), "-m", "pytest", "-q", str(target)]
    result, directory = managed_capture(tmp_path, argv, tmp_path)
    assert result.returncode == 0, (result.stdout + result.stderr).decode()
    row = cli.report(directory)["invocations"][0]
    assert row["capture"]["status"] == "unsupported"
    assert row["argv"] == row["effective_argv"] == argv
    assert (tmp_path / "original-target.jsonl").is_file()


def test_concurrent_verification_sessions_share_manifest_without_progress_clobber(tmp_path):
    project = tmp_path / "project"
    project.mkdir()
    (project / "pytest.ini").write_text("[pytest]\n")
    (project / "test_one.py").write_text("def test_one(): pass\n")
    script = tmp_path / "verifier.py"
    script.write_text("""import contextlib,json,os,sys,uuid
from pathlib import Path
import runs,verify
from types import SimpleNamespace
from harness import write_json_atomic
from verify_evidence.capture import optional_owner
root=Path.cwd()
verify.ROOT=root
directory=runs.current_run()
assert directory is not None
session=uuid.uuid4().hex
local=directory/'verify-evidence-receipt'/'sessions'/session
step=verify.Step('pytest','pytest')
boundary=verify.Boundary('evidence','actual','tiny native pytest',(step,),frozenset())
argv=(sys.argv[1],'-m','pytest','-q','test_one.py')
planned=verify.Planned(boundary,{'boundary':boundary.id,'cargo_profile':'release'},(verify.PlannedStep(step,argv),))
summary=verify.Summary(local/'summary.json',[planned],verify.Options())
summary.data['session_id']=session
with optional_owner(directory,root) as owner:
    assert owner is not None
    owner.session_id=session
    def progress(**fields):
        write_json_atomic(local/'progress.json',{'session_id':session,**fields})
    runtime=verify.Runtime(observe=lambda requirements:{},run=lambda *args:None,
        fixture=lambda options:contextlib.nullcontext(),
        owner=lambda requirements,report:contextlib.nullcontext(SimpleNamespace(environment=dict)),
        base_env=dict(os.environ),target_dir=root/'target',evidence=owner,
        launch_request=lambda request:verify.launch(
            request.argv,request.env,request.log,request.live,
            stdout=request.stdout,observer=request.observer,cwd=request.cwd),
        report=lambda message:None,progress=progress)
    results=verify.execute([planned],runtime,verify.Options(),directory/'verify'/session,on_result=summary.update)
    summary.finish(results,'completed')
    raise SystemExit(0 if results[0]['outcome']=='passed' else 1)
""")
    wrapper = (
        "import subprocess,sys; jobs=[subprocess.Popen("
        "[sys.executable,*sys.argv[1:]]) for _ in range(2)]; "
        "raise SystemExit(max(job.wait() for job in jobs))"
    )
    result, directory = managed_capture(
        tmp_path,
        [
            getattr(sys, "_base_executable", sys.executable),
            "-c",
            wrapper,
            str(script),
            str(ROOT / ".venv/bin/python"),
        ],
        project,
    )
    assert result.returncode == 0, (result.stdout + result.stderr).decode()
    parent = json.loads((directory / "summary.json").read_text())
    assert len(parent["sessions"]) == 2
    assert all(session["termination"] == "completed" for session in parent["sessions"])
    rows = cli.report(directory)["invocations"]
    declared = [row for row in rows if row["boundary"] == "evidence:actual"]
    assert len(declared) == 2 and len({row["session"] for row in declared}) == 2
    assert all(row["command"]["exit"] == 0 for row in declared)
    progress = list((directory / "verify-evidence-receipt/sessions").glob("*/progress.json"))
    assert len(progress) == 2
    assert len({json.loads(path.read_text())["session_id"] for path in progress}) == 2


def test_uv_native_pytest_probe_uses_actual_checkout_configuration():
    from verify_evidence.pytest_adapter import _tool_versions

    metadata = _tool_versions(
        ROOT,
        dict(os.environ),
        ["uv", "run", "--no-sync", "pytest", "-q", "tests/scripts/test_verify_evidence_capture.py"],
    )
    assert metadata["pytest"] == "9.1.1" and metadata["pytest-reportlog"] == "1.0.0"
    assert metadata["configuration"]["path"] == str(ROOT / "pyproject.toml")


def test_json_native_observation_retains_declared_span_owner_context():
    from verify_evidence.runtime_adapter import observation

    ident = "c" * 32
    spans = [
        {"name": "compiler_producer", "attempt": "actual-attempt", "producer": "actual-producer"}
    ]
    current = {"name": "native_call", "attempt": "actual-attempt"}
    event = {
        "target": "lctx_phase",
        "fields": {
            "phase": "native_call_acknowledgement",
            "phase_id": 9,
            "status": "begin",
            "invocation_id": ident,
            "process_id": 22,
        },
        "spans": spans,
        "span": current,
    }
    record = observation(json.dumps(event), ident)
    assert record is not None
    assert record["observation"]["span_context"] == [*spans, current]
    assert "producer" not in record["observation"]
    assert record["phase_identity"] == {
        "invocation_id": ident,
        "process_id": 22,
        "process_start_ticks": None,
        "phase_id": 9,
    }
