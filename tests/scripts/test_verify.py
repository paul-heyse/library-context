"""Launcher behavior, distinct from real family qualification."""

from verify import FAMILIES, environment_owner, execute, prerequisites, readiness
from workspace_env import Readiness


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


def observed(ready_requirements, calls):
    def observe(requirement):
        calls.append(requirement)
        return Readiness(
            requirement,
            "native" if requirement == "native-python" else requirement,
            requirement in ready_requirements,
            (),
            "environment is outdated",
        )

    return observe


def test_pure_readiness_observes_nothing():
    commands, calls = [], []
    assert readiness(set(), lambda c: commands.append(c) or 0, observed(set(), calls)) == {}
    assert commands == [] and calls == []


def test_readiness_observes_and_never_synchronizes(capsys):
    commands, calls = [], []
    ready = readiness(
        {"tools", "native-python", "cli"},
        lambda command: commands.append(command) or 0,
        observed({"tools"}, calls),
    )
    assert ready == {"tools": True, "native-python": False, "cli": True}
    assert calls == ["tools", "native-python"]
    # The product binary build remains a run step; nothing runs `uv sync`.
    assert commands == [("cargo", "build", "--release", "-p", "lctx")]
    assert not any(command[:2] == ("uv", "sync") for command in commands)
    assert "native-python: blocked: run just sync native" in capsys.readouterr().out


def test_blocked_boundary_names_its_repair_route(capsys):
    result = execute(["serving"], {"tools": True, "native-serving": True}, lambda c: 0, (), "mcp")
    assert result == {"serving": "blocked"}
    assert "native-python: just sync native" in capsys.readouterr().out


def test_native_readiness_failure_blocks_both_effect_families():
    commands = []
    ready = readiness(
        {"native-store", "native-serving"},
        lambda command: commands.append(command) or 1,
        observed(set(), []),
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


def test_ownership_follows_boundary_requirements(tmp_path, monkeypatch):
    import workspace_env

    monkeypatch.setenv("XDG_RUNTIME_DIR", str(tmp_path))
    monkeypatch.delenv("LCTX_ENV_OWNERSHIP", raising=False)
    locks = tmp_path / "library-context" / "locks"
    # Pure Rust (and store-only) boundaries take no lock at all.
    for name, command in (("model", None), ("providers", "flow"), ("tooling", "docs")):
        with environment_owner(prerequisites(name, command)) as owned:
            assert owned.resources == () and owned.acquired == ()
    assert not locks.exists()
    with environment_owner(prerequisites("tooling", "python")) as owned:
        assert [r.kind for r in owned.acquired] == ["environment"]
        assert owned.mode == "shared"
    with environment_owner(prerequisites("serving", "mcp")) as owned:
        assert [r.kind for r in owned.acquired] == ["environment", "extension"]
        assert owned.resources[0].path == workspace_env.environment_path()


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
                assert command[:5] == (
                    "uv",
                    "run",
                    "--no-sync",
                    "python",
                    "scripts/native_controls.py",
                )
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


def test_native_cargo_configuration_reaches_cargo_and_nextest_only():
    from native_controls import cargo_command

    override = "profile.release.package.cpg-core.opt-level=0"
    assert cargo_command(["cargo", "test", "--release"], [override]) == [
        "cargo",
        "--config",
        override,
        "test",
        "--release",
    ]
    assert cargo_command(["cargo", "nextest", "run", "--release"], [override]) == [
        "cargo",
        "nextest",
        "run",
        "--config",
        override,
        "--release",
    ]
    command = ["uv", "run", "--no-sync", "pytest"]
    assert cargo_command(command, [override]) == command
    assert cargo_command(["cargo", "test"], []) == ["cargo", "test"]
