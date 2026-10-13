"""Focused host-tool controls; no server or library execution is substituted as qualification."""

from __future__ import annotations

import contextlib
import json
import os
import sys

import pytest
from test_surrealdb_service import (
    lifecycle_campaign_ready as lifecycle_campaign_ready,
)
from test_surrealdb_service import (
    recoverable as recoverable,
)

import runs
import surrealdb_development as development
import surrealdb_service as service


def compiled_info():
    return {
        "format": 1,
        "kind": "lctx-surreal-dev-build-info",
        "dependencies": {
            name: {
                "version": version,
                "source": "compiled-resolved-lock-and-workspace-patch",
                "patch_profile": {
                    "status": "declared_build_input",
                    "manifest": f"third_party/{name}/Cargo.toml",
                    "manifest_blake3": "a" * 64,
                    "documentation": f"third_party/{name}/README.lctx.md",
                    "documentation_blake3": "b" * 64,
                },
            }
            for name, version in (("surrealkit", "1.0.0-beta.1"), ("surrealdb", "3.3.0"), ("surrealdb-rpc", "3.3.0"))
        },
        "schema": {
            "source": "compiled-embedded-native-schema",
            "profile_identity": "c" * 64,
            "authority_correspondence": "passed",
            "modules": ["control", "canonical", "search", "compiler", "completion"],
        },
    }


def finite_provenance(worker):
    return {"worker_sha256": service.file_sha256(worker), "recorded_at": runs.now(), "build_info": compiled_info()}


def test_copy_tests_preserves_nested_suite_toml_and_sql_only(tmp_path):
    source, destination = tmp_path / "source", tmp_path / "destination"
    (source / "tests/nested").mkdir(parents=True)
    (source / ".venv").mkdir()
    (source / ".venv/pyvenv.cfg").write_text("unrelated environment")
    (source / "tests/nested/suite.toml").write_text('[test]\nname="control"\n')
    (source / "tests/nested/query.surql").write_text("RETURN 7;")
    (source / "tests/nested/output.json").write_text("old report")
    development.copy_tests(source, destination)
    assert (destination / "tests/nested/suite.toml").read_bytes() == (
        source / "tests/nested/suite.toml"
    ).read_bytes()
    assert (destination / "tests/nested/query.surql").read_text() == "RETURN 7;"
    assert not (destination / "tests/nested/output.json").exists()
    assert not (destination / ".venv").exists()
    assert (destination / "tests/nested/suite.toml").stat().st_mode & 0o777 == 0o600


@pytest.mark.parametrize("link", ["directory", "file", "nested"])
def test_copy_tests_refuses_symlink_inputs(tmp_path, link):
    source, destination = tmp_path / "source", tmp_path / "destination"
    source.mkdir()
    external = tmp_path / "external"
    external.mkdir()
    (external / "query.surql").write_text("RETURN 7")
    if link == "directory":
        (source / "tests").symlink_to(external, target_is_directory=True)
    else:
        (source / "tests").mkdir()
        (source / "tests/linked").symlink_to(
            external if link == "nested" else external / "query.surql",
            target_is_directory=link == "nested",
        )
    with pytest.raises(service.FixtureBlocked, match=r"physical tests directory|symlinks"):
        development.copy_tests(source, destination)


def test_development_worker_is_discoverable_before_report_exists(
    lifecycle_campaign_ready, tmp_path
):
    installation, _ = lifecycle_campaign_ready
    run_path = tmp_path / "owned-run"
    run_path.mkdir()
    record = {
        "owner": {**service.ProcessIdentity.of().to_json(), "pid": 99999999, "start_ticks": 1},
        "child": None,
        "termination": "completed",
        "cleanup": {"status": "confirmed", "survivors": []},
    }
    service.private_json(run_path / runs.RECORD, record)
    ledger = installation.directory / "development" / run_path.name
    ledger.mkdir(parents=True)
    service.private_json(
        ledger / "record.json",
        {
            "installation_id": installation.id,
            "server_generation": bytes(installation.record["service_generation"]).hex(),
            "run_path": str(run_path),
            "outcome": "failed",
            "report": {"status": "not_created"},
        },
    )
    (observed,) = service.development_workers(installation)
    assert observed["id"] == "owned-run"
    assert observed["report"] == {"status": "not_created"}
    assert not observed["pending"]
    (run_path / runs.RECORD).unlink()
    (observed,) = service.development_workers(installation)
    assert observed["pending"] and observed["cleanup"]["status"] == "unknown"


