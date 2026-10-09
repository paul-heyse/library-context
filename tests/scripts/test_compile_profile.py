"""Compilation profile transport, report currency and run lifecycle contracts."""

from __future__ import annotations

import argparse
import json
import os
import signal
import struct
import subprocess
import sys
import time
from pathlib import Path

import pytest

import compile_profile as profile
import compile_profile_capture as capture
import compile_profile_tools as tools
import runs
from harness import ProcessIdentity, write_json_atomic


@pytest.fixture
def root(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "runs"))
    monkeypatch.delenv("LCTX_RUN_DIR", raising=False)
    monkeypatch.delenv("LCTX_RUN_ID", raising=False)
    return tmp_path


@pytest.mark.parametrize(
    "command",
    [
        [
            "cargo",
            "build",
            "--profile",
            "local-test-candidate",
            "-p",
            "cpg-core",
            "--features",
            "x",
            "-j",
            "6",
            "--target-dir",
            "/tmp/owned",
        ],
        [
            "cargo",
            "+nightly-2026-09-29",
            "--config",
            "build.jobs=4",
            "rustc",
            "-p",
            "cpg-core",
            "--",
            "-Copt-level=1",
        ],
        [
            "cargo",
            "nextest",
            "run",
            "--cargo-profile",
            "local-test-candidate",
            "-E",
            "package(cpg-core)",
            "--",
            "filter",
        ],
        ["cargo", "nextest", "list", "--message-format", "json"],
    ],
)
def test_direct_cargo_flags_preserve_argument_order(command):
    updated, direct = profile.cargo_command(command)
    assert direct
    remaining = updated.copy()
    index = remaining.index("-Zbuild-analysis")
    del remaining[index : index + 4]
    remaining.remove("--timings")
    if "--" in command:
        assert updated.index("--timings") < updated.index("--")
    if "nextest" in command:
        assert updated.index("-Zbuild-analysis") == updated.index("nextest") + 2
    option = (
        "--cargo-message-format=json,json-render-diagnostics"
        if "nextest" in command
        else "--message-format=json-render-diagnostics"
    )
    remaining.remove(option)
    assert remaining == command


@pytest.mark.parametrize(
    "command",
    [
        ["just", "verify", "--select", "compiler"],
        ["cargo", "metadata"],
        ["cargo", "nextest", "archive"],
        ["bash", "-c", "cargo test"],
    ],
)
def test_arbitrary_wrappers_are_transparent(command):
    assert profile.cargo_command(command) == (command, False)


def test_existing_timings_and_rustc_boundary_retained():
    command = ["cargo", "rustc", "--timings=json", "--", "--cfg", "custom"]
    updated, _ = profile.cargo_command(command)
    assert updated.count("--timings=json") == 1
    assert "--timings" not in updated
    assert updated[-3:] == ["--", "--cfg", "custom"]


def test_missing_analysis_blocks_before_launch(root, monkeypatch, capsys):
    monkeypatch.setattr(
        profile,
        "doctor",
        lambda **_: {"outcome": "blocked", "checks": {"cargo_analysis": {"outcome": "blocked"}}},
    )
    monkeypatch.setattr(runs, "cmd_run", lambda _: pytest.fail("must not launch"))
    args = argparse.Namespace(
        command=["--", "cargo", "build"], focus="cpg-core", background=False, label=None, json=False
    )
    assert profile.cmd_record(args) == 75
    assert "cargo_analysis" in capsys.readouterr().out
    assert not runs.runs_root().exists()


def test_record_reuses_run_launcher_and_keeps_original_command(root, monkeypatch):
    monkeypatch.setattr(profile, "doctor", lambda **_: {"outcome": "passed"})
    monkeypatch.setattr(profile, "focus_manifests", lambda *_: ["/workspace/cpg-core"])
    passed = []
    monkeypatch.setattr(runs, "cmd_run", lambda args: passed.append(args) or 0)
    args = argparse.Namespace(
        command=["--", "just", "verify", "--select", "compiler"],
        focus="workspace",
        background=True,
        label="compile",
        json=True,
    )
    assert profile.cmd_record(args) == 0
    assert passed[0].background and passed[0].label == "compile" and passed[0].json
    assert passed[0].command[-4:] == ["just", "verify", "--select", "compiler"]


def test_profiling_environment_and_product_exit_are_independent(root, monkeypatch):
    run_dir = runs.new_run_dir()
    write_json_atomic(
        run_dir / "record.json", {"owner": ProcessIdentity.of().to_json(), "termination": None}
    )
    monkeypatch.setenv("LCTX_RUN_DIR", str(run_dir))
    monkeypatch.setenv("RUSTC_WRAPPER", "existing-wrapper")
    monkeypatch.setattr(profile, "focus_manifests", lambda *_: ["/workspace/core"])
    monkeypatch.setattr(profile, "provenance", lambda *_: {"version": 1})
    command = [
        sys.executable,
        "-c",
        "import os,json,sys; "
        "print(json.dumps({k:v for k,v in os.environ.items() "
        "if k.startswith('LCTX_COMPILE_PROFILE')})); sys.exit(7)",
    ]
    args = argparse.Namespace(command=command, focus="cpg-core")
    assert profile.cmd_internal(args) == 7
    meta = json.loads((run_dir / "compile-profile/record.json").read_text())
    env = meta["environment"]
    assert Path(env["LCTX_COMPILE_PROFILE_DIR"]).is_absolute()
    assert json.loads(env["LCTX_COMPILE_PROFILE_FOCUS"]) == ["/workspace/core"]
    assert env["LCTX_COMPILE_PROFILE_DELEGATE"] == "existing-wrapper"
    assert meta["product"] == {"outcome": "failed", "exit_code": 7, "signals": []}
    assert meta["telemetry"]["outcome"] == "not_run"


