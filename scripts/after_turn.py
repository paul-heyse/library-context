#!/usr/bin/env python3
"""End-of-turn pipeline for Claude Code and Codex.

Everything that is not functional testing happens when the main agent stops, in one place:

- ``stop`` (Stop hook) runs the ``sync`` recipes (generators and formatting), then starts the job;
- the job runs the ``ready`` recipes and every dependency of ``just hygiene``, lets a fixer agent
  repair what it can, re-runs those checks, then runs the ``after`` recipes without waiting on them;
- ``prompt`` (UserPromptSubmit hook) holds a session's next turn until the job has checked what
  that session's stop left (another session starts at once) and shows findings the fixer left to
  the operator only;
- ``check <id>...`` re-runs named steps; ``guard`` (PreToolUse hook) confines a fixer's shell to
  exactly those.

Repository facts live in ``.config/after-turn.toml``; this file is the same in every repository
(canonical copy: project-template). Nothing reaches the main agent's context and every hook exits
0. State and logs live in the worktree's git directory under ``after-turn/``.
``AFTER_TURN_FIXER=off`` disables the fixer. Hooks run it on Python 3.14 through ``PYTHON``.
"""

from __future__ import annotations

import argparse
import contextlib
import dataclasses
import datetime
import fcntl
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
import tomllib
from collections.abc import Iterator, Sequence
from pathlib import Path
from typing import Any

ROLE_ENV = "AFTER_TURN_ROLE"
CHECKS_ENV = "AFTER_TURN_CHECKS"
FIXER_ENV = "AFTER_TURN_FIXER"

CONFIG_PATH = Path(".config") / "after-turn.toml"
HYGIENE_RECIPE = "hygiene"
# The interpreter every hook and fixer command uses: pinned, whatever `python3` is on PATH.
PYTHON = "uv run --no-project --python 3.14 python"
FIXER_COMMAND = f"{PYTHON} scripts/after_turn.py check"
FIXER_TOOLS = ("Read", "Edit", "Write", "Grep", "Glob")
CLAUDE_EFFORTS = ("low", "medium", "high", "xhigh", "max")

PENDING_GRACE = 30
# A session's next prompt waits at most this long; the hook's own timeout (1800 s) stays above it.
PROMPT_WAIT = 25 * 60


@dataclasses.dataclass(frozen=True)
class Config:
    """The repository's steps, read from ``.config/after-turn.toml``; each step is a just recipe."""

    sync: tuple[str, ...] = ()
    sync_when: dict[str, tuple[str, ...]] = dataclasses.field(default_factory=dict)
    ready: tuple[str, ...] = ()
    operator_only: frozenset[str] = frozenset()
    after: tuple[str, ...] = ()
    after_outputs: tuple[str, ...] = ()
    protected: tuple[str, ...] = ()
    check_timeout: float = 1200
    claude_model: str = "claude-sonnet-5-5"
    claude_default_effort: str = "high"
    codex_model: str = "gpt-6.1-sol"
    codex_effort: str = "medium"
    fixer_timeout: float = 20 * 60

    @property
    def not_fixable(self) -> frozenset[str]:
        """Steps whose failures always go to the operator."""
        return self.operator_only | frozenset(self.ready)


