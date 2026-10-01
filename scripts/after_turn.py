#!/usr/bin/env python3
"""End-of-turn pipeline for Claude Code and Codex.

The hooks run only automatic steps, in one place, when the main agent stops:

- ``stop`` (Stop hook) runs the ``sync`` recipes (generators and formatting), then starts the job;
- the job runs the ``ready`` recipes (readiness: tools, images, services), writes a report, then
  runs the ``after`` recipes without waiting on them;
- ``prompt`` (UserPromptSubmit hook) never waits; it shows the operator, never the model, the
  steps that failed in the last report.

Non-functional checks (``just hygiene``: type errors, lint, policy and generated-file checks) are
not run or fixed here: agents run ``just hygiene`` once at scope end and fix what fails.

Repository facts live in ``.config/after-turn.toml``; this file is the same in every repository
(canonical copy: project-template). Nothing reaches the main agent's context and every hook exits
0. State and logs live in the worktree's git directory under ``after-turn/``. Hooks run it on
Python 3.14 through ``uv run --no-project --python 3.14 python``.
"""

from __future__ import annotations

import argparse
import contextlib
import dataclasses
import datetime
import fcntl
import json
import os
import re
import signal
import subprocess
import sys
import time
import tomllib
from collections.abc import Iterator, Sequence
from pathlib import Path
from typing import Any

CONFIG_PATH = Path(".config") / "after-turn.toml"


@dataclasses.dataclass(frozen=True)
class Config:
    """The repository's steps, read from ``.config/after-turn.toml``; each step is a just recipe."""

    sync: tuple[str, ...] = ()
    sync_when: dict[str, tuple[str, ...]] = dataclasses.field(default_factory=dict)
    ready: tuple[str, ...] = ()
    after: tuple[str, ...] = ()
    step_timeout: float = 1200


def load_config(root: Path) -> Config:
    path = root / CONFIG_PATH
    if not path.exists():
        return Config()
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    return Config(
        sync=tuple(data.get("sync", ())),
        sync_when={k: tuple(v) for k, v in data.get("sync_when", {}).items()},
        ready=tuple(data.get("ready", ())),
        after=tuple(data.get("after", ())),
        step_timeout=float(data.get("step_timeout", Config.step_timeout)),
    )


Payload = dict[str, Any]
Report = dict[str, Any]


def now() -> str:
    return datetime.datetime.now().astimezone().isoformat(timespec="seconds")


def repo_root() -> Path:
    env = os.environ.get("CLAUDE_PROJECT_DIR")
    if env:
        return Path(env)
    out = subprocess.run(
        ["git", "-C", str(Path(__file__).resolve().parent), "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
        check=True,
    )
    return Path(out.stdout.strip())


def state_dir(root: Path) -> Path:
    out = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "--absolute-git-dir"],
        capture_output=True,
        text=True,
        check=True,
    )
    path = Path(out.stdout.strip()) / "after-turn"
    (path / "steps").mkdir(parents=True, exist_ok=True)
    return path


def read_payload() -> Payload:
    try:
        data = json.loads(sys.stdin.read() or "{}")
    except OSError, ValueError:
        return {}
    return data if isinstance(data, dict) else {}


def append_log(state: Path, name: str, line: str) -> None:
    with (state / name).open("a", encoding="utf-8") as fh:
        fh.write(f"{now()} {line}\n")


