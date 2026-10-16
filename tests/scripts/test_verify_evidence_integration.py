"""Bounded native capture/runtime journeys, independent of product/service tests."""

from __future__ import annotations

import contextlib
import datetime as dt
import json
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path
from types import SimpleNamespace
from typing import Self

import pytest

import runs
import storage_lifecycle as lifecycle
import storage_owners
import verify
import workspace_env
from harness import ProcessIdentity, group_members
from storage_lifecycle import Storage
from verify_evidence import context, pytest_adapter
from verify_evidence.capture import CaptureOwner, optional_owner
from verify_evidence.projection import cases, prepare

ROOT = Path(__file__).resolve().parents[2]
STDLIB_PYTHON = getattr(sys, "_base_executable", sys.executable)


def git(root, *args):
    return subprocess.run(
        ["git", *args], cwd=root, check=True, capture_output=True, text=True
    ).stdout


@pytest.fixture
def project(tmp_path, monkeypatch):
    root = tmp_path / "checkout"
    root.mkdir()
    git(root, "init", "-q")
    (root / ".gitignore").write_text(".venv/\nscripts\n.pytest_cache/\n**/__pycache__/\n")
    (root / "pytest.ini").write_text("[pytest]\n")
    (root / "test_case.py").write_text('def test_pass(): print("retained-native-output")\n')
    # Reuse the locked tools environment read-only. Copy the input manifests
    # needed for uv's --locked --check observation; no sync/install occurs.
    for name in (
        "pyproject.toml",
        "uv.lock",
        "python/lctx_mcp/pyproject.toml",
        "python/lctx_semantics/pyproject.toml",
    ):
        destination = root / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / name, destination)
    (root / ".venv").symlink_to(ROOT / ".venv", target_is_directory=True)
    (root / "scripts").symlink_to(ROOT / "scripts", target_is_directory=True)
    git(root, "add", ".")
    git(
        root,
        "-c",
        "user.name=Evidence Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "-qm",
        "synthetic capture inputs",
    )
    monkeypatch.setattr(verify, "ROOT", root)
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "runs"))
    return root


def capture_owner(root):
    directory = runs.new_run_dir()
    owner = runs.Owner(
        directory,
        ["synthetic-evidence-journey"],
        label=None,
        cwd=str(root),
        mode="foreground",
        stream=False,
    )
    owner.write()
    return directory


def planned(root, *, restart=False, fixture=False, args=()):
    step = verify.Step("pytest", "pytest", restart_after=restart)
    boundary = verify.Boundary(
        "evidence",
        "synthetic",
        "tiny native pytest",
        (step,),
        frozenset({"tools"}),
        fixture=fixture,
    )
    argv = (
        str(ROOT / ".venv/bin/python"),
        "-m",
        "pytest",
        "-c",
        str(root / "pytest.ini"),
        "--confcutdir",
        str(root),
        "-q",
        *args,
        str(root / "test_case.py"),
    )
    return verify.Planned(boundary, {"cargo_profile": "release"}, (verify.PlannedStep(step, argv),))


def runtime(owner, root, *, fixture=lambda options: contextlib.nullcontext()):
    requests = []

    def launch(request):
        requests.append(request)
        return verify.launch(
            request.argv,
            request.env,
            request.log,
            request.live,
            stdout=request.stdout,
            observer=request.observer,
            cwd=request.cwd,
        )

    return verify.Runtime(
        observe=lambda requirements: {r: workspace_env.observe(r, root=root) for r in requirements},
        run=lambda argv, env, log, live: verify.launch(argv, env, log, live),
        fixture=fixture,
        owner=lambda requirements, report: workspace_env.ownership(
            "shared", requirements, root=root, report=report
        ),
        base_env=dict(os.environ),
        target_dir=root / "target",
        evidence=owner,
        launch_request=launch,
        report=lambda message: None,
    ), requests


