#!/usr/bin/env python3
"""Normalize inherited Cargo paths for recipes, SQLx and interactive shells.

Use ``python3 scripts/build_environment.py --shell`` with eval in a shell, or
``python3 scripts/build_environment.py -- COMMAND ...``. Intentional external
targets use LCTX_CARGO_TARGET_DIR. Prefer Cargo config for non-default paths:
exported CARGO_* paths participate in sccache's Rust key.
"""

from __future__ import annotations

import hashlib
import os
import shlex
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET_KEYS = ("CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR")
NATIVE_INPUT_KEYS = ("LCTX_NATIVE_SEMANTICS_INPUTS", "LCTX_NATIVE_STORAGE_INPUTS")


def native_input_fingerprints(root: Path) -> dict[str, str]:
    """Key uv's native builds by the content and membership of their declared inputs."""
    fingerprints = {}
    for member, key in zip(("lctx_semantics", "lctx_storage"), NATIVE_INPUT_KEYS, strict=True):
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


def normalized_env(
    source: dict[str, str], root: Path = ROOT, *, native_inputs: bool = True
) -> dict[str, str]:
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
    if native_inputs:
        env.update(native_input_fingerprints(root))
    else:
        for key in NATIVE_INPUT_KEYS:
            env.pop(key, None)
    return env


def shell_changes(before: dict[str, str], after: dict[str, str]) -> str:
    return "\n".join(
        f"export {key}={shlex.quote(after[key])}" if key in after else f"unset {key}"
        for key in (*TARGET_KEYS, *NATIVE_INPUT_KEYS)
        if before.get(key) != after.get(key)
    )


def main() -> None:
    before = dict(os.environ)
    env = normalized_env(before)
    args = sys.argv[1:]
    if args == ["--shell"]:
        print(shell_changes(before, env))
    elif len(args) > 1 and args[0] == "--":
        os.execvpe(args[1], args[1:], env)
    else:
        raise SystemExit("usage: build_environment.py --shell | -- COMMAND ...")


if __name__ == "__main__":
    main()
