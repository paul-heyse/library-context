"""Diagnostic placement, relocation and actual selected-reader replay controls."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import signal
import subprocess
import time
from pathlib import Path

import pytest

import compile_profile as profile
import compile_profile_capture as capture
import compile_profile_tools as tools
import storage_archive
from harness import write_json_atomic


@pytest.fixture(autouse=True)
def isolated_reader_generations(tmp_path_factory, monkeypatch):
    monkeypatch.setenv(
        "LCTX_COMPILE_PROFILE_READERS_ROOT",
        str(tmp_path_factory.getbasetemp() / "reader-generations"),
    )


def test_unavailable_selected_root_blocks_before_instrumentation(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(
        profile, "doctor", lambda **_: pytest.fail("selected destination must be checked first")
    )
    args = argparse.Namespace(
        command=["cargo", "build"],
        diagnostic_root=str(tmp_path / "missing"),
        focus="cpg-core",
        background=False,
        label=None,
        json=True,
    )
    assert profile.cmd_record(args) == 75
    assert json.loads(capsys.readouterr().out)["outcome"] == "blocked"


def test_advisories_do_not_limit_compiler_settings(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_STORAGE_RECORDING_ADVISORY_BYTES", str(2**64))
    before = os.environ.copy()
    data = profile.capacity_observation(tmp_path)
    assert data["advisory"]
    assert "self-profile" in data["support_limit"]
    assert os.environ == before
    assert "-Zself-profile-events=default,args,llvm" in capture.profile_flags(tmp_path)


def test_closed_chunk_resolver_translates_only_recorded_root(tmp_path):
    original = tmp_path / "old" / "compile-profile"
    relocated = tmp_path / "new" / "compile-profile"
    unit = relocated / "units" / "a"
    unit.mkdir(parents=True)
    write_json_atomic(relocated / "record.json", {"original_profile_root": str(original)})
    write_json_atomic(
        unit / "collector.json",
        {
            "pid": 999999999,
            "identity": {"pid": 999999999, "uid": os.getuid(), "start_time": 1},
            "phase": "interrupted",
            "exit_code": -9,
        },
    )
    (unit / "perf.data.123").write_bytes(b"closed bytes")
    log = f"[ perf record: Dump {original / 'units/a/perf.data.123'} ]\n"
    (unit / "collector.log").write_text(log)
    assert capture.read_completed_chunks(unit) == [unit / "perf.data.123"]
    assert (unit / "collector.log").read_text() == log
    assert profile.resolve_recorded_path(relocated, str(original / "units/a")) == unit
    assert profile.resolve_recorded_path(relocated, "/foreign/a") == Path("/foreign/a")


def test_derived_report_scope_is_separate(tmp_path):
    raw = tmp_path / "compile-profile" / "units" / "unit" / "raw.mm_profdata"
    assert profile.report_location(raw) == tmp_path / "compile-profile-reports" / "unit"


def test_actual_compiler_capture_archive_replays_without_original_location(tmp_path, monkeypatch):
    readiness = tools.check()
    if readiness["status"] != "passed":
        pytest.skip("blocked: exact measureme readers unavailable: " + str(readiness["errors"]))
    original = tmp_path / "original"
    unit = original / "units" / "actual"
    unit.mkdir(parents=True)
    source = tmp_path / "input.rs"
    source.write_text(
        "fn main() { let values: Vec<_> = (0..100).map(|x| x*x).collect(); "
        'println!("{:?}", values); }'
    )
    command = [
        "rustc",
        str(source),
        "--crate-name",
        "storage_replay",
        "-o",
        str(tmp_path / "program"),
        f"-Zself-profile={unit}",
        "-Zself-profile-events=default,args,llvm",
    ]
    result = subprocess.run(command, cwd=profile.ROOT, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    write_json_atomic(unit / "record.json", {"phase": "completed", "exit_code": 0})
    write_json_atomic(
        original / "record.json",
        {"original_profile_root": str(original), "storage_lifecycle_version": 1},
    )
    monkeypatch.setenv("LCTX_COMPILE_PROFILE_TIMELINE_MIN_US", "0")
    path = tmp_path / "capture.tar.zst"
    archived = profile.archive_profile(original, path, ("compiler-summary", "compiler-timeline"))
    write_json_atomic(tmp_path / "archive-qualification.json", archived)
    assert archived["outcome"] == "passed", archived
    assert archived["replay_qualified"]
    # Only disposable originals are removed; readers must resolve the archive's local copies.
    shutil.rmtree(original)
    source.unlink()
    (tmp_path / "program").unlink()
    restored = tmp_path / "restored"
    storage_archive.restore_archive(path, restored)
    monkeypatch.setattr(tools, "VERSION", "future-version")
    monkeypatch.setattr(tools, "tools_root", lambda: tmp_path / "unavailable-current-tools")
    replay = profile.replay_directory(restored)
    assert replay["capabilities"]["compiler-summary"]["outcome"] == "passed", replay
    assert replay["capabilities"]["compiler-timeline"]["outcome"] == "passed", replay
    assert replay["capabilities"]["source-annotation"]["outcome"] == "blocked"
    # Partial ownership does not upgrade interrupted capture state, even with readable raw.
    record = json.loads((restored / "units/actual/record.json").read_text())
    record.update(phase="interrupted", exit_code=-15)
    write_json_atomic(restored / "units/actual/record.json", record)
    interrupted = profile.replay_directory(restored)
    assert interrupted["partial"]
    assert interrupted["capabilities"]["compiler-summary"]["outcome"] == "passed"


@pytest.mark.parametrize("interrupted", [False, True])
@pytest.mark.parametrize("static", [False, True])
def test_actual_perf_symbols_and_disassembly_survive_original_binary_removal(
    tmp_path, interrupted, static, monkeypatch
):
    if not shutil.which("cc"):
        pytest.skip("blocked: C compiler needed for disposable native replay control")
    original = tmp_path / "original"
    unit = original / "units" / "sampled"
    unit.mkdir(parents=True)
    source = tmp_path / "spin.c"
    source.write_text("""#include <time.h>
