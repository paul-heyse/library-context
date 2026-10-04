"""Verify independent Ruff/ty's exact source and observational patch (ADR-0117)."""

from __future__ import annotations

import argparse
import hashlib
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UPSTREAM = "3265ed1f944c98bb4c04d632fbefb1257cdb583d"
FORK = "git+https://github.com/paul-heyse/ruff"
PATCH = ROOT / "third_party/ruff-0.16.10.patch"
DRIVER = ROOT / "crates/cpg-extract/src/ruff_context.rs"
LOCK = ROOT / "Cargo.lock"
PINS = ROOT / "docs/pins.md"
LOCKED = re.compile(r'source = "' + re.escape(FORK) + r'\?[^"#]*#([0-9a-f]{40})"')
CONSTANT = re.compile(r'pub const (RUFF_REVISION|RUFF_PATCH_SHA256): &str =\s*"([0-9a-f]+)"')


def one_revision(lock: str, driver: str, pins: str, patch: bytes) -> tuple[str | None, list[str]]:
    revisions = set(LOCKED.findall(lock))
    if len(revisions) != 1:
        return None, [
            f"Cargo.lock has {len(revisions)} independent Ruff revisions: {sorted(revisions)}"
        ]
    revision = revisions.pop()
    constants = dict(CONSTANT.findall(driver))
    digest = hashlib.sha256(patch).hexdigest()
    problems = []
    for key, expected in (("RUFF_REVISION", revision), ("RUFF_PATCH_SHA256", digest)):
        if constants.get(key) != expected:
            problems.append(f"{key} differs from locked source/committed patch")
        if expected not in pins:
            problems.append(f"docs/pins.md lacks {key}")
    return revision, problems


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--checkout", type=Path)
    args = parser.parse_args(argv)
    revision, problems = one_revision(
        LOCK.read_text(), DRIVER.read_text(), PINS.read_text(), PATCH.read_bytes()
    )
    if revision is None or problems:
        for problem in problems:
            print(f"ruff-fork: {problem}")
        return 1
    candidates = sorted(Path.home().glob(f".cargo/git/checkouts/ruff-*/{revision[:7]}"))
    checkout = args.checkout or (candidates[0] if candidates else None)
    if checkout is None or not checkout.exists():
        print("ruff-fork: failed (locked checkout absent; run cargo fetch)")
        return 1

    def git(*arguments: str, stdin: bytes | None = None) -> bytes:
        return subprocess.run(
            ["git", "-C", str(checkout), *arguments], input=stdin, capture_output=True, check=True
        ).stdout

    if git("rev-parse", f"{revision}^").decode().strip() != UPSTREAM:
        problems.append("fork parent differs from reviewed Ruff 0.16.10 upstream")
    observed = git("patch-id", "--stable", stdin=git("diff", UPSTREAM, revision)).split()[:1]
    expected = git("patch-id", "--stable", stdin=PATCH.read_bytes()).split()[:1]
    if not expected or expected != observed:
        problems.append("fork differs from upstream plus the committed observational patch")
    for problem in problems:
        print(f"ruff-fork: {problem}")
    if not problems:
        print(
            f"ruff-fork: passed ({revision[:8]} locked, driver/pins/digest agree, "
            "upstream plus exact patch)"
        )
    return int(bool(problems))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