def load_config(root: Path) -> Config:
    path = root / CONFIG_PATH
    if not path.exists():
        return Config()
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    fixer = data.get("fixer", {})
    defaults = Config()
    return Config(
        sync=tuple(data.get("sync", ())),
        sync_when={k: tuple(v) for k, v in data.get("sync_when", {}).items()},
        ready=tuple(data.get("ready", ())),
        operator_only=frozenset(data.get("operator_only", ())),
        after=tuple(data.get("after", ())),
        after_outputs=tuple(data.get("after_outputs", ())),
        protected=tuple(data.get("protected", ())),
        check_timeout=float(data.get("check_timeout", defaults.check_timeout)),
        claude_model=fixer.get("claude_model", defaults.claude_model),
        claude_default_effort=fixer.get("claude_default_effort", defaults.claude_default_effort),
        codex_model=fixer.get("codex_model", defaults.codex_model),
        codex_effort=fixer.get("codex_effort", defaults.codex_effort),
        fixer_timeout=float(fixer.get("timeout", defaults.fixer_timeout)),
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
    for sub in ("checks", "sessions"):
        (path / sub).mkdir(parents=True, exist_ok=True)
    return path


def is_fixer() -> bool:
    return os.environ.get(ROLE_ENV) == "fixer"


def read_payload() -> Payload:
    try:
        data = json.loads(sys.stdin.read() or "{}")
    except OSError, ValueError:
        return {}
    return data if isinstance(data, dict) else {}


def append_log(state: Path, name: str, line: str) -> None:
    with (state / name).open("a", encoding="utf-8") as fh:
        fh.write(f"{now()} {line}\n")


# --- locks -------------------------------------------------------------------------------------


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


def job_busy(state: Path) -> bool:
    """Whether a job holds the lock, or a stop has just asked for one."""
    with try_lock(state / "job.lock") as acquired:
        if not acquired:
            return True
    pending = state / "job-pending"
    # A job takes the lock within milliseconds of its stop; an older unclaimed mark means the job
    # never started, and waiting on it would only delay the turn.
    with contextlib.suppress(OSError):
        return time.time() - pending.stat().st_mtime < PENDING_GRACE
    return False


def session_of(payload: Payload) -> str | None:
    session = payload.get("session_id")
    return session if isinstance(session, str) and session else None


def session_mark(state: Path, session: str) -> Path:
    """The mark a session's stop leaves until a job has checked the tree that stop left."""
    return state / "sessions" / re.sub(r"[^A-Za-z0-9._-]", "_", session)


def session_marks(state: Path) -> dict[Path, int]:
    marks: dict[Path, int] = {}
    for mark in (state / "sessions").iterdir():
        with contextlib.suppress(OSError):
            marks[mark] = mark.stat().st_mtime_ns
    return marks


def release_sessions(marks: dict[Path, int]) -> None:
    """Drop the marks a finished run covered; a mark touched again since belongs to a later run."""
    for mark, mtime in marks.items():
        with contextlib.suppress(OSError):
            if mark.stat().st_mtime_ns == mtime:
                mark.unlink()


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
    command: Sequence[str],
    root: Path,
    log: Path,
    *,
    env: dict[str, str] | None = None,
    stdin_text: str | None = None,
    timeout: float | None = None,
) -> int:
    """Run ``command`` in ``root`` with its output in ``log``; kill it after ``timeout`` s."""
    with log.open("w", encoding="utf-8") as fh:
        fh.write(f"$ {' '.join(command)}\n")
        fh.flush()
        proc = subprocess.Popen(
            list(command),
            cwd=root,
            env=env,
            stdin=subprocess.PIPE if stdin_text is not None else subprocess.DEVNULL,
            stdout=fh,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=True,
        )
        try:
            proc.communicate(input=stdin_text, timeout=timeout)
            return proc.returncode
        except subprocess.TimeoutExpired:
            with contextlib.suppress(ProcessLookupError):
                os.killpg(proc.pid, signal.SIGKILL)
            proc.wait()
            fh.write(f"\n[after-turn] killed after {timeout:.0f} s\n")
            return 124


def hygiene_checks(root: Path) -> list[str]:
    """The check ids: the dependencies of the justfile's `hygiene` recipe, in order."""
    out = subprocess.run(
        ["just", "--dump", "--dump-format", "json"],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    )
    return checks_from_dump(json.loads(out.stdout))


def checks_from_dump(dump: dict[str, Any]) -> list[str]:
    recipe = dump["recipes"].get(HYGIENE_RECIPE)
    return [dep["recipe"] for dep in recipe["dependencies"]] if recipe else []


def run_check(check: str, root: Path, state: Path, config: Config) -> dict[str, Any]:
    log = state / "checks" / f"{check}.log"
    started = time.monotonic()
    try:
        rc = run_logged(["just", check], root, log, env=step_env(), timeout=config.check_timeout)
    except Exception as exc:  # a step that cannot start is a failed step
        log.write_text(f"$ just {check}\nerror: could not run the step: {exc!r}\n")
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


def fingerprint(root: Path, config: Config) -> str:
    """HEAD, the tracked diff and untracked files, without the ``after`` recipes' outputs."""
    excludes = [f":(exclude,glob){p}" for p in config.after_outputs]

    def git(*args: str) -> bytes:
        return subprocess.run(["git", *args], cwd=root, capture_output=True, check=True).stdout

    digest = hashlib.sha256()
    digest.update(git("rev-parse", "HEAD"))
    digest.update(git("diff", "HEAD", "--binary", "--", ".", *excludes))
    untracked = git("ls-files", "-o", "--exclude-standard", "-z", "--", ".", *excludes)
    for name in sorted(filter(None, untracked.split(b"\0"))):
        with contextlib.suppress(OSError):
            stat = os.lstat(root / os.fsdecode(name))
            digest.update(name + f"\0{stat.st_size}\0{stat.st_mtime_ns}\0".encode())
    return digest.hexdigest()