def test_development_setup_failure_retains_pre_effect_worker_owner_without_report(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    installation, _ = lifecycle_campaign_ready
    worker = tmp_path / "worker"
    worker.write_text("#!/bin/sh\nexit 1\n")
    worker.chmod(0o700)
    run_path = tmp_path / "owned-run"
    run_path.mkdir()
    monkeypatch.setattr(development.runs, "new_run_dir", lambda: run_path)
    monkeypatch.setattr(service, "borrow", lambda inst: contextlib.nullcontext())
    monkeypatch.setattr(development, "worker_build_info", finite_provenance)

    def refuse_ddl(inst, sql, *, cfg):
        (ledger,) = (installation.directory / "development").glob("*/record.json")
        assert json.loads(ledger.read_text())["run_path"] == str(run_path)
        assert runs.load_record(run_path) is not None, (
            "recoverable owner precedes the first DDL effect"
        )
        raise service.FixtureBlocked("query", "controlled setup refusal")

    monkeypatch.setattr(service, "sql", refuse_ddl)
    # The real host owner must exit before its pre-spawn receipt becomes drainable.
    child = os.fork()
    if child == 0:
        try:
            with pytest.raises(service.FixtureBlocked, match="setup refusal"):
                development.run_worker(installation, installation.id, "test", worker)
        except BaseException:
            import traceback

            traceback.print_exc()
            os._exit(1)
        os._exit(0)
    assert os.waitpid(child, 0)[1] == 0
    (observed,) = service.development_workers(installation)
    assert observed["report"] == {"status": "not_created"}
    assert observed["outcome"] == "failed"
    assert not observed["pending"], "pre-spawn failure has no child descendants"


def interrupted_token_input(installation, tmp_path):
    run_path = tmp_path / "token-worker-run"
    project = run_path / "project"
    (project / "tests").mkdir(parents=True, mode=0o700)
    ledger = installation.directory / "development" / run_path.name
    ledger.mkdir(parents=True, mode=0o700)
    generation = bytes(installation.record["service_generation"]).hex()
    original = '[actors.token]\nkind="token"\ntoken="' + development.TOKEN_MARKER + '"\n'
    config = project / "tests/config.toml"
    config.write_text("partially-written-issued-credential")
    config.chmod(0o600)
    owner = {**service.ProcessIdentity.of().to_json(), "pid": 99999999, "start_ticks": 1}
    service.private_json(
        ledger / "record.json",
        {
            "schema": 1,
            "installation_id": installation.id,
            "server_generation": generation,
            "run_path": str(run_path),
            "owner": owner,
            "database": "lifecycle_" + "a" * 32,
            "operation": "test",
            "outcome": "failed",
            "credential_storage": development.protect_token_input(installation, run_path),
        },
    )
    service.private_json(
        run_path / runs.RECORD,
        {
            "id": run_path.name,
            "owner": owner,
            "child": None,
            "cleanup": {"status": "confirmed", "survivors": []},
        },
    )
    service.private_json(
        run_path / "target-binding.json",
        {
            "format": 1,
            "purpose": service.LIFECYCLE_PURPOSE,
            "target_kind": "campaign",
            "installation_id": installation.id,
            "server_generation": generation,
            "namespace": service.NAMESPACE,
            "database": "lifecycle_" + "a" * 32,
            "runtime_config": str(installation.runtime_path("validation", installer=True)),
            "allowed_operations": ["test"],
            "project_dir": str(project),
            "report_path": str(project / "test-report.json"),
        },
    )
    service.private_json(
        project / development.TOKEN_RESTORE_JOURNAL,
        {
            "schema": 1,
            "purpose": "surrealdb-lifecycle-token-restore",
            "installation_id": installation.id,
            "server_generation": generation,
            "project_dir": str(project),
            "config_path": str(config),
            "original": original,
        },
    )
    return ledger, run_path, config, original


@pytest.mark.parametrize("partial", ["", "partially-written-issued-credential"])
def test_token_recovery_restores_interrupted_input_and_preserves_reports_and_source(
    lifecycle_campaign_ready, tmp_path, partial
):
    installation, _ = lifecycle_campaign_ready
    ledger, run_path, config, original = interrupted_token_input(installation, tmp_path)
    config.write_text(partial)
    source = tmp_path / "source-config.toml"
    source.write_text("original source input")
    report = run_path / "project/test-report.json"
    report.write_text('{"suites":[]}')
    assert development.restore_token_input(installation, ledger) == {"status": "passed"}
    assert config.read_text() == original
    assert config.stat().st_mode & 0o777 == 0o600
    assert report.read_text() == '{"suites":[]}' and source.read_text() == "original source input"
    assert not (run_path / "project" / development.TOKEN_RESTORE_JOURNAL).exists()
    assert development.restore_token_input(installation, ledger) == {"status": "not_applicable"}


@pytest.mark.parametrize(
    "change",
    [
        "cleanup",
        "live_parent",
        "owner",
        "binding",
        "journal",
        "config_link",
        "journal_link",
        "project_link",
    ],
)
def test_token_recovery_refuses_unproven_drainage_scope_or_physical_path(
    lifecycle_campaign_ready, tmp_path, change
):
    installation, _ = lifecycle_campaign_ready
    ledger, run_path, config, _ = interrupted_token_input(installation, tmp_path)
    record_path = run_path / runs.RECORD
    if change in {"cleanup", "live_parent", "owner"}:
        record = json.loads(record_path.read_text())
        if change == "cleanup":
            record["cleanup"] = {"status": "unknown"}
        else:
            record["owner"] = service.ProcessIdentity.of().to_json()
            if change == "live_parent":
                row = json.loads((ledger / "record.json").read_text())
                row["owner"] = record["owner"]
                service.private_json(ledger / "record.json", row)
        service.private_json(record_path, record)
    elif change in {"binding", "journal"}:
        path = (
            run_path / "target-binding.json"
            if change == "binding"
            else run_path / "project" / development.TOKEN_RESTORE_JOURNAL
        )
        row = json.loads(path.read_text())
        row["server_generation"] = "ff" * 32
        service.private_json(path, row)
    else:
        path = {
            "config_link": config,
            "journal_link": run_path / "project" / development.TOKEN_RESTORE_JOURNAL,
            "project_link": run_path / "project",
        }[change]
        external = tmp_path / "external"
        path.rename(external)
        path.symlink_to(external, target_is_directory=change == "project_link")
    with pytest.raises(service.FixtureBlocked, match="token recovery"):
        development.restore_token_input(installation, ledger)
    assert config.read_text() == "partially-written-issued-credential"


def test_campaign_disposal_restores_interrupted_token_before_credential_removal(
    lifecycle_campaign_ready, tmp_path
):
    installation, _ = lifecycle_campaign_ready
    _, run_path, config, original = interrupted_token_input(installation, tmp_path)
    report = run_path / "project/test-report.json"
    report.write_text('{"suites":[]}')
    receipt = service.dispose_campaign(installation, installation.id)
    assert receipt["steps"]["credential-removal"] == "passed"
    assert config.read_text() == original and report.exists()
    assert not (installation.directory / "server.env").exists()


def test_campaign_disposal_keeps_credentials_and_recovery_obligation_on_wrong_journal(
    lifecycle_campaign_ready, tmp_path
):
    installation, _ = lifecycle_campaign_ready
    _, run_path, config, original = interrupted_token_input(installation, tmp_path)
    journal = run_path / "project" / development.TOKEN_RESTORE_JOURNAL
    row = json.loads(journal.read_text())
    row["installation_id"] = "different-campaign"
    service.private_json(journal, row)
    with pytest.raises(service.FixtureBlocked, match="journal differs"):
        service.dispose_campaign(installation, installation.id)
    receipt = json.loads((installation.directory / "disposal.json").read_text())
    assert receipt["steps"].get("credential-removal") != "passed"
    assert (installation.directory / "server.env").exists()
    assert config.read_text() != original and journal.exists()


@pytest.mark.parametrize("ending", ["failure", "sigterm"])
def test_parent_recovers_token_input_after_actual_worker_exit_or_signal(
    lifecycle_campaign_ready, tmp_path, monkeypatch, ending
):
    """Exercise the real process owner with a finite worker; no beta/server claims."""
    installation, _ = lifecycle_campaign_ready
    worker = tmp_path / "finite-worker"
    worker.write_text(
        "#!"
        + sys.executable
        + "\n"
        + """import json, os, signal, sys
from pathlib import Path
binding = json.loads(Path(sys.argv[sys.argv.index("--binding") + 1]).read_text())
project = Path(binding["project_dir"])
(project / "tests").mkdir(mode=0o700)
config = project / "tests/config.toml"
original = 'token="LCTX_WORKER_ISSUED_TOKEN"\\n'
journal = {"schema":1, "purpose":"surrealdb-lifecycle-token-restore",
    "installation_id":binding["installation_id"], "server_generation":binding["server_generation"],
    "project_dir":str(project), "config_path":str(config), "original":original}
path = project / "sample-token-restore.json"
path.write_text(json.dumps(journal)); path.chmod(0o600)
config.write_text("partial-issued-credential"); config.chmod(0o600)
"""
        + ("os.kill(os.getpid(), signal.SIGTERM)\n" if ending == "sigterm" else "sys.exit(1)\n")
    )
    worker.chmod(0o700)
    run_path = tmp_path / "finite-child-run"
    run_path.mkdir(mode=0o700)
    monkeypatch.setattr(development.runs, "new_run_dir", lambda: run_path)
    monkeypatch.setattr(service, "borrow", lambda inst: contextlib.nullcontext())
    monkeypatch.setattr(development, "worker_build_info", finite_provenance)
    monkeypatch.setattr(service, "sql", lambda *args, **kwargs: [{"result": None}])
    summary = development.run_worker(installation, installation.id, "test", worker)
    assert summary["outcome"] == "failed" and summary["child_cleanup"]["status"] == "confirmed"
    assert summary["credential_cleanup"] == {"status": "passed"}
    assert (
        run_path / "project/tests/config.toml"
    ).read_text() == 'token="LCTX_WORKER_ISSUED_TOKEN"\n'
    assert not (run_path / "project" / development.TOKEN_RESTORE_JOURNAL).exists()


@pytest.mark.parametrize("cleanup,command_exit", [("confirmed", 0), ("failed", 1)])
def test_inspection_preserves_independent_terminal_unknown_and_cleanup_axes(
    lifecycle_campaign_ready, tmp_path, monkeypatch, cleanup, command_exit
):
    installation, _ = lifecycle_campaign_ready
    run_path = tmp_path / "inspect-run"
    run_path.mkdir()
    owner = {**service.ProcessIdentity.of().to_json(), "pid": 99999999, "start_ticks": 1}
    service.private_json(run_path / runs.RECORD, {
        "owner": owner, "child": None, "termination": "completed",
        "cleanup": {"status": cleanup, "reason": "secret-credential-path"},
    })
    ledger = installation.directory / "development" / run_path.name
    ledger.mkdir(parents=True)
    row = {
        "installation_id": installation.id,
        "server_generation": bytes(installation.record["service_generation"]).hex(),
        "run_path": str(run_path), "owner": owner, "recorded_at": runs.now(),
        "operation": "test", "database": "lifecycle_" + "a" * 32,
        "worker_sha256": "d" * 64,
        "worker_provenance": {"worker_sha256": "d" * 64, "recorded_at": runs.now(), "build_info": compiled_info()},
        "command_exit": command_exit, "outcome": "passed" if command_exit == 0 else "failed",
        "report": {"status": "passed", "path": "secret-credential-path"},
        "child_cleanup": {"status": cleanup, "reason": "secret-credential-path"},
        "credential_cleanup": {"status": "failed", "original": "secret-credential"},
        "remote": {"status": "unknown", "reason": "secret-credential"},
        "runtime_config": "secret-credential-path",
    }
    row["worker_provenance"]["build_info"]["credentials"] = "secret-credential"
    service.private_json(ledger / "record.json", row)
    service.private_json(installation.directory / "maintenance.json", {"operation": "closed"})
    monkeypatch.setattr(service, "sql", lambda *args, **kwargs: pytest.fail("ordinary inspection must not connect"))
    monkeypatch.setattr(service, "borrow", lambda *args: pytest.fail("ordinary inspection must not seek admission"))
    before = (ledger / "record.json").read_bytes()
    value = development.inspect_campaign(installation, installation.id, False)
    (observed,) = value["workers"]
    assert observed["operation"] == "test" and observed["database"] == row["database"]
    assert observed["worker_sha256"] == row["worker_sha256"]
    assert observed["command_exit"] == command_exit
    assert observed["child_cleanup"] == {"status": cleanup}
    assert observed["cleanup"] == {"status": cleanup}
    assert observed["credential_cleanup"] == {"status": "failed"}
    assert observed["report"] == {"status": "passed"}
    assert observed["remote"] == {"status": "unknown"}
    assert observed["pending"] == (cleanup != "confirmed")
    assert observed["observation"]["source"] == "development-ledger-and-managed-run"
    assert observed["observation"]["run_owner"] == owner
    assert observed["observation"]["owner_correspondence"] == "matches"
    assert observed["observation"]["observed_at"] > 0
    assert observed["worker_provenance"]["build_info"]["schema"]["profile_identity"] == "c" * 64
    assert observed["routes"]["disposal"][-1] == installation.id
    assert value["native"]["status"] == "not_inspected"
    assert "secret-credential" not in json.dumps(value)
    assert (ledger / "record.json").read_bytes() == before
    row["worker_provenance"]["worker_sha256"] = "e" * 64
    service.private_json(ledger / "record.json", row)
    assert service.development_workers(installation)[0]["worker_provenance"]["build_info"] == {"status": "unknown"}
    record = runs.load_record(run_path)
    record["owner"]["start_ticks"] += 1
    service.private_json(run_path / runs.RECORD, record)
    assert service.development_workers(installation)[0]["observation"]["owner_correspondence"] == "differs"


def test_inspection_refuses_wrong_target_and_preserves_missing_evidence(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    installation, _ = lifecycle_campaign_ready
    with pytest.raises(service.FixtureBlocked, match="exact owned"):
        development.inspect_campaign(installation, "missing-target", False)
    run_path = tmp_path / "missing-run"
    ledger = installation.directory / "development" / run_path.name
    ledger.mkdir(parents=True)
    service.private_json(ledger / "record.json", {
        "installation_id": installation.id,
        "server_generation": bytes(installation.record["service_generation"]).hex(),
        "run_path": str(run_path),
    })
    (observed,) = service.development_workers(installation)
    assert observed["pending"]
    assert observed["command_exit"] is None
    assert observed["worker_sha256"] is None
    assert observed["worker_provenance"]["build_info"] == {"status": "missing"}
    assert observed["observation"]["run_record"] == "missing"
    for field in ("child_cleanup", "credential_cleanup", "remote", "report"):
        assert observed[field] == {"status": "unknown"}
    @contextlib.contextmanager
    def closed(inst):
        raise service.FixtureBlocked("maintenance", "admission closed")
        yield
    monkeypatch.setattr(service, "borrow", closed)
    monkeypatch.setattr(service, "sql", lambda *args, **kwargs: pytest.fail("closed admission cannot connect"))
    with pytest.raises(service.FixtureBlocked, match="admission closed"):
        development.inspect_campaign(installation, installation.id, True)


@pytest.mark.parametrize("fault", [None, "missing", "invalid", "changed"])
def test_build_info_handshake_is_bound_to_worker_bytes_and_safe_on_refusal(tmp_path, monkeypatch, fault):
    worker = tmp_path / "worker"
    value = compiled_info()
    if fault == "invalid":
        value["schema"]["profile_identity"] = "secret-credential"
    worker.write_text("#!" + sys.executable + "\nprint(" + repr(json.dumps(value)) + ")\n")
    worker.chmod(0o700)
    if fault == "missing":
        worker = tmp_path / "missing"
    elif fault == "changed":
        original_run = development.subprocess.run
        def replace(*args, **kwargs):
            result = original_run(*args, **kwargs)
            worker.write_text("changed")
            return result
        monkeypatch.setattr(development.subprocess, "run", replace)
    if fault:
        with pytest.raises(service.FixtureBlocked) as error:
            development.worker_build_info(worker)
        assert "secret-credential" not in str(error.value)
    else:
        observed = development.worker_build_info(worker)
        assert observed["worker_sha256"] == service.file_sha256(worker)
        assert observed["build_info"] == value
        assert observed["recorded_at"]
