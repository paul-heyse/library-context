"""Executable closure controls; these do not claim native service qualification."""

import json
import shutil
import subprocess
from pathlib import Path
from types import SimpleNamespace

import pytest

import storage_service as assets
from storage_lifecycle import Blocked, durable_json


def test_owned_executable_survives_checkout_and_state_replacement(tmp_path):
    checkout = tmp_path / "checkout"
    checkout.mkdir()
    executable = checkout / "lctx"
    executable.write_text("#!/bin/sh\nprintf 'owned-installer\\n'\n")
    executable.chmod(0o700)
    state = tmp_path / "surrealdb"
    state.mkdir()
    generation = assets.prepare(state, executable)
    assert not Path(generation["path"]).is_relative_to(state)
    assert assets.digest(executable) == generation["sha256"]
    shutil.rmtree(checkout)
    shutil.rmtree(state)
    result = subprocess.run([generation["path"]], capture_output=True, text=True, check=True)
    assert result.stdout == "owned-installer\n"
    assert Path(generation["path"]).stat().st_mode & 0o777 == 0o500


def test_corrupt_generation_or_symlink_cannot_replace_owned_bytes(tmp_path):
    executable = tmp_path / "lctx"
    executable.write_bytes(b"test")
    executable.chmod(0o700)
    state = tmp_path / "surrealdb"
    generation = assets.prepare(state, executable)
    target = Path(generation["path"])
    target.chmod(0o700)
    target.write_bytes(b"corrupt")
    with pytest.raises(Blocked, match="conflicts"):
        assets.prepare(state, executable)
    link = tmp_path / "link"
    link.symlink_to(executable)
    with pytest.raises(Blocked, match="regular"):
        assets.prepare(state, link)


def test_legacy_backup_rebinds_same_digest_and_preserves_service_identity(tmp_path):
    source = tmp_path / "lctx"
    source.write_bytes(b"same-version")
    source.chmod(0o700)
    generation = assets.prepare(tmp_path / "service", source)
    current = {"installer": generation["path"], "installer_generation": generation}
    archived = {
        "installer": "/removed-checkout/target/release/lctx",
        "installation_id": "fixed",
        "service_generation": [7] * 32,
    }
    rebound = assets.rebind_restored(archived, current, generation["sha256"])
    assert rebound["installer"] == generation["path"]
    assert rebound["installation_id"] == archived["installation_id"]
    assert rebound["service_generation"] == archived["service_generation"]
    with pytest.raises(Blocked, match="differs"):
        assets.rebind_restored(archived, current, "0" * 64)


def test_sanitized_observation_keeps_unresolved_transfer_dependencies(tmp_path):
    root = tmp_path / "service"
    root.mkdir(mode=0o700)
    installation = SimpleNamespace(
        directory=root, id="owned", record={"installer": "/checkout/lctx", "password": "private"}
    )
    durable_json(
        root / "installer-transfer.json",
        {"previous": "/checkout/lctx", "replacement": "/stable/lctx"},
    )
    row = assets.dependencies(installation)
    assert {item["path"] for item in row["dependencies"]} == {"/checkout/lctx", "/stable/lctx"}
    assert "private" not in json.dumps(row)
    assert not row["stable_installer"]


def test_actual_service_survival_runner_requires_explicit_execution(monkeypatch):
    import qualify_storage_service_survival as survival

    def unexpected():
        pytest.fail("a preview must not acquire service or checkout state")

    monkeypatch.setattr(survival.runs, "current_run", unexpected)
    assert survival.qualify(Path("unused-handle.json"), "entity")["outcome"] == "not_run"


def test_service_survival_witness_is_complete_nonempty_and_preserves_duplicates():
    import qualify_storage_service_survival as survival

    details = {"views": [{"relation": "entity", "rows": 2}]}
    assert survival.relation_rows(details, "entity", 2) == 2
    for count in (0, 3, True):
        with pytest.raises(ValueError, match="nonempty"):
            survival.relation_rows({"views": [{"relation": "entity", "rows": count}]}, "entity", 2)
    with pytest.raises(ValueError, match="one exact"):
        survival.relation_rows({"views": details["views"] * 2}, "entity", 2)
    before = {"audit": {"handle": "exact"}, "details": details, "rows": [{"id": 1}, {"id": 1}]}
    assert survival.same_publication(before, dict(before))
    assert not survival.same_publication(before, {**before, "rows": [{"id": 1}, {"id": 2}]})
    assert not survival.same_publication(before, {**before, "audit": {"handle": "changed"}})