def file_hashes(root: Path) -> dict[str, str]:
    """Content hashes of every changed or untracked file, to see what a fixer touched."""
    names = subprocess.run(
        ["git", "ls-files", "-m", "-o", "--exclude-standard", "-z"],
        cwd=root,
        capture_output=True,
        check=True,
    ).stdout
    hashes: dict[str, str] = {}
    for raw in filter(None, names.split(b"\0")):
        name = os.fsdecode(raw)
        with contextlib.suppress(OSError):
            hashes[name] = hashlib.sha256((root / name).read_bytes()).hexdigest()
    return hashes


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


# --- the fixer ---------------------------------------------------------------------------------


def fixer_enabled() -> bool:
    return os.environ.get(FIXER_ENV, "on").lower() not in {"off", "0", "false", "no"}


def claude_effort(value: object, config: Config) -> str:
    if isinstance(value, str) and value in CLAUDE_EFFORTS:
        return value
    return config.claude_default_effort


def fixer_env(checks: Sequence[str]) -> dict[str, str]:
    # The child is its own session: drop the parent's Claude session variables.
    env = {k: v for k, v in os.environ.items() if not k.startswith("CLAUDE")}
    env[ROLE_ENV] = "fixer"
    env[CHECKS_ENV] = ",".join(checks)
    return env


def fixer_brief(root: Path, config: Config) -> str:
    brief = (root / "scripts" / "after_turn_fixer.md").read_text(encoding="utf-8")
    protected = (*config.protected, *config.after_outputs)
    if protected:
        paths = ", ".join(f"`{p}`" for p in protected)
        brief += f"\nNever edit these paths, whatever a finding says: {paths}.\n"
    return brief


def fixer_prompt(checks: Sequence[str], results: dict[str, Any], tail_lines: int = 150) -> str:
    parts = [
        "The end-of-turn checks below failed on the tree the main agent left. Fix them.",
        f"Re-run a check only as `{FIXER_COMMAND} <id>`, for these ids: {', '.join(checks)}.",
    ]
    for check in checks:
        log = Path(results[check]["log"])
        lines = log.read_text(errors="replace").splitlines() if log.exists() else []
        tail = "\n".join(lines[-tail_lines:])
        parts.append(f"## {check} (exit {results[check]['rc']})\n```\n{tail}\n```")
    return "\n\n".join(parts)


def fixer_command(harness: str, effort: str, root: Path, state: Path, config: Config) -> list[str]:
    if harness == "codex":
        return [
            "codex",
            "exec",
            "-m",
            config.codex_model,
            "-c",
            f'model_reasoning_effort="{config.codex_effort}"',
            # The guard hook, not the sandbox, confines commands; checks may need shared build
            # directories and local services outside a workspace sandbox.
            "-s",
            "danger-full-access",
            "--dangerously-bypass-hook-trust",
            "-C",
            str(root),
            "-o",
            str(state / "fixer-last-message.md"),
            "-",
        ]
    settings = {"permissions": {"allow": [f"Bash({FIXER_COMMAND} *)", *FIXER_TOOLS]}}
    return [
        "claude",
        "-p",
        "--model",
        config.claude_model,
        "--effort",
        claude_effort(effort, config),
        "--permission-mode",
        "dontAsk",
        "--tools",
        ",".join(("Bash", *FIXER_TOOLS)),
        "--strict-mcp-config",
        "--settings",
        json.dumps(settings),
        "--append-system-prompt",
        fixer_brief(root, config),
        "--output-format",
        "text",
    ]


def run_fixer(
    harness: str,
    effort: str,
    checks: Sequence[str],
    results: dict[str, Any],
    root: Path,
    state: Path,
    config: Config,
) -> dict[str, Any]:
    command = fixer_command(harness, effort, root, state, config)
    prompt = fixer_prompt(checks, results)
    if harness == "codex":
        prompt = fixer_brief(root, config) + "\n\n" + prompt
    info: dict[str, Any] = {
        "harness": harness,
        "model": config.codex_model if harness == "codex" else config.claude_model,
        "effort": config.codex_effort if harness == "codex" else claude_effort(effort, config),
        "checks": list(checks),
        "log": str(state / "fixer.log"),
    }
    if shutil.which(command[0]) is None:
        info.update(rc=127, changed=[], error=f"{command[0]} not found")
        return info
    before = file_hashes(root)
    started = time.monotonic()
    info["rc"] = run_logged(
        command,
        root,
        state / "fixer.log",
        env=fixer_env(checks),
        stdin_text=prompt,
        timeout=config.fixer_timeout,
    )
    info["seconds"] = round(time.monotonic() - started, 1)
    after = file_hashes(root)
    info["changed"] = sorted(k for k in set(before) | set(after) if before.get(k) != after.get(k))
    return info


