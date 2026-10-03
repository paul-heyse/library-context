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


EXTRA = f"""
[[package]]
name = "ruff_python_ast"
version = "0.0.14"
source = "{check_family.REGISTRY}"

[[package]]
name = "ruff_python_ast"
version = "0.0.16"
source = "{check_family.RUFF_SOURCE}"

[[package]]
name = "ty_python_core"
version = "0.0.16"
source = "{check_family.RUFF_SOURCE}"
dependencies = ["ruff_python_ast 0.0.16", "salsa"]

[[package]]
name = "salsa"
version = "0.28.5"

[[package]]
name = "cpg-flow"
version = "0.1.0"
dependencies = ["ty_python_core", "ruff_python_ast 0.0.16"]

[[package]]
name = "pyrefly"
version = "1.4.0-dev.3"
dependencies = ["ruff_python_ast 0.0.14"]
"""


def test_declared_source_families_are_allowed_in_their_scopes(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + EXTRA)
    assert check_family.duplicates(lock) == {}
    assert check_family.extra_scope(lock) == []


def test_latest_family_outside_its_scope_fails(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    stray = '\n[[package]]\nname = "cpg-core"\nversion = "0.1.0"\n'
    stray += 'dependencies = ["ruff_python_ast 0.0.16"]\n'
    lock.write_text(LOCK + EXTRA + stray)
    assert any("cpg-core" in p and "outside latest" in p for p in check_family.extra_scope(lock))


def test_salsa_patch_release_fails(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + EXTRA.replace('version = "0.28.5"', 'version = "0.28.4"'))
    assert any("salsa resolves to 0.28.4" in p for p in check_family.extra_scope(lock))


def test_equal_version_from_another_source_cannot_split_the_latest_family(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    wrong = f'\n[[package]]\nname = "ruff_python_ast"\nversion = "0.0.16"\nsource = "{check_family.REGISTRY}"\n'
    lock.write_text(LOCK + EXTRA + wrong)
    problems = check_family.extra_scope(lock)
    assert any("undeclared analyzer source" in p for p in problems)
    assert any("ambiguous dependency ruff_python_ast 0.0.16" in p for p in problems)
    qualified = EXTRA.replace('"ruff_python_ast 0.0.16"', f'"ruff_python_ast 0.0.16 ({check_family.RUFF_SOURCE})"')
    lock.write_text(LOCK + qualified + wrong)
    assert not any("ambiguous" in p for p in check_family.extra_scope(lock))
    assert any("undeclared" in p for p in check_family.extra_scope(lock))


def test_bare_dependency_is_not_expanded_to_all_versions(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + EXTRA.replace('"ruff_python_ast 0.0.16"', '"ruff_python_ast"'))
    assert any("ambiguous dependency ruff_python_ast" in p for p in check_family.extra_scope(lock))


def test_embedded_family_cannot_enter_ty(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + EXTRA.replace('dependencies = ["ruff_python_ast 0.0.16", "salsa"]', 'dependencies = ["ruff_python_ast 0.0.14", "salsa"]'))
    assert any("outside Pyrefly embedded Ruff" in p for p in check_family.extra_scope(lock))