def test_actual_capture_runtime_storage_and_selective_projection(project):
    run_dir = capture_owner(project)
    with optional_owner(run_dir, project) as owner:
        assert isinstance(owner, CaptureOwner)
        execution, requests = runtime(owner, project)
        result = verify.run_boundary(
            planned(project), execution, verify.Options(), run_dir / "python.log", False
        )
        assert result["outcome"] == "passed", result
        assert len(requests) == 1 and "--report-log" in requests[0].argv
        invocation = owner.invocations[0]
        row = json.loads((invocation.directory / "invocation.json").read_text())
        assert row["capture"]["status"] == "effective", row["capture"]
        assert row["tool_version"] == {"pytest": "9.1.1", "pytest-reportlog": "1.0.0"}
        assert row["command"]["exit"] == 0 and row["command"]["completed"]
        assert row["spawn"]["pid"] > 0
        assert row["postprocessing"]["status"] == "completed"
        assert row["boundary_final"]["outcome"] == "passed"
        assert row["finalization"]["channels"][pytest_adapter.REPORT]["status"] == "available"
        prepare(invocation.directory)
        rows = cases(invocation.directory)["cases"]
        assert any(
            r["nodeid"].endswith("::test_pass")
            and r["phase"] == "call"
            and r["outcome"] == "passed"
            for r in rows
        )
        raw_record = Storage().get(owner.manifest["storage"]["raw"])
        receipt_record = Storage().get(owner.manifest["storage"]["receipt"])
        assert (
            raw_record["owner"] == receipt_record["owner"] == {"kind": "run", "path": str(run_dir)}
        )
        assert raw_record["category"] == "run-log" and receipt_record["category"] == "run-receipt"
        assert storage_owners.observe(raw_record)["state"] == "active"


def test_unsupported_capture_keeps_original_native_command_and_passing_result(project):
    run_dir = capture_owner(project)
    # User-selected report destination is a capture conflict, while native pytest
    # is still valid and must run exactly as requested.
    outside = project.parent / "operator-report.jsonl"
    original = planned(project, args=("--report-log", str(outside)))
    with optional_owner(run_dir, project) as owner:
        assert owner is not None
        execution, requests = runtime(owner, project)
        result = verify.run_boundary(
            original, execution, verify.Options(), run_dir / "python.log", False
        )
        assert result["outcome"] == "passed", result
        assert tuple(requests[0].argv) == original.steps[0].argv
        invocation = owner.invocations[0]
        assert invocation.row["capture"]["status"] == "unsupported"
        assert invocation.row["command"]["exit"] == 0
        assert invocation.row["artifacts"] == {}
        assert outside.exists()
        assert not (invocation.directory / pytest_adapter.REPORT).exists()