# --- operator messages -------------------------------------------------------------------------


def first_finding(log: str) -> str:
    lines = [line.strip() for line in log.splitlines()[1:] if line.strip()]
    for line in lines:
        if re.search(r"\b(error|failed|missing|blocked|finding)", line, re.IGNORECASE):
            return line[:160]
    return lines[-1][:160] if lines else ""


def leftovers(report: Report) -> list[str]:
    return [cid for cid, result in report.get("checks", {}).items() if result["status"] != "passed"]


def operator_message(report: Report, logs: str = ".git/after-turn/") -> str | None:
    left = leftovers(report)
    fixer = report.get("fixer") or {}
    changed = fixer.get("changed") or []
    if not left and not changed:
        return None
    parts: list[str] = []
    if changed:
        parts.append(
            f"the fixer ({fixer.get('model')}) changed {len(changed)} file(s): "
            + ", ".join(changed[:6])
            + (" …" if len(changed) > 6 else "")
        )
    if left:
        details = []
        for cid in left:
            log = Path(report["checks"][cid]["log"])
            text = log.read_text(errors="replace") if log.exists() else ""
            details.append(f"{cid}: {first_finding(text)}".rstrip(": "))
        parts.append(f"{len(left)} left: " + "; ".join(details))
    return "End-of-turn checks: " + " | ".join(parts) + f" (logs: {logs})"


def logs_label(root: Path, state: Path) -> str:
    try:
        return str(state.relative_to(root)) + "/"
    except ValueError:
        return str(state) + "/"


# --- subcommands -------------------------------------------------------------------------------


def cmd_stop(harness: str) -> int:
    if is_fixer():
        return 0
    payload = read_payload()
    session = session_of(payload)
    root = repo_root()
    state = state_dir(root)
    config = load_config(root)
    effort_field = payload.get("effort")
    effort = (
        effort_field.get("level") if isinstance(effort_field, dict) else None
    ) or os.environ.get("CLAUDE_EFFORT", "")
    # What decides whether a recipe's bash reads startup files (see step_env).
    shell = " ".join(f"{k}={os.environ.get(k, '-')}" for k in ("SHLVL", "BASH_ENV", "SSH_CLIENT"))
    append_log(
        state,
        "stop.log",
        f"{harness} session={session or '-'} payload={sorted(payload)} effort={effort or '-'} "
        + shell,
    )
    results: dict[str, Any] = {}
    for step in config.sync:
        when = config.sync_when.get(step)
        if when and not changed_since_head(root, when):
            continue
        results[step] = run_check(step, root, state, config)
    (state / "sync.json").write_text(json.dumps(results, indent=2) + "\n")
    if session:
        session_mark(state, session).touch()
    (state / "job-pending").touch()
    subprocess.Popen(
        [
            sys.executable,
            str(Path(__file__).resolve()),
            "job",
            "--harness",
            harness,
            "--effort",
            effort,
        ],
        cwd=root,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=(state / "job.err").open("a"),
        start_new_session=True,
    )
    return 0


def cmd_job(harness: str, effort: str) -> int:
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
                requested = session_marks(state)
                try:
                    run_job_once(harness, effort, root, state, config)
                finally:
                    release_sessions(requested)
    run_after(root, state, config)
    return 0


def run_job_once(harness: str, effort: str, root: Path, state: Path, config: Config) -> None:
    sync = json.loads((state / "sync.json").read_text()) if (state / "sync.json").exists() else {}
    current = fingerprint(root, config)
    last = load_report(state)
    if last and last.get("complete") and last.get("fingerprint") == current:
        append_log(state, "job.log", "tree unchanged since the last report; skipped")
        return
    report: Report = {"id": now(), "harness": harness, "complete": False, "shown": False}
    results: dict[str, Any] = dict(sync)
    for step in (*config.ready, *hygiene_checks(root)):
        results[step] = run_check(step, root, state, config)
    failed = [cid for cid, r in results.items() if r["status"] != "passed"]
    fixable = [cid for cid in failed if cid not in config.not_fixable]
    append_log(state, "job.log", f"failed={failed} fixable={fixable}")
    if fixable and fixer_enabled():
        report["fixer"] = run_fixer(harness, effort, fixable, results, root, state, config)
        for check in fixable:
            results[check] = run_check(check, root, state, config)
        append_log(state, "job.log", f"fixer rc={report['fixer'].get('rc')}")
    report.update(
        checks=results, fingerprint=fingerprint(root, config), complete=True, finished=now()
    )
    save_report(state, report)


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
                run_logged(["just", step], root, state / f"{step}.log", env=step_env())