#include <stdio.h>
volatile unsigned long sink;
__attribute__((noinline)) void storage_spin(void) {
    for(unsigned long i=0;i<1000000;i++) sink += i;
}
int main(int argc, char **argv) {
    FILE *ready=fopen(argv[1], "w"); fputs("ready", ready); fclose(ready);
    clock_t start=clock();
    while((double)(clock()-start)/CLOCKS_PER_SEC<0.8) storage_spin();
    return 0;
}
""")
    binary = tmp_path / "spin"
    built = subprocess.run(
        [
            "cc",
            "-O2",
            "-g",
            "-Wl,--build-id",
            *(["-static"] if static else []),
            str(source),
            "-o",
            str(binary),
        ],
        capture_output=True,
        text=True,
    )
    assert built.returncode == 0, built.stderr
    chunk = unit / "perf.data"
    command = [
        capture.resolve_perf(),
        "record",
        "-e",
        "cycles:u",
        "-F",
        "199",
        "-g",
        "-o",
        str(chunk),
    ]
    if interrupted:
        command += ["--switch-output=signal"]
    ready = tmp_path / "sampled-program-ready"
    command += ["--", str(binary), str(ready)]
    child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    identity = capture.process_identity(child.pid)
    if interrupted:
        while not ready.exists() and child.poll() is None:
            time.sleep(0.01)
        time.sleep(0.2)
        if child.poll() is None:
            child.send_signal(signal.SIGUSR2)
            time.sleep(0.15)
        if child.poll() is None:
            child.send_signal(signal.SIGINT)
    stdout, stderr = child.communicate()
    recorded = subprocess.CompletedProcess(command, child.returncode, stdout, stderr)
    if recorded.returncode and not (interrupted and "[ perf record: Dump " in recorded.stderr):
        pytest.skip("blocked: actual perf capture prerequisite: " + recorded.stderr)
    (unit / "collector.log").write_text(recorded.stderr)
    write_json_atomic(
        unit / "record.json",
        {
            **identity,
            "phase": "interrupted" if interrupted else "completed",
            "exit_code": recorded.returncode,
        },
    )
    write_json_atomic(
        unit / "collector.json",
        {
            "phase": "completed",
            "exit_code": recorded.returncode,
            "pid": identity["pid"],
            "identity": identity,
        },
    )
    write_json_atomic(
        original / "record.json",
        {"original_profile_root": str(original), "storage_lifecycle_version": 1},
    )
    assert capture.read_completed_chunks(unit), recorded.stderr
    prepared = profile.prepare_symbol_closure(
        original, source_files=(source,), expected_symbols=("storage_spin",)
    )
    assert prepared["outcome"] == "passed", prepared
    path = tmp_path / "sampled.tar.zst"
    result = profile.archive_profile(
        original, path, ("sampled-symbolized-report", "disassembly", "source-annotation")
    )
    write_json_atomic(tmp_path / "archive-qualification.json", result)
    assert result["outcome"] == "passed", result
    assert result["replay"]["isolation"]["outcome"] == "passed"
    shutil.rmtree(original)
    binary.unlink()
    source.unlink()
    restored = tmp_path / "restored"
    storage_archive.restore_archive(path, restored)
    # Replay owns an exact generation and does not depend on future PATH selection.
    monkeypatch.setattr(capture, "resolve_perf", lambda: str(tmp_path / "unavailable-current-perf"))
    real_which = shutil.which
    monkeypatch.setattr(
        shutil, "which", lambda name: None if name in {"objdump", "samply"} else real_which(name)
    )
    replay = profile.replay_directory(restored)
    assert replay["partial"] is interrupted
    assert replay["capabilities"]["sampled-symbolized-report"]["outcome"] == "passed", replay
    assert replay["capabilities"]["disassembly"]["outcome"] == "passed", replay
    assert replay["capabilities"]["source-annotation"]["outcome"] == "passed", replay
    assert replay["capabilities"]["sampled-symbolized-import"]["outcome"] == "blocked", replay


def test_restore_has_fresh_owner_and_temporary_raw_lifetime(tmp_path, monkeypatch):
    import runs
    from storage_lifecycle import Storage

    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "runs"))
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "state"))
    source = tmp_path / "original"
    source.mkdir()
    write_json_atomic(
        source / "record.json",
        {"original_profile_root": str(source), "storage_lifecycle_version": 1},
    )
    (source / "raw").write_bytes(b"exact original bytes")
    archive = tmp_path / "capture.tar.zst"
    storage_archive.create_archive(source, archive)
    restored = tmp_path / "restored"
    result = profile.restore_profile(archive, restored)
    assert result["outcome"] == "passed"
    raw = Storage().get(result["raw_id"])
    assert raw["path"] == str(restored)
    obligation = next(iter(raw["obligations"].values()))
    assert obligation["temporary_days"] == 14
    assert obligation["until"] is not None
    assert (restored / "raw").read_bytes() == b"exact original bytes"
    assert profile.profile_dir(result["run"]) == restored
    assert runs.view(Path(result["run_dir"]))["cleanup"]["status"] == "confirmed"


def test_external_capture_completes_after_run_cleanup(tmp_path, monkeypatch):
    import sys

    import runs
    from storage_lifecycle import Storage

    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "runs"))
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "state"))
    selected = tmp_path / "diagnostics"
    selected.mkdir()
    launched = subprocess.run(
        [
            sys.executable,
            str(Path(runs.__file__)),
            "run",
            "--json",
            "--",
            sys.executable,
            str(profile.SCRIPT),
            "_record",
            "--diagnostic-root",
            str(selected),
            "--focus",
            "cpg-core",
            "--",
            sys.executable,
            "-c",
            "print('external capture')",
        ],
        capture_output=True,
        text=True,
    )
    assert launched.returncode == 0, launched.stderr
    records = Storage().records()
    raw = next(row for row in records if row["category"] == "profile-raw")
    assert Path(raw["path"]).is_relative_to(selected)
    assert next(iter(raw["obligations"].values()))["until"] is not None
    run_dir = Path(raw["owner"]["path"])
    assert runs.view(run_dir)["cleanup"]["status"] == "confirmed"
    assert profile.profile_dir(str(run_dir)) == Path(raw["path"])


def test_missing_isolation_prerequisite_blocks_and_preserves_original(tmp_path, monkeypatch):
    original = tmp_path / "original"
    original.mkdir()
    (original / "capture").write_bytes(b"preserved")
    real_which = shutil.which
    monkeypatch.setattr(shutil, "which", lambda name: None if name == "bwrap" else real_which(name))
    result = profile.archive_profile(original, tmp_path / "capture.tar.zst", ("compiler-summary",))
    assert result["outcome"] == "blocked"
    assert "bubblewrap unavailable" in result["replay"]["reason"]
    assert not (tmp_path / "capture.tar.zst").exists()
    assert (original / "capture").read_bytes() == b"preserved"


def test_recorded_reader_generation_is_native_owned_and_protected(tmp_path):
    from storage_lifecycle import Storage

    binding = tools.prepare_reader_generation(("sampled-symbolized-report", "disassembly"))
    assert tools.reader_binding_valid(binding)
    assert Path(binding["readers"]["perf"]["path"]) != Path(capture.resolve_perf())
    row = next(row for row in Storage().records() if row["path"] == binding["generation_root"])
    assert row["category"] == "native-content"
    assert row["owner"]["kind"] == "native"
    assert Storage().plan([row["id"]])["dispositions"][0]["disposition"] == "external"


def test_restore_obligations_use_current_policy_durations(tmp_path, monkeypatch):
    import storage_lifecycle

    store = storage_lifecycle.Storage()
    store.policy["categories"]["profile-raw"]["temporary_days"] = 2
    store.policy["categories"]["run-receipt"]["temporary_days"] = 17
    monkeypatch.setattr(storage_lifecycle, "Storage", lambda **_: store)
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "runs"))
    source = tmp_path / "source"
    source.mkdir()
    (source / "raw").write_bytes(b"immutable raw")
    archive = tmp_path / "capture.tar.zst"
    storage_archive.create_archive(source, archive)
    result = profile.restore_profile(archive, tmp_path / "restored")
    raw = store.get(result["raw_id"])
    assert next(iter(raw["obligations"].values()))["temporary_days"] == 2
    receipt = next(row for row in store.records() if row["category"] == "run-receipt")
    assert next(iter(receipt["obligations"].values()))["temporary_days"] == 17
    assert profile.lifecycle_components(Path(result["run_dir"]))[0]["temporary_days"] == 2