def test_service_survival_handle_uses_exact_typed_database_and_generation():
    import qualify_storage_service_survival as survival

    installation = SimpleNamespace(record={"service_generation": [7] * 32})
    valid = {"database": {"namespace": "library_context", "database": "validation"},
             "service_generation": "07" * 32}
    survival.validation_handle(valid, installation)
    for changed in (
        {**valid, "database": "validation"},
        {**valid, "database": {"namespace": "other", "database": "validation"}},
        {**valid, "database": {"namespace": "library_context", "database": "main"}},
        {**valid, "service_generation": "08" * 32},
    ):
        with pytest.raises(survival.service.FixtureBlocked, match="installed validation publication"):
            survival.validation_handle(changed, installation)


def test_service_survival_resolution_hold_is_indefinite_and_preserves_existing_retention(tmp_path):
    import qualify_storage_service_survival as survival
    from storage_lifecycle import Storage

    run = tmp_path / "run"
    run.mkdir()
    output = run / "sm8-service-survival"
    output.mkdir()
    storage = Storage()
    ident = storage.publish(run, "run-receipt", {"kind": "run", "path": str(run)},
                            "run:control", temporary_days=90, managed=True)
    hold = survival.retain_checkout_run(run, output)
    obligation = storage.get(ident)["obligations"][hold["consumer"]]
    assert obligation["temporary_days"] is None and obligation["until"] is None
    assert obligation["released_at"] is None
    assert json.loads((run / survival.runs.RETAIN).read_bytes()) == hold["marker_content"]
    survival.release_checkout_run(run, hold)
    assert not (run / survival.runs.RETAIN).exists()
    assert storage.get(ident)["obligations"][hold["consumer"]]["released_at"]

    marker = run / survival.runs.RETAIN
    marker.write_text("caller-retained\n")
    hold = survival.retain_checkout_run(run, output)
    assert not hold["marker_owned"]
    survival.release_checkout_run(run, hold)
    assert marker.read_text() == "caller-retained\n"


@pytest.mark.parametrize("fault", ["branch-only", "missing-registered", "after-add", "interrupt", "unregistered-path",
                                  "changed-branch", "cleanup-refused", None])