@contextlib.contextmanager
def try_lock(path: Path) -> Iterator[bool]:
    """Hold an exclusive lock on ``path`` if it is free; yield whether it was acquired."""
    with path.open("a") as fh:
        try:
            fcntl.flock(fh, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            yield False
            return
        try:
            yield True
        finally:
            fcntl.flock(fh, fcntl.LOCK_UN)


# --- running steps -----------------------------------------------------------------------------


def step_env() -> dict[str, str]:
    """The environment for steps, in which a recipe's shell reads no startup file.

    Bash reads ``BASH_ENV`` in every non-interactive shell, and Debian's bash reads the system
    bashrc in a top-level ``bash -c`` (``SHLVL`` unset) that looks remote: ``SSH_CLIENT`` set or
    a socket on stdin. Recipe shells under Codex Desktop's hooks read it; under ``set -u`` it fails
    on ``PS1``, and a recipe's comment line then exits 1.
    """
    env = {k: v for k, v in os.environ.items() if k != "BASH_ENV"}
    if not env.get("SHLVL", "").isdigit() or int(env["SHLVL"]) < 1:
        env["SHLVL"] = "1"
    return env


def run_logged(
    command: Sequence[str], root: Path, log: Path, *, timeout: float | None = None
) -> int:
    """Run ``command`` in ``root`` with its output in ``log``; kill it after ``timeout`` s."""
    with log.open("w", encoding="utf-8") as fh:
        fh.write(f"$ {' '.join(command)}\n")
        fh.flush()
        proc = subprocess.Popen(
            list(command),
            cwd=root,
            env=step_env(),
            stdin=subprocess.DEVNULL,
            stdout=fh,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=True,
        )
        try:
            proc.wait(timeout=timeout)
            return proc.returncode
        except subprocess.TimeoutExpired:
            with contextlib.suppress(ProcessLookupError):
                os.killpg(proc.pid, signal.SIGKILL)
            proc.wait()
            fh.write(f"\n[after-turn] killed after {timeout:.0f} s\n")
            return 124


def run_step(step: str, root: Path, state: Path, config: Config) -> dict[str, Any]:
    log = state / "steps" / f"{step}.log"
    started = time.monotonic()
    try:
        rc = run_logged(["just", step], root, log, timeout=config.step_timeout)
    except Exception as exc:  # a step that cannot start is a failed step
        log.write_text(f"$ just {step}\nerror: could not run the step: {exc!r}\n")
        rc = 127
    return {
        "status": "passed" if rc == 0 else "failed",
        "rc": rc,
        "seconds": round(time.monotonic() - started, 1),
        "log": str(log),
    }


def changed_since_head(root: Path, patterns: Sequence[str]) -> bool:
    """Whether any file matching the glob ``patterns`` differs from HEAD or is untracked."""
    specs = [f":(glob){p}" for p in patterns]
    diff = subprocess.run(["git", "diff", "--quiet", "HEAD", "--", *specs], cwd=root)
    untracked = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard", "--", *specs],
        cwd=root,
        capture_output=True,
        text=True,
    )
    return diff.returncode != 0 or bool(untracked.stdout.strip())


def load_report(state: Path) -> Report | None:
    try:
        report = json.loads((state / "report.json").read_text())
    except OSError, ValueError:
        return None
    return report if isinstance(report, dict) else None


def save_report(state: Path, report: Report) -> None:
    tmp = state / "report.json.tmp"
    tmp.write_text(json.dumps(report, indent=2) + "\n")
    tmp.replace(state / "report.json")


# --- operator messages -------------------------------------------------------------------------


def first_finding(log: str) -> str:
    lines = [line.strip() for line in log.splitlines()[1:] if line.strip()]
    for line in lines:
        if re.search(r"\b(error|failed|missing|blocked|finding)", line, re.IGNORECASE):
            return line[:160]
    return lines[-1][:160] if lines else ""


def failed_steps(report: Report) -> list[str]:
    return [sid for sid, result in report.get("steps", {}).items() if result["status"] != "passed"]


def operator_message(report: Report, logs: str = ".git/after-turn/") -> str | None:
    failed = failed_steps(report)
    if not failed:
        return None
    details = []
    for sid in failed:
        log = Path(report["steps"][sid]["log"])
        text = log.read_text(errors="replace") if log.exists() else ""
        details.append(f"{sid}: {first_finding(text)}".rstrip(": "))
    return "End-of-turn steps failed: " + "; ".join(details) + f" (logs: {logs})"


