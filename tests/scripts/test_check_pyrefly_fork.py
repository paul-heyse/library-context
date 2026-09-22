from __future__ import annotations

import hashlib
from pathlib import Path

import check_pyrefly_fork as fork


def test_literal_const_and_prefix_reads_are_found(tmp_path: Path) -> None:
    (tmp_path / "a.rs").write_text(
        'fn f() { std::env::var("PYREFLY_STACK_SIZE"); }\n'
        'const ENV_VAR: &str = "VIRTUAL_ENV";\nfn g() { std::env::var_os(ENV_VAR); }\n'
        'static P: &str = "PYREFLY_";\n'
    )
    assert fork.env_reads(tmp_path) == {"PYREFLY_STACK_SIZE", "VIRTUAL_ENV", "PYREFLY_"}


def test_unclassified_names_are_reported() -> None:
    known, unknown = fork.classify({"PYREFLY_STACK_SIZE", "SOMETHING_NEW"})
    assert known == {"PYREFLY_STACK_SIZE": "refused"}
    assert unknown == ["SOMETHING_NEW"]


def test_missing_checkout_is_not_run(tmp_path: Path) -> None:
    assert fork.main(["--checkout", str(tmp_path / "absent")]) == 0


REV = "b9f28575ce2baa93dbc416670a34592501b3fcd4"
LOCK = f'source = "{fork.FORK}?rev={REV}#{REV}"\n'
PATCH = b"diff --git a/x b/x\n"
SHA = hashlib.sha256(PATCH).hexdigest()
DRIVER = (
    f'pub const PYREFLY_REV: &str = "{REV}";\n'
    f'pub const PYREFLY_PATCH_SHA256: &str =\n    "{SHA}";\n'
)
PINS = f"| pyrefly | {REV} | {SHA} |"


def test_one_revision_everywhere_passes() -> None:
    assert fork.one_revision(LOCK * 3, DRIVER, PINS, PATCH) == (REV, [])


def test_a_lock_the_driver_does_not_name_fails() -> None:
    other = "0" * 40
    rev, problems = fork.one_revision(LOCK.replace(REV, other), DRIVER, PINS, PATCH)
    assert rev == other
    assert any("PYREFLY_REV" in p for p in problems)
    assert any("pins.md lacks the revision" in p for p in problems)


def test_two_locked_commits_or_a_changed_patch_fail() -> None:
    two = LOCK + LOCK.replace(REV, "1" * 40)
    assert fork.one_revision(two, DRIVER, PINS, PATCH)[0] is None
    _, problems = fork.one_revision(LOCK, DRIVER, PINS, PATCH + b"+more\n")
    assert any("PYREFLY_PATCH_SHA256" in p for p in problems)


def test_the_repository_names_one_revision() -> None:
    rev, problems = fork.one_revision(
        fork.LOCK.read_text(),
        fork.DRIVER.read_text(),
        fork.PINS.read_text(),
        fork.PATCH.read_bytes(),
    )
    assert rev is not None and problems == []
