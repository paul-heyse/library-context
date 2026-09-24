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


def test_second_ruff_line_fails(tmp_path: Path) -> None:
    """Two ruff lines split the AST types Pyrefly shares with our walker (ADR-0012)."""
    lock = tmp_path / "Cargo.lock"
    ruff = '\n[[package]]\nname = "ruff_python_ast"\nversion = "{}"\n'
    lock.write_text(LOCK + ruff.format("0.0.11") + ruff.format("0.0.13"))
    assert check_family.duplicates(lock) == {"ruff_python_ast": ["0.0.11", "0.0.13"]}
    assert check_family.main([str(lock)]) == 1


EXTRA = """
[[package]]
name = "ruff_python_ast"
version = "0.0.11"

[[package]]
name = "ruff_python_ast"
version = "0.0.14"

[[package]]
name = "ty_python_core"
version = "0.0.14"
dependencies = ["ruff_python_ast 0.0.14", "salsa"]

[[package]]
name = "salsa"
version = "0.28.2"

[[package]]
name = "cpg-flow"
version = "0.1.0"
dependencies = ["ty_python_core", "ruff_python_ast 0.0.14"]
"""


def test_the_declared_flow_family_is_allowed_in_its_scope(tmp_path: Path) -> None:
    """ADR-0002's declared extra family: ruff/ty 0.0.14 only under `cpg-flow` (ADR-0022)."""
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + EXTRA)
    assert check_family.duplicates(lock) == {}
    assert check_family.extra_scope(lock) == []


def test_the_flow_family_outside_its_scope_fails(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    stray = '\n[[package]]\nname = "cpg-core"\nversion = "0.1.0"\n'
    stray += 'dependencies = ["ruff_python_ast 0.0.14"]\n'
    lock.write_text(LOCK + EXTRA + stray)
    problems = check_family.extra_scope(lock)
    assert problems and "cpg-core" in problems[0]


def test_a_salsa_patch_release_fails(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + EXTRA.replace('version = "0.28.2"', 'version = "0.28.4"'))
    assert any("salsa resolves to 0.28.4" in p for p in check_family.extra_scope(lock))
