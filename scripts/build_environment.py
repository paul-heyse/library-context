#!/usr/bin/env python3
"""Normalize inherited Cargo paths for recipes and interactive shells.

Use ``python3 scripts/build_environment.py --shell`` with eval in a shell,
``python3 scripts/build_environment.py -- COMMAND ...`` to launch one command, or
``--explain [-- COMMAND ...]`` to report the effective Cargo paths, interpreters, selected uv
environment and uv settings. Intentional external targets use LCTX_CARGO_TARGET_DIR. Prefer Cargo
config for non-default paths: exported CARGO_* paths participate in sccache's Rust key.

This file is the justfile shell, so it runs under the system ``python3`` (3.12 on this machine)
and must stay parseable there: no 3.13+ syntax such as PEP 758 unparenthesised ``except A, B:``.
It imports only the standard library, never the harness helpers (``workspace_env``, ``harness``,
``surrealdb_fixture``). Native input fingerprints are computed only on request
(``native_inputs=True``): by native synchronization and native readiness, not for every recipe.
The launcher path drops ``UV_NO_SYNC``: repository launchers carry explicit ``--no-sync`` or
``--no-project``, and uv warns when ``UV_NO_SYNC`` meets ``--no-project``.
"""

from __future__ import annotations

import hashlib
import os
import shlex
import sys
import tomllib
from collections.abc import Mapping
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET_KEYS = ("CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR")
NATIVE_INPUT_KEYS = ("LCTX_NATIVE_SEMANTICS_INPUTS",)
BUILD_DIR_KEY = "CARGO_BUILD_BUILD_DIR"
# A checkout's own Cargo build directory (`just worktree --build-dir own`), gitignored. Absent, the
# tracked `.cargo/config.toml` selects ADR-0079's shared build directory.
BUILD_DIR_SELECTION = Path(".dev") / "build-dir"
LAUNCHER_DROPPED_KEYS = ("UV_NO_SYNC",)
# An absolute UV_PROJECT_ENVIRONMENT outside this checkout (e.g. main's `.venv` inherited by a
# worktree session) would make recipes sync and import another checkout's environment. It is
# dropped like a foreign target directory; LCTX_ALLOW_FOREIGN_ENV=1 keeps a deliberate one.
PROJECT_ENV_KEY = "UV_PROJECT_ENVIRONMENT"
FOREIGN_ENV_OVERRIDE = "LCTX_ALLOW_FOREIGN_ENV"


def native_input_fingerprints(root: Path) -> dict[str, str]:
    """Key uv's native builds by the content and membership of their declared inputs."""
    fingerprints = {}
    for member, key in zip(("lctx_semantics",), NATIVE_INPUT_KEYS, strict=True):
        package = root / "python" / member
        project = package / "pyproject.toml"
        if not project.is_file():
            continue
        cache_keys = tomllib.loads(project.read_text())["tool"].get("uv", {}).get("cache-keys", [])
        if {"env": key} not in cache_keys:
            continue
        files = {
            path.resolve()
            for entry in cache_keys
            if "file" in entry
            for path in package.glob(entry["file"])
            if path.is_file()
        }
        digest = hashlib.sha256()
        for path in sorted(files):
            name = path.relative_to(root).as_posix().encode()
            content = path.read_bytes()
            for part in (name, content):
                digest.update(len(part).to_bytes(8, "little"))
                digest.update(part)
        fingerprints[key] = digest.hexdigest()
    return fingerprints


def checkout_build_dir(root: Path = ROOT) -> str | None:
    """The checkout's own build directory, when one was selected for it."""
    try:
        value = (root / BUILD_DIR_SELECTION).read_text().strip()
    except OSError:
        return None
    return value or None


def foreign_project_environment(source: Mapping[str, str], root: Path = ROOT) -> bool:
    """An inherited absolute UV_PROJECT_ENVIRONMENT outside this checkout, not deliberately allowed."""
    value = source.get(PROJECT_ENV_KEY, "").strip()
    if not value or not Path(value).expanduser().is_absolute():
        return False
    if source.get(FOREIGN_ENV_OVERRIDE, "") in ("1", "true"):
        return False
    return not Path(os.path.abspath(Path(value).expanduser())).is_relative_to(root.resolve())


def normalized_env(
    source: dict[str, str], root: Path = ROOT, *, native_inputs: bool = False
) -> dict[str, str]:
    """Normalize inherited target paths; add native input keys only when requested."""
    env = source.copy()
    root = root.resolve()
    for key in TARGET_KEYS:
        value = env.get(key)
        if value is None:
            continue
        path = (root / value).resolve()
        if not value.strip() or path == root / "target" or not path.is_relative_to(root):
            env.pop(key, None)
    override = env.get("LCTX_CARGO_TARGET_DIR")
    if override and override.strip():
        env["CARGO_TARGET_DIR"] = str((root / override).resolve())
        env.pop("CARGO_BUILD_TARGET_DIR", None)
    if foreign_project_environment(env, root):
        env.pop(PROJECT_ENV_KEY)
    own = checkout_build_dir(root)
    if own:
        env[BUILD_DIR_KEY] = own
    if native_inputs:
        env.update(native_input_fingerprints(root))
    else:
        for key in NATIVE_INPUT_KEYS:
            env.pop(key, None)
    return env


def launcher_env(source: dict[str, str], root: Path = ROOT) -> dict[str, str]:
    """The environment of the ``-- COMMAND`` launcher (the justfile shell)."""
    env = normalized_env(source, root)
    for key in LAUNCHER_DROPPED_KEYS:
        env.pop(key, None)
    return env


