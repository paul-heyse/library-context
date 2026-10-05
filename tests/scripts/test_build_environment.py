from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

from build_environment import NATIVE_INPUT_KEYS, ROOT, normalized_env, shell_changes


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
    for member, key in zip(("lctx_semantics", "lctx_storage"), NATIVE_INPUT_KEYS, strict=True):
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
    baseline = normalized_env({}, tmp_path)
    old.write_text("changed semantic input")
    os.utime(old, (1000, 1000))
    changed = normalized_env({}, tmp_path)
    assert all(changed[key] != baseline[key] for key in NATIVE_INPUT_KEYS)
    assert latest.stat().st_mtime == 2000
    old.unlink()
    removed = normalized_env({}, tmp_path)
    assert all(removed[key] != changed[key] for key in NATIVE_INPUT_KEYS)
    assert latest.stat().st_mtime == 2000
    (tmp_path / "unrelated.rs").write_text("not an adapter input")
    assert normalized_env({}, tmp_path) == removed


def test_native_build_keys_preserve_package_scope_and_shell_propagation(tmp_path):
    native_projects(tmp_path)
    baseline = normalized_env({}, tmp_path)
    (tmp_path / "python/lctx_storage/src/bridge.rs").write_text("storage effect")
    changed = normalized_env({}, tmp_path)
    assert changed[NATIVE_INPUT_KEYS[0]] == baseline[NATIVE_INPUT_KEYS[0]]
    assert changed[NATIVE_INPUT_KEYS[1]] != baseline[NATIVE_INPUT_KEYS[1]]
    exports = shell_changes({}, changed)
    result = subprocess.check_output(
        ["bash", "-c", exports + '\nprintf "%s" "$LCTX_NATIVE_STORAGE_INPUTS"'], text=True
    )
    assert result == changed[NATIVE_INPUT_KEYS[1]]


def test_pure_cargo_normalization_skips_native_artifact_keys(tmp_path, monkeypatch):
    import build_environment

    def unexpected(root):
        raise AssertionError("pure Rust must not fingerprint unrelated Python artifacts")

    monkeypatch.setattr(build_environment, "native_input_fingerprints", unexpected)
    inherited = {key: "stale" for key in NATIVE_INPUT_KEYS}
    assert normalized_env(inherited, tmp_path, native_inputs=False) == {}
