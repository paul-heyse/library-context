from __future__ import annotations

import hashlib

import check_ruff_fork as fork


def test_exact_source_and_digest_are_required() -> None:
    revision = "a" * 40
    patch = b"observational patch"
    digest = hashlib.sha256(patch).hexdigest()
    lock = f'source = "{fork.FORK}?rev={revision}#{revision}"\n'
    driver = f'pub const RUFF_REVISION: &str = "{revision}";\npub const RUFF_PATCH_SHA256: &str = "{digest}";'
    pins = revision + digest
    assert fork.one_revision(lock, driver, pins, patch) == (revision, [])
    assert fork.one_revision(lock, driver, pins, patch + b"changed")[1]
    assert fork.one_revision(lock, driver, "", patch)[1]
    assert (
        fork.one_revision(lock + lock.replace(revision, "b" * 40), driver, pins, patch)[0] is None
    )


def test_repository_names_exact_locked_fork() -> None:
    revision, problems = fork.one_revision(
        fork.LOCK.read_text(),
        fork.DRIVER.read_text(),
        fork.PINS.read_text(),
        fork.PATCH.read_bytes(),
    )
    assert revision is not None and not problems