def project_environment(root: Path = ROOT, source: Mapping[str, str] | None = None) -> Path:
    """The uv project environment selected for this checkout.

    ``UV_PROJECT_ENVIRONMENT`` is used as-is when absolute and resolved against the checkout root
    when relative; the default is ``.venv``. An absolute value outside this checkout is ignored
    unless ``LCTX_ALLOW_FOREIGN_ENV=1`` selects it deliberately. ``sys.prefix`` and ``VIRTUAL_ENV`` never select it:
    uv itself ignores a non-matching ``VIRTUAL_ENV`` without ``--active``.
    """
    selection = os.environ if source is None else source
    value = selection.get("UV_PROJECT_ENVIRONMENT", "").strip()
    if foreign_project_environment(selection, root):
        value = ""
    path = Path(value).expanduser() if value else Path(".venv")
    return Path(os.path.abspath(path if path.is_absolute() else root.resolve() / path))


def _cargo_path(value: str, root: Path, env: dict[str, str]) -> str:
    cargo_home = env.get("CARGO_HOME") or str(Path.home() / ".cargo")
    for template, replacement in (
        ("{workspace-root}", str(root)),
        ("{cargo-cache-home}", cargo_home),
    ):
        value = value.replace(template, replacement)
    return str(Path(value) if Path(value).is_absolute() else root / value)


def explain(source: dict[str, str], root: Path = ROOT) -> list[str]:
    """Effective paths and settings for a command launched here, without fingerprinting."""
    root = root.resolve()
    env = launcher_env(source, root)
    try:
        config = tomllib.loads((root / ".cargo" / "config.toml").read_text()).get("build", {})
    except OSError:
        config = {}
    if env.get("CARGO_TARGET_DIR"):
        target = f"{env['CARGO_TARGET_DIR']} (CARGO_TARGET_DIR)"
    elif env.get("CARGO_BUILD_TARGET_DIR"):
        target = f"{env['CARGO_BUILD_TARGET_DIR']} (CARGO_BUILD_TARGET_DIR)"
    elif "target-dir" in config:
        target = f"{_cargo_path(config['target-dir'], root, env)} (.cargo/config.toml)"
    else:
        target = f"{root / 'target'} (Cargo default)"
    own = checkout_build_dir(root)
    if own:
        inherited = source.get(BUILD_DIR_KEY)
        replaced = f"; replaced inherited {inherited}" if inherited and inherited != own else ""
        build = f"{own} (own: {BUILD_DIR_SELECTION}{replaced}; bare cargo needs `just env --`)"
    elif env.get(BUILD_DIR_KEY):
        build = f"{env[BUILD_DIR_KEY]} ({BUILD_DIR_KEY})"
    elif "build-dir" in config:
        build = f"{_cargo_path(config['build-dir'], root, env)} (.cargo/config.toml)"
    else:
        build = "same as the target directory (Cargo default)"
    dropped = [
        key
        for key in (*TARGET_KEYS, PROJECT_ENV_KEY, *NATIVE_INPUT_KEYS, *LAUNCHER_DROPPED_KEYS)
        if key in source and source.get(key) != env.get(key)
    ]
    venv = project_environment(root, env)
    selected_by = (
        "UV_PROJECT_ENVIRONMENT" if env.get("UV_PROJECT_ENVIRONMENT", "").strip() else "default"
    )
    project_python = venv / "bin" / "python"
    try:
        pinned = (root / ".python-version").read_text().strip()
    except OSError:
        pinned = "(none)"
    active = env.get("VIRTUAL_ENV")
    if active and Path(os.path.abspath(active)) != venv:
        active += " (ignored by uv project commands without --active)"
    uv_settings = sorted(key for key in env if key.startswith("UV_"))
    lines = [
        f"checkout:            {root}",
        f"cargo target dir:    {target}",
        f"cargo build dir:     {build}",
        f"launcher python:     {sys.executable} ({sys.version.split()[0]})",
        f"project python pin:  {pinned} (.python-version)",
        f"selected uv env:     {venv} ({selected_by};"
        f" {'present' if project_python.exists() else 'absent'})",
        f"VIRTUAL_ENV:         {active or '(unset)'}",
        "uv settings:         "
        + (", ".join(f"{key}={env[key]}" for key in uv_settings) or "(none)"),
        f"dropped by launcher: {', '.join(dropped) or '(none)'}",
        "native input keys:   not computed here (just sync native / native readiness)",
    ]
    return lines


def shell_changes(before: dict[str, str], after: dict[str, str]) -> str:
    return "\n".join(
        f"export {key}={shlex.quote(after[key])}" if key in after else f"unset {key}"
        for key in (*TARGET_KEYS, BUILD_DIR_KEY, PROJECT_ENV_KEY, *NATIVE_INPUT_KEYS)
        if before.get(key) != after.get(key)
    )


def main() -> None:
    before = dict(os.environ)
    args = sys.argv[1:]
    if args == ["--shell"]:
        print(shell_changes(before, normalized_env(before)))
        return
    if args[:1] == ["--explain"]:
        command = args[2:] if args[1:2] == ["--"] else None
        if len(args) > 1 and not command:
            raise SystemExit("usage: build_environment.py --explain [-- COMMAND ...]")
        print("\n".join(explain(before)), file=sys.stderr if command else sys.stdout, flush=True)
        if command is None:
            return
        args = ["--", *command]
    if len(args) > 1 and args[0] == "--":
        os.execvpe(args[1], args[1:], launcher_env(before))
    raise SystemExit(
        "usage: build_environment.py --shell | --explain [-- COMMAND ...] | -- COMMAND ..."
    )


if __name__ == "__main__":
    main()