def logs_label(root: Path, state: Path) -> str:
    try:
        return str(state.relative_to(root)) + "/"
    except ValueError:
        return str(state) + "/"


# --- subcommands -------------------------------------------------------------------------------


def cmd_stop(harness: str) -> int:
    payload = read_payload()
    root = repo_root()
    state = state_dir(root)
    config = load_config(root)
    # What decides whether a recipe's bash reads startup files (see step_env).
    shell = " ".join(f"{k}={os.environ.get(k, '-')}" for k in ("SHLVL", "BASH_ENV", "SSH_CLIENT"))
    session = payload.get("session_id") or "-"
    append_log(state, "stop.log", f"{harness} session={session} payload={sorted(payload)} {shell}")
    results: dict[str, Any] = {}
    for step in config.sync:
        when = config.sync_when.get(step)
        if when and not changed_since_head(root, when):
            continue
        results[step] = run_step(step, root, state, config)
    (state / "sync.json").write_text(json.dumps(results, indent=2) + "\n")
    (state / "job-pending").touch()
    subprocess.Popen(
        [sys.executable, str(Path(__file__).resolve()), "job", "--harness", harness],
        cwd=root,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=(state / "job.err").open("a"),
        start_new_session=True,
    )
    return 0


def cmd_job(harness: str) -> int:
    root = repo_root()
    state = state_dir(root)
    config = load_config(root)
    pending = state / "job-pending"
    while pending.exists():
        with try_lock(state / "job.lock") as acquired:
            if not acquired:
                return 0  # the running job re-checks the pending mark after releasing its lock
            while pending.exists():
                pending.unlink(missing_ok=True)
                run_job_once(harness, root, state, config)
    run_after(root, state, config)
    return 0


def run_job_once(harness: str, root: Path, state: Path, config: Config) -> None:
    sync = json.loads((state / "sync.json").read_text()) if (state / "sync.json").exists() else {}
    steps: dict[str, Any] = dict(sync)
    for step in config.ready:
        steps[step] = run_step(step, root, state, config)
    report: Report = {"id": now(), "harness": harness, "shown": False, "steps": steps}
    report["finished"] = now()
    save_report(state, report)
    append_log(state, "job.log", f"failed={failed_steps(report)}")


def run_after(root: Path, state: Path, config: Config) -> None:
    """Run the ``after`` recipes once per burst of stops; nothing waits on them."""
    if not config.after:
        return
    (state / "after-pending").touch()
    with try_lock(state / "after.lock") as acquired:
        if not acquired:
            return
        while (state / "after-pending").exists():
            (state / "after-pending").unlink(missing_ok=True)
            for step in config.after:
                run_logged(["just", step], root, state / f"{step}.log")


def cmd_prompt(harness: str) -> int:
    read_payload()
    root = repo_root()
    state = state_dir(root)
    report = load_report(state)
    if not report or report.get("shown"):
        return 0
    message = operator_message(report, logs_label(root, state))
    report["shown"] = True
    save_report(state, report)
    if message:
        output: dict[str, Any] = {"systemMessage": message}
        if harness == "claude":
            output["suppressOutput"] = True
        print(json.dumps(output))
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="End-of-turn pipeline for Claude Code and Codex")
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("stop", "prompt", "job"):
        sub.add_parser(name).add_argument("--harness", choices=("claude", "codex"), required=True)
    args = parser.parse_args(argv)
    # Hooks never fail the harness: errors go to a log, never to the agent.
    try:
        if args.command == "stop":
            return cmd_stop(args.harness)
        if args.command == "prompt":
            return cmd_prompt(args.harness)
        return cmd_job(args.harness)
    except Exception as exc:  # a hook must not surface errors to the agent
        with contextlib.suppress(Exception):
            append_log(state_dir(repo_root()), "error.log", f"{args.command}: {exc!r}")
        return 0


if __name__ == "__main__":
    sys.exit(main())
