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
        self.assertEqual((config.sync, config.ready, config.after), ((), (), ()))
        self.assertEqual(config.step_timeout, 1200)

    def test_values(self) -> None:
        root = Path(tempfile.mkdtemp())
        (root / ".config").mkdir()
        (root / ".config" / "after-turn.toml").write_text(
            'sync = ["fmt"]\nready = ["doctor-check"]\nafter = ["catalog"]\nstep_timeout = 5\n'
            '[sync_when]\nbuild-features = ["Cargo.lock"]\n'
        )
        config = after_turn.load_config(root)
        self.assertEqual(config.sync, ("fmt",))
        self.assertEqual(config.ready, ("doctor-check",))
        self.assertEqual(config.after, ("catalog",))
        self.assertEqual(config.sync_when, {"build-features": ("Cargo.lock",)})
        self.assertEqual(config.step_timeout, 5.0)


class ReportTests(unittest.TestCase):
    def test_operator_message_lists_failed_steps(self) -> None:
        log = Path(tempfile.mkdtemp()) / "doctor-check.log"
        log.write_text("$ just doctor-check\nchecking\nerror: cargo-nextest is missing\n")
        passed = {"status": "passed", "rc": 0, "log": str(log.parent / "none.log")}
        failed = {"status": "failed", "rc": 2, "log": str(log)}
        report = {"steps": {"fmt": passed, "doctor-check": failed}}
        message = after_turn.operator_message(report)
        self.assertEqual(
            message,
            "End-of-turn steps failed: doctor-check: error: cargo-nextest is missing "
            "(logs: .git/after-turn/)",
        )
        self.assertIsNone(after_turn.operator_message({"steps": {"fmt": passed}}))

    def test_sync_when_detects_committed_root_and_nested_manifests(self) -> None:
        repo = committed_repo({"Cargo.toml": "[workspace]\n", "crates/a/Cargo.toml": "[package]\n"})
        patterns = ("Cargo.lock", "**/Cargo.toml")
        state = Path(tempfile.mkdtemp())
        self.assertTrue(after_turn.sync_needed(repo, state, "build-features", patterns))
        marker = state / "build-features.inputs"
        marker.write_text(after_turn.sync_inputs(repo, patterns))
        self.assertFalse(after_turn.sync_needed(repo, state, "build-features", patterns))
        (repo / "Cargo.toml").write_text("[workspace]\nmembers = []\n")
        git(repo, "commit", "-qam", "root")
        self.assertTrue(after_turn.sync_needed(repo, state, "build-features", patterns))
        marker.write_text(after_turn.sync_inputs(repo, patterns))
        (repo / "crates" / "a" / "Cargo.toml").write_text("[package]\nname = 'a'\n")
        git(repo, "commit", "-qam", "nested")
        self.assertTrue(after_turn.sync_needed(repo, state, "build-features", patterns))

    def test_sync_inputs_detect_untracked_deletion_and_config(self) -> None:
        repo = committed_repo({"Cargo.toml": "[workspace]\n", ".config/hakari.toml": "old\n"})
        patterns = ("**/Cargo.toml", ".config/hakari.toml")
        original = after_turn.sync_inputs(repo, patterns)
        nested = repo / "crates/new/Cargo.toml"
        nested.parent.mkdir(parents=True)
        nested.write_text("[package]\n")
        self.assertNotEqual(after_turn.sync_inputs(repo, patterns), original)
        nested.unlink()
        self.assertEqual(after_turn.sync_inputs(repo, patterns), original)
        (repo / ".config/hakari.toml").write_text("new\n")
        git(repo, "commit", "-qam", "config")
        changed = after_turn.sync_inputs(repo, patterns)
        self.assertNotEqual(changed, original)
        (repo / "Cargo.toml").unlink()
        self.assertNotEqual(after_turn.sync_inputs(repo, patterns), changed)
        git(repo, "commit", "-qam", "delete")
        self.assertNotEqual(after_turn.sync_inputs(repo, patterns), changed)


def patched(stack: contextlib.ExitStack, state: Path, config: object = None) -> None:
    stack.enter_context(mock.patch.object(after_turn, "repo_root", return_value=state))
    stack.enter_context(mock.patch.object(after_turn, "state_dir", return_value=state))
    stack.enter_context(mock.patch.object(after_turn, "read_payload", return_value={}))
    if config is not None:
        stack.enter_context(mock.patch.object(after_turn, "load_config", return_value=config))


