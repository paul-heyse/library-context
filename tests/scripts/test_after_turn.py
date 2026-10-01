"""The end-of-turn pipeline (scripts/after_turn.py): stdlib unittest, one file for every repo."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "after_turn.py"


def load_script():  # returns the module
    spec = importlib.util.spec_from_file_location("after_turn", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules["after_turn"] = module
    spec.loader.exec_module(module)
    return module


after_turn = load_script()


def git(repo: Path, *args: str) -> None:
    identity = ["-c", "commit.gpgsign=false", "-c", "user.email=t@t", "-c", "user.name=t"]
    subprocess.run(["git", "-C", str(repo), *identity, *args], check=True, capture_output=True)


def committed_repo(files: dict[str, str]) -> Path:
    repo = Path(tempfile.mkdtemp(prefix="after-turn-"))
    git(repo, "init", "-q")
    for name, text in files.items():
        (repo / name).parent.mkdir(parents=True, exist_ok=True)
        (repo / name).write_text(text)
    git(repo, "add", ".")
    git(repo, "commit", "-qm", "init")
    return repo


class ConfigTests(unittest.TestCase):
    def test_defaults_without_a_config(self) -> None:
        config = after_turn.load_config(Path(tempfile.mkdtemp()))
        self.assertEqual(config.sync, ())
        self.assertEqual(config.claude_model, "claude-sonnet-5-5")
        self.assertEqual(config.codex_model, "gpt-6.1-sol")

    def test_values_and_operator_set(self) -> None:
        root = Path(tempfile.mkdtemp())
        (root / ".config").mkdir()
        (root / ".config" / "after-turn.toml").write_text(
            'sync = ["fmt"]\nready = ["doctor-check"]\noperator_only = ["store-check"]\n'
            'after_outputs = ["docs/catalog.*"]\ncheck_timeout = 5\n'
            '[sync_when]\nbuild-features = ["Cargo.lock"]\n[fixer]\nclaude_default_effort = "max"\n'
        )
        config = after_turn.load_config(root)
        self.assertEqual(config.sync, ("fmt",))
        self.assertEqual(config.sync_when, {"build-features": ("Cargo.lock",)})
        self.assertEqual(config.not_fixable, frozenset({"doctor-check", "store-check"}))
        self.assertEqual(config.check_timeout, 5.0)
        self.assertEqual(config.claude_default_effort, "max")

    def test_hygiene_ids_are_the_recipe_dependencies_in_order(self) -> None:
        deps = [{"recipe": "ruff"}, {"recipe": "clippy"}]
        dump = {"recipes": {"hygiene": {"dependencies": deps}}}
        self.assertEqual(after_turn.checks_from_dump(dump), ["ruff", "clippy"])
        self.assertEqual(after_turn.checks_from_dump({"recipes": {}}), [])


class GuardTests(unittest.TestCase):
    command = after_turn.FIXER_COMMAND

    def test_only_the_assigned_checks_in_the_pinned_form(self) -> None:
        allowed = {"clippy", "types"}
        self.assertIsNone(after_turn.guard_decision(f"{self.command} clippy", allowed))
        self.assertIsNone(after_turn.guard_decision(f"{self.command} clippy types", allowed))
        for command in (
            f"{self.command} ruff",
            f"{self.command} clippy && just test-all",
            "python3 scripts/after_turn.py check clippy",
            "just clippy",
            f"{after_turn.PYTHON} scripts/after_turn.py stop --harness claude",
        ):
            self.assertIsNotNone(after_turn.guard_decision(command, allowed), command)

    def run_guard(self, env: dict[str, str], command: str) -> str:
        payload = json.dumps({"tool_name": "Bash", "tool_input": {"command": command}})
        out = subprocess.run(
            [sys.executable, str(SCRIPT), "guard"],
            input=payload,
            capture_output=True,
            text=True,
            env=env,
            check=True,
        )
        return out.stdout

    def test_hook_denies_in_a_fixer_and_is_silent_otherwise(self) -> None:
        base = {k: v for k, v in os.environ.items() if k != after_turn.ROLE_ENV}
        self.assertEqual(self.run_guard(base, "just test-all"), "")
        fixer = {**base, after_turn.ROLE_ENV: "fixer", after_turn.CHECKS_ENV: "clippy"}
        decision = json.loads(self.run_guard(fixer, "just test-all"))["hookSpecificOutput"]
        self.assertEqual(decision["permissionDecision"], "deny")
        self.assertEqual(self.run_guard(fixer, f"{self.command} clippy"), "")


class FixerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp())
        (self.root / "scripts").mkdir()
        (self.root / "scripts" / "after_turn_fixer.md").write_text("Brief.\n")
        self.config = after_turn.Config(protected=("tools/x",), after_outputs=("docs/c.*",))

    def test_claude_is_sonnet_at_the_session_effort_else_the_default(self) -> None:
        command = after_turn.fixer_command("claude", "xhigh", self.root, self.root, self.config)
        self.assertEqual(command[command.index("--model") + 1], "claude-sonnet-5-5")
        self.assertEqual(command[command.index("--effort") + 1], "xhigh")
        fallback = after_turn.fixer_command("claude", "", self.root, self.root, self.config)
        self.assertEqual(fallback[fallback.index("--effort") + 1], "high")
        settings = json.loads(command[command.index("--settings") + 1])
        self.assertIn(f"Bash({after_turn.FIXER_COMMAND} *)", settings["permissions"]["allow"])

    def test_codex_is_gpt_6_1_sol_at_medium(self) -> None:
        command = after_turn.fixer_command("codex", "xhigh", self.root, self.root, self.config)
        self.assertEqual(command[:2], ["codex", "exec"])
        self.assertEqual(command[command.index("-m") + 1], "gpt-6.1-sol")
        self.assertIn('model_reasoning_effort="medium"', command)
        self.assertIn("--dangerously-bypass-hook-trust", command)

    def test_brief_names_the_protected_paths(self) -> None:
        brief = after_turn.fixer_brief(self.root, self.config)
        self.assertIn("`tools/x`", brief)
        self.assertIn("`docs/c.*`", brief)

    def test_env_is_its_own_session(self) -> None:
        with mock.patch.dict(os.environ, {"CLAUDECODE": "1", "CLAUDE_CODE_SESSION_ID": "parent"}):
            env = after_turn.fixer_env(["clippy", "types"])
        self.assertFalse(any(key.startswith("CLAUDE") for key in env))
        self.assertEqual(env[after_turn.ROLE_ENV], "fixer")
        self.assertEqual(env[after_turn.CHECKS_ENV], "clippy,types")


class ReportTests(unittest.TestCase):
    def test_operator_message_lists_leftovers_and_fixer_changes(self) -> None:
        log = Path(tempfile.mkdtemp()) / "store-check.log"
        log.write_text("$ just store-check\nconnecting\nerror: role lctx_app is missing\n")
        passed = {"status": "passed", "rc": 0, "log": str(log.parent / "none.log")}
        failed = {"status": "failed", "rc": 2, "log": str(log)}
        report = {
            "checks": {"clippy": passed, "store-check": failed},
            "fixer": {"model": "claude-sonnet-5-5", "changed": ["crates/a.rs"]},
        }
        message = after_turn.operator_message(report)
        self.assertIsNotNone(message)
        self.assertIn("1 left: store-check: error: role lctx_app is missing", message)
        self.assertIn("changed 1 file(s): crates/a.rs", message)
        self.assertIsNone(after_turn.operator_message({"checks": {"clippy": passed}}))

    def test_fingerprint_ignores_the_after_outputs(self) -> None:
        repo = committed_repo({"docs/catalog.jsonl": "a\n", "code.py": "x = 1\n"})
        config = after_turn.Config(after_outputs=("docs/catalog.*",))
        base = after_turn.fingerprint(repo, config)
        (repo / "docs" / "catalog.jsonl").write_text("b\n")
        self.assertEqual(after_turn.fingerprint(repo, config), base)
        (repo / "code.py").write_text("x = 2\n")
        self.assertNotEqual(after_turn.fingerprint(repo, config), base)

    def test_sync_when_matches_root_and_nested_manifests(self) -> None:
        repo = committed_repo({"Cargo.toml": "[workspace]\n", "crates/a/Cargo.toml": "[package]\n"})
        patterns = ("Cargo.lock", "**/Cargo.toml")
        self.assertFalse(after_turn.changed_since_head(repo, patterns))
        (repo / "Cargo.toml").write_text("[workspace]\nmembers = []\n")
        self.assertTrue(after_turn.changed_since_head(repo, patterns))
        git(repo, "commit", "-qam", "root")
        (repo / "crates" / "a" / "Cargo.toml").write_text("[package]\nname = 'a'\n")
        self.assertTrue(after_turn.changed_since_head(repo, patterns))


class PromptTests(unittest.TestCase):
    def test_leftovers_shown_once_and_never_in_a_fixer(self) -> None:
        root = Path(tempfile.mkdtemp())
        state = root / "after-turn"
        (state / "checks").mkdir(parents=True)
        log = state / "checks" / "types.log"
        log.write_text("$ just types\nerror: bad type\n")
        failed = {"status": "failed", "rc": 1, "log": str(log)}
        report = {"complete": True, "shown": False, "checks": {"types": failed}}
        (state / "report.json").write_text(json.dumps(report))
        patches = (
            mock.patch.object(after_turn, "repo_root", return_value=root),
            mock.patch.object(after_turn, "state_dir", return_value=state),
            mock.patch.object(after_turn, "read_payload", return_value={}),
        )
        env = {k: v for k, v in os.environ.items() if k != after_turn.ROLE_ENV}
        with contextlib.ExitStack() as stack:
            for patch in patches:
                stack.enter_context(patch)
            stack.enter_context(mock.patch.dict(os.environ, env, clear=True))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(after_turn.cmd_prompt("claude"), 0)
            shown = json.loads(out.getvalue())
            self.assertTrue(shown["systemMessage"].startswith("End-of-turn checks: 1 left: types"))
            # Nothing for the model: the operator sees the message, the model sees no output.
            self.assertEqual(set(shown), {"systemMessage", "suppressOutput"})

            again = io.StringIO()
            with contextlib.redirect_stdout(again):
                after_turn.cmd_prompt("claude")
            self.assertEqual(again.getvalue(), "")  # shown once

            os.environ[after_turn.ROLE_ENV] = "fixer"
            fixer = io.StringIO()
            with contextlib.redirect_stdout(fixer):
                after_turn.cmd_prompt("codex")
            self.assertEqual(fixer.getvalue(), "")

    def test_a_session_waits_only_for_its_own_checks(self) -> None:
        state = Path(tempfile.mkdtemp()) / "after-turn"
        (state / "sessions").mkdir(parents=True)
        after_turn.session_mark(state, "stopped").touch()
        env = {k: v for k, v in os.environ.items() if k != after_turn.ROLE_ENV}

        def prompt(payload: dict[str, str]) -> str:
            out = io.StringIO()
            patches = (
                mock.patch.dict(os.environ, env, clear=True),
                mock.patch.object(after_turn, "repo_root", return_value=state),
                mock.patch.object(after_turn, "state_dir", return_value=state),
                mock.patch.object(after_turn, "read_payload", return_value=payload),
                mock.patch.object(after_turn, "job_busy", return_value=True),
                mock.patch.object(after_turn, "PROMPT_WAIT", 0),
                contextlib.redirect_stdout(out),
            )
            with contextlib.ExitStack() as stack:
                for patch in patches:
                    stack.enter_context(patch)
                after_turn.cmd_prompt("codex")
            return out.getvalue()

        # The job is busy throughout; only the session whose stop requested it is held.
        self.assertIn("still running", prompt({"session_id": "stopped"}))
        self.assertEqual(prompt({"session_id": "new-thread"}), "")
        self.assertIn("still running", prompt({}))  # no session id: wait for any job


class JobTests(unittest.TestCase):
    def test_a_run_releases_only_the_sessions_it_covered(self) -> None:
        state = Path(tempfile.mkdtemp()) / "after-turn"
        (state / "sessions").mkdir(parents=True)
        covered, stopped_again, stopped_later = (
            after_turn.session_mark(state, name) for name in ("a", "b", "c")
        )
        covered.touch()
        stopped_again.touch()
        (state / "job-pending").touch()

        def run_once(*_args: object) -> None:
            # Mid-run, session b stops again and session c stops for the first time.
            later = stopped_again.stat().st_mtime_ns + 1_000_000_000
            os.utime(stopped_again, ns=(later, later))
            stopped_later.touch()

        with contextlib.ExitStack() as stack:
            stack.enter_context(mock.patch.object(after_turn, "repo_root", return_value=state))
            stack.enter_context(mock.patch.object(after_turn, "state_dir", return_value=state))
            stack.enter_context(
                mock.patch.object(after_turn, "load_config", return_value=after_turn.Config())
            )
            stack.enter_context(mock.patch.object(after_turn, "run_job_once", side_effect=run_once))
            self.assertEqual(after_turn.cmd_job("codex", ""), 0)
        self.assertFalse(covered.exists())
        self.assertTrue(stopped_again.exists())
        self.assertTrue(stopped_later.exists())


class StepEnvTests(unittest.TestCase):
    def test_step_env_drops_bash_env_and_keeps_a_shell_level(self) -> None:
        bare = {k: v for k, v in os.environ.items() if k not in {"BASH_ENV", "SHLVL"}}
        with mock.patch.dict(os.environ, {**bare, "BASH_ENV": "/x"}, clear=True):
            env = after_turn.step_env()
        self.assertNotIn("BASH_ENV", env)
        self.assertEqual(env["SHLVL"], "1")
        with mock.patch.dict(os.environ, {**bare, "SHLVL": "3"}, clear=True):
            self.assertEqual(after_turn.step_env()["SHLVL"], "3")

    @unittest.skipUnless(shutil.which("just") and shutil.which("bash"), "needs just and bash")
    def test_a_comment_line_passes_when_a_startup_file_fails(self) -> None:
        shell = 'set shell := ["bash", "-euo", "pipefail", "-c"]\n'
        repo = committed_repo({"justfile": shell + "\nlint:\n    # a comment\n    true\n"})
        startup = repo / "startup.sh"
        startup.write_text(': "$AFTER_TURN_TEST_UNSET"\n')  # fails under set -u, like PS1
        env = {k: v for k, v in os.environ.items() if k != "AFTER_TURN_TEST_UNSET"}
        env["BASH_ENV"] = str(startup)
        # Control: the bare recipe fails on its comment line.
        bare = subprocess.run(["just", "lint"], cwd=repo, env=env, capture_output=True, text=True)
        self.assertNotEqual(bare.returncode, 0, bare.stderr)
        state = repo / "after-turn"
        (state / "checks").mkdir(parents=True)
        with mock.patch.dict(os.environ, env, clear=True):
            result = after_turn.run_check("lint", repo, state, after_turn.Config())
        self.assertEqual(result["status"], "passed", Path(result["log"]).read_text())


if __name__ == "__main__":
    unittest.main()
