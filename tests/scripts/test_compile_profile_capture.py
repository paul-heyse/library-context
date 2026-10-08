from __future__ import annotations

import io
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path
from types import SimpleNamespace

import pytest

import compile_profile_capture as capture

WRAPPER = Path(__file__).resolve().parents[2] / "scripts/compile_profile_wrapper.py"


def executable(path: Path, source: str) -> Path:
    path.write_text(f"#!{sys.executable}\n" + source)
    path.chmod(0o755)
    return path


@pytest.fixture
def environment(tmp_path: Path) -> dict[str, str]:
    return {
        **os.environ,
        "LCTX_COMPILE_PROFILE_DIR": str(tmp_path / "capture"),
        "LCTX_COMPILE_PROFILE_FOCUS": json.dumps([str(tmp_path / "crate")]),
        "CARGO_MANIFEST_DIR": str(tmp_path / "crate"),
        "LCTX_COMPILE_PROFILE_MIN_FREE_BYTES": "0",
        "CARGO_MAKEFLAGS": "--jobserver-auth=123,124 -j",
    }


@pytest.fixture
def fake_tools(tmp_path: Path) -> tuple[Path, Path, Path]:
    compiler = executable(
        tmp_path / "rustc",
        """
import json, os, sys, time
from pathlib import Path
fd = os.environ.get("TEST_FD")
Path(os.environ["OBSERVED"]).write_text(json.dumps({
    "argv": sys.argv[1:], "makeflags": os.environ.get("CARGO_MAKEFLAGS"),
    "fd": os.fstat(int(fd)).st_ino if fd else None,
    "rustflags": os.environ.get("RUSTFLAGS"),
}))
print("compiler stdout", flush=True)
print('time: {"pass":"test","time":0.1}', file=sys.stderr, flush=True)
print('{"reason":"compiler-message","message":{"message":"diagnostic"}}', file=sys.stderr)
print("compiler stderr", file=sys.stderr, flush=True)
time.sleep(float(os.environ.get("COMPILER_SLEEP", "0.35")))
raise SystemExit(int(os.environ.get("COMPILER_EXIT", "0")))
""",
    )
    delegate = executable(
        tmp_path / "delegate",
        """
import json, os, sys
from pathlib import Path
fd = os.environ.get("TEST_FD")
Path(os.environ["OBSERVED"]).write_text(json.dumps({
    "argv": sys.argv[1:], "environment": dict(os.environ),
    "fd": os.fstat(int(fd)).st_ino if fd else None,
}))
raise SystemExit(7)
""",
    )
    perf = executable(
        tmp_path / "perf",
        """
import os, signal, sys, time
from pathlib import Path
output = Path(sys.argv[sys.argv.index("-o") + 1])
def stopped(*_):
    if os.environ.get("PERF_SIGNAL_EXIT"):
        active.close()
        completed = output.with_name(output.name + ".202610080001")
        output.rename(completed)
        print(f"[ perf record: Dump {completed} ]", file=sys.stderr, flush=True)
        signal.signal(signal.SIGINT, signal.SIG_DFL)
        os.kill(os.getpid(), signal.SIGINT)
    sys.exit(0)
signal.signal(signal.SIGINT, stopped)
with output.open("wb") as active:
    active.write(b"active")
    active.flush()
    output.with_name(output.name + ".ready").write_text("ready")
    while True:
        time.sleep(0.05)
""",
    )
    return compiler, delegate, perf


def arguments(compiler: Path, crate: str = "cpg_core", test: bool = False) -> list[str]:
    source = "src/lib.rs" if crate == "cpg_core" else f"tests/{crate}.rs"
    return [
        str(compiler),
        "--crate-name",
        crate,
        source,
        "--out-dir",
        "/output",
        "--cfg",
        'feature="native"',
        *(["--test"] if test else []),
    ]


def run_wrapper(
    args: list[str], env: dict[str, str], *, pass_fds: tuple[int, ...] = ()
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(WRAPPER), *args],
        env=env,
        capture_output=True,
        text=True,
        timeout=15,
        pass_fds=pass_fds,
    )


def test_manifest_selection_includes_non_primary_integration(
    environment: dict[str, str], fake_tools: tuple[Path, Path, Path]
) -> None:
    environment.pop("CARGO_PRIMARY_PACKAGE", None)
    args = arguments(fake_tools[0], "compiler_catalog", test=True)[1:]
    assert capture.selected_invocation(args, environment) == args
    environment["CARGO_MANIFEST_DIR"] += "_unselected"
    assert capture.selected_invocation(["@nonexistent"], environment) is None