def cmd_prompt(harness: str) -> int:
    if is_fixer():
        return 0
    session = session_of(read_payload())
    root = repo_root()
    state = state_dir(root)
    logs = logs_label(root, state)

    def waiting() -> bool:
        # A session waits only for checks its own stop requested; a new session starts at once.
        if session is not None and not session_mark(state, session).exists():
            return False
        return job_busy(state)

    deadline = time.monotonic() + PROMPT_WAIT
    while waiting() and time.monotonic() < deadline:
        time.sleep(1)
    if waiting():
        message: str | None = (
            f"End-of-turn checks still running after {PROMPT_WAIT // 60} min; "
            f"this turn started without waiting (logs: {logs})"
        )
    else:
        report = load_report(state)
        message = None
        if report and report.get("complete") and not report.get("shown"):
            message = operator_message(report, logs)
            report["shown"] = True
            save_report(state, report)
    if message:
        output: dict[str, Any] = {"systemMessage": message}
        if harness == "claude":
            output["suppressOutput"] = True
        print(json.dumps(output))
    return 0


def guard_decision(command: str, allowed: set[str]) -> str | None:
    """None when a fixer may run ``command``; otherwise the reason it may not."""
    match = re.fullmatch(rf"\s*{re.escape(FIXER_COMMAND)}((?: [a-z0-9-]+)+)\s*", command)
    if match and set(match.group(1).split()) <= allowed:
        return None
    names = ", ".join(sorted(allowed))
    return f"the end-of-turn fixer may only run `{FIXER_COMMAND} <id>` for: {names}"


def cmd_guard() -> int:
    if not is_fixer():
        return 0
    payload = read_payload()
    if payload.get("tool_name") != "Bash":
        return 0
    tool_input = payload.get("tool_input")
    command = tool_input.get("command", "") if isinstance(tool_input, dict) else ""
    allowed = {c for c in os.environ.get(CHECKS_ENV, "").split(",") if c}
    reason = guard_decision(command if isinstance(command, str) else "", allowed)
    if reason:
        decision = {"permissionDecision": "deny", "permissionDecisionReason": reason}
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", **decision}}))
    return 0


def cmd_check(checks: Sequence[str]) -> int:
    root = repo_root()
    state = state_dir(root)
    config = load_config(root)
    known = {*config.sync, *config.ready, *hygiene_checks(root)}
    unknown = [c for c in checks if c not in known]
    if unknown:
        print(f"unknown check id(s): {', '.join(unknown)}; known: {', '.join(sorted(known))}")
        return 2
    failed = []
    for check in checks:
        result = run_check(check, root, state, config)
        print(Path(result["log"]).read_text(errors="replace"), end="", flush=True)
        print(
            f"== {check}: {result['status']} (exit {result['rc']}, {result['seconds']} s)",
            flush=True,
        )
        if result["status"] != "passed":
            failed.append(check)
    return 1 if failed else 0


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="End-of-turn pipeline for Claude Code and Codex")
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("stop", "prompt"):
        sub.add_parser(name).add_argument("--harness", choices=("claude", "codex"), required=True)
    job = sub.add_parser("job")
    job.add_argument("--harness", choices=("claude", "codex"), required=True)
    job.add_argument("--effort", default="")
    sub.add_parser("guard")
    check = sub.add_parser("check")
    check.add_argument("ids", nargs="+")
    args = parser.parse_args(argv)
    if args.command == "check":
        return cmd_check(args.ids)
    if args.command == "guard":
        return cmd_guard()
    # Hooks never fail the harness: errors go to a log, never to the agent.
    try:
        if args.command == "stop":
            return cmd_stop(args.harness)
        if args.command == "prompt":
            return cmd_prompt(args.harness)
        return cmd_job(args.harness, args.effort)
    except Exception as exc:  # a hook must not surface errors to the agent
        with contextlib.suppress(Exception):
            append_log(state_dir(repo_root()), "error.log", f"{args.command}: {exc!r}")
        return 0


if __name__ == "__main__":
    sys.exit(main())
