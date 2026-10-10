"""Actual helper processes own pin generations and explicit original-input reader leases."""

from __future__ import annotations

import contextlib
import datetime as dt
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

import storage_acquisition as acquisition
import storage_owners
from storage_lifecycle import Blocked, Storage


@pytest.fixture
def project(tmp_path, monkeypatch):
    library = tmp_path / "libraries/demo"
    library.mkdir(parents=True)
    (library / "pyproject.toml").write_text('[project]\nname="demo"\nversion="0"\n')
    (library / "uv.lock").write_text("version = 1\n")
    (library / ".python-version").write_text("3.14.7\n")
    tools = tmp_path / "tools"
    tools.mkdir()
    uv = tools / "uv"
    monkeypatch.setenv("TEST_REAL_UV", shutil.which("uv"))
    uv.write_text(
        f"#!{sys.executable}\n" + "import json,os,pathlib,subprocess,sys\n"
        "pathlib.Path(os.environ['UV_PROJECT_ENVIRONMENT']).mkdir(parents=True,exist_ok=True)\n"
        "pathlib.Path(os.environ['TEST_ACQUIRE_ARGS']).write_text(json.dumps({'args':sys.argv[1:],'env':dict(os.environ)}))\n"
        "if os.environ.get('TEST_ACQUIRE_DESCENDANT'):\n"
        " p=subprocess.Popen(['sleep','60']); "
        "pathlib.Path(os.environ['TEST_ACQUIRE_DESCENDANT']).write_text(str(p.pid))\n"
        "sys.exit(int(os.environ.get('TEST_ACQUIRE_EXIT','0')))\n"
    )
    uv.chmod(0o755)
    monkeypatch.setenv("PATH", str(tools) + os.pathsep + os.environ["PATH"])
    monkeypatch.setenv("TEST_ACQUIRE_ARGS", str(tmp_path / "args.json"))
    return {
        "library": str(library),
        "environment_root": str(tmp_path / "envs/demo"),
        "source_root": str(tmp_path / "sources/demo"),
        "synchronize": True,
        "reinstall": False,
        "source": None,
    }


