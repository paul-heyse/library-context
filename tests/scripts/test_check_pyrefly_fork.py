from __future__ import annotations

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
