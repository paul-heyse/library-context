#!/usr/bin/env python3
"""Normalize inherited Cargo paths for recipes, SQLx and interactive shells.

Use ``python3 scripts/build_environment.py --shell`` with eval in a shell, or
``python3 scripts/build_environment.py -- COMMAND ...``. Intentional external
targets use LCTX_CARGO_TARGET_DIR. Prefer Cargo config for non-default paths:
exported CARGO_* paths participate in sccache's Rust key.
"""

from __future__ import annotations

import os
import shlex
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET_KEYS = ("CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR")


def normalized_env(source: dict[str, str], root: Path = ROOT) -> dict[str, str]:
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
    return env


def shell_changes(before: dict[str, str], after: dict[str, str]) -> str:
    return "\n".join(
        f"export {key}={shlex.quote(after[key])}" if key in after else f"unset {key}"
        for key in TARGET_KEYS
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