@pytest.mark.parametrize("probe", ["-vV", "--version", "--print=file-names"])
def test_probes_delegate_original_argv_and_environment(
    tmp_path: Path, environment: dict[str, str], fake_tools: tuple[Path, Path, Path], probe: str
) -> None:
    compiler, delegate, _ = fake_tools
    response = tmp_path / "probe.args"
    response.write_text(probe + "\n")
    observed = tmp_path / "observed.json"
    environment.update(LCTX_COMPILE_PROFILE_DELEGATE=str(delegate), OBSERVED=str(observed))
    result = run_wrapper([str(compiler), "@" + str(response)], environment)
    assert result.returncode == 7
    record = json.loads(observed.read_text())
    assert record["argv"] == [str(compiler), "@" + str(response)]
    assert record["environment"]["CARGO_MAKEFLAGS"] == environment["CARGO_MAKEFLAGS"]
    assert (
        record["environment"]["LCTX_COMPILE_PROFILE_DIR"] == environment["LCTX_COMPILE_PROFILE_DIR"]
    )
    assert not Path(environment["LCTX_COMPILE_PROFILE_DIR"]).exists()


def test_unselected_delegation_preserves_jobserver_descriptor(
    tmp_path: Path, environment: dict[str, str], fake_tools: tuple[Path, Path, Path]
) -> None:
    compiler, delegate, _ = fake_tools
    observed = tmp_path / "observed.json"
    environment.update(
        LCTX_COMPILE_PROFILE_DELEGATE=str(delegate),
        OBSERVED=str(observed),
        CARGO_MANIFEST_DIR=str(tmp_path / "other"),
    )
    read_fd, write_fd = os.pipe()
    try:
        environment["TEST_FD"] = str(read_fd)
        result = run_wrapper([str(compiler), "@missing-file"], environment, pass_fds=(read_fd,))
        assert result.returncode == 7
        assert json.loads(observed.read_text())["fd"] == os.fstat(read_fd).st_ino
    finally:
        os.close(read_fd)
        os.close(write_fd)


def test_newline_argfiles_follow_pinned_single_pass_syntax(tmp_path: Path) -> None:
    response = tmp_path / "arguments"
    response.write_bytes(b"--crate-name\r\ncrate\n\n@nested\nvertical\x0bspace\nlast\r")
    assert capture.expand_response_files(["@" + str(response)]) == [
        "--crate-name",
        "crate",
        "",
        "@nested",
        "vertical\vspace",
        "last\r",
    ]
    response.write_text("")
    assert capture.expand_response_files(["@" + str(response)]) == []


@pytest.mark.parametrize("flag", [["-Zshell-argfiles"], ["-Z", "shell-argfiles"]])
def test_enabled_shell_argfiles_refuse_explicitly(flag: list[str]) -> None:
    with pytest.raises(ValueError, match=r"@shell.*unsupported"):
        capture.expand_response_files([*flag, "@shell:args"])


def test_selected_compiler_bypasses_delegate_retains_flags_fds_and_unique_records(
    tmp_path: Path, environment: dict[str, str], fake_tools: tuple[Path, Path, Path]
) -> None:
    compiler, delegate, perf = fake_tools
    observed = tmp_path / "observed.json"
    environment.update(
        LCTX_COMPILE_PROFILE_DELEGATE=str(delegate),
        OBSERVED=str(observed),
        LCTX_COMPILE_PROFILE_PERF=str(perf),
        RUSTFLAGS="-Copt-level=1",
    )
    read_fd, write_fd = os.pipe()
    try:
        environment["TEST_FD"] = str(read_fd)
        for _ in range(2):
            result = run_wrapper(
                arguments(compiler, "compiler_catalog", True), environment, pass_fds=(read_fd,)
            )
            assert result.returncode == 0, result.stderr
            assert result.stdout == "compiler stdout\n"
            assert '"pass":"test"' in result.stderr
            assert "compiler stderr" in result.stderr
            observation = json.loads(observed.read_text())
            assert observation["fd"] == os.fstat(read_fd).st_ino
            assert observation["makeflags"] == environment["CARGO_MAKEFLAGS"]
            assert observation["rustflags"] == "-Copt-level=1"
            assert observation["argv"][:8] == arguments(compiler, "compiler_catalog", True)[1:9]
    finally:
        os.close(read_fd)
        os.close(write_fd)
    records = list(Path(environment["LCTX_COMPILE_PROFILE_DIR"]).glob("units/*/record.json"))
    assert len(records) == 2
    for path in records:
        record = json.loads(path.read_text())
        assert record["kind"] == "integration_test"
        assert record["phase"] == "completed"
        assert record["exit_code"] == 0
        assert record["uid"] == os.getuid()
        assert record["features"] == ['feature="native"']
        assert record["collector"]["exit_code"] == 0
        assert record["effective_argv"] == record["input_argv"] + record["profile_flags"]
        assert Path(record["sidecars"]["time_passes"]).read_text() == '{"pass":"test","time":0.1}\n'


