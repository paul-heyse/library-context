"""Pinned TOML composition in the checkout tools interpreter, never in bare Rust runs.

The caller owns the destination and its storage admission. Configuration paths are resolved
by nextest against the unchanged workspace, not the generated config's directory.
"""

from __future__ import annotations

import json
import sys
from importlib.metadata import version
from pathlib import Path

import tomlkit

RECORD = {
    "enabled": True,
    "max-records": 1,
    "max-total-size": "18446744073709551615",
    "max-age": "18446744073709551615s",
    "max-output-size": "256MiB",
}
JUNIT = {
    "store-success-output": True,
    "store-failure-output": True,
    "report-skipped": "all",
    "flaky-fail-status": "failure",
}


def compose(request: dict) -> dict:
    """Change only evidence fields; preserve all other keys and override ordering."""
    if version("tomlkit") != "0.15.1":
        raise ValueError("capture requires tomlkit==0.15.1; repair: just sync tools")
    repo = tomlkit.parse(request["repo_text"])
    user = tomlkit.parse(request["user_text"])
    profiles = repo.setdefault("profile", tomlkit.table())
    selected = profiles.setdefault(request["profile"], tomlkit.table())
    junit = selected.setdefault("junit", tomlkit.table())
    junit.update(JUNIT | {"path": request["junit"]})
    # Nextest resolves each override field independently. This matching first override
    # supplies only evidence fields, leaving retries/scheduling/timeouts in their owners.
    overrides = selected.setdefault("overrides", tomlkit.aot())
    managed = tomlkit.table()
    managed["filter"] = "all()"
    managed["junit"] = JUNIT
    overrides.insert(0, managed)

    user.setdefault("experimental", tomlkit.table())["record"] = True
    user.setdefault("record", tomlkit.table()).update(RECORD)
    overrides = user.setdefault("overrides", tomlkit.aot())
    managed = tomlkit.table()
    managed["platform"] = "cfg(all())"
    managed["record"] = RECORD
    overrides.insert(0, managed)
    destination = Path(request["directory"])
    repo_path = destination / "nextest-repo.toml"
    user_path = destination / "nextest-user.toml"
    repo_path.write_text(tomlkit.dumps(repo))
    user_path.write_text(tomlkit.dumps(user))
    return {"repo": str(repo_path), "user": str(user_path)}


if __name__ == "__main__":
    try:
        print(json.dumps(compose(json.load(sys.stdin))))
    except (ValueError, KeyError, TypeError, OSError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1) from error