def test_repeated_cancellation_during_finalization_keeps_product_exit_and_receipt(
    root, monkeypatch
):
    run_dir = runs.new_run_dir()
    write_json_atomic(
        run_dir / "record.json", {"owner": ProcessIdentity.of().to_json(), "termination": None}
    )
    monkeypatch.setenv("LCTX_RUN_DIR", str(run_dir))
    monkeypatch.setattr(profile, "focus_manifests", lambda *_: ["/workspace/core"])
    monkeypatch.setattr(profile, "provenance", lambda *_: {"version": 1})
    previous = signal.getsignal(signal.SIGTERM)

    def finalizing(_directory):
        handler = signal.getsignal(signal.SIGTERM)
        assert callable(handler), "cancellation handler was restored before terminal receipts"
        handler(signal.SIGTERM, None)
        handler(signal.SIGTERM, None)
        return []

    monkeypatch.setattr(profile, "unit_records", finalizing)
    assert (
        profile.cmd_internal(
            argparse.Namespace(
                command=[sys.executable, "-c", "import sys; sys.exit(7)"], focus="cpg-core"
            )
        )
        == 7
    )
    final = json.loads((run_dir / "compile-profile/record.json").read_text())
    assert final["product"] == {"outcome": "failed", "exit_code": 7, "signals": [signal.SIGTERM]}
    assert signal.getsignal(signal.SIGTERM) == previous