@pytest.mark.parametrize("failure", ["missing_perf", "low_disk"])
def test_observer_failure_preserves_compiler_exit_and_diagnostics(
    tmp_path: Path, environment: dict[str, str], fake_tools: tuple[Path, Path, Path], failure: str
) -> None:
    compiler, _, perf = fake_tools
    environment.update(
        OBSERVED=str(tmp_path / "observed.json"),
        COMPILER_EXIT="19",
        LCTX_COMPILE_PROFILE_PERF=str(perf),
    )
    if failure == "missing_perf":
        environment["LCTX_COMPILE_PROFILE_PERF"] = str(tmp_path / "not-installed")
    else:
        environment["LCTX_COMPILE_PROFILE_MIN_FREE_BYTES"] = str(2**63)
    result = run_wrapper(arguments(compiler), environment)
    assert result.returncode == 19
    assert "compiler stderr" in result.stderr
    path = next(Path(environment["LCTX_COMPILE_PROFILE_DIR"]).glob("units/*/record.json"))
    record = json.loads(path.read_text())
    assert record["exit_code"] == 19
    assert record["collector"]["status"] in ("failed", "blocked")


def test_sampler_identity_and_cancellation_never_signal_target(
    tmp_path: Path, fake_tools: tuple[Path, Path, Path]
) -> None:
    target = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(15)"])
    sampler = None
    try:
        identity = capture.process_identity(target.pid)
        with pytest.raises(ValueError, match="does not match"):
            capture.start_sampler(
                tmp_path, target.pid, {**identity, "start_time": "invalid"}, str(fake_tools[2])
            )
        sampler = capture.start_sampler(tmp_path, target.pid, identity, str(fake_tools[2]))
        deadline = time.monotonic() + 5
        while not (tmp_path / "perf.data.ready").exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        (tmp_path / "perf.data.20261008").write_bytes(b"completed")
        chunks = capture.read_completed_chunks(tmp_path)
        assert tmp_path / "perf.data" not in chunks
        assert tmp_path / "perf.data.20261008" in chunks
        assert capture.stop_sampler(sampler) == 0
        assert target.poll() is None
        assert tmp_path / "perf.data" in capture.read_completed_chunks(tmp_path)
    finally:
        if sampler is not None:
            capture.stop_sampler(sampler)
        target.terminate()
        target.wait(timeout=5)


def test_doctor_policy_two_needs_no_capability(monkeypatch: pytest.MonkeyPatch) -> None:
    original_read = Path.read_text
    monkeypatch.setattr(
        Path,
        "read_text",
        lambda path, *args, **kwargs: (
            "2"
            if str(path).endswith("perf_event_paranoid")
            else original_read(path, *args, **kwargs)
        ),
    )
    monkeypatch.setattr(capture.shutil, "which", lambda _: "/usr/bin/perf")
    monkeypatch.setattr(
        capture.subprocess,
        "run",
        lambda args, **kwargs: subprocess.CompletedProcess(
            args, 0, "perf version 7.0" if "--version" in args else "", ""
        ),
    )
    result = capture.doctor("/usr/bin/perf")
    assert result["status"] == "ready"
    assert result["attach_test"] == "not_run"