class PromptTests(unittest.TestCase):
    def prompt(self, state: Path, harness: str = "claude") -> str:
        out = io.StringIO()
        with contextlib.ExitStack() as stack:
            patched(stack, state)
            stack.enter_context(contextlib.redirect_stdout(out))
            self.assertEqual(after_turn.cmd_prompt(harness), 0)
        return out.getvalue()

    def test_failed_steps_shown_once_to_the_operator_only(self) -> None:
        state = Path(tempfile.mkdtemp()) / "after-turn"
        (state / "steps").mkdir(parents=True)
        log = state / "steps" / "doctor-check.log"
        log.write_text("$ just doctor-check\nerror: tool missing\n")
        failed = {"status": "failed", "rc": 1, "log": str(log)}
        (state / "report.json").write_text(json.dumps({"shown": False, "steps": {"x": failed}}))
        shown = json.loads(self.prompt(state))
        self.assertTrue(shown["systemMessage"].startswith("End-of-turn steps failed: x: error"))
        # Nothing for the model: the operator sees the message, the model sees no output.
        self.assertEqual(set(shown), {"systemMessage", "suppressOutput"})
        self.assertEqual(self.prompt(state), "")  # shown once

    def test_silent_without_a_report_or_a_failure(self) -> None:
        state = Path(tempfile.mkdtemp()) / "after-turn"
        (state / "steps").mkdir(parents=True)
        self.assertEqual(self.prompt(state, "codex"), "")
        passed = {"status": "passed", "rc": 0, "log": "-"}
        (state / "report.json").write_text(json.dumps({"shown": False, "steps": {"x": passed}}))
        self.assertEqual(self.prompt(state, "codex"), "")
        self.assertTrue(json.loads((state / "report.json").read_text())["shown"])


class StopTests(unittest.TestCase):
    def test_successful_generator_records_resulting_inputs_and_retries_failures(self) -> None:
        repo = committed_repo({"Cargo.toml": "[workspace]\n"})
        state = Path(tempfile.mkdtemp())
        config = after_turn.Config(sync=("build-features",), sync_when={"build-features": ("Cargo.lock", "**/Cargo.toml")})
        outcomes = iter(["failed", "passed", "passed"])
        ran = []

        def step(name, *_args):
            ran.append(name)
            outcome = next(outcomes)
            if outcome == "passed":
                # The real generator also updates watched manifests/lockfiles.
                (repo / "Cargo.lock").write_text("resolved\n")
            return {"status": outcome, "rc": int(outcome != "passed"), "log": "-"}

        real_popen = subprocess.Popen

        def popen(command, *args, **kwargs):
            if command[0] == sys.executable and command[1] == str(SCRIPT):
                return mock.Mock()
            return real_popen(command, *args, **kwargs)

        with contextlib.ExitStack() as stack:
            patched(stack, state, config)
            stack.enter_context(mock.patch.object(after_turn, "repo_root", return_value=repo))
            stack.enter_context(mock.patch.object(after_turn, "run_step", side_effect=step))
            stack.enter_context(mock.patch.object(after_turn.subprocess, "Popen", side_effect=popen))
            after_turn.cmd_stop("codex")
            self.assertFalse((state / "build-features.inputs").exists())
            after_turn.cmd_stop("codex")
            self.assertEqual(len(ran), 2)
            after_turn.cmd_stop("codex")
            self.assertEqual(len(ran), 2, "unchanged successful generation must not rerun")
            (repo / "Cargo.toml").write_text("[workspace]\nmembers = []\n")
            git(repo, "commit", "-qam", "dependency change before stop")
            after_turn.cmd_stop("codex")
            self.assertEqual(len(ran), 3, "committing the change must not hide it")


class JobTests(unittest.TestCase):
    def test_a_burst_runs_ready_once_then_after(self) -> None:
        state = Path(tempfile.mkdtemp()) / "after-turn"
        (state / "steps").mkdir(parents=True)
        (state / "sync.json").write_text(json.dumps({"fmt": {"status": "passed"}}))
        (state / "job-pending").touch()
        config = after_turn.Config(ready=("doctor-check",), after=("catalog",))
        ran: list[str] = []

        def step(name: str, *_args: object) -> dict[str, object]:
            ran.append(name)
            # A stop during this run asks for one more.
            if len(ran) == 1:
                (state / "job-pending").touch()
            return {"status": "passed", "rc": 0, "log": "-"}

        def logged(command: list[str], *_args: object, **_kwargs: object) -> int:
            ran.append(command[-1])
            return 0

        with contextlib.ExitStack() as stack:
            patched(stack, state, config)
            stack.enter_context(mock.patch.object(after_turn, "run_step", side_effect=step))
            stack.enter_context(mock.patch.object(after_turn, "run_logged", side_effect=logged))
            self.assertEqual(after_turn.cmd_job("claude"), 0)
        self.assertEqual(ran, ["doctor-check", "doctor-check", "catalog"])
        report = json.loads((state / "report.json").read_text())
        self.assertEqual(set(report["steps"]), {"fmt", "doctor-check"})
        self.assertFalse(report["shown"])


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
        (state / "steps").mkdir(parents=True)
        with mock.patch.dict(os.environ, env, clear=True):
            result = after_turn.run_step("lint", repo, state, after_turn.Config())
        self.assertEqual(result["status"], "passed", Path(result["log"]).read_text())


if __name__ == "__main__":
    unittest.main()
