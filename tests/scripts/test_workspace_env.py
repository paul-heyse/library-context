"""Scoped preparation, observed readiness and managed environment ownership (D1)."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

import pytest

import harness
import workspace_env
from build_environment import NATIVE_INPUT_KEYS, ROOT
from harness import ProcessIdentity
from workspace_env import (
    OWNERSHIP_KEY,
    OwnershipConflict,
    holders,
    observe,
    ownership,
    resources_for,
    sync_command,
    uv_environment,
)

SCRIPT = ROOT / "scripts" / "workspace_env.py"


@pytest.fixture
def isolated_locks(tmp_path, monkeypatch):
    runtime = tmp_path / "runtime"
    runtime.mkdir()
    monkeypatch.setenv("XDG_RUNTIME_DIR", str(runtime))
    monkeypatch.delenv(OWNERSHIP_KEY, raising=False)
    monkeypatch.delenv("UV_PROJECT_ENVIRONMENT", raising=False)
    return runtime / "library-context" / "locks"


def child_env(**extra: str) -> dict[str, str]:
    env = {k: v for k, v in os.environ.items() if k != OWNERSHIP_KEY}
    env.update(extra)
    return env


def hold(script: Path, mode: str, requirement: str, *command: str, **env: str):
    return subprocess.Popen(
        [sys.executable, str(script), "hold", mode, requirement, "--", *command],
        env=child_env(**env),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def wait_for(predicate, timeout: float = 10.0) -> None:
    deadline = time.monotonic() + timeout
    while not predicate():
        assert time.monotonic() < deadline, "condition not reached"
        time.sleep(0.05)


def test_requirements_scope_resources_and_pure_work_takes_none(tmp_path):
    root = tmp_path / "checkout"
    assert resources_for((), root) == ()
    assert resources_for({"native-store", "cli"}, root) == ()
    (environment,) = resources_for("tools", root, {})
    assert (environment.kind, environment.path) == ("environment", root / ".venv")
    native = resources_for("native-python", root, {})
    assert [r.kind for r in native] == ["environment", "extension"]
    assert native[1].path == root / "python/lctx_semantics/python/lctx_semantics"
    (service,) = resources_for("vllm", root, {"UV_PROJECT_ENVIRONMENT": "/main/.venv"})
    assert service.path == root / "services/vllm/.venv"
    # The environment's identity follows UV_PROJECT_ENVIRONMENT, never VIRTUAL_ENV; a foreign
    # absolute selection counts only when deliberately allowed.
    (ignored,) = resources_for("tools", root, {"UV_PROJECT_ENVIRONMENT": "/main/.venv"})
    assert ignored.path == root / ".venv"
    allowed = {"UV_PROJECT_ENVIRONMENT": "/main/.venv", "LCTX_ALLOW_FOREIGN_ENV": "1"}
    (moved,) = resources_for("tools", root, allowed)
    assert moved.path == Path("/main/.venv")
    (relative,) = resources_for("tools", root, {"UV_PROJECT_ENVIRONMENT": "env"})
    assert relative.path == root / "env"
    assert resources_for("tools", root, {"VIRTUAL_ENV": "/other"})[0].path == root / ".venv"


def test_lock_directory_falls_back_without_runtime_dir(monkeypatch, tmp_path):
    assert workspace_env.lock_directory({"XDG_RUNTIME_DIR": str(tmp_path)}) == (
        tmp_path / "library-context" / "locks"
    )
    fallback = workspace_env.lock_directory({"XDG_RUNTIME_DIR": str(tmp_path / "absent")})
    assert fallback == Path.home() / ".cache" / "library-context" / "locks"


def test_nested_ownership_reuses_and_never_deadlocks(isolated_locks):
    with ownership("exclusive", "native") as outer:
        assert len(outer.acquired) == 2
        assert json.loads(os.environ[OWNERSHIP_KEY])[0]["mode"] == "exclusive"
        with ownership("shared", "tools") as inner:
            assert inner.acquired == ()
        with ownership("exclusive", "native-python") as inner:
            assert inner.acquired == ()
        # A child process reuses the ancestor's ownership instead of waiting on it.
        nested = subprocess.run(
            [sys.executable, str(SCRIPT), "hold", "exclusive", "native", "--", "true"],
            env=outer.environment(os.environ),
            timeout=20,
            check=False,
        )
        assert nested.returncode == 0
    assert OWNERSHIP_KEY not in os.environ


def test_nested_exclusive_under_shared_is_refused_not_deadlocked(isolated_locks):
    with (
        ownership("shared", "native"),
        pytest.raises(OwnershipConflict, match="held shared"),
        ownership("exclusive", "tools"),
    ):
        pass
    # A partially covered request acquires only what the ancestor does not hold.
    with ownership("shared", "tools"), ownership("shared", "native") as inner:
        assert [r.kind for r in inner.acquired] == ["extension"]


def test_dead_owner_token_is_not_reused(isolated_locks, monkeypatch):
    dead = subprocess.Popen(["true"])
    identity = ProcessIdentity.of(dead.pid)
    dead.wait()
    (resource,) = resources_for("tools")
    token = [
        {
            "name": resource.name,
            "path": str(resource.path),
            "mode": "exclusive",
            "owner": identity.to_json(),
        }
    ]
    monkeypatch.setenv(OWNERSHIP_KEY, json.dumps(token))
    with ownership("shared", "tools") as owned:
        assert owned.acquired == (resource,)


def test_two_readers_overlap_and_sync_reports_and_waits(isolated_locks):
    first = hold(SCRIPT, "shared", "native", "sleep", "3")
    second = hold(SCRIPT, "shared", "native-python", "sleep", "3")
    environment, extension = resources_for("native")
    wait_for(lambda: len(holders(environment)) == 2 and len(holders(extension)) == 2)
    assert {h.pid for h in holders(environment)} == {first.pid, second.pid}
    started = time.monotonic()
    writer = hold(SCRIPT, "exclusive", "native", "true")
    _, report = writer.communicate(timeout=30)
    assert writer.returncode == 0
    assert time.monotonic() - started > 1.0
    assert "waiting for exclusive environment ownership" in report
    assert f"pid {first.pid} (shared, started " in report
    assert f"pid {second.pid}" in report
    for reader in (first, second):
        assert reader.wait(timeout=10) == 0
    wait_for(lambda: holders(environment) == [])


def test_absolute_environment_from_another_checkout_contends(isolated_locks, tmp_path):
    other = tmp_path / "other-checkout"
    (other / "scripts").mkdir(parents=True)
    for name in ("workspace_env.py", "build_environment.py", "harness.py"):
        shutil.copy(ROOT / "scripts" / name, other / "scripts" / name)
    main_env = str(ROOT / ".venv")
    holder = hold(SCRIPT, "shared", "tools", "sleep", "2")
    (environment,) = resources_for("tools")
    wait_for(lambda: len(holders(environment)) == 1)
    writer = hold(
        other / "scripts/workspace_env.py",
        "exclusive",
        "tools",
        "true",
        UV_PROJECT_ENVIRONMENT=main_env,
        LCTX_ALLOW_FOREIGN_ENV="1",
    )
    _, report = writer.communicate(timeout=30)
    assert writer.returncode == 0
    assert f"ownership of {main_env}:" in report and f"pid {holder.pid}" in report
    holder.wait(timeout=10)
    # Without the inherited selection the other checkout owns its own environment: no wait.
    free = hold(SCRIPT.parent.parent / "scripts/workspace_env.py", "shared", "tools", "true")
    alone = hold(other / "scripts/workspace_env.py", "exclusive", "tools", "true")
    assert "waiting" not in alone.communicate(timeout=30)[1]
    free.wait(timeout=10)


def test_routes_are_scoped_and_only_native_computes_the_key(monkeypatch):
    monkeypatch.setenv("UV_PROJECT_ENVIRONMENT", "/main/.venv")
    monkeypatch.setenv("VIRTUAL_ENV", "/main/.venv")
    monkeypatch.setenv("UV_NO_SYNC", "1")
    key = NATIVE_INPUT_KEYS[0]
    assert sync_command("tools") == ("uv", "sync", "--locked", "--inexact", "--only-group", "dev")
    assert sync_command("native", check=True) == ("uv", "sync", "--locked", "--check", "--inexact")
    assert sync_command("vllm")[-3:] == ("--project", str(ROOT / "services/vllm"), "--locked")
    tools, native, vllm = (uv_environment(route) for route in ("tools", "native", "vllm"))
    assert key not in tools and key not in vllm and len(native[key]) == 64
    assert all("VIRTUAL_ENV" not in env and "UV_NO_SYNC" not in env for env in (tools, native))
    # A foreign absolute selection is dropped unless deliberately allowed (ADR-0134, P5 incident).
    assert "UV_PROJECT_ENVIRONMENT" not in tools
    monkeypatch.setenv("LCTX_ALLOW_FOREIGN_ENV", "1")
    assert uv_environment("tools")["UV_PROJECT_ENVIRONMENT"] == "/main/.venv"
    assert "UV_PROJECT_ENVIRONMENT" not in vllm


def test_missing_vllm_environment_names_its_route(tmp_path):
    root = tmp_path / "checkout"
    (root / "services/vllm").mkdir(parents=True)
    for name in ("pyproject.toml", "uv.lock"):
        shutil.copy(ROOT / "services/vllm" / name, root / "services/vllm" / name)
    # The lock names a local wheel under docs/; keep it resolvable so only the env is missing.
    (root / "docs").symlink_to(ROOT / "docs")
    readiness = observe("vllm", root=root)
    assert not readiness.ready
    assert readiness.detail == "error: The environment is outdated"
    assert readiness.repair == "just sync vllm"
    assert "blocked: run just sync vllm" in readiness.message()
    assert not (root / "services/vllm/.venv").exists()


def test_unknown_requirements_are_not_observed():
    with pytest.raises(KeyError):
        observe("native-store")


def test_pending_sync_is_not_starved_by_a_stream_of_new_readers(isolated_locks):
    """flock has no writer preference; the gate makes new readers queue behind a pending sync."""
    (environment,) = resources_for("tools")
    first = hold(SCRIPT, "shared", "tools", "sleep", "1.5")
    wait_for(lambda: len(holders(environment)) == 1)
    started = time.monotonic()
    writer = hold(SCRIPT, "exclusive", "tools", "true")
    wait_for(lambda: [h.pid for h in holders(environment, gate=True)] == [writer.pid])
    readers = []
    # Overlapping readers that would keep the shared lock continuously held for ~5 s.
    while time.monotonic() - started < 5.0:
        readers.append(hold(SCRIPT, "shared", "tools", "sleep", "1.0"))
        time.sleep(0.3)
        if writer.poll() is not None:
            break
    writer.wait(timeout=30)
    waited = time.monotonic() - started
    assert waited < 4.0, f"the pending sync waited {waited:.1f} s behind new readers"
    reports = [reader.communicate(timeout=30)[1] for reader in [first, *readers]]
    assert any(f"waiting for pending sync (pid {writer.pid})" in report for report in reports)
    wait_for(lambda: holders(environment) == [])


def test_pending_sync_never_blocks_a_nested_acquisition_of_another_resource(isolated_locks):
    """A tree holding the environment shared can still add the extension while a sync waits."""
    environment, _ = resources_for("native")
    with ownership("shared", "tools"):
        writer = hold(SCRIPT, "exclusive", "native", "true")
        wait_for(lambda: [h.pid for h in holders(environment, gate=True)] == [writer.pid])
        started = time.monotonic()
        with ownership("shared", "native") as inner:
            assert [r.kind for r in inner.acquired] == ["extension"]
        assert time.monotonic() - started < 2.0
        assert writer.poll() is None  # still waiting on the tree's shared environment
    assert writer.wait(timeout=30) == 0


def test_holders_come_from_the_kernel_lock_table(isolated_locks):
    (environment,) = resources_for("tools")
    with ownership("shared", "tools"):
        (holder,) = holders(environment)
        assert (holder.pid, holder.mode) == (os.getpid(), "shared")
        assert "pytest" in holder.command or "python" in holder.command
    assert holders(environment) == []
    assert not list(isolated_locks.glob("*.holders"))


def test_foreign_namespace_owner_counts_while_its_lock_is_held(isolated_locks):
    (resource,) = resources_for("tools")
    owner = {"pid": 4_000_000, "start_ticks": 1, "boot_id": harness.boot_id(), "pid_namespace": 1}
    entry = {"name": resource.name, "path": str(resource.path), "mode": "shared", "owner": owner}
    token = {OWNERSHIP_KEY: json.dumps([entry])}
    assert workspace_env.inherited(token) == {}
    with ownership("shared", "tools", env={}):
        assert resource.name in workspace_env.inherited(token)
