from __future__ import annotations

from pathlib import Path

import check_family

LOCK = """version = 4

[[package]]
name = "arrow-array"
version = "59.3.0"

[[package]]
name = "datafusion"
version = "55.1.0"

[[package]]
name = "syn"
version = "1.0.0"

[[package]]
name = "syn"
version = "2.0.0"
"""


def test_single_versions_pass_and_unrelated_duplicates_are_ignored(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK)
    assert check_family.duplicates(lock) == {}
    assert check_family.main([str(lock)]) == 0


def test_second_arrow_fails(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + '\n[[package]]\nname = "arrow-array"\nversion = "58.0.0"\n')
    assert check_family.duplicates(lock) == {"arrow-array": ["58.0.0", "59.3.0"]}
    assert check_family.main([str(lock)]) == 1


def test_missing_lockfile_is_not_run_not_failure(tmp_path: Path) -> None:
    assert check_family.main([str(tmp_path / "absent.lock")]) == 0
