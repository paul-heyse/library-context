from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

from build_environment import (
    NATIVE_INPUT_KEYS,
    ROOT,
    launcher_env,
    normalized_env,
    project_environment,
    shell_changes,
)


@pytest.mark.parametrize("value", ["", " ", "target", "target/../target", "/other/checkout/target"])
def test_default_and_foreign_targets_are_removed(tmp_path: Path, value: str) -> None:
    source = {"CARGO_TARGET_DIR": value, "CARGO_BUILD_TARGET_DIR": value, "KEEP": "1"}
    assert normalized_env(source, tmp_path) == {"KEEP": "1"}
    assert source["CARGO_TARGET_DIR"] == value


def test_explicit_targets_and_non_target_configuration(tmp_path: Path) -> None:
    source = {"CARGO_TARGET_DIR": "target/measure", "SCCACHE_SERVER_UDS": "/cache/socket"}
    assert normalized_env(source, tmp_path) == source
    source["LCTX_CARGO_TARGET_DIR"] = "/explicit/external target"
    assert normalized_env(source, tmp_path)["CARGO_TARGET_DIR"] == "/explicit/external target"


def test_shell_unsets_and_quotes_and_exec_propagates(tmp_path: Path) -> None:
    source = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / "target"))
    shell = shell_changes(source, normalized_env(source))
    result = subprocess.check_output(
        ["bash", "-c", f'{shell}\n test -z "${{CARGO_TARGET_DIR+x}}" && echo cleared'],
        env=source,
        text=True,
    )
    assert result.strip() == "cleared"

    assert (
        shell_changes({}, {"CARGO_TARGET_DIR": "a b'c"}) == "export CARGO_TARGET_DIR='a b'\"'\"'c'"
    )
    result = subprocess.check_output(
        [
            sys.executable,
            str(ROOT / "scripts/build_environment.py"),
            "--",
            sys.executable,
            "-c",
            "import os; print(os.getenv('CARGO_TARGET_DIR', 'cleared'))",
        ],
        cwd=tmp_path,
        env=source,
        text=True,
    )
    assert result.strip() == "cleared"


def native_projects(root: Path) -> tuple[Path, Path]:
    sources = root / "crates" / "model"
    sources.mkdir(parents=True)
    for member, key in zip(("lctx_semantics",), NATIVE_INPUT_KEYS, strict=True):
        package = root / "python" / member
        package.mkdir(parents=True)
        (package / "pyproject.toml").write_text(
            '[tool.uv]\ncache-keys = [{ file = "../../crates/model/*.rs" },'
            f' {{ file = "src/*.rs" }}, {{ env = "{key}" }}]\n'
        )
        (package / "src").mkdir()
    old, latest = sources / "old.rs", sources / "latest.rs"
    old.write_text("old semantic input")
    latest.write_text("latest semantic input")
    os.utime(old, (1000, 1000))
    os.utime(latest, (2000, 2000))
    return old, latest


def test_native_build_keys_detect_content_and_membership_beyond_latest_timestamp(tmp_path):
    old, latest = native_projects(tmp_path)
    baseline = normalized_env({}, tmp_path, native_inputs=True)
    old.write_text("changed semantic input")
    os.utime(old, (1000, 1000))
    changed = normalized_env({}, tmp_path, native_inputs=True)
    assert all(changed[key] != baseline[key] for key in NATIVE_INPUT_KEYS)
    assert latest.stat().st_mtime == 2000
    old.unlink()
    removed = normalized_env({}, tmp_path, native_inputs=True)
    assert all(removed[key] != changed[key] for key in NATIVE_INPUT_KEYS)
    assert latest.stat().st_mtime == 2000
    (tmp_path / "unrelated.rs").write_text("not an adapter input")
    assert normalized_env({}, tmp_path, native_inputs=True) == removed


def test_native_build_keys_propagate_to_shell(tmp_path):
    native_projects(tmp_path)
    baseline = normalized_env({}, tmp_path, native_inputs=True)
    (tmp_path / "python/lctx_semantics/src/bridge.rs").write_text("semantic contract")
    changed = normalized_env({}, tmp_path, native_inputs=True)
    assert changed[NATIVE_INPUT_KEYS[0]] != baseline[NATIVE_INPUT_KEYS[0]]
    exports = shell_changes({}, changed)
    result = subprocess.check_output(
        ["bash", "-c", exports + '\nprintf "%s" "$LCTX_NATIVE_SEMANTICS_INPUTS"'], text=True
    )
    assert result == changed[NATIVE_INPUT_KEYS[0]]


