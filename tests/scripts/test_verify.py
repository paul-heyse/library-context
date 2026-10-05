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
    import verify

    def unexpected(root):
        raise AssertionError("pure preparation must not inspect native adapter inputs")

    monkeypatch.setattr(verify, "native_input_fingerprints", unexpected)
    commands = []
    assert prepare(set(), lambda command: commands.append(command) or 0) == {}
    assert commands == []


def test_inexact_union_preparation_never_syncs_during_assertions():
    commands = []
    ready = prepare({"tools", "mcp", "semantics", "storage"}, lambda command: commands.append(command) or 0)
    sync = [c for c in commands if c[:2] == ("uv", "sync")]
    assert len(sync) == 2  # Root tools and MCP's actual native dependency closure.
    assert all("--locked" in c and "--inexact" in c for c in sync)
    assert sync[0][-2:] == ("--only-group", "dev")
    assert sync[1][-2:] == ("--package", "lctx-mcp")
    assert all(ready[r] for r in ("tools", "mcp", "semantics", "storage"))
    execute(["model", "analytics"], ready, lambda command: commands.append(command) or 0)
    assert [c for c in commands if c[:2] == ("uv", "sync")] == sync


def test_tools_readiness_is_independent_of_failed_native_preparation():
    def run(command):
        return 1 if "lctx-mcp" in command else 0

    ready = prepare({"tools", "mcp"}, run)
    assert ready["tools"]
    assert not ready["mcp"]


def test_required_families_never_allow_empty_selection():
    for family in FAMILIES.values():
        for command in family.commands:
            if command[:2] == ("cargo", "nextest"):
                assert "--no-tests=fail" in command
                assert "--no-fail-fast" in command


def test_boundary_filter_scopes_real_prerequisites_and_preserves_ordinary_filters():
    commands = []
    assert prerequisites("providers", "flow") == frozenset()
    assert prerequisites("serving", "producer") == frozenset({"postgres"})
    result = execute(["providers"], {}, lambda command: commands.append(command) or 0,
                     ("-E", "test(capture)"), "flow")
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
    result = execute(["model", "analytics"], {}, lambda c: commands.append(c) or (127 if "lctx-model" in c else 0))
    assert result == {"model": "blocked", "analytics": "passed"}
    assert len(commands) == 2


def test_disposable_pg_check_belongs_only_to_the_store_boundary():
    assert prerequisites("store", "python") == frozenset({"postgres", "cli", "tools"})
    assert any("tests/scripts/test_postgres_serving.py" in command for command in FAMILIES["store"].commands)
    assert "--ignore=tests/scripts/test_postgres_serving.py" in FAMILIES["tooling"].commands[0]
