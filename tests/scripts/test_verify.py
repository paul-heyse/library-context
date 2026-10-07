"""Launcher behavior, distinct from real family qualification."""

from verify import FAMILIES, environment_owner, execute, prepare, prerequisites


def test_independent_failure_does_not_skip_remaining_families():
    commands = []

    def run(command):
        commands.append(command)
        return 1 if "lctx-model" in command else 0

    result = execute(["model", "analytics"], {}, run)
    assert result == {"model": "failed", "analytics": "passed"}
    assert len(commands) == 2


def test_readiness_failure_blocks_dependents_but_pure_controls_still_run():
    commands = []

    def run(command):
        commands.append(command)
        return 0

    result = execute(["providers", "model"], {"tools": False}, run)
    assert result == {"providers": "blocked", "model": "passed"}
    assert commands == list(FAMILIES["model"].commands)


def test_pure_preparation_invokes_no_environment_or_store_setup(monkeypatch):

    commands = []
    assert prepare(set(), lambda command: commands.append(command) or 0) == {}
    assert commands == []


def test_compiler_readiness_has_no_retired_store_or_binding_setup():
    commands = []
    ready = prepare({"tools", "cli"}, lambda command: commands.append(command) or 0)
    assert ready == {"tools": True, "cli": True}
    assert commands == [
        ("uv", "sync", "--locked", "--inexact", "--only-group", "dev"),
        ("cargo", "build", "--release", "-p", "lctx"),
    ]


def test_native_readiness_failure_blocks_both_effect_families():
    commands = []
    ready = prepare(
        {"native-store", "native-serving"}, lambda command: commands.append(command) or 1
    )
    assert ready == {"native-store": False, "native-serving": False}
    assert execute(["store", "serving"], ready, lambda command: commands.append(command) or 0) == {
        "store": "blocked",
        "serving": "blocked",
    }
    assert len(commands) == 1 and commands[0][:3] == ("docker", "image", "inspect")


def test_required_families_never_allow_empty_selection():
    for family in FAMILIES.values():
        for command in family.commands:
            if command[:2] == ("cargo", "nextest"):
                assert "--no-tests=fail" in command
                assert "--no-fail-fast" in command


def test_boundary_filter_scopes_real_prerequisites_and_preserves_ordinary_filters():
    commands = []
    assert prerequisites("providers", "flow") == frozenset()
    assert prerequisites("compiler", "producer") == frozenset({"tools", "native-store"})
    result = execute(
        ["providers"],
        {},
        lambda command: commands.append(command) or 0,
        ("-E", "test(capture)"),
        "flow",
    )
    assert result == {"providers": "passed"}
    assert len(commands) == 1 and "cpg-flow" in commands[0]
    assert commands[0][-2:] == ("-E", "test(capture)")


def test_pure_family_owns_no_python_environment_directory(tmp_path, monkeypatch):
    import verify

    monkeypatch.setattr(verify, "ROOT", tmp_path)
    with environment_owner(False):
        assert not (tmp_path / ".venv").exists()
    with environment_owner(True):
        assert (tmp_path / ".venv/.verification.lock").is_file()


def test_missing_executable_is_reported_and_other_families_continue(monkeypatch):
    import verify

    def missing(*args, **kwargs):
        raise FileNotFoundError("deliberately absent tool")

    monkeypatch.setattr(verify.subprocess, "run", missing)
    assert verify.launch(("absent-contract-tool",), {}) == 127
    commands = []
    result = execute(
        ["model", "analytics"],
        {},
        lambda c: commands.append(c) or (127 if "lctx-model" in c else 0),
    )
    assert result == {"model": "blocked", "analytics": "passed"}
    assert len(commands) == 2


def test_compiler_selections_reference_current_binaries():
    import verify

    for family in ("compiler", "providers"):
        for command in FAMILIES[family].commands:
            if "-p" not in command:
                assert command[:5] == ("uv", "run", "--no-sync", "python", "scripts/native_controls.py")
                continue
            package = command[command.index("-p") + 1]
            for index, argument in enumerate(command):
                if argument == "--test":
                    assert (
                        verify.ROOT / "crates" / package / "tests" / (command[index + 1] + ".rs")
                    ).is_file()
    assert all(
        "postgres" not in prerequisites("compiler", boundary)
        for boundary in (None, "producer", "cli")
    )
    assert FAMILIES["store"].commands and FAMILIES["serving"].commands
    assert all(
        command[:5] == ("uv", "run", "--no-sync", "python", "scripts/native_controls.py")
        for family in ("store", "serving")
        for command in FAMILIES[family].commands
    )


def test_explicit_native_targets_replace_family_defaults():
    from native_controls import compiler_packages

    for boundary, package in (
        ("compiler", "cpg-core"),
        ("compiler-cli", "lctx"),
        ("providers", "cpg-extract"),
    ):
        for selected in (["--lib"], ["--test", "chosen"], ["--test=chosen"]):
            assert compiler_packages(boundary, selected) == ["-p", package]
        defaults = compiler_packages(boundary, ["-E", "test(chosen)"])
        assert defaults[:2] == ["-p", package]
        assert len(defaults) > 2