def test_cancel_uses_existing_run_group_and_retains_profile(root):
    launcher = subprocess.run(
        [
            sys.executable,
            str(Path(runs.__file__)),
            "run",
            "--background",
            "--json",
            "--",
            sys.executable,
            str(profile.SCRIPT),
            "_record",
            "--focus",
            "cpg-core",
            "--",
            sys.executable,
            "-c",
            "import time; time.sleep(90)",
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert launcher.returncode == 0, launcher.stderr
    run_id = json.loads(launcher.stdout)["id"]
    run_dir = runs.resolve(run_id)
    deadline = time.monotonic() + 20
    try:
        while time.monotonic() < deadline:
            meta = (
                json.loads((run_dir / "compile-profile/record.json").read_text())
                if (run_dir / "compile-profile/record.json").exists()
                else {}
            )
            if "command_identity" in meta:
                break
            time.sleep(0.05)
        assert "command_identity" in meta
        cancelled = subprocess.run(
            [sys.executable, str(Path(runs.__file__)), "cancel", run_id, "--json"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert cancelled.returncode == 0, cancelled.stderr
        assert json.loads(cancelled.stdout)["state"] == "cancelled"
        final = json.loads((run_dir / "compile-profile/record.json").read_text())
        assert final["product"]["exit_code"] == -signal.SIGTERM
        assert not ProcessIdentity.from_json(meta["command_identity"]).alive()
    finally:
        if runs.state_of(run_dir, runs.load_record(run_dir)) == "running":
            subprocess.run([sys.executable, str(Path(runs.__file__)), "cancel", run_id], timeout=30)


@pytest.mark.parametrize("before_retention", [False, True])
def test_cancel_stops_separate_session_descendants_and_preserves_unrelated_process(
    root, before_retention
):
    receipt = root / "escaped.json"
    gate = root / "allow-observation"
    cargo = """
import json,os,pathlib,subprocess,sys,time
sys.path.insert(0,'scripts')
from compile_profile_capture import process_identity
compiler=subprocess.Popen([sys.executable,'-c','import time; time.sleep(90)'])
pathlib.Path(sys.argv[1]).write_text(json.dumps([
    process_identity(os.getpid()),process_identity(compiler.pid)]))
time.sleep(90)
"""
    nextest = """
import subprocess,sys,time
subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2]],start_new_session=True)
time.sleep(90)
"""
    unrelated = subprocess.Popen(
        [sys.executable, "-c", "import time; time.sleep(90)"], start_new_session=True
    )
    unrelated_identity = capture.process_identity(unrelated.pid)
    run_dir = None
    identities = []
    try:
        runner = [sys.executable, str(profile.SCRIPT)]
        if before_retention:
            runner = [
                sys.executable,
                "-c",
                """
import pathlib,sys
sys.path.insert(0,'scripts')
import compile_profile as profile
gate=pathlib.Path(sys.argv[1])
observe=profile.OwnedCommandTree.observe
def delayed(self):
    if gate.exists(): observe(self)
profile.OwnedCommandTree.observe=delayed
raise SystemExit(profile.main(sys.argv[2:]))
""",
                str(gate),
            ]
        launched = subprocess.run(
            [
                sys.executable,
                str(Path(runs.__file__)),
                "run",
                "--background",
                "--json",
                "--",
                *runner,
                "_record",
                "--focus",
                "cpg-core",
                "--",
                sys.executable,
                "-c",
                nextest,
                cargo,
                str(receipt),
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert launched.returncode == 0, launched.stderr
        run_dir = runs.resolve(json.loads(launched.stdout)["id"])
        deadline = time.monotonic() + 20
        retained = []
        while time.monotonic() < deadline:
            if receipt.exists():
                identities = json.loads(receipt.read_text())
            path = run_dir / "compile-profile/owned-descendants.json"
            if path.exists():
                retained = json.loads(path.read_text())
            captured = identities and {item["pid"] for item in identities}.issubset(
                {item["pid"] for item in retained}
            )
            if identities and (captured or before_retention):
                break
            time.sleep(0.05)
        assert identities
        if before_retention:
            assert not {item["pid"] for item in identities}.intersection(
                {item["pid"] for item in retained}
            )
        else:
            assert captured
        record = runs.load_record(run_dir)
        assert record is not None
        assert identities[0]["pid"] != os.getpgid(ProcessIdentity.from_json(record["child"]).pid)
        cancelling = subprocess.Popen(
            [sys.executable, str(Path(runs.__file__)), "cancel", run_dir.name, "--json"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if before_retention:
            profile_pid = record["child"]["pid"]
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                fields = (
                    Path(f"/proc/{identities[0]['pid']}/stat").read_text().rsplit(")", 1)[1].split()
                )
                if int(fields[1]) == profile_pid:
                    break
                time.sleep(0.01)
            assert int(fields[1]) == profile_pid, "escaped Cargo was not adopted by the subreaper"
            gate.touch()
        stdout, stderr = cancelling.communicate(timeout=30)
        cancelled = subprocess.CompletedProcess(
            cancelling.args, cancelling.returncode, stdout, stderr
        )
        assert cancelled.returncode == 0, cancelled.stderr
        assert json.loads(cancelled.stdout)["state"] == "cancelled"
        assert all(not capture.identity_matches(item["pid"], item) for item in identities)
        assert capture.identity_matches(unrelated.pid, unrelated_identity)
        final = json.loads((run_dir / "compile-profile/record.json").read_text())
        assert any(item["pgid"] == identities[0]["pid"] for item in final["descendant_signals"])
        assert final["product"]["exit_code"] == -signal.SIGTERM
    finally:
        if run_dir is not None and runs.state_of(run_dir, runs.load_record(run_dir)) == "running":
            subprocess.run(
                [sys.executable, str(Path(runs.__file__)), "cancel", run_dir.name], timeout=30
            )
        for item in identities:
            if capture.identity_matches(item["pid"], item):
                os.kill(item["pid"], signal.SIGKILL)
        unrelated.terminate()
        unrelated.wait(timeout=5)


@pytest.mark.parametrize("changed", [{"start_time": "0"}, {"uid": -1}])
def test_descendant_cancellation_refuses_changed_identity(changed):
    target = subprocess.Popen(
        [sys.executable, "-c", "import time; time.sleep(90)"], start_new_session=True
    )
    try:
        tree = profile.OwnedCommandTree(target.pid)
        tree.processes[target.pid].update(changed)
        tree.signal_escaped(signal.SIGTERM)
        assert target.poll() is None
        assert not tree.actions
    finally:
        target.terminate()
        target.wait(timeout=5)


def test_cancel_reaches_adopted_child_when_root_exited_before_identity_receipt(root):
    receipt = root / "already-orphaned.json"
    runner = """
import pathlib,sys,time
sys.path.insert(0,'scripts')
import compile_profile as profile
initialize=profile.OwnedCommandTree.__init__
identify=profile.ProcessIdentity.of
def after_exit(self,pid,*args):
    deadline=time.monotonic()+5
    while time.monotonic()<deadline:
        fields=pathlib.Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()
        if fields[0]=='Z': break
        time.sleep(.01)
    assert fields[0]=='Z'
    def unavailable(candidate=None):
        if candidate==pid: raise ProcessLookupError('exited root namespace receipt unavailable')
        return identify(candidate)
    profile.ProcessIdentity.of=unavailable
    initialize(self,pid,*args)
profile.OwnedCommandTree.__init__=after_exit
raise SystemExit(profile.main(sys.argv[1:]))
"""
    command = """
import json,pathlib,subprocess,sys
sys.path.insert(0,'scripts')
from compile_profile_capture import process_identity
child=subprocess.Popen([sys.executable,'-c','import time; time.sleep(90)'],
    start_new_session=True)
pathlib.Path(sys.argv[1]).write_text(json.dumps(process_identity(child.pid)))
"""
    launched = subprocess.run(
        [
            sys.executable,
            str(Path(runs.__file__)),
            "run",
            "--background",
            "--json",
            "--",
            sys.executable,
            "-c",
            runner,
            "_record",
            "--focus",
            "cpg-core",
            "--",
            sys.executable,
            "-c",
            command,
            str(receipt),
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert launched.returncode == 0, launched.stderr
    run_dir = runs.resolve(json.loads(launched.stdout)["id"])
    identity = None
    try:
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if receipt.exists():
                identity = json.loads(receipt.read_text())
            retained = run_dir / "compile-profile/owned-descendants.json"
            if (
                identity
                and retained.exists()
                and any(item["pid"] == identity["pid"] for item in json.loads(retained.read_text()))
            ):
                break
            time.sleep(0.05)
        assert identity and retained.exists()
        assert capture.identity_matches(identity["pid"], identity)
        cancelled = subprocess.run(
            [sys.executable, str(Path(runs.__file__)), "cancel", run_dir.name, "--json"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert cancelled.returncode == 0, cancelled.stderr
        assert json.loads(cancelled.stdout)["state"] == "cancelled"
        assert not capture.identity_matches(identity["pid"], identity)
        final = json.loads((run_dir / "compile-profile/record.json").read_text())
        assert "command_identity" not in final
        assert any(item.startswith("command identity receipt:") for item in final["stdout_errors"])
        assert final["product"]["exit_code"] == 0
        assert final["product"]["signals"] == [signal.SIGTERM]
    finally:
        if runs.state_of(run_dir, runs.load_record(run_dir)) == "running":
            subprocess.run(
                [sys.executable, str(Path(runs.__file__)), "cancel", run_dir.name], timeout=30
            )
        if identity and capture.identity_matches(identity["pid"], identity):
            os.kill(identity["pid"], signal.SIGKILL)


def test_cancellation_preserves_slow_sampler_finalization_beyond_compiler_grace(root):
    perf = root / "slow-perf"
    perf.write_text(
        f"#!{sys.executable}\n"
        + r"""
import pathlib,signal,sys,time
output=pathlib.Path(sys.argv[sys.argv.index('-o')+1])
active=output.open('wb'); active.write(b'raw sample'); active.flush()
def finish(*_):
    signal.signal(signal.SIGINT,signal.SIG_IGN)
    time.sleep(6.25)
    active.close()
    completed=output.with_name(output.name+'.202610080001')
    output.rename(completed)
    print('[ perf record: Dump '+str(completed)+' ]',flush=True)
    raise SystemExit(0)
signal.signal(signal.SIGINT,finish)
signal.signal(signal.SIGTERM,lambda *_:sys.exit(91))
output.with_name('ready').touch()
while True: time.sleep(.05)
"""
    )
    perf.chmod(0o755)
    command = r"""
import os,pathlib,sys,time
sys.path.insert(0,'scripts')
import compile_profile_capture as capture
unit=pathlib.Path(os.environ['LCTX_COMPILE_PROFILE_DIR'])/'units'/'slow-sampler'
identity=capture.process_identity(os.getpid())
sampler=capture.start_sampler(unit,os.getpid(),identity,sys.argv[1])
capture.write_json(unit/'record.json',dict(identity,phase='running',kind='normal',
    collector={'pid':sampler.pid,'status':'running'},sidecars={}))
time.sleep(90)
"""
    launched = subprocess.run(
        [
            sys.executable,
            str(Path(runs.__file__)),
            "run",
            "--background",
            "--json",
            "--",
            sys.executable,
            str(profile.SCRIPT),
            "_record",
            "--focus",
            "cpg-core",
            "--",
            sys.executable,
            "-c",
            command,
            str(perf),
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert launched.returncode == 0, launched.stderr
    run_dir = runs.resolve(json.loads(launched.stdout)["id"])
    unit = run_dir / "compile-profile/units/slow-sampler"
    sampler_identity = None
    try:
        deadline = time.monotonic() + 20
        while not (unit / "ready").exists() and time.monotonic() < deadline:
            time.sleep(0.05)
        assert (unit / "ready").exists()
        sampler = json.loads((unit / "sampler.json").read_text())
        sampler_identity = sampler["identity"]
        started = time.monotonic()
        cancelled = subprocess.run(
            [sys.executable, str(Path(runs.__file__)), "cancel", run_dir.name, "--json"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert cancelled.returncode == 0, cancelled.stderr
        assert json.loads(cancelled.stdout)["state"] == "cancelled"
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            collector = json.loads((unit / "collector.json").read_text())
            if collector["phase"] == "completed":
                break
            time.sleep(0.05)
        assert collector["phase"] == "completed"
        assert collector["exit_code"] == 0
        assert time.monotonic() - started > 5
        assert capture.read_completed_chunks(unit) == [unit / "perf.data.202610080001"]
        assert (unit / "perf.data.202610080001").read_bytes() == b"raw sample"
        final = json.loads((run_dir / "compile-profile/record.json").read_text())
        assert all(item["pgid"] != sampler["pid"] for item in final["descendant_signals"])
    finally:
        if runs.state_of(run_dir, runs.load_record(run_dir)) == "running":
            subprocess.run(
                [sys.executable, str(Path(runs.__file__)), "cancel", run_dir.name], timeout=30
            )
        if sampler_identity and capture.identity_matches(sampler_identity["pid"], sampler_identity):
            os.killpg(sampler_identity["pid"], signal.SIGKILL)


def test_self_profile_format_guard_fails_before_tools(tmp_path, monkeypatch):
    path = tmp_path / "incompatible.mm_profdata"
    path.write_bytes(b"MMPD" + struct.pack("<I", 8))
    monkeypatch.setattr(
        profile, "execute", lambda *_args, **_kw: pytest.fail("must not run incompatible reader")
    )
    assert profile.compiler_report(path)["outcome"] == "blocked"
    assert not (tmp_path / "reports").exists()


def test_matching_format_requires_exact_tool_receipt(tmp_path, monkeypatch):
    path = tmp_path / "profile.mm_profdata"
    path.write_bytes(b"MMPD" + struct.pack("<I", 9))
    monkeypatch.setattr(
        tools, "check", lambda _: {"status": "blocked", "errors": ["binary differs"]}
    )
    result = profile.compiler_report(path)
    assert result["outcome"] == "blocked"
    assert result["readiness"]["errors"] == ["binary differs"]


def test_closed_sample_report_refreshes_only_changed_input(tmp_path, monkeypatch):
    chunk = tmp_path / "perf.data.1"
    chunk.write_bytes(b"first")
    calls = []

    def execute(argv, **_):
        calls.append(argv)
        return {"outcome": "passed", "stdout": "hot symbol", "stderr": "", "exit_code": 0}

    monkeypatch.setattr(profile, "execute", execute)
    first = profile.sampled_report(chunk)
    assert first["outcome"] == "passed"
    profile.sampled_report(chunk)
    assert len(calls) == 1
    chunk.write_bytes(b"updated-longer")
    profile.sampled_report(chunk)
    assert len(calls) == 2
    assert "--children" in calls[0] and "--stdio" in calls[0]


def test_status_skips_open_chunks_and_is_readonly(root, monkeypatch):
    directory = runs.new_run_dir() / "compile-profile"
    unit_dir = directory / "units/a"
    write_json_atomic(unit_dir / "record.json", {"phase": "running", "pid": 1})
    monkeypatch.setattr(capture, "read_completed_chunks", lambda _: [])
    monkeypatch.setattr(runs, "view", lambda _: {"state": "running"})
    monkeypatch.setattr(
        profile, "sampled_report", lambda *_args, **_kw: pytest.fail("must not analyze open data")
    )
    data = profile.status_data(directory.parent.name)
    assert data["units"][0]["completed_chunks"] == []
    assert data["units"][0]["recent_hotspots"] is None
    assert not (directory / "report.json").exists()


def test_report_never_reads_active_compiler_sidecars(root, monkeypatch):
    directory = runs.new_run_dir() / "compile-profile"
    unit_dir = directory / "units/a"
    write_json_atomic(unit_dir / "record.json", {"phase": "running", "pid": 1})
    (unit_dir / "active.mm_profdata").write_bytes(b"MMPD" + struct.pack("<I", 9))
    monkeypatch.setattr(capture, "read_completed_chunks", lambda _: [])
    monkeypatch.setattr(runs, "view", lambda _: {"state": "running"})
    monkeypatch.setattr(
        profile,
        "compiler_report",
        lambda *_args, **_kw: pytest.fail("must not analyze active profile"),
    )
    data = profile.report_data(directory.parent.name)
    assert data["units"][0]["outcome"] == "not_run"


def test_attach_refuses_dead_or_foreign_owned_run(root, monkeypatch):
    run_dir = runs.new_run_dir()
    write_json_atomic(run_dir / "record.json", {"termination": "completed"})
    with pytest.raises(ValueError, match="live owned run"):
        profile.cmd_attach(argparse.Namespace(run=run_dir.name))


def test_provenance_excludes_heldout_and_fixtures():
    assert not profile.source_allowed("fixtures/python/sealed.py")
    assert not profile.source_allowed("evaluation/heldout/example.rs")
    assert not profile.source_allowed(".venv-local/lib/code.py")
    assert profile.source_allowed("crates/cpg-core/src/lib.rs")


def test_session_logs_require_exact_identity_and_keep_malformed_evidence(root, monkeypatch):
    destination = root / "profile"
    destination.mkdir()
    cargo_home = root / "cargo-home"
    (cargo_home / "log").mkdir(parents=True)
    good = "owned-123"
    foreign = "other-456"
    command = ["cargo", "build", "--profile", "test", "--target-dir", str(root / "target")]
    start = {
        "reason": "build-started",
        "run_id": good,
        "workspace_root": str(root),
        "cwd": str(Path.cwd()),
        "target_dir": str(root / "target"),
        "profile": "test",
        "command": ["/tools/cargo", *command[1:]],
    }
    (cargo_home / "log" / f"{good}.jsonl").write_text(json.dumps(start) + "\n")
    (cargo_home / "log" / f"{foreign}.jsonl").write_text("must not read")
    monkeypatch.setattr(
        profile,
        "execute",
        lambda *_a, **_k: {
            "outcome": "passed",
            "stdout": json.dumps({"workspace_root": str(root)}),
        },
    )
    result = profile.retain_cargo_sessions(
        destination, [good], {"CARGO_HOME": str(cargo_home)}, command
    )
    assert result["outcome"] == "passed"
    assert not (destination / "cargo" / f"{foreign}.jsonl").exists()
    assert result["sessions"][0]["build"]["profile"] == "test"
    malformed = "owned-truncated"
    content = json.dumps({**start, "run_id": malformed}) + '\n{"truncated":'
    (cargo_home / "log" / f"{malformed}.jsonl").write_text(content)
    blocked = profile.retain_cargo_sessions(
        destination, [malformed], {"CARGO_HOME": str(cargo_home)}, command
    )
    assert blocked["outcome"] == "passed"
    assert blocked["partial"]
    retained = blocked["sessions"][0]
    assert retained["partial"]
    assert retained["dropped_tail_bytes"] == len(b'{"truncated":')
    assert retained["analysis_scope"] == "complete_prefix"
    assert retained["parsed_entries"] == 1
    assert Path(retained["path"]).read_text() == content


@pytest.mark.parametrize(
    "difference", ["profile", "target_dir", "command", "workspace_root", "cwd"]
)
def test_session_contract_mismatches_remain_blocked(root, monkeypatch, difference):
    destination = root / "profile"
    destination.mkdir()
    home = root / "home"
    (home / "log").mkdir(parents=True)
    command = [
        "cargo",
        "build",
        "--profile=local-test-candidate",
        "--target-dir",
        str(root / "target"),
    ]
    start = {
        "reason": "build-started",
        "run_id": "exact",
        "workspace_root": str(root),
        "cwd": str(Path.cwd()),
        "target_dir": str(root / "target"),
        "profile": "local-test-candidate",
        "command": ["/tools/cargo", *command[1:]],
    }
    start[difference] = ["cargo", "test"] if difference == "command" else "different"
    (home / "log/exact.jsonl").write_text(json.dumps(start) + "\n")
    monkeypatch.setattr(
        profile,
        "execute",
        lambda *_a, **_k: {
            "outcome": "passed",
            "stdout": json.dumps({"workspace_root": str(root)}),
        },
    )
    result = profile.retain_cargo_sessions(
        destination, ["exact"], {"CARGO_HOME": str(home)}, command
    )
    assert result["outcome"] == "blocked"
    assert Path(result["sessions"][0]["path"]).is_file()


def test_raw_perf_view_uses_local_import(root, monkeypatch):
    directory = runs.new_run_dir() / "compile-profile"
    unit = directory / "units/a"
    write_json_atomic(unit / "record.json", {"phase": "completed"})
    chunk = unit / "perf.data.123"
    chunk.write_bytes(b"perf")
    monkeypatch.setattr(capture, "read_completed_chunks", lambda _: [chunk])
    monkeypatch.setattr(profile.shutil, "which", lambda _: "/tools/samply")
    calls = []
    monkeypatch.setattr(profile.subprocess, "call", lambda argv: calls.append(argv) or 0)
    assert (
        profile.cmd_view(
            argparse.Namespace(run=directory.parent.name, kind="sampled", function=None)
        )
        == 0
    )
    assert calls == [["/tools/samply", "import", "--address", "127.0.0.1", str(chunk)]]


def test_observer_storage_failure_keeps_draining_and_preserves_product_exit(
    root, monkeypatch, capsys
):
    run_dir = runs.new_run_dir()
    write_json_atomic(
        run_dir / "record.json", {"owner": ProcessIdentity.of().to_json(), "termination": None}
    )
    monkeypatch.setenv("LCTX_RUN_DIR", str(run_dir))
    monkeypatch.setattr(profile, "focus_manifests", lambda *_: ["/workspace/core"])
    monkeypatch.setattr(profile, "provenance", lambda *_: {"version": 1})
    original_open = Path.open

    def limited_open(path, *args, **kwargs):
        if path.name == "command.stdout.log":
            raise OSError(28, "No space left on device")
        return original_open(path, *args, **kwargs)

    monkeypatch.setattr(Path, "open", limited_open)
    original_write = profile.write_json_atomic
    writes = []

    def limited_write(path, data):
        writes.append(path)
        if len(writes) >= 3:
            raise OSError(28, "No space left on device")
        original_write(path, data)

    monkeypatch.setattr(profile, "write_json_atomic", limited_write)
    code = profile.cmd_internal(
        argparse.Namespace(
            focus="cpg-core",
            command=[sys.executable, "-c", "import sys; print('x'*131072); sys.exit(7)"],
        )
    )
    assert code == 7
    output = capsys.readouterr()
    assert len(output.out) >= 131072
    assert "telemetry final receipt failed" in output.err


def test_launcher_death_reaches_owned_command_without_orphan(root):
    launched = subprocess.run(
        [
            sys.executable,
            str(Path(runs.__file__)),
            "run",
            "--background",
            "--json",
            "--",
            sys.executable,
            str(profile.SCRIPT),
            "_record",
            "--focus",
            "cpg-core",
            "--",
            sys.executable,
            "-c",
            "import time; time.sleep(90)",
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert launched.returncode == 0, launched.stderr
    run_dir = runs.resolve(json.loads(launched.stdout)["id"])
    deadline = time.monotonic() + 20
    try:
        while time.monotonic() < deadline:
            path = run_dir / "compile-profile/record.json"
            meta = json.loads(path.read_text()) if path.exists() else {}
            if "command_identity" in meta:
                break
            time.sleep(0.05)
        assert "command_identity" in meta
        command_identity = ProcessIdentity.from_json(meta["command_identity"])
        record = runs.load_record(run_dir)
        assert record is not None
        owner = ProcessIdentity.from_json(record["owner"])
        assert owner.alive() and command_identity.alive()
        import os

        os.kill(owner.pid, signal.SIGKILL)
        deadline = time.monotonic() + 10
        while command_identity.alive() and time.monotonic() < deadline:
            time.sleep(0.05)
        assert not command_identity.alive()
        assert runs.state_of(run_dir, runs.load_record(run_dir)) == "interrupted"
    finally:
        if runs.child_survivors(runs.load_record(run_dir) or {}):
            subprocess.run(
                [sys.executable, str(Path(runs.__file__)), "cancel", run_dir.name], timeout=30
            )


def test_nextest_runner_profile_is_not_cargo_profile(root, monkeypatch):
    destination = root / "profile"
    destination.mkdir()
    home = root / "home"
    (home / "log").mkdir(parents=True)
    command = [
        "cargo",
        "nextest",
        "run",
        "--profile",
        "ci",
        "--cargo-profile",
        "local-test-candidate",
    ]
    start = {
        "reason": "build-started",
        "run_id": "exact",
        "workspace_root": str(root),
        "cwd": str(Path.cwd()),
        "target_dir": str(root / "target"),
        "profile": "local-test-candidate",
        "command": ["/tools/cargo", "test", "--profile", "local-test-candidate"],
    }
    (home / "log/exact.jsonl").write_text(json.dumps(start) + "\n")
    monkeypatch.setattr(
        profile,
        "execute",
        lambda *_a, **_k: {
            "outcome": "passed",
            "stdout": json.dumps({"workspace_root": str(root)}),
        },
    )
    assert (
        profile.retain_cargo_sessions(destination, ["exact"], {"CARGO_HOME": str(home)}, command)[
            "outcome"
        ]
        == "passed"
    )


def test_cancel_retains_completed_chunks_and_partial_compiler_sidecars(root, monkeypatch):
    script = r"""
import json,os,pathlib,sys,time,struct
sys.path.insert(0, 'scripts')
from compile_profile_capture import process_identity
from harness import write_json_atomic
unit=pathlib.Path(os.environ['LCTX_COMPILE_PROFILE_DIR'])/'units'/'partial-compiler'
unit.mkdir(parents=True)
identity=process_identity(os.getpid())
write_json_atomic(unit/'record.json',dict(identity,phase='running',kind='normal',collector={'status':'cancelled'},sidecars={}))
(unit/'partial.mm_profdata').write_bytes(b'MMPD'+struct.pack('<I',8)+b'raw retained partial')
(unit/'time-passes.jsonl').write_text('{"pass":"analysis","time":1}\n')
chunk=unit/'perf.data.123'
chunk.write_bytes(b'closed sample bytes retained')
(unit/'collector.log').write_text('[ perf record: Dump '+str(chunk)+' ]\n')
write_json_atomic(unit/'collector.json',dict(pid=os.getpid(),identity=identity,phase='running'))
print('partial compiler prefix retained',flush=True)
time.sleep(90)
"""
    launched = subprocess.run(
        [
            sys.executable,
            str(Path(runs.__file__)),
            "run",
            "--background",
            "--json",
            "--",
            sys.executable,
            str(profile.SCRIPT),
            "_record",
            "--focus",
            "cpg-core",
            "--",
            sys.executable,
            "-c",
            script,
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert launched.returncode == 0, launched.stderr
    run_dir = runs.resolve(json.loads(launched.stdout)["id"])
    unit = run_dir / "compile-profile/units/partial-compiler"
    deadline = time.monotonic() + 20
    try:
        while not (unit / "collector.json").exists() and time.monotonic() < deadline:
            time.sleep(0.05)
        assert (unit / "collector.json").exists()
        cancelled = subprocess.run(
            [sys.executable, str(Path(runs.__file__)), "cancel", run_dir.name, "--json"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert cancelled.returncode == 0, cancelled.stderr
        assert json.loads(cancelled.stdout)["state"] == "cancelled"
        assert (run_dir / "compile-profile/source.patch").exists()
        assert (run_dir / runs.RETAIN).exists()
        assert "partial compiler prefix retained" in (run_dir / "output.log").read_text()
        chunks = capture.read_completed_chunks(unit)
        assert chunks == [unit / "perf.data.123"]
        monkeypatch.setattr(
            profile, "sampled_report", lambda chunk, **_: {"outcome": "passed", "path": str(chunk)}
        )
        report = profile.report_data(run_dir.name)
        assert report["partial"]
        assert report["units"][0]["partial"]
        assert report["units"][0]["lifecycle"] == "terminated"
        assert report["units"][0]["compiler"][0]["outcome"] == "blocked"
        assert report["units"][0]["sampled"][0]["path"] == str(chunks[0])
        assert "still active" not in (report["units"][0]["compiler_reason"] or "")
        assert (unit / "partial.mm_profdata").read_bytes().endswith(b"raw retained partial")
        assert chunks[0].read_bytes() == b"closed sample bytes retained"
    finally:
        if runs.state_of(run_dir, runs.load_record(run_dir)) == "running":
            subprocess.run(
                [sys.executable, str(Path(runs.__file__)), "cancel", run_dir.name], timeout=30
            )


def test_failed_completed_compiler_is_partial(root, monkeypatch):
    directory = runs.new_run_dir() / "compile-profile"
    unit = directory / "units/failed"
    write_json_atomic(
        unit / "record.json", {"phase": "completed", "exit_code": 1, "status": "failed"}
    )
    monkeypatch.setattr(runs, "view", lambda _: {"state": "completed"})
    monkeypatch.setattr(capture, "read_completed_chunks", lambda _: [])
    report = profile.report_data(directory.parent.name)
    assert report["partial"]
    assert report["units"][0]["partial"]
    assert "failed compiler" in report["units"][0]["compiler_reason"]


def test_live_thread_metrics_observe_cpu_and_wait_fields():
    import os

    metrics = profile.process_metrics(os.getpid())
    assert metrics["sampled_at_ns"] > 0
    assert metrics["rss_bytes"] > 0
    assert metrics["cpu_seconds"] >= 0
    assert metrics["threads"]
    main = next(thread for thread in metrics["threads"] if thread["tid"] == os.getpid())
    assert main["label"]
    assert main["cpu_seconds"] >= 0
    assert main["processor"] >= 0
    assert main["state"] in "RSDZTtXxKWPIN"
    assert "wait_channel" in main


def test_compiler_readers_have_no_timeout_and_threshold_is_cache_identity(tmp_path, monkeypatch):
    path = tmp_path / "profile.mm_profdata"
    raw = b"MMPD" + struct.pack("<I", 9)
    path.write_bytes(raw)
    monkeypatch.delenv("LCTX_COMPILE_PROFILE_TIMELINE_MIN_US", raising=False)
    monkeypatch.setattr(
        tools, "check", lambda _: {"status": "passed", "receipt": {"revision": "selected"}}
    )
    calls = []

    def observed(argv, **kwargs):
        calls.append((argv, kwargs))
        return {"outcome": "passed", "stdout": "", "stderr": "", "exit_code": 0}

    monkeypatch.setattr(profile, "execute", observed)
    filtered = profile.compiler_report(path)
    assert len(calls) == 2
    assert all(call[1]["timeout"] is None for call in calls)
    assert calls[1][0][1:3] == ["--minimum-duration", "1000"]
    assert filtered["timeline_minimum_duration_us"] == 1000
    assert filtered["timeline_filtered"]
    assert profile.compiler_report(path) == filtered
    assert len(calls) == 2
    monkeypatch.setenv("LCTX_COMPILE_PROFILE_TIMELINE_MIN_US", "0")
    full = profile.compiler_report(path)
    assert len(calls) == 4
    assert calls[3][0][1:3] == ["--minimum-duration", "0"]
    assert full["timeline_minimum_duration_us"] == 0
    assert not full["timeline_filtered"]
    assert profile.compiler_report(path) == full
    assert len(calls) == 4
    assert path.read_bytes() == raw


@pytest.mark.parametrize("value", ["-1", "nan", "1.5"])
def test_invalid_timeline_threshold_refused_before_reader(tmp_path, monkeypatch, value):
    monkeypatch.setenv("LCTX_COMPILE_PROFILE_TIMELINE_MIN_US", value)
    monkeypatch.setattr(
        profile,
        "execute",
        lambda *_a, **_kw: pytest.fail("invalid threshold must not launch reader"),
    )
    with pytest.raises(ValueError, match="TIMELINE_MIN_US must be an integer >= 0"):
        profile.compiler_report(tmp_path / "unread.mm_profdata")


@pytest.mark.parametrize(
    "tail,expected_outcome,partial",
    [
        (b'{"unfinished":', "passed", True),
        (b'{"unfinished":\n', "blocked", False),
        (b'{"broken":\n{"reason":"event","run_id":"owned"}\n', "blocked", False),
        (b'{"reason":"event","run_id":"owned"}', "passed", False),
        (b"42\n", "blocked", False),
        (b'{"reason":"event","run_id":"foreign"}\n{"unfinished":', "blocked", True),
    ],
)
def test_cargo_prefix_recovery_only_drops_unterminated_final_line(
    root, monkeypatch, tail, expected_outcome, partial
):
    destination = root / "profile"
    destination.mkdir()
    home = root / "home"
    (home / "log").mkdir(parents=True)
    command = ["cargo", "build"]
    start = {
        "reason": "build-started",
        "run_id": "owned",
        "workspace_root": str(root),
        "cwd": str(Path.cwd()),
        "target_dir": str(root / "target"),
        "profile": "dev",
        "command": ["/tools/cargo", "build"],
    }
    raw = json.dumps(start).encode() + b"\n" + tail
    (home / "log/owned.jsonl").write_bytes(raw)
    monkeypatch.setattr(
        profile,
        "execute",
        lambda *_a, **_kw: {
            "outcome": "passed",
            "stdout": json.dumps({"workspace_root": str(root)}),
        },
    )
    result = profile.retain_cargo_sessions(
        destination, ["owned"], {"CARGO_HOME": str(home)}, command
    )
    assert result["outcome"] == expected_outcome
    session = result["sessions"][0]
    assert session.get("partial", False) == partial
    assert Path(session["path"]).read_bytes() == raw
    if partial:
        assert session["dropped_tail_bytes"] == len(b'{"unfinished":')