def test_recipes_and_explain_never_fingerprint_native_inputs(tmp_path, monkeypatch):
    import build_environment

    def unexpected(root):
        raise AssertionError("only native sync and readiness fingerprint native inputs")

    monkeypatch.setattr(build_environment, "native_input_fingerprints", unexpected)
    inherited = {key: "stale" for key in NATIVE_INPUT_KEYS}
    # The default (every recipe through the justfile shell) drops a stale inherited key.
    assert normalized_env(inherited, tmp_path) == {}
    assert launcher_env(dict(inherited, UV_NO_SYNC="1"), tmp_path) == {}
    report = "\n".join(build_environment.explain(dict(inherited, UV_NO_SYNC="1"), tmp_path))
    assert "not computed here" in report
    assert "UV_NO_SYNC" in report.split("dropped by launcher:")[1]


def test_launcher_drops_uv_no_sync_without_warning(tmp_path):
    """Recipes carry explicit --no-sync/--no-project; UV_NO_SYNC with --no-project warns."""
    source = dict(os.environ, UV_NO_SYNC="1")
    script = str(ROOT / "scripts/build_environment.py")
    printed = subprocess.run(
        [sys.executable, script, "--", "printenv", "UV_NO_SYNC"],
        env=source,
        capture_output=True,
        text=True,
        check=False,
    )
    assert printed.returncode == 1 and printed.stdout == ""
    probe = [
        sys.executable,
        script,
        "--",
        "uv",
        "run",
        "--no-project",
        "--offline",
        "--no-python-downloads",
        "python",
        "-c",
        "pass",
    ]
    launched = subprocess.run(probe, env=source, cwd=ROOT, capture_output=True, text=True)
    assert launched.returncode == 0
    assert "--no-sync" not in launched.stderr
    # Control: the same launch with UV_NO_SYNC kept does warn.
    direct = subprocess.run(probe[3:], env=source, cwd=ROOT, capture_output=True, text=True)
    assert "has no effect when used alongside `--no-project`" in direct.stderr


def test_selected_environment_ignores_interpreter_and_virtual_env(tmp_path):
    root = tmp_path / "checkout"
    assert project_environment(root, {"VIRTUAL_ENV": "/elsewhere"}) == root / ".venv"
    assert project_environment(root, {"UV_PROJECT_ENVIRONMENT": "envs/x"}) == root / "envs/x"
    # A foreign absolute selection is ignored unless deliberately allowed.
    assert project_environment(root, {"UV_PROJECT_ENVIRONMENT": "/abs/.venv"}) == root / ".venv"
    allowed = {"UV_PROJECT_ENVIRONMENT": "/abs/.venv", "LCTX_ALLOW_FOREIGN_ENV": "1"}
    assert project_environment(root, allowed) == Path("/abs/.venv")


@pytest.mark.skipif(not Path("/usr/bin/python3").exists(), reason="no system interpreter")
def test_explain_runs_under_the_system_interpreter():
    """The justfile shell runs under the system python3 (3.12 here) and must parse there."""
    result = subprocess.run(
        ["/usr/bin/python3", str(ROOT / "scripts/build_environment.py"), "--explain"],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    for label in ("cargo target dir:", "cargo build dir:", "launcher python:", "selected uv env:"):
        assert label in result.stdout


def test_foreign_absolute_project_environment_is_dropped_unless_allowed(tmp_path):
    root = tmp_path / "worktree"
    root.mkdir()
    foreign = str(tmp_path / "main" / ".venv")
    env = normalized_env({"UV_PROJECT_ENVIRONMENT": foreign}, root)
    assert "UV_PROJECT_ENVIRONMENT" not in env
    own = str(root / ".venv")
    assert normalized_env({"UV_PROJECT_ENVIRONMENT": own}, root)["UV_PROJECT_ENVIRONMENT"] == own
    assert (
        normalized_env({"UV_PROJECT_ENVIRONMENT": ".venv"}, root)["UV_PROJECT_ENVIRONMENT"]
        == ".venv"
    )
    allowed = {"UV_PROJECT_ENVIRONMENT": foreign, "LCTX_ALLOW_FOREIGN_ENV": "1"}
    assert normalized_env(allowed, root)["UV_PROJECT_ENVIRONMENT"] == foreign
