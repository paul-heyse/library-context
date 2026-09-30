from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest

import after_turn

ROOT = Path(__file__).resolve().parents[2]


def test_hygiene_ids_are_the_recipe_dependencies_in_order() -> None:
    dump = {"recipes": {"hygiene": {"dependencies": [{"recipe": "ruff"}, {"recipe": "clippy"}]}}}
    assert after_turn.checks_from_dump(dump) == ["ruff", "clippy"]
    live = after_turn.hygiene_checks(ROOT)
    assert "clippy" in live and "store-check" in live and "types" in live


def test_guard_allows_only_the_assigned_checks() -> None:
    allowed = {"clippy", "types"}
    assert after_turn.guard_decision("python3 scripts/after_turn.py check clippy", allowed) is None
    assert (
        after_turn.guard_decision("python3 scripts/after_turn.py check clippy types", allowed)
        is None
    )
    for command in (
        "python3 scripts/after_turn.py check ruff",
        "python3 scripts/after_turn.py check clippy && just test-all",
        "just clippy",
        "cargo clippy",
        "python3 scripts/after_turn.py stop --harness claude",
    ):
        assert after_turn.guard_decision(command, allowed), command


def run_guard(env: dict[str, str], payload: dict[str, object]) -> str:
    out = subprocess.run(
        ["python3", str(ROOT / "scripts" / "after_turn.py"), "guard"],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        env=env,
        check=True,
    )
    return out.stdout


def test_guard_denies_in_a_fixer_and_is_silent_otherwise(monkeypatch: pytest.MonkeyPatch) -> None:
    payload: dict[str, object] = {"tool_name": "Bash", "tool_input": {"command": "just test-all"}}
    monkeypatch.delenv(after_turn.ROLE_ENV, raising=False)
    base = dict(os.environ)
    assert run_guard(base, payload) == ""
    fixer = {**base, after_turn.ROLE_ENV: "fixer", after_turn.CHECKS_ENV: "clippy"}
    decision = json.loads(run_guard(fixer, payload))["hookSpecificOutput"]
    assert decision["permissionDecision"] == "deny"
    allowed: dict[str, object] = {
        "tool_name": "Bash",
        "tool_input": {"command": "python3 scripts/after_turn.py check clippy"},
    }
    assert run_guard(fixer, allowed) == ""


def test_claude_fixer_is_sonnet_at_the_session_effort_else_high(tmp_path: Path) -> None:
    command = after_turn.fixer_command("claude", "xhigh", ROOT, tmp_path)
    assert command[command.index("--model") + 1] == "claude-sonnet-5-5"
    assert command[command.index("--effort") + 1] == "xhigh"
    fallback = after_turn.fixer_command("claude", "", ROOT, tmp_path)
    assert fallback[fallback.index("--effort") + 1] == "high"
    settings = json.loads(command[command.index("--settings") + 1])
    assert "Bash(python3 scripts/after_turn.py check *)" in settings["permissions"]["allow"]


def test_codex_fixer_is_gpt_6_1_sol_at_medium(tmp_path: Path) -> None:
    command = after_turn.fixer_command("codex", "xhigh", ROOT, tmp_path)
    assert command[:2] == ["codex", "exec"]
    assert command[command.index("-m") + 1] == "gpt-6.1-sol"
    assert 'model_reasoning_effort="medium"' in command
    assert "--dangerously-bypass-hook-trust" in command


def test_fixer_env_is_its_own_session(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("CLAUDECODE", "1")
    monkeypatch.setenv("CLAUDE_CODE_SESSION_ID", "parent")
    env = after_turn.fixer_env(["clippy", "types"])
    assert not any(key.startswith("CLAUDE") for key in env)
    assert env[after_turn.ROLE_ENV] == "fixer"
    assert env[after_turn.CHECKS_ENV] == "clippy,types"


def test_operator_message_lists_leftovers_and_fixer_changes(tmp_path: Path) -> None:
    log = tmp_path / "store-check.log"
    log.write_text("$ just store-check\nconnecting\nerror: role lctx_app is missing\n")
    passed = {"status": "passed", "rc": 0, "log": str(tmp_path / "none.log")}
    report = {
        "checks": {"clippy": passed, "store-check": {"status": "failed", "rc": 2, "log": str(log)}},
        "fixer": {"model": "claude-sonnet-5-5", "changed": ["crates/a.rs"]},
    }
    message = after_turn.operator_message(report)
    assert message is not None
    assert "1 left: store-check: error: role lctx_app is missing" in message
    assert "changed 1 file(s): crates/a.rs" in message
    assert after_turn.operator_message({"checks": {"clippy": passed}, "fixer": None}) is None


def test_operator_only_checks_never_reach_the_fixer() -> None:
    for check in ("store-check", "gold", "tools", "postgres-images", "adr-index"):
        assert check in after_turn.OPERATOR_ONLY
    for check in ("clippy", "ruff", "types", "fmt", "docs-check", "lint-agents"):
        assert check not in after_turn.OPERATOR_ONLY


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def test_fingerprint_ignores_the_catalog_outputs(tmp_path: Path) -> None:
    git(tmp_path, "init", "-q")
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "library-utilization.jsonl").write_text("a\n")
    (tmp_path / "code.py").write_text("x = 1\n")
    git(tmp_path, "add", ".")
    git(tmp_path, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "init")
    base = after_turn.fingerprint(tmp_path)
    (tmp_path / "docs" / "library-utilization.jsonl").write_text("b\n")
    assert after_turn.fingerprint(tmp_path) == base
    (tmp_path / "code.py").write_text("x = 2\n")
    assert after_turn.fingerprint(tmp_path) != base


def test_prompt_shows_leftovers_once_and_never_in_a_fixer(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    state = tmp_path / "after-turn"
    (state / "checks").mkdir(parents=True)
    log = state / "checks" / "types.log"
    log.write_text("$ just types\nerror: bad type\n")
    report = {
        "complete": True,
        "shown": False,
        "checks": {"types": {"status": "failed", "rc": 1, "log": str(log)}},
    }
    (state / "report.json").write_text(json.dumps(report))
    monkeypatch.setattr(after_turn, "repo_root", lambda: tmp_path)
    monkeypatch.setattr(after_turn, "state_dir", lambda root: state)
    monkeypatch.setattr(after_turn, "read_payload", lambda: {})
    monkeypatch.delenv(after_turn.ROLE_ENV, raising=False)

    assert after_turn.cmd_prompt("claude") == 0
    shown = json.loads(capsys.readouterr().out)
    assert shown["systemMessage"].startswith("End-of-turn checks: 1 left: types: error: bad type")
    assert shown["suppressOutput"] is True
    assert set(shown) == {"systemMessage", "suppressOutput"}  # nothing for the model

    assert after_turn.cmd_prompt("claude") == 0
    assert capsys.readouterr().out == ""  # shown once

    monkeypatch.setenv(after_turn.ROLE_ENV, "fixer")
    assert after_turn.cmd_prompt("codex") == 0
    assert capsys.readouterr().out == ""
