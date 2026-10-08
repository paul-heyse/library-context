"""Scoped maintenance selects only in-scope work and reports what it skipped (ADR-0134)."""

import pytest

from maintenance import fmt, in_scope, report, turn_end


def recorder():
    commands = []

    def runner(command):
        commands.append(list(command))
        return 0

    return commands, runner


def test_fmt_formats_only_named_files_and_skips_child_modules(tmp_path):
    (tmp_path / "Cargo.toml").write_text('[workspace.package]\nedition = "2024"\n')
    commands, runner = recorder()
    steps = fmt(["crates/a/src/lib.rs", "scripts/x.py", "docs/y.md"], runner, tmp_path)
    assert commands[0] == [
        "rustfmt",
        "--edition",
        "2024",
        "--unstable-features",
        "--skip-children",
        "crates/a/src/lib.rs",
    ]
    assert commands[1] == ["uv", "run", "--no-sync", "ruff", "format", "scripts/x.py"]
    assert all("docs/y.md" not in command for command in commands)
    assert [step.ran for step in steps] == [True, True]


def test_turn_end_runs_generators_only_for_in_scope_inputs(tmp_path):
    (tmp_path / "Cargo.toml").write_text("[workspace]\n")
    commands, runner = recorder()
    steps = turn_end(["docs/plans/p.md"], runner, tmp_path)
    assert commands == []
    assert {step.name: step.ran for step in steps} == {
        "adr-index": False,
        "build-features": False,
        "rustfmt": False,
        "ruff": False,
    }
    commands.clear()
    turn_end(["docs/adr/0134-x.md", "crates/a/Cargo.toml"], runner, tmp_path)
    assert ["just", "adr-index"] in commands and ["just", "build-features"] in commands


def test_report_names_skipped_steps_and_fails_on_failures(capsys):
    from maintenance import Step

    assert report("turn-end", [Step("adr-index", False, "none"), Step("ruff", True, "1")]) == 0
    assert "not performed: adr-index" in capsys.readouterr().out
    assert report("turn-end", [Step("ruff", True, "1", code=1)]) == 1


def test_scope_refuses_paths_outside_the_checkout_and_drops_missing(tmp_path):
    (tmp_path / "a.py").write_text("")
    assert in_scope(["a.py", "missing.py"], tmp_path) == ["a.py"]
    with pytest.raises(SystemExit):
        in_scope(["../outside.py"], tmp_path)