def start(request):
    child = subprocess.Popen(
        [sys.executable, acquisition.__file__],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    assert child.stdin is not None and child.stdout is not None and child.stderr is not None
    child.stdin.write(json.dumps(request) + "\n")
    child.stdin.flush()
    reply = json.loads(child.stdout.readline())
    assert reply["outcome"] == "passed", (reply, child.stderr.read())
    return child, reply


def finish(child):
    child.stdin.write('{"action":"finish"}\n')
    child.stdin.flush()
    assert json.loads(child.stdout.readline())["finished"]
    child.stdin.close()
    assert child.wait(timeout=20) == 0, child.stderr.read()


def test_new_pin_releases_old_generation_only_after_actual_reader_ack(project):
    first, old = start(project)
    second = None
    try:
        old_row = next(row for row in Storage().records() if row["path"] == old["environment"])
        Path(project["library"], "uv.lock").write_text("version = 1\n# new pin\n")
        assert storage_owners.observe(old_row)["state"] == "active"
        second, new = start(project)
        assert new["environment"] != old["environment"]
        assert storage_owners.observe(old_row)["state"] == "active"
        assert all(
            value.get("until") is None
            for value in Storage().get(old_row["id"])["obligations"].values()
        )
        with pytest.raises(Blocked):
            acquisition.release_generation(old_row["id"])
        finish(first)
        first = None
        old_row = Storage().get(old_row["id"])
        assert storage_owners.observe(old_row)["state"] == "released"
        assert all(value.get("until") for value in old_row["obligations"].values())
        assert (
            Storage().disposition(
                old_row, references=[], clock=dt.datetime.now(dt.UTC) + dt.timedelta(days=8)
            )["disposition"]
            == "eligible"
        )
        assert Path(old["environment"]).is_dir()
        finish(second)
        second = None
        assert (
            storage_owners.observe(
                next(row for row in Storage().records() if row["path"] == new["environment"])
            )["state"]
            == "warm"
        )
    finally:
        for child in (first, second):
            if child is not None:
                child.stdin.close()
                child.wait(timeout=20)


def test_eof_is_not_cleanup_and_pin_edit_alone_is_not_release(project):
    child, reply = start(project)
    assert child.stdin is not None
    child.stdin.close()
    assert child.wait(timeout=20) == 75
    row = next(row for row in Storage().records() if row["path"] == reply["environment"])
    assert storage_owners.observe(row)["state"] == "unresolved"
    with pytest.raises(Blocked, match="acknowledgement"):
        acquisition.release_generation(row["id"])
    assert all(item.get("until") is None for item in row["obligations"].values())


def test_writer_flags_copy_isolation_descendant_cleanup_and_unique_reinstall(
    project, tmp_path, monkeypatch
):
    from harness import ProcessIdentity

    marker = tmp_path / "descendant"
    monkeypatch.setenv("TEST_ACQUIRE_DESCENDANT", str(marker))
    monkeypatch.setenv("UV_INDEX_URL", "https://invalid.example")
    monkeypatch.setenv("VIRTUAL_ENV", "/foreign")
    child, original = start(project)
    try:
        args = json.loads(Path(os.environ["TEST_ACQUIRE_ARGS"]).read_text())
        assert args["args"][-2:] == ["--link-mode", "copy"]
        assert "--frozen" in args["args"] and "--no-config" in args["args"]
        assert "UV_INDEX_URL" not in args["env"] and "VIRTUAL_ENV" not in args["env"]
        with contextlib.suppress(ProcessLookupError):
            assert not ProcessIdentity.of(int(marker.read_text())).alive()
        replacement, newer = start({**project, "reinstall": True})
        try:
            assert newer["environment"] != original["environment"]
        finally:
            finish(replacement)
    finally:
        finish(child)


def test_explicit_owner_release_and_managed_source_commit(project, tmp_path):
    source = tmp_path / "upstream"
    source.mkdir()
    subprocess.run(["git", "init", "-q", str(source)], check=True)
    subprocess.run(
        [
            "git",
            "-C",
            str(source),
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "pinned",
        ],
        check=True,
    )
    commit = subprocess.check_output(
        ["git", "-C", str(source), "rev-parse", "HEAD"], text=True
    ).strip()
    definition = Path(project["library"], "pyproject.toml")
    definition.write_text(
        definition.read_text() + f'\n[tool.lctx.source]\nrepository="{source}"\ncommit="{commit}"\n'
    )
    request = {**project, "source": {"repository": str(source), "commit": commit}}
    child, reply = start(request)
    finish(child)
    assert (
        subprocess.check_output(
            ["git", "-C", reply["source"], "rev-parse", "HEAD"], text=True
        ).strip()
        == commit
    )
    rows = [row for row in Storage().records() if row["category"] == "acquired-source"]
    assert len(rows) == 1
    assert acquisition.release_generation(rows[0]["id"])["outcome"] == "passed"
    assert storage_owners.observe(Storage().get(rows[0]["id"]))["state"] == "released"


def test_failed_writer_keeps_registered_generation_until_authoritative_grace(project, monkeypatch):
    monkeypatch.setenv("TEST_ACQUIRE_EXIT", "8")
    child = subprocess.Popen(
        [sys.executable, acquisition.__file__],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    output, error = child.communicate(json.dumps(project) + "\n", timeout=20)
    assert child.returncode == 75, error
    assert json.loads(output)["outcome"] == "blocked"
    (row,) = Storage().records()
    assert storage_owners.observe(row)["state"] == "released"
    assert all(item.get("until") for item in row["obligations"].values())
    assert Path(row["path"]).is_dir()
    assert "grace" in " ".join(Storage().disposition(row, references=[])["reasons"])


def test_relative_and_absolute_requests_share_selector_and_finished_receipts_expire(
    project, tmp_path, monkeypatch
):
    monkeypatch.chdir(tmp_path)
    relative = {
        **project,
        **{
            key: str(Path(project[key]).relative_to(tmp_path))
            for key in ("library", "environment_root", "source_root")
        },
    }
    child, first = start(relative)
    finish(child)
    owner = Path(first["owner"])
    lease_path = owner / "leases" / f"{first['lease']}.json"
    row = json.loads(lease_path.read_text())
    row["finished_at"] = "2000-01-01T00:00:00+00:00"
    lease_path.write_text(json.dumps(row))
    child, second = start(project)
    try:
        assert second["owner"] == first["owner"]
        assert second["environment"] == first["environment"]
        assert not lease_path.exists()
    finally:
        finish(child)


@pytest.mark.parametrize("change", ["overlap", "state", "symlink", "flag"])
def test_malformed_or_authority_overlapping_requests_refused_before_writer(
    project, tmp_path, change
):
    request = dict(project)
    if change == "overlap":
        request["source_root"] = request["environment_root"]
    elif change == "state":
        request["environment_root"] = str(tmp_path)
    elif change == "symlink":
        alias = tmp_path / "alias"
        alias.symlink_to(Path(project["library"]).parent)
        request["library"] = str(alias / "demo")
    else:
        request["synchronize"] = "yes"
    with pytest.raises(Blocked):
        acquisition.Lease(request)
    assert not Path(os.environ["TEST_ACQUIRE_ARGS"]).exists()


def test_actual_uv_preserves_enrolled_generation_physical_identity(project, tmp_path):
    library = Path(project["library"])
    (library / "pyproject.toml").write_text(
        '[project]\nname="storage-probe"\nversion="0"\nrequires-python="==3.14.*"\n'
    )
    (library / "uv.lock").unlink()  # Replace only this disposable stub lock with an actual uv lock.
    real_uv = os.environ["TEST_REAL_UV"]
    subprocess.run(
        [real_uv, "lock", "--project", str(library), "--offline", "--python", "3.14.7"],
        check=True,
        capture_output=True,
    )
    wrapper = tmp_path / "tools/uv"
    wrapper.write_text(f'#!/bin/sh\nexec "{real_uv}" "$@"\n')
    wrapper.chmod(0o755)
    child, reply = start(project)
    finish(child)
    row = next(row for row in Storage().records() if row["path"] == reply["environment"])
    from storage_lifecycle import physical

    assert physical(Path(reply["environment"])) == row["identity"]
    assert storage_owners.observe(row)["state"] == "warm"
