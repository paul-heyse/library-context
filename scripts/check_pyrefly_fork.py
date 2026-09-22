"""Check the pinned Pyrefly fork (ADR-0012, DESIGN §4.2.6; review F11, slice-1 review F4).

1. One revision: `Cargo.lock` is the authority for what is built. Every pyrefly crate is locked to
   one fork commit, and the driver's `PYREFLY_REV` (which feeds `producer_id`) and `docs/pins.md`
   name that commit. The driver's `PYREFLY_PATCH_SHA256` and `docs/pins.md` name the sha256 of
   `third_party/pyrefly-1.3.1.patch`.
2. The locked commit in cargo's git checkout is the upstream tag plus exactly the committed patch:
   its parent is the tag, and its `git patch-id --stable` equals the committed patch's (blob-id
   abbreviations differ between clones, so bytes are not compared).
3. Every environment variable the pinned source reads is classified. An unclassified name fails,
   so a pin bump forces the refused-variable list (`cpg_extract::REFUSED_ENV`) to be re-derived.

Usage: check_pyrefly_fork.py [--checkout DIR]   (default: cargo's checkout of the locked rev)
Exit 0 when all hold, 1 on a mismatch. Without a checkout, 1 still runs and 2 and 3 are `not_run`.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TAG_COMMIT = "3e3177d0f4755b56c2d5a710d830eed89b14c2e3"
FORK = "git+https://github.com/paul-heyse/pyrefly"
LOCK = ROOT / "Cargo.lock"
DRIVER = ROOT / "crates" / "cpg-extract" / "src" / "config.rs"
PINS = ROOT / "docs" / "pins.md"
PATCH = ROOT / "third_party" / "pyrefly-1.3.1.patch"

# How each variable is neutralized. A new name must be added deliberately.
CLASSES: dict[str, set[str]] = {
    # Refused by the driver: they change behaviour or output (cpg_extract::REFUSED_ENV + prefix).
    "refused": {
        "PYREFLY_STACK_SIZE",
        "PYREFLY_FIXPOINT_DETAILS",
        "PYSA_DUMP",
        "PYSA_DUMP_CALL_GRAPH",
    },
    # Interpreter discovery: disabled by skip_interpreter_query + explicit site-package path (S2).
    "neutralized_by_config": {"PYTHONPATH", "VIRTUAL_ENV", "CONDA_PREFIX", "PATH"},
    # Read by build scripts at compile time, never at run time.
    "build_time": {"OUT_DIR", "OUT", "TYPESHED_ROOT", "STUBS_ROOT"},
    # Tracing output only; never a fact.
    "log_only": {"PYREFLY_LOG"},
    # CLI argument overrides (`pyrefly_util::args`); the library driver never parses CLI args.
    "cli_only": {"PYREFLY_"},
    # Read only inside #[cfg(test)] modules, benches or doc generators.
    "test_only": {
        "PYDANTIC_TEST_PATH",
        "UPDATE_EXPECT",
        "GLEAN_SNAPSHOTS_WRITE_PATH",
        "ERROR_KINDS_DOC_PATH",
        "DJANGO_TEST_PATH",
        "ATTRS_TEST_PATH",
        "TEST_FILES_PATH",
        "STUBGEN_UPDATE_SNAPSHOTS",
        "STUBGEN_TEST_PATH",
        "SHAPE_EXTENSIONS_TEST_PATH",
        "PYREFLY_BENCH_ROOT",
        "PYREFLY_BENCH_FILE",
        "MARSHMALLOW_TEST_PATH",
        "GLEAN_SNAPSHOTS_PATH",
        "FACTORY_BOY_TEST_PATH",
        "ERROR_PRESETS_PATH",
        "COVERAGE_TEST_WRITE_PATH",
        "COVERAGE_TEST_PATH",
        "CONFIG_DOC_PATH",
        "CINDERX_FIXTURES_PATH",
        "LAZINESS_TEST_PATH",
        "UPDATE_SNAPSHOTS",
    },
}

LITERAL = re.compile(r'env::var(?:_os)?\(\s*"([A-Z0-9_]+)"\s*\)')
CONST_READ = re.compile(r"env::var(?:_os)?\(\s*([A-Z0-9_]+)\s*\)")
CONST_DEF = re.compile(r'const\s+([A-Z0-9_]+)\s*:\s*&(?:\'static\s+)?str\s*=\s*"([A-Z0-9_]+)"')
PREFIX = re.compile(r'"(PYREFLY_|PYSA_DUMP)"')
LOCKED = re.compile(r'source = "' + re.escape(FORK) + r'\?[^"#]*#([0-9a-f]{40})"')
DRIVER_CONST = re.compile(r'pub const (PYREFLY_REV|PYREFLY_PATCH_SHA256): &str =\s*"([0-9a-f]+)"')


def one_revision(lock: str, driver: str, pins: str, patch: bytes) -> tuple[str | None, list[str]]:
    """The locked fork commit, and every way the driver, pins and patch disagree with it."""
    problems = []
    locked = set(LOCKED.findall(lock))
    if len(locked) != 1:
        return None, [f"Cargo.lock locks {len(locked)} fork commits, not one: {sorted(locked)}"]
    rev = locked.pop()
    consts = dict(DRIVER_CONST.findall(driver))
    sha = hashlib.sha256(patch).hexdigest()
    if consts.get("PYREFLY_REV") != rev:
        problems.append(f"PYREFLY_REV is {consts.get('PYREFLY_REV')}, Cargo.lock has {rev}")
    if consts.get("PYREFLY_PATCH_SHA256") != sha:
        problems.append(f"PYREFLY_PATCH_SHA256 differs from {PATCH.name} ({sha[:12]}...)")
    for name, value in (("revision", rev), ("patch sha256", sha)):
        if value not in pins:
            problems.append(f"docs/pins.md lacks the {name} {value[:12]}...")
    return rev, problems


def env_reads(root: Path) -> set[str]:
    """Environment variable names read anywhere in the Pyrefly source tree."""
    names: set[str] = set()
    for f in sorted(root.rglob("*.rs")):
        text = f.read_text(errors="replace")
        consts = dict(CONST_DEF.findall(text))
        names.update(LITERAL.findall(text))
        names.update(consts[c] for c in CONST_READ.findall(text) if c in consts)
        names.update(PREFIX.findall(text))
    return names


def classify(names: set[str]) -> tuple[dict[str, str], list[str]]:
    known = {n: cls for cls, members in CLASSES.items() for n in members}
    return {n: known[n] for n in names if n in known}, sorted(n for n in names if n not in known)


def default_checkout(rev: str) -> Path | None:
    hits = sorted(Path.home().glob(f".cargo/git/checkouts/pyrefly-*/{rev[:7]}"))
    return hits[0] if hits else None


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--checkout", type=Path)
    args = ap.parse_args(argv)
    rev, problems = one_revision(
        LOCK.read_text(), DRIVER.read_text(), PINS.read_text(), PATCH.read_bytes()
    )
    for p in problems:
        print(f"pyrefly-fork: {p}")
    if rev is None or problems:
        return 1
    checkout = args.checkout or default_checkout(rev)
    if checkout is None or not checkout.exists():
        print(f"pyrefly-fork: one revision {rev[:8]}; tag+patch and env: not_run (run cargo fetch)")
        return 0

    def git(*a: str, stdin: bytes | None = None) -> bytes:
        return subprocess.run(
            ["git", "-C", str(checkout), *a], input=stdin, capture_output=True, check=True
        ).stdout

    ok = True
    parent = git("rev-parse", f"{rev}^").decode().strip()
    if parent != TAG_COMMIT:
        print(f"pyrefly-fork: parent of {rev[:8]} is {parent[:8]}, not the 1.3.1 tag")
        ok = False
    fork_id = git("patch-id", "--stable", stdin=git("diff", TAG_COMMIT, rev)).split()[:1]
    ours_id = git("patch-id", "--stable", stdin=PATCH.read_bytes()).split()[:1]
    if not fork_id or fork_id != ours_id:
        print(f"pyrefly-fork: {rev[:8]} is not tag + {PATCH.name} (patch-id differs)")
        ok = False
    _, unknown = classify(env_reads(checkout / "pyrefly") | env_reads(checkout / "crates"))
    for name in unknown:
        print(f"pyrefly-fork: unclassified environment variable {name} (re-derive REFUSED_ENV)")
        ok = False
    if ok:
        print(
            f"pyrefly-fork: ok ({rev[:8]} locked, named by the driver and pins, = tag + patch;"
            " every environment read classified)"
        )
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