def test_survival_partial_creation_reconciles_exact_owner_and_retains_failures(tmp_path, monkeypatch, fault):
    """Controlled local Git/service observations, never an actual survival qualification."""
    import contextlib
    import qualify_storage_service_survival as survival

    run = tmp_path / "run"
    run.mkdir()
    attachment = tmp_path / "attachment"
    attachment.mkdir()
    attached = attachment / "test.json"
    attached.write_text("{}")
    runtime = attachment / "compiler-runtime.json"
    cfg = {"database": "validation", "namespace": "library_context"}
    runtime.write_text(json.dumps({**cfg, "selection": str(attachment / "scratch" / "selected.json")}))
    monkeypatch.setenv("LCTX_SURREAL_TEST_CONFIG", str(attached))
    monkeypatch.setenv("LCTX_COMPILER_RUNTIME_CONFIG", str(runtime))
    installation = SimpleNamespace(id="control", record={"service_generation": [7] * 32}, runtime=lambda: cfg)
    handle = tmp_path / "handle.json"
    handle.write_text(json.dumps({"database": cfg, "service_generation": "07" * 32}))
    installer = tmp_path / "installer"
    installer.write_bytes(b"controlled-executable")
    installation.record["installer"] = str(installer)
    monkeypatch.setattr(survival.runs, "current_run", lambda: run)
    monkeypatch.setattr(survival.service.Installation, "load", lambda: installation)
    monkeypatch.setattr(survival.fixture.Server, "open", lambda _id: installation)
    monkeypatch.setattr(survival.fixture, "inspection_scope", lambda *_args: None)
    monkeypatch.setattr(survival.service, "borrow", lambda *_args: contextlib.nullcontext())
    monkeypatch.setattr(survival, "observe_service", lambda _i: {"installer": {
        "path": str(installer), "sha256": survival.service.file_sha256(installer)}})
    archive = tmp_path / "archive"
    archive.write_bytes(b"controlled-archive")
    monkeypatch.setattr(survival, "approved_recovery", lambda _i: (archive, {"exact": "controlled"}))
    monkeypatch.setattr(survival, "publication", lambda *_args, **_kw: {
        "audit": {"exact": "controlled"}, "details": {}, "rows": [{"id": 1}]})
    events = []
    state = {"branch_tip": None, "registrations": []}
    commit = "a" * 40

    def git(*args, **kwargs):
        if args == ("rev-parse", "HEAD"):
            body = commit.encode()
        elif args == ("rev-parse", "--path-format=absolute", "--git-common-dir"):
            body = b"/controlled/common.git"
        elif args[:2] == ("worktree", "list"):
            body = b"".join(b"\0".join((key + " " + value).encode() for key, value in row.items())
                             + b"\0\0" for row in state["registrations"])
        elif args[:3] == ("rev-parse", "--verify", "--quiet"):
            body = (state["branch_tip"] or "").encode()
            return SimpleNamespace(returncode=0 if body else 1, stdout=body)
        elif args[0] == "check-ignore":
            body = b""
        else:
            pytest.fail(f"unexpected controlled Git request {args}")
        return SimpleNamespace(returncode=0, stdout=body)

    monkeypatch.setattr(survival.worktree, "git", git)

    def retain(_run, _output):
        events.append("retained")
        return {"consumer": "controlled-resolution"}

    monkeypatch.setattr(survival, "retain_checkout_run", retain)
    monkeypatch.setattr(survival, "release_checkout_run", lambda *_args: events.append("released"))

    def create(name, **kwargs):
        receipt = json.loads((run / "sm8-service-survival" / "receipt.json").read_bytes())
        assert events == ["retained"]
        assert receipt["checkout_creation_started"] and receipt["checkout_intent"]["before"] == {
            "path_present": False, "branch_tip": None, "registrations": []}
        events.append("create")
        target = kwargs["base"] / name
        state["target"] = target
        state["branch_tip"] = commit
        if fault not in ("branch-only", "missing-registered"):
            target.mkdir()
            if fault != "unregistered-path":
                state["registrations"] = [{"worktree": str(target), "HEAD": commit,
                                           "branch": "refs/heads/wt/" + name}]
        if fault == "missing-registered":
            state["registrations"] = [{"worktree": str(target), "HEAD": commit,
                                       "branch": "refs/heads/wt/" + name}]
        if fault == "changed-branch":
            state["branch_tip"] = "b" * 40
        if fault == "interrupt":
            raise KeyboardInterrupt("controlled interruption after add")
        if fault:
            raise RuntimeError("controlled create interruption")
        return 0

    def remove(name, **kwargs):
        assert state["target"].exists(), "missing target must never reach generic removal/global prune"
        assert not kwargs.get("force", False)
        assert kwargs["into"] == commit and kwargs["base"] / name == state["target"]
        events.append("remove")
        if fault == "cleanup-refused":
            return 1
        if state["target"].exists():
            shutil.rmtree(state["target"])
        state.update(branch_tip=None, registrations=[])
        return 0

    monkeypatch.setattr(survival.worktree, "create", create)
    monkeypatch.setattr(survival.worktree, "remove", remove)
    if fault:
        expected_error = (BaseExceptionGroup if fault in ("branch-only", "missing-registered", "changed-branch", "unregistered-path", "cleanup-refused")
                          else KeyboardInterrupt if fault == "interrupt" else RuntimeError)
        with pytest.raises(expected_error):
            survival.qualify(handle, "entity", execute=True)
    else:
        assert survival.qualify(handle, "entity", execute=True)["outcome"] == "passed"
    receipt = json.loads((run / "sm8-service-survival" / "receipt.json").read_bytes())
    assert receipt["outcome"] == ("failed" if fault else "passed")
    assert ("released" in events) == (fault is None)
    if fault in ("changed-branch", "unregistered-path"):
        assert "remove" not in events and not receipt["checkout_removed"]
        assert state["target"].exists()
    elif fault in ("branch-only", "missing-registered"):
        assert "remove" not in events and not receipt["checkout_removed"]
        assert not state["target"].exists() and state["branch_tip"] == commit
        assert bool(state["registrations"]) == (fault == "missing-registered")
        assert "exact root resolution required" in receipt["errors"][1]
    elif fault == "cleanup-refused":
        assert events.count("remove") == 1 and not receipt["checkout_removed"]
        assert len(receipt["errors"]) == 2
    else:
        assert receipt["checkout_removed"] and state["branch_tip"] is None