def test_stdlib_coordinator_retains_available_capture_end_to_end(project):
    code = """import contextlib,json,os,sys
from pathlib import Path
import runs,verify,workspace_env
from verify_evidence.capture import optional_owner
root = Path(sys.argv[1])
verify.ROOT = root
directory = runs.new_run_dir()
record = runs.Owner(directory, ["stdlib-coordinator-control"], label=None,
                    cwd=str(root), mode="foreground", stream=False)
record.write()
step = verify.Step("pytest", "pytest")
boundary = verify.Boundary("evidence", "stdlib", "synthetic", (step,), frozenset({"tools"}))
argv = (str(root / ".venv/bin/python"), "-m", "pytest", "-c", str(root / "pytest.ini"),
        "--confcutdir", str(root), "-q", str(root / "test_case.py"))
planned = verify.Planned(boundary, {"cargo_profile":"release"}, (verify.PlannedStep(step,argv),))
with optional_owner(directory, root) as owner:
    runtime = verify.Runtime(
        observe=lambda requirements:{r:workspace_env.observe(r,root=root) for r in requirements},
        run=lambda argv,env,log,live:verify.launch(argv,env,log,live),
        fixture=lambda options:contextlib.nullcontext(),
        owner=lambda requirements,report:workspace_env.ownership(
            "shared",requirements,root=root,report=report),
        base_env=dict(os.environ), target_dir=root/"target", evidence=owner,
        launch_request=lambda request:verify.launch(
            request.argv,request.env,request.log,request.live,
            stdout=request.stdout,observer=request.observer,cwd=request.cwd),
        report=lambda message:None)
    result=verify.run_boundary(planned,runtime,verify.Options(),directory/"python.log",False)
    row=owner.invocations[0].row
    print(json.dumps({"outcome":result["outcome"],"capture":row["capture"],
        "finalization":row.get("finalization"),"tool_version":row.get("tool_version")}))
"""
    env = dict(os.environ, PYTHONPATH=str(ROOT / "scripts"))
    result = subprocess.run(
        [STDLIB_PYTHON, "-c", code, str(project)],
        cwd=project,
        env=env,
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    row = json.loads(result.stdout)
    assert row["outcome"] == "passed" and row["capture"]["status"] == "effective"
    assert row["finalization"]["channels"][pytest_adapter.REPORT]["status"] == "available"
    assert row["tool_version"] == {"pytest": "9.1.1", "pytest-reportlog": "1.0.0"}


@pytest.mark.parametrize("failure", ["postprocessing", "fixture-finalization"])
def test_command_exit_is_durable_before_later_finalization_exception(project, failure):
    run_dir = capture_owner(project)
    with optional_owner(run_dir, project) as owner:
        assert owner is not None

        def assert_durable():
            row = json.loads((owner.invocations[0].directory / "invocation.json").read_text())
            assert row["command"]["exit"] == 0 and row["command"]["completed"]
            assert row["finalization"]["channels"][pytest_adapter.REPORT]["session_finished"]

        class Attachment:
            id = "synthetic-attachment"
            server = SimpleNamespace(id="no-database-server", kind="synthetic", exited=lambda: None)

            def environment(self, env):
                return dict(env)

            def restart(self):
                assert_durable()
                raise RuntimeError("postprocessing failure")

        @contextlib.contextmanager
        def fixture(options):
            yield Attachment() if failure == "postprocessing" else None
            assert_durable()
            raise RuntimeError("fixture-finalization failure")

        execution, _ = runtime(owner, project, fixture=fixture)
        selected = planned(project, restart=failure == "postprocessing", fixture=True)
        with pytest.raises(RuntimeError, match=failure):
            verify.run_boundary(
                selected, execution, verify.Options(), run_dir / "python.log", False
            )
        assert_durable()
        row = owner.invocations[0].row
        assert row["boundary_final"]["error"] == "RuntimeError"
        assert row["boundary_final"]["fixture_finalization"]["status"] == "failed"
        assert row["postprocessing"]["status"] == (
            "started" if failure == "postprocessing" else "completed"
        )


def test_source_observations_distinguish_dirty_bytes_and_exclude_protected_inputs(
    project, monkeypatch
):
    protected = [
        ".env",
        "credentials/token",
        "fixtures/held.py",
        "gold/held.py",
        "heldout/held.py",
        "target/object",
        "build/evidence",
        "private/key",
        "server.pem",
    ]
    for name in protected:
        path = project / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("protected synthetic input")
    # Even explicitly tracked protected material must not be opened by hashing.
    git(project, "add", "-f", *protected)
    for name in ("docs/design.md", "third_party/vendor.rs"):
        path = project / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("relevant source input")
    opened = []
    actual = context.digest_file

    def digest(path):
        opened.append(path.relative_to(project).as_posix())
        return actual(path)

    monkeypatch.setattr(context, "digest_file", digest)
    observer = context.SourceObserver(project, scope="full")
    before = observer.observe()
    assert before["available"]
    assert not set(protected).intersection(opened)
    assert {"docs/design.md", "third_party/vendor.rs"} <= set(before["files"])
    assert set(protected) <= set(before["excluded"])
    (project / "test_case.py").write_text("def test_pass(): assert True\n")
    (project / "new-input.py").write_text("untracked = True\n")
    after = observer.observe()
    assert before["head"] == after["head"]
    assert before["id"] != after["id"]
    assert before["files"]["test_case.py"] != after["files"]["test_case.py"]
    assert "new-input.py" in after["files"]
    assert "unknown" in after["source_to_binary"]


def test_successful_restart_preserves_original_invocation_fixture_and_stable_receipt(project):
    run_dir = capture_owner(project)
    with optional_owner(run_dir, project) as owner:
        assert owner is not None
        receipt_id = owner.manifest["storage"]["receipt"]
        storage = Storage()
        receipt_before = storage.get(receipt_id)
        assert Path(receipt_before["path"]).is_dir()

        class Attachment:
            id = "first-logical-attempt"
            server = SimpleNamespace(
                id="stable-service",
                kind="synthetic",
                exited=lambda: None,
                record={"service_generation": "observed-service-generation"},
            )

            def environment(self, env):
                return dict(env)

            def restart(self):
                assert owner.invocations[0].row["command"]["exit"] == 0
                self.id = "second-logical-attempt"

        @contextlib.contextmanager
        def fixture(options):
            yield Attachment()

        first = planned(project, restart=True, fixture=True)
        second = verify.Step("consumer", "pytest")
        boundary = verify.Boundary(
            "evidence",
            "restart",
            "logical attempt transition",
            (first.steps[0].step, second),
            frozenset({"tools"}),
            fixture=True,
        )
        selection = verify.Planned(
            boundary,
            first.selection,
            (first.steps[0], verify.PlannedStep(second, first.steps[0].argv)),
        )
        execution, _ = runtime(owner, project, fixture=fixture)
        result = verify.run_boundary(
            selection, execution, verify.Options(), run_dir / "python.log", False
        )
        assert result["outcome"] == "passed", result
        assert result["fixture"]["attachment"] == "second-logical-attempt"
        assert result["fixture"]["attempts"] == ["first-logical-attempt", "second-logical-attempt"]
        assert owner.invocations[0].row["fixture"]["attachment"] == "first-logical-attempt"
        assert "attempts" not in owner.invocations[0].row["fixture"]
        assert owner.invocations[1].row["fixture"]["attachment"] == "second-logical-attempt"
        for invocation in owner.invocations:
            persisted = json.loads((invocation.directory / "invocation.json").read_text())
            assert persisted["fixture"] == invocation.row["fixture"]
            assert persisted["fixture"]["service_generation"] == "observed-service-generation"
            assert persisted["boundary_final"]["fixture_finalization"]["status"] == "completed"
        # Atomic manifest replacement occurs repeatedly within this stable
        # registered receipt directory; its own physical binding remains valid.
        receipt_after = storage.retain(receipt_id, "stable-receipt-control", "receipt")
        assert receipt_after["identity"] == receipt_before["identity"]
        assert (run_dir / "verify-evidence.json").resolve() == owner.manifest_path


OWNED_COORDINATOR = """import contextlib,os,sys
from pathlib import Path
import runs,verify,workspace_env
from verify_evidence.capture import optional_owner
root=Path(sys.argv[1])
verify.ROOT=root
directory=runs.current_run()
assert directory is not None
step=verify.Step("pytest","pytest")
boundary=verify.Boundary("evidence","owned","synthetic",(step,),frozenset({"tools"}))
argv=(str(root/".venv/bin/python"),"-m","pytest","-c",str(root/"pytest.ini"),
      "--confcutdir",str(root),"-q",str(root/"test_case.py"))
planned=verify.Planned(boundary,{"cargo_profile":"release"},(verify.PlannedStep(step,argv),))
with optional_owner(directory,root) as owner:
    runtime=verify.Runtime(
        observe=lambda requirements:{r:workspace_env.observe(r,root=root) for r in requirements},
        run=lambda argv,env,log,live:verify.launch(argv,env,log,live),
        fixture=lambda options:contextlib.nullcontext(),
        owner=lambda requirements,report:workspace_env.ownership(
            "shared",requirements,root=root,report=report),
        base_env=dict(os.environ),target_dir=root/"target",evidence=owner,
        launch_request=lambda request:verify.launch(
            request.argv,request.env,request.log,request.live,
            stdout=request.stdout,observer=request.observer,cwd=request.cwd),
        report=lambda message:None)
    result=verify.run_boundary(planned,runtime,verify.Options(),directory/"python.log",False)
    raise SystemExit(0 if result["outcome"]=="passed" else 1)
"""


def start_owned_coordinator(project, console, **environment):
    env = dict(os.environ, PYTHONPATH=str(ROOT / "scripts"), **environment)
    return subprocess.Popen(
        [
            STDLIB_PYTHON,
            str(ROOT / "scripts/runs.py"),
            "run",
            "--label",
            "synthetic capture",
            "--",
            STDLIB_PYTHON,
            "-c",
            OWNED_COORDINATOR,
            str(project),
        ],
        cwd=project,
        env=env,
        stdout=console,
        stderr=subprocess.STDOUT,
    )


def wait_for(predicate, console, timeout=20):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(0.01)
    raise AssertionError(console.read_text())


def owned_run_directory(project):
    directories = list((project.parent / "runs").iterdir())
    assert len(directories) == 1
    return directories[0]


def stop_owned_launcher(launcher):
    # The live run owner remains responsible for its recorded child's group.
    # Never reconstruct signalling authority from a surviving numeric PGID.
    if launcher.poll() is None:
        launcher.terminate()
    launcher.wait(timeout=20)


def test_hard_stopped_coordinator_preserves_prefix_and_run_owner_confirms_cleanup(project):
    marker = project.parent / "inflight"
    (project / "test_case.py").write_text("""import os,pathlib,time
def test_a_pass(): print("completed-before-hard-stop")
def test_b_inflight():
    print("unfinished-call-output",flush=True)
    pathlib.Path(os.environ["INFLIGHT_MARKER"]).write_text("ready")
    time.sleep(60)
""")
    console = project.parent / "launcher.log"
    with console.open("wb") as output:
        launcher = start_owned_coordinator(project, output, INFLIGHT_MARKER=str(marker))
        try:
            wait_for(lambda: marker.exists() or launcher.poll() is not None, console)
            assert marker.exists(), console.read_text()
            assert launcher.poll() is None
            run_dir = owned_run_directory(project)
            record = runs.load_record(run_dir)
            assert record is not None and record["phase"] == "running"
            coordinator = ProcessIdentity.from_json(record["child"])
            assert coordinator.alive() and coordinator.pid != launcher.pid
            manifest = json.loads((run_dir / "verify-evidence.json").read_text())
            invocation = run_dir / "verify-evidence" / manifest["invocations"][0]["id"]
            live = list(pytest_adapter.records(invocation))
            assert any(row["phase"] == "call" and row["outcome"] == "passed" for row in live)
            assert any(
                row["phase"] == "start" and row["nodeid"].endswith("::test_b_inflight")
                for row in live
            )
            # Kill only the coordinator; its still-live run owner owns cleanup.
            os.kill(coordinator.pid, signal.SIGKILL)
            assert launcher.wait(timeout=20) == 128 + signal.SIGKILL
            final = runs.load_record(run_dir)
            assert final is not None
            assert final["exit"] == {"signal": signal.SIGKILL, "name": "SIGKILL"}
            assert final["cleanup"]["status"] == "confirmed"
            assert group_members(record["child"]["pgid"]) == []
            row = json.loads((invocation / "invocation.json").read_text())
            assert "command" not in row and "boundary_final" not in row
            assert row["capture"]["status"] == "effective"
            retained = list(pytest_adapter.records(invocation))
            assert any("completed-before-hard-stop" in row["payload"] for row in retained)
            assert not any("unfinished-call-output" in row["payload"] for row in retained)
            assert not any(
                row["phase"] == "call" and row["nodeid"].endswith("::test_b_inflight")
                for row in retained
            )
            availability = pytest_adapter.finalize(invocation)
            assert availability["channels"][pytest_adapter.REPORT]["status"] == "partial"
            assert availability["channels"][pytest_adapter.REPORT]["available_prefix"]
            assert not availability["channels"][pytest_adapter.REPORT]["session_finished"]
        finally:
            stop_owned_launcher(launcher)


def completed_owned_capture(project):
    console = project.parent / "launcher.log"
    with console.open("wb") as output:
        launcher = start_owned_coordinator(project, output)
        try:
            assert launcher.wait(timeout=30) == 0, console.read_text()
        finally:
            stop_owned_launcher(launcher)
    run_dir = owned_run_directory(project)
    record = runs.load_record(run_dir)
    assert record is not None
    assert record["cleanup"]["status"] == "confirmed"
    assert not runs.owner_alive(run_dir, record)
    manifest = json.loads((run_dir / "verify-evidence.json").read_text())
    return run_dir, manifest


def beyond_ordinary_expiry(monkeypatch):
    future = dt.datetime.now(dt.UTC) + dt.timedelta(days=120)

    class FutureDateTime(dt.datetime):
        @classmethod
        def now(cls, tz: dt.tzinfo | None = None) -> Self:
            return cls.fromtimestamp(future.timestamp(), tz)

    # Advance observation time, leaving the authoritative cleanup receipt and
    # retention timestamps intact. This never substitutes file age for release.
    monkeypatch.setattr(
        lifecycle,
        "dt",
        SimpleNamespace(datetime=FutureDateTime, UTC=dt.UTC, timedelta=dt.timedelta),
    )
    return future


def test_exact_raw_hold_outlives_expiry_and_release_only_retires_raw_component(
    project, monkeypatch
):
    run_dir, manifest = completed_owned_capture(project)
    storage = Storage()
    raw_id, receipt_id = manifest["storage"]["raw"], manifest["storage"]["receipt"]
    for object_id in (raw_id, receipt_id):
        row = storage.get(object_id)
        assert row["obligations"][f"run:{run_dir.name}"]["until"]
    storage.retain(raw_id, "citation:exact-native-original", "raw-replay")
    storage.retain(receipt_id, "receipt:independent-consumer", "receipt")
    beyond_ordinary_expiry(monkeypatch)
    assert storage.plan([raw_id], references=[])["dispositions"][0]["disposition"] == "retained"
    with pytest.raises(lifecycle.Blocked, match="citation:exact-native-original"):
        storage.retire(raw_id, references=[])
    storage.release(
        raw_id, "citation:exact-native-original", "synthetic citation explicitly retired"
    )
    assert storage.plan([raw_id], references=[])["dispositions"][0]["disposition"] == "eligible"
    assert storage.plan([receipt_id], references=[])["dispositions"][0]["disposition"] == "retained"
    assert storage.retire(raw_id, references=[])["action"] == "retired"
    assert not (run_dir / "verify-evidence").exists()
    assert (run_dir / "verify-evidence.json").exists() and (run_dir / "record.json").exists()
    assert (
        storage.get(receipt_id)["obligations"]["receipt:independent-consumer"]["released_at"]
        is None
    )


def test_actual_evidence_reader_admission_excludes_raw_retirement(project, monkeypatch):
    run_dir, manifest = completed_owned_capture(project)
    storage = Storage()
    raw_id = manifest["storage"]["raw"]
    beyond_ordinary_expiry(monkeypatch)
    assert storage.plan([raw_id], references=[])["dispositions"][0]["disposition"] == "eligible"
    marker = project.parent / "reader-ready"
    code = """import sys
from pathlib import Path
from verify_evidence import cli
actual=cli.report
def held(root):
    Path(sys.argv[2]).write_text("admitted")
    sys.stdin.readline()
    return actual(root)
cli.report=held
raise SystemExit(cli.main(["report",sys.argv[1],"--json"]))
"""
    child = subprocess.Popen(
        [STDLIB_PYTHON, "-c", code, str(run_dir), str(marker)],
        env=dict(os.environ, PYTHONPATH=str(ROOT / "scripts")),
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        console = project.parent / "launcher.log"
        wait_for(lambda: marker.exists() or child.poll() is not None, console)
        assert marker.exists()
        with pytest.raises(lifecycle.Blocked, match="busy"):
            storage.retire(raw_id, references=[])
        stdout, stderr = child.communicate("reader complete\n", timeout=20)
        assert child.returncode == 0, stderr
        assert json.loads(stdout)["invocations"]
        assert storage.retire(raw_id, references=[])["action"] == "retired"
        assert (run_dir / "verify-evidence.json").exists()
    finally:
        if child.poll() is None:
            child.terminate()
        child.communicate(timeout=20)
