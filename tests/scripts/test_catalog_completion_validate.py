"""Selected validation planning and ownership controls; no database or compiler is launched."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from typing import ClassVar

import pytest

import catalog_completion_validate as validation


@pytest.fixture
def isolated(tmp_path, monkeypatch):
    # The managed parent protects tmp_path; inert children use their own storage state.
    monkeypatch.delenv("LCTX_STORAGE_ADMISSION", raising=False)
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "storage"))
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "runs"))
    monkeypatch.delenv("LCTX_RUN_DIR", raising=False)
    monkeypatch.delenv("LCTX_RUN_ID", raising=False)
    monkeypatch.setattr(validation, "STATE", tmp_path / "campaigns")
    run = tmp_path / "aggregate"
    run.mkdir()
    return validation.Validation(run)


def test_print_is_read_only_and_contains_exact_remaining_scope(monkeypatch, capsys):
    def forbidden(*_args, **_kwargs):
        raise AssertionError("preview must not create owners or launch commands")

    monkeypatch.setattr(validation, "Validation", forbidden)
    monkeypatch.setattr(validation.runs, "new_run_dir", forbidden)
    monkeypatch.setattr(validation, "spawn_group", forbidden)
    monkeypatch.setattr(validation, "Storage", forbidden)
    assert validation.main(["--print", "--json"]) == 0
    plan = json.loads(capsys.readouterr().out)
    names = [step["name"] for step in plan]
    assert names[:3] == ["build-cli", "build-controls", "ready"]
    assert names.index("epoch-dispose") < names.index("pilot-produce")
    assert names.index("pilot-produce") < names.index("pilot-check")
    assert all(
        "--release" in step["command"]
        for step in plan
        if step["name"] in {"workspace-check", "clippy"}
    )
    configs = {step["service_config"] for step in plan if step["name"].endswith("-install")}
    assert len(configs) == 5 and str(validation.RETAINED) not in configs
    assert all(
        "maintenance" in step["command"]
        for step in plan
        if step["name"] in {"document-lexical", "analyzer", "epoch"}
    )
    assert "--run-ignored" in next(s for s in plan if s["name"] == "epoch")["command"]
    assert "--retain-serving" in next(s for s in plan if s["name"] == "pilot-produce")["command"]
    assert "--serving" in next(s for s in plan if s["name"] == "pilot-check")["command"]
    assert not any("qualify" in s["command"] or "test-all" in s["command"] for s in plan)
    assert all(
        "--no-fail-fast" in batch.command and "--no-tests=fail" in batch.command
        for batch in validation.BATCHES
    )


@pytest.mark.parametrize(
    ("name", "selection"),
    [("document-lexical", validation.MUTATIONS), ("analyzer", validation.ANALYZER)],
)
def test_publication_mutations_select_the_ignored_publisher_unit_tests(name, selection):
    batch = next(batch for batch in validation.BATCHES if batch.name == name)
    command = batch.command
    assert command[command.index("-p") + 1] == "lctx-publisher"
    assert "--lib" in command and "--test" not in command
    assert command[command.index("--run-ignored") + 1] == "only"
    assert command[command.index("-E") + 1] == selection
    assert "--no-tests=fail" in command
    # Cold restore remains the publication integration target, separately from mutations.
    cold = next(batch for batch in validation.BATCHES if batch.name == "cold").command
    assert cold[cold.index("--test") + 1] == "publication" and "--lib" not in cold


def test_children_remove_inherited_native_authority_and_preserve_run_and_user_bus():
    source = {
        "LCTX_RUN_DIR": "/managed/run",
        "LCTX_RUN_ID": "run",
        "INSTA_UPDATE": "always",
        "LCTX_SURREAL_SERVICE_CONFIG": "/operator/installation.json",
        "LCTX_SURREAL_INSTALLER_CONFIG": "/private/root.json",
        "LCTX_SURREAL_MAINTENANCE_TOKEN": "secret",
        "LCTX_FIXTURE_ATTACHMENT": "stale",
        "LCTX_NATIVE_SERVING_CONFIG": "/operator/viewer.json",
        "LCTX_NATIVE_DRAIN_DIRECTORY": "/old/drain",
        "LCTX_COMPILER_RUNTIME_CONFIG": "/old/runtime",
        "LCTX_RETAIN_NATIVE_FIXTURE_CONFIG": "/old/viewer",
        "DBUS_SESSION_BUS_ADDRESS": "unix:path=/explicit/bus",
    }
    plain = validation.child_environment(source, None)
    assert plain["LCTX_RUN_DIR"] == "/managed/run" and plain["LCTX_RUN_ID"] == "run"
    assert plain["INSTA_UPDATE"] == "no"
    assert not any(
        key.startswith(("LCTX_SURREAL_", "LCTX_FIXTURE_", "LCTX_NATIVE_")) for key in plain
    )
    selected = validation.child_environment(source, "/new/installation.json")
    assert selected["LCTX_SURREAL_SERVICE_CONFIG"] == "/new/installation.json"
    assert selected["DBUS_SESSION_BUS_ADDRESS"] == "unix:path=/explicit/bus"
    assert selected["XDG_RUNTIME_DIR"] == f"/run/user/{os.getuid()}"


def test_real_child_failure_and_success_have_distinct_retained_run_receipts(isolated):
    assert not isolated.execute("build-cli", [sys.executable, "-c", "print('partial');exit(7)"])
    assert isolated.execute("build-controls", [sys.executable, "-c", "print('independent')"])
    first, second = isolated.row("build-cli"), isolated.row("build-controls")
    assert first["run"] != second["run"]
    assert first["exit"] == {"code": 7} and first["launcher_exit"] == 7
    assert first["cleanup"]["status"] == first["launcher_cleanup"]["status"] == "confirmed"
    assert Path(first["log"]).read_text() == "partial\n"
    assert Path(second["log"]).read_text() == "independent\n"
    summary = json.loads((isolated.run / "summary.json").read_text())
    assert summary["steps"][0]["outcome"] == "failed"
    assert (isolated.run / "summary.md").is_file()
    record = validation.runs.load_record(Path(first["run"]))
    assert record is not None
    receipt = record["storage"]["receipt"]
    assert isolated.consumer in validation.Storage().get(receipt)["obligations"]


def test_inert_steps_under_real_managed_parent_settle_delegated_ownership(isolated):
    parent = validation.runs.new_run_dir()
    program = (
        f"import sys;sys.path.insert(0,{str(validation.ROOT / 'scripts')!r});"
        "import runs;from catalog_completion_validate import Validation;"
        "v=Validation(runs.current_run());"
        "assert not v.execute('build-cli',[sys.executable,'-c',\"print('failure');exit(7)\"]);"
        "assert v.execute('build-controls',[sys.executable,'-c',\"print('success')\"])"
    )
    owner = validation.runs.Owner(
        parent,
        [sys.executable, "-c", program],
        label="inert-parent",
        cwd=str(validation.ROOT),
        mode="foreground",
        stream=False,
    )
    assert owner.run() == 0
    assert validation.runs.host_cleanup(parent)["status"] == "confirmed"
    summary = json.loads((parent / "summary.json").read_text())
    first, second = summary["steps"][:2]
    assert first["outcome"] == "failed" and second["outcome"] == "passed"
    assert all(step["cleanup"]["status"] == "confirmed" for step in (first, second))
    assert all(Path(step["run"]) != parent for step in (first, second))


def install_record(config: Path, identity="80029a30-3dd0-4203-9bb0-9a3c11ebd2d5"):
    config.parent.mkdir(parents=True)
    config.write_text(
        json.dumps(
            {
                "installation_id": identity,
                "kind": "campaign",
                "purpose": "surrealdb-lifecycle",
                "state_root": str(config.parent),
                "binary": {"generation": {"identity": validation.SERVER.parent.name}},
            }
        )
    )


@pytest.mark.parametrize("install_passed", [False, True])
def test_failed_logical_batch_still_disposes_exact_target_and_participants(
    isolated,
    monkeypatch,
    install_passed,
):
    import surrealdb_service

    called = []
    participants = []

    def execute(name, command=None, **_kwargs):
        called.append((name, command))
        if name.endswith("-dispose"):
            assert command is not None
            selectors = command[5::2]
            resolved = [validation.runs.resolve(selector) for selector in selectors]
            assert resolved == participants
            references = [surrealdb_service._campaign_participant_run(path) for path in resolved]
            assert [row["run_path"] for row in references] == [str(path) for path in participants]
            return True
        path = validation.runs.new_run_dir()
        owner = validation.runs.Owner(
            path,
            [sys.executable, "-c", "raise SystemExit(0)"],
            label=name,
            cwd=str(validation.ROOT),
            mode="foreground",
            stream=False,
        )
        assert owner.run() == 0
        participants.append(path)
        isolated.row(name).update(run=str(path), log=str(path / "output.log"), outcome="failed")
        if name == "composition-install":
            install_record(Path(isolated.row(name)["service_config"]))
            return install_passed
        return False

    monkeypatch.setattr(isolated, "execute", execute)
    assert isolated.batch(validation.BATCHES[0], True) == validation.BatchResult(False, True)
    name, command = called[-1]
    assert name == "composition-dispose"
    assert command is not None
    assert command[:4] == [
        "just",
        "service",
        "dispose-campaign",
        "80029a30-3dd0-4203-9bb0-9a3c11ebd2d5",
    ]
    expected = [item for path in participants for item in ("--participant-run", path.name)]
    assert command[4:] == expected
    assert validation.RETAINED_ID not in command


def test_existing_campaign_is_never_reused_or_disposed(isolated, monkeypatch):
    config = Path(isolated.row("cold-install")["service_config"])
    config.parent.mkdir(parents=True)
    monkeypatch.setattr(isolated, "execute", lambda *_a, **_k: pytest.fail("must refuse reuse"))
    assert isolated.batch(validation.BATCHES[1], True) == validation.BatchResult(False, False)
    assert isolated.row("cold-dispose")["outcome"] == "blocked"


def test_target_b_identity_is_never_disposable(tmp_path):
    config = tmp_path / "installation.json"
    config.parent.mkdir(exist_ok=True)
    # A fresh-looking path must not provide a way to dispose the retained UUID.
    config.write_text(
        json.dumps(
            {
                "installation_id": validation.RETAINED_ID,
                "kind": "campaign",
                "purpose": "surrealdb-lifecycle",
                "state_root": str(config.parent),
                "binary": {"generation": {"identity": validation.SERVER.parent.name}},
            }
        )
    )
    with pytest.raises(ValueError, match="target B"):
        validation.campaign_identity(config)


def test_cancellation_retains_campaign_without_new_disposal(isolated, monkeypatch):
    called = []

    def execute(name, *_args, **_kwargs):
        called.append(name)
        isolated.row(name)["run"] = f"/runs/{name}"
        isolated.row(name)["log"] = f"/runs/{name}/output.log"
        if name.endswith("-install"):
            install_record(Path(isolated.row(name)["service_config"]))
            return True
        isolated.stopped = True
        raise InterruptedError("cancelled")

    monkeypatch.setattr(isolated, "execute", execute)
    with pytest.raises(InterruptedError):
        isolated.batch(validation.BATCHES[0], True)
    assert called == ["composition-install", "composition"]
    assert isolated.row("composition-dispose")["outcome"] == "not_run"
    assert "retained" in isolated.row("composition-dispose")["reason"]


def test_independent_batches_and_quality_continue_but_pilot_is_blocked(isolated, monkeypatch):
    called = []
    monkeypatch.setattr(isolated, "execute", lambda name, *_a, **_k: called.append(name) or True)
    monkeypatch.setattr(
        isolated,
        "batch",
        lambda batch, _ready, _reason: (
            called.append(batch.name) or validation.BatchResult(batch.name != "composition", True)
        ),
    )
    isolated.validate()
    assert called[:8] == [
        "build-cli",
        "build-controls",
        "ready",
        "composition",
        "cold",
        "document-lexical",
        "analyzer",
        "epoch",
    ]
    assert all(name in called for name, _ in validation.QUALITY)
    assert "pilot-produce" not in called and "pilot-check" not in called
    assert isolated.row("pilot-produce")["outcome"] == "blocked"


@pytest.mark.parametrize("dispose_passed", [False, True])
def test_unresolved_disposal_holds_later_native_work_but_not_quality(
    isolated, monkeypatch, dispose_passed
):
    called = []

    def execute(name, *_args, **_kwargs):
        called.append(name)
        if name.endswith("-install"):
            install_record(Path(isolated.row(name)["service_config"]))
            return True
        if name.endswith("-dispose"):
            return dispose_passed
        return name not in {batch.name for batch in validation.BATCHES}

    monkeypatch.setattr(isolated, "execute", execute)
    isolated.validate()
    native = [name for name in called if name.endswith("-install")]
    assert native == (
        [f"{batch.name}-install" for batch in validation.BATCHES]
        if dispose_passed
        else ["composition-install"]
    )
    assert all(name in called for name, _ in validation.QUALITY)
    assert "pilot-produce" not in called and "pilot-check" not in called
    if not dispose_passed:
        for batch in validation.BATCHES[1:]:
            for name in (f"{batch.name}-install", batch.name, f"{batch.name}-dispose"):
                row = isolated.row(name)
                assert row["outcome"] == "blocked"
                assert "composition checked disposal unresolved" in row["reason"]


def test_exception_is_reported_without_preventing_independent_step(isolated, monkeypatch):
    def fail(*_args, **_kwargs):
        raise OSError("receipt unavailable")

    monkeypatch.setattr(isolated, "_execute", fail)
    assert not isolated.execute("build-cli")
    assert "receipt unavailable" in isolated.row("build-cli")["reason"]
    assert not isolated.stopped


@pytest.fixture
def completed_owner(isolated, monkeypatch):
    child_run = isolated.run / "child"
    child_run.mkdir()
    monkeypatch.setattr(validation.runs, "new_run_dir", lambda: child_run)
    monkeypatch.setattr(validation, "spawn_group", lambda *_a, **_k: object())

    class Guard:
        cleanup: ClassVar = {"status": "confirmed"}
        returncode = 0

        def __enter__(self):
            return self

        def __exit__(self, *_args):
            return None

    monkeypatch.setattr(validation, "SpawnGuard", lambda _child: Guard())
    monkeypatch.setattr(validation, "observe_exit", lambda _child: 0)
    monkeypatch.setattr(
        validation.runs,
        "load_record",
        lambda _run: {
            "exit": {"code": 0},
            "termination": "completed",
            "storage": {},
        },
    )
    monkeypatch.setattr(validation.runs, "host_cleanup", lambda *_a: {"status": "confirmed"})
    return Guard


def test_launcher_failure_cannot_be_hidden_by_successful_child_record(
    isolated,
    completed_owner,
):
    completed_owner.returncode = 1
    assert not isolated.execute("build-cli", ["unused"])
    row = isolated.row("build-cli")
    assert row["exit"] == {"code": 0} and row["launcher_exit"] == 1
    assert row["outcome"] == "failed" and "launcher failed" in row["reason"]


@pytest.mark.parametrize("case", ["missing", "unreleased", "not_observed", "confirmed"])
def test_runtime_consumes_actual_native_finality_independently_of_exit(
    isolated,
    completed_owner,
    monkeypatch,
    case,
):
    config = Path(isolated.row("composition")["service_config"])

    def spawn(*_args, **_kwargs):
        if case != "missing":
            path = config.parent / "attachments/owned/record.json"
            path.parent.mkdir(parents=True)
            row = {
                "id": "owned",
                "released": case != "unreleased",
                "command_cleanup": {"status": "confirmed"},
            }
            if case == "confirmed":
                drain = path.parent / "native-drain"
                drain.mkdir()
                row["native_drain"] = {
                    "directory": str(drain),
                    "attachment": "owned",
                    "generation": "00" * 32,
                }
                (drain / "123-456-1.json").write_text(
                    json.dumps(
                        {
                            "schema": 1,
                            "kind": "native-process-owner",
                            "owner": "workspace",
                            "attachment": "owned",
                            "generation": "00" * 32,
                            "registration": 1,
                            "process": {"pid": 123, "start_ticks": 456},
                            "state": "complete",
                            "completion": {
                                "local": "terminal",
                                "remote": "confirmed",
                                "failures": [],
                                "committed": [],
                                "storage": [],
                            },
                        }
                    )
                )
            path.write_text(json.dumps(row))
        return object()

    monkeypatch.setattr(validation, "spawn_group", spawn)
    if case == "unreleased":
        import surrealdb_fixture

        monkeypatch.setattr(
            surrealdb_fixture, "native_completion", lambda *_a: {"status": "confirmed"}
        )
    assert isolated.execute("composition", ["unused"]) == (case == "confirmed")
    row = isolated.row("composition")
    assert row["exit"] == {"code": 0} and row["cleanup"]["status"] == "confirmed"
    if case == "confirmed":
        assert row["native_finality"] == "confirmed"
    else:
        assert "finality unconfirmed" in row["reason"]
    if case == "not_observed":
        assert row["attachments"][0]["native_completion"]["status"] == "not_observed"


def test_evidenced_readiness_block_remains_blocked_without_native_attachment(
    isolated,
    completed_owner,
    monkeypatch,
):
    completed_owner.returncode = 75
    monkeypatch.setattr(
        validation.runs,
        "load_record",
        lambda _run: {
            "exit": {"code": 75},
            "termination": "completed",
            "storage": {},
        },
    )
    assert not isolated.execute("composition", ["unused"])
    assert isolated.row("composition")["outcome"] == "blocked"
    assert isolated.row("composition")["native_finality"] == "unconfirmed"


@pytest.mark.parametrize("fresh", [False, True])
def test_junit_copied_before_next_batch_without_reusing_stale_report(
    isolated,
    completed_owner,
    monkeypatch,
    fresh,
):
    target = isolated.run / "target"
    source = target / "nextest/default/junit.xml"
    source.parent.mkdir(parents=True)
    source.write_text("old report")
    monkeypatch.setenv("LCTX_CARGO_TARGET_DIR", str(target))

    def spawn(*_args, **_kwargs):
        if fresh:
            source.write_text("new batch report")
        return object()

    monkeypatch.setattr(validation, "spawn_group", spawn)
    assert isolated.execute("build-cli", ["unused"], junit=True) == fresh
    row = isolated.row("build-cli")
    if fresh:
        source.write_text("next batch overwrote shared report")
        assert Path(row["junit"]).read_text() == "new batch report"
    else:
        assert row["junit"] is None


def test_interrupted_child_uses_exact_existing_run_cancellation_and_stops(
    isolated,
    completed_owner,
    monkeypatch,
):
    cancelled = []

    def interrupt(_child):
        raise InterruptedError("cancelled")

    monkeypatch.setattr(validation, "observe_exit", interrupt)
    monkeypatch.setattr(
        validation.runs,
        "cancel",
        lambda run: (
            cancelled.append(run)
            or {
                "host_cleanup": {"status": "unknown"},
            }
        ),
    )
    with pytest.raises(InterruptedError):
        isolated.execute("build-cli", ["unused"])
    assert cancelled == [Path(isolated.row("build-cli")["run"])] * 2
    assert isolated.stopped
    with pytest.raises(InterruptedError, match="no further commands"):
        isolated.execute("build-controls", ["unused"])