def test_wrapper_cancellation_stops_observer_without_stopping_compiler(
    tmp_path: Path, environment: dict[str, str], fake_tools: tuple[Path, Path, Path]
) -> None:
    compiler, _, perf = fake_tools
    environment.update(
        OBSERVED=str(tmp_path / "observed.json"),
        COMPILER_SLEEP="1.5",
        LCTX_COMPILE_PROFILE_PERF=str(perf),
    )
    wrapper = subprocess.Popen(
        [sys.executable, str(WRAPPER), *arguments(compiler)],
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        root = Path(environment["LCTX_COMPILE_PROFILE_DIR"])
        deadline = time.monotonic() + 5
        ready = []
        while not ready and time.monotonic() < deadline:
            ready = list(root.glob("units/*/perf.data.ready"))
            time.sleep(0.02)
        assert ready
        path = ready[0].parent / "record.json"
        record = json.loads(path.read_text())
        wrapper.send_signal(signal.SIGTERM)
        time.sleep(0.2)
        assert capture.identity_matches(record["pid"], record)
        assert wrapper.poll() is None
        wrapper.communicate(timeout=5)
        assert wrapper.returncode == 0
        record = json.loads(path.read_text())
        assert record["collector"]["reason"] == "cancelled"
        assert record["exit_code"] == 0
    finally:
        if wrapper.poll() is None:
            wrapper.kill()
            wrapper.communicate(timeout=5)


def test_completed_announcements_work_with_private_collector_fds(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    identity = capture.process_identity(os.getpid())
    capture.write_json(
        tmp_path / "collector.json", {"pid": os.getpid(), "identity": identity, "phase": "running"}
    )
    announced = tmp_path / "perf.data.202610080001"
    unannounced = tmp_path / "perf.data.202610080002"
    for path in (announced, unannounced, tmp_path / "perf.data"):
        path.write_bytes(b"capture")
    (tmp_path / "collector.log").write_text(f"[ perf record: Dump {announced} ]\n")
    original_iterdir = Path.iterdir

    def private_fd(path: Path):
        if path == Path(f"/proc/{os.getpid()}/fd"):
            raise PermissionError("CAP_PERFMON process fd directory is private")
        return original_iterdir(path)

    monkeypatch.setattr(Path, "iterdir", private_fd)
    assert capture.read_completed_chunks(tmp_path) == [announced]


def test_abrupt_collector_exit_does_not_prove_finalization(tmp_path: Path) -> None:
    capture.write_json(
        tmp_path / "collector.json",
        {
            "pid": os.getpid(),
            "identity": {"pid": os.getpid(), "uid": os.getuid(), "start_time": "previous-process"},
            "phase": "completed",
            "exit_code": -9,
        },
    )
    base = tmp_path / "perf.data"
    rotated = tmp_path / "perf.data.202610080001"
    base.write_bytes(b"incomplete")
    rotated.write_bytes(b"complete")
    assert capture.read_completed_chunks(tmp_path) == []
    (tmp_path / "collector.log").write_text(f"[ perf record: Dump {rotated} ]\n")
    assert capture.read_completed_chunks(tmp_path) == [rotated]


def test_timing_tee_strips_only_pinned_prefix_and_keeps_raw_diagnostics(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    stderr = io.BytesIO()
    monkeypatch.setattr(capture.sys, "stderr", SimpleNamespace(buffer=stderr))
    data = (
        b'time: {"pass":"codegen","time":2.0}\n'
        b'{"reason":"compiler-message","message":"diagnostic"}\n'
        b'{"pass":"unprefixed","time":3.0}\n'
        b"time: ordinary diagnostic text\n"
    )
    errors: list[str] = []
    capture._tee_stderr(io.BytesIO(data), tmp_path / "stderr", tmp_path / "times", errors)
    assert not errors
    assert stderr.getvalue() == data
    assert (tmp_path / "stderr").read_bytes() == data
    assert (tmp_path / "times").read_bytes() == b'{"pass":"codegen","time":2.0}\n'


def test_graceful_perf_signal_exit_requires_completed_output(
    tmp_path: Path, environment: dict[str, str], fake_tools: tuple[Path, Path, Path]
) -> None:
    compiler, _, perf = fake_tools
    environment.update(
        OBSERVED=str(tmp_path / "observed.json"),
        PERF_SIGNAL_EXIT="1",
        LCTX_COMPILE_PROFILE_PERF=str(perf),
    )
    result = run_wrapper(arguments(compiler), environment)
    assert result.returncode == 0
    path = next(Path(environment["LCTX_COMPILE_PROFILE_DIR"]).glob("units/*/record.json"))
    record = json.loads(path.read_text())
    assert record["collector"]["exit_code"] == -signal.SIGINT
    assert record["collector"]["status"] == "passed"
    assert capture.sampler_succeeded(path.parent, -signal.SIGINT)
    empty = tmp_path / "empty-unit"
    empty.mkdir()
    assert not capture.sampler_succeeded(empty, -signal.SIGINT)


def test_storage_failure_runs_original_compiler_with_original_jobserver(
    tmp_path: Path, environment: dict[str, str], fake_tools: tuple[Path, Path, Path]
) -> None:
    compiler, _, _ = fake_tools
    blocked_root = tmp_path / "not-a-directory"
    blocked_root.write_text("preserved")
    observed = tmp_path / "observed.json"
    environment.update(
        OBSERVED=str(observed), COMPILER_EXIT="19", LCTX_COMPILE_PROFILE_DIR=str(blocked_root)
    )
    read_fd, write_fd = os.pipe()
    try:
        environment["TEST_FD"] = str(read_fd)
        result = run_wrapper(arguments(compiler), environment, pass_fds=(read_fd,))
        assert result.returncode == 19
        observation = json.loads(observed.read_text())
        assert observation["argv"] == arguments(compiler)[1:]
        assert observation["fd"] == os.fstat(read_fd).st_ino
        assert "compiler stderr" in result.stderr
        assert "storage unavailable" in result.stderr
        assert blocked_root.read_text() == "preserved"
    finally:
        os.close(read_fd)
        os.close(write_fd)


@pytest.mark.parametrize(
    "source,expected",
    [
        ("src/lib.rs", "unit_test"),
        ("src/main.rs", "unit_test"),
        ("tests/library.rs", "integration_test"),
    ],
)
def test_absolute_test_source_classifies_target_kind(
    environment: dict[str, str], source: str, expected: str
) -> None:
    source_path = str(Path(environment["CARGO_MANIFEST_DIR"]) / source)
    args = ["--crate-name", "ordinary_package", source_path, "--test"]
    assert capture.invocation_kind(args, environment) == expected


@pytest.mark.parametrize("failure", ["log_open", "timing_open", "write"])
def test_telemetry_storage_errors_always_drain_and_forward_stderr(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, failure: str
) -> None:
    stderr = io.BytesIO()
    monkeypatch.setattr(capture.sys, "stderr", SimpleNamespace(buffer=stderr))
    raw = (b'compiler diagnostic\ntime: {"pass":"codegen","time":2.0}\n') * 1000
    stream = io.BytesIO(raw)
    log = tmp_path / "stderr"
    timing = tmp_path / "timing"
    original_open = Path.open

    class FullDisk(io.BytesIO):
        def write(self, data):
            raise OSError("storage is full")

    def failed_open(path: Path, *args, **kwargs):
        if (failure == "log_open" and path == log) or (failure == "timing_open" and path == timing):
            raise OSError("storage unavailable")
        if failure == "write" and path == log:
            return FullDisk()
        return original_open(path, *args, **kwargs)

    monkeypatch.setattr(Path, "open", failed_open)
    errors: list[str] = []
    capture._tee_stderr(stream, log, timing, errors)
    assert errors
    assert stream.closed
    assert stderr.getvalue() == raw


def test_killed_observer_owner_finalizes_sampler_without_signalling_target(
    tmp_path: Path, fake_tools: tuple[Path, Path, Path]
) -> None:
    target = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(20)"])
    unit = tmp_path / "unit"
    owner = subprocess.Popen(
        [
            sys.executable,
            "-c",
            "import compile_profile_capture as c,sys,time; from pathlib import Path; "
            "pid=int(sys.argv[2]); p=c.start_sampler(Path(sys.argv[1]),pid,"
            "c.process_identity(pid),sys.argv[3]); print(p.pid,flush=True); time.sleep(20)",
            str(unit),
            str(target.pid),
            str(fake_tools[2]),
        ],
        env={**os.environ, "PYTHONPATH": str(WRAPPER.parent), "PERF_SIGNAL_EXIT": "1"},
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        deadline = time.monotonic() + 5
        while not (unit / "perf.data.ready").exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        assert (unit / "perf.data.ready").exists()
        before = json.loads((unit / "collector.json").read_text())
        owner.kill()
        owner.communicate(timeout=5)
        record = before
        deadline = time.monotonic() + 5
        while record["phase"] != "completed" and time.monotonic() < deadline:
            time.sleep(0.02)
            record = json.loads((unit / "collector.json").read_text())
        assert record["phase"] == "completed"
        assert record["reason"] == "owner_exit"
        assert record["exit_code"] == -signal.SIGINT
        assert not capture.identity_matches(record["recorder_pid"], record["recorder_identity"])
        assert target.poll() is None
        assert capture.read_completed_chunks(unit)
    finally:
        if owner.poll() is None:
            owner.kill()
            owner.communicate(timeout=5)
        target.terminate()
        target.wait(timeout=5)
