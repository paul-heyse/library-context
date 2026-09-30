#!/usr/bin/env python3
"""End-of-turn pipeline for Claude Code and Codex (ADR-0104).

Everything that is not functional testing happens when the main agent stops, in one place:

- ``stop`` (Stop hook) regenerates derived files and formats the tree, then starts the job;
- the job prepares the environment, runs every ``just hygiene`` check, lets a fixer agent repair
  what failed, re-runs those checks and refreshes the library catalog;
- ``prompt`` (UserPromptSubmit hook) holds the next turn until the job is done and shows the
  findings the fixer left to the operator only;
- ``check <id>...`` re-runs named checks; ``guard`` (PreToolUse hook) confines a fixer's shell to
  exactly those.

Nothing here reaches the main agent's context and every hook exits 0. State and logs live in the
worktree's git directory under ``after-turn/``. ``LCTX_AFTER_TURN_FIXER=off`` disables the fixer.
"""

from __future__ import annotations

import argparse
import contextlib
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
from collections.abc import Iterator, Sequence
from pathlib import Path
from typing import Any

ROLE_ENV = "LCTX_AFTER_TURN_ROLE"
CHECKS_ENV = "LCTX_AFTER_TURN_CHECKS"
FIXER_ENV = "LCTX_AFTER_TURN_FIXER"

CLAUDE_MODEL = "claude-sonnet-5-5"
CLAUDE_EFFORTS = ("low", "medium", "high", "xhigh", "max")
CLAUDE_DEFAULT_EFFORT = "high"
CODEX_MODEL = "gpt-6.1-sol"
CODEX_EFFORT = "medium"

FIXER_TIMEOUT = 20 * 60
PENDING_GRACE = 30
# The next prompt waits at most this long; the hook's own timeout (1800 s) stays above it.
PROMPT_WAIT = 25 * 60

# Generators run synchronously at stop, in this order. `build-features` only after a dependency
# manifest changed.
GENERATORS: dict[str, list[str]] = {
    "skills-sync": ["just", "skills-sync"],
    "adr-index": ["just", "adr", "index"],
    "build-features": ["just", "build-features"],
    "fmt": ["just", "fmt"],
}
# Readiness steps run first in the job. Their failures are the operator's, never the fixer's.
READINESS = ("postgres-images", "tools")
# Checks the fixer never gets: they need the operator (a store, a skill refresh, a tool, a pull).
OPERATOR_ONLY = frozenset(
    {"store-check", "gold", "skills-sync", "adr-index", "build-features", *READINESS}
)
# The catalog's own outputs never count as a change to the tree.
FINGERPRINT_EXCLUDES = (":(exclude,glob)docs/library-utilization.*",)
FIXER_COMMAND = "python3 scripts/after_turn.py check"
FIXER_TOOLS = ("Read", "Edit", "Write", "Grep", "Glob")

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
    (path / "checks").mkdir(parents=True, exist_ok=True)
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


# --- running commands --------------------------------------------------------------------------


def run_logged(
    command: Sequence[str],
    root: Path,
    log: Path,
    *,
    echo: bool = False,
    env: dict[str, str] | None = None,
    stdin_text: str | None = None,
    timeout: float | None = None,
) -> int:
    """Run ``command`` in ``root``, writing its output to ``log`` (and stdout when ``echo``)."""
    with log.open("w", encoding="utf-8") as fh:
        fh.write(f"$ {' '.join(command)}\n")
        fh.flush()
        proc = subprocess.Popen(
            list(command),
            cwd=root,
            env=env,
            stdin=subprocess.PIPE if stdin_text is not None else subprocess.DEVNULL,
            stdout=subprocess.PIPE if echo else fh,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=True,
        )
        try:
            if echo:
                assert proc.stdout is not None
                for line in proc.stdout:
                    fh.write(line)
                    sys.stdout.write(line)
                return proc.wait(timeout=timeout)
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
    return [dep["recipe"] for dep in dump["recipes"]["hygiene"]["dependencies"]]


def check_command(check: str, root: Path) -> list[str]:
    if check in GENERATORS:
        return GENERATORS[check]
    if check == "postgres-images":
        return ["bash", "-c", postgres_images_script(root)]
    if check == "tools":
        return ["bash", "-c", 'out=$(just doctor 2>&1); echo "$out"; ! grep -q MISSING <<<"$out"']
    return ["just", check]


def postgres_images_script(root: Path) -> str:
    specs = [root / "specs" / "postgres-image.txt", root / "specs" / "postgres-vector-image.txt"]
    images = ["postgres:" + specs[0].read_text().strip(), specs[1].read_text().strip()]
    inspect = " && ".join(f"docker image inspect {image!r} >/dev/null 2>&1" for image in images)
    return f"if {inspect}; then echo 'images present'; else just postgres-test-setup; fi"


def run_check(check: str, root: Path, state: Path, *, echo: bool = False) -> dict[str, Any]:
    log = state / "checks" / f"{check}.log"
    started = time.monotonic()
    try:
        rc = run_logged(check_command(check, root), root, log, echo=echo)
    except Exception as exc:
        log.write_text(f"$ {check}\nerror: could not run the check: {exc!r}\n")
        rc = 127
    return {
        "status": "passed" if rc == 0 else "failed",
        "rc": rc,
        "seconds": round(time.monotonic() - started, 1),
        "log": str(log),
    }


def dependencies_changed(root: Path) -> bool:
    manifests = ["Cargo.lock", ":(glob)**/Cargo.toml"]
    diff = subprocess.run(["git", "diff", "--quiet", "HEAD", "--", *manifests], cwd=root)
    untracked = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard", "--", *manifests],
        cwd=root,
        capture_output=True,
        text=True,
    )
    return diff.returncode != 0 or bool(untracked.stdout.strip())


def fingerprint(root: Path) -> str:
    """HEAD, the tracked diff and untracked files, without the catalog's own outputs."""

    def git(*args: str) -> bytes:
        return subprocess.run(["git", *args], cwd=root, capture_output=True, check=True).stdout

    digest = hashlib.sha256()
    digest.update(git("rev-parse", "HEAD"))
    digest.update(git("diff", "HEAD", "--binary", "--", ".", *FINGERPRINT_EXCLUDES))
    untracked = git("ls-files", "-o", "--exclude-standard", "-z", "--", ".", *FINGERPRINT_EXCLUDES)
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


def claude_effort(value: object) -> str:
    return value if isinstance(value, str) and value in CLAUDE_EFFORTS else CLAUDE_DEFAULT_EFFORT


def fixer_env(checks: Sequence[str]) -> dict[str, str]:
    # The child is its own session: drop the parent's Claude session variables.
    env = {k: v for k, v in os.environ.items() if not k.startswith("CLAUDE")}
    env[ROLE_ENV] = "fixer"
    env[CHECKS_ENV] = ",".join(checks)
    return env


def fixer_brief(root: Path) -> str:
    return (root / "scripts" / "after_turn_fixer.md").read_text()


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


def fixer_command(harness: str, effort: str, root: Path, state: Path) -> list[str]:
    if harness == "codex":
        return [
            "codex",
            "exec",
            "-m",
            CODEX_MODEL,
            "-c",
            f'model_reasoning_effort="{CODEX_EFFORT}"',
            # The guard hook, not the sandbox, confines commands; checks need the shared cargo
            # build directory and the local PostgreSQL, both outside a workspace sandbox.
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
        CLAUDE_MODEL,
        "--effort",
        claude_effort(effort),
        "--permission-mode",
        "dontAsk",
        "--tools",
        ",".join(("Bash", *FIXER_TOOLS)),
        "--strict-mcp-config",
        "--settings",
        json.dumps(settings),
        "--append-system-prompt",
        fixer_brief(root),
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
) -> dict[str, Any]:
    command = fixer_command(harness, effort, root, state)
    prompt = fixer_prompt(checks, results)
    if harness == "codex":
        prompt = fixer_brief(root) + "\n\n" + prompt
    info: dict[str, Any] = {
        "harness": harness,
        "model": CODEX_MODEL if harness == "codex" else CLAUDE_MODEL,
        "effort": CODEX_EFFORT if harness == "codex" else claude_effort(effort),
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
        timeout=FIXER_TIMEOUT,
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


def operator_message(report: Report) -> str | None:
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
    return "End-of-turn checks: " + " | ".join(parts) + " (logs: .git/after-turn/)"


# --- subcommands -------------------------------------------------------------------------------


def cmd_stop(harness: str) -> int:
    if is_fixer():
        return 0
    payload = read_payload()
    root = repo_root()
    state = state_dir(root)
    effort_field = payload.get("effort")
    effort = (
        effort_field.get("level") if isinstance(effort_field, dict) else None
    ) or os.environ.get("CLAUDE_EFFORT", "")
    append_log(state, "stop.log", f"{harness} payload={sorted(payload)} effort={effort or '-'}")
    results: dict[str, Any] = {}
    for gen in GENERATORS:
        if gen == "build-features" and not dependencies_changed(root):
            continue
        results[gen] = run_check(gen, root, state)
    (state / "sync.json").write_text(json.dumps(results, indent=2) + "\n")
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
    pending = state / "job-pending"
    while pending.exists():
        with try_lock(state / "job.lock") as acquired:
            if not acquired:
                return 0  # the running job re-checks the pending mark after releasing its lock
            while pending.exists():
                pending.unlink(missing_ok=True)
                run_job_once(harness, effort, root, state)
    refresh_catalog(root, state)
    return 0


def run_job_once(harness: str, effort: str, root: Path, state: Path) -> None:
    sync = json.loads((state / "sync.json").read_text()) if (state / "sync.json").exists() else {}
    current = fingerprint(root)
    last = load_report(state)
    if last and last.get("complete") and last.get("fingerprint") == current:
        append_log(state, "job.log", "tree unchanged since the last report; skipped")
        return
    report: Report = {"id": now(), "harness": harness, "complete": False, "shown": False}
    results: dict[str, Any] = dict(sync)
    for step in READINESS:
        results[step] = run_check(step, root, state)
    for check in hygiene_checks(root):
        results[check] = run_check(check, root, state)
    failed = [cid for cid, r in results.items() if r["status"] != "passed"]
    fixable = [cid for cid in failed if cid not in OPERATOR_ONLY]
    append_log(state, "job.log", f"failed={failed} fixable={fixable}")
    if fixable and fixer_enabled():
        report["fixer"] = run_fixer(harness, effort, fixable, results, root, state)
        for check in fixable:
            results[check] = run_check(check, root, state)
        append_log(state, "job.log", f"fixer rc={report['fixer'].get('rc')}")
    report.update(checks=results, fingerprint=fingerprint(root), complete=True, finished=now())
    save_report(state, report)


def refresh_catalog(root: Path, state: Path) -> None:
    (state / "catalog-pending").touch()
    with try_lock(state / "catalog.lock") as acquired:
        if not acquired:
            return
        while (state / "catalog-pending").exists():
            (state / "catalog-pending").unlink(missing_ok=True)
            run_logged(["just", "library-catalog"], root, state / "catalog.log")


def cmd_prompt(harness: str) -> int:
    if is_fixer():
        return 0
    read_payload()
    root = repo_root()
    state = state_dir(root)
    deadline = time.monotonic() + PROMPT_WAIT
    while job_busy(state) and time.monotonic() < deadline:
        time.sleep(1)
    if job_busy(state):
        message: str | None = (
            f"End-of-turn checks still running after {PROMPT_WAIT // 60} min; "
            "this turn started without waiting (logs: .git/after-turn/)"
        )
    else:
        report = load_report(state)
        message = None
        if report and report.get("complete") and not report.get("shown"):
            message = operator_message(report)
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
    match = re.fullmatch(r"\s*python3 scripts/after_turn\.py check((?: [a-z0-9-]+)+)\s*", command)
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
    known = {*GENERATORS, *READINESS, *hygiene_checks(root)}
    unknown = [c for c in checks if c not in known]
    if unknown:
        print(f"unknown check id(s): {', '.join(unknown)}; known: {', '.join(sorted(known))}")
        return 2
    failed = []
    for check in checks:
        print(f"== {check}", flush=True)
        result = run_check(check, root, state, echo=True)
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
    except Exception as exc:
        with contextlib.suppress(Exception):
            append_log(state_dir(repo_root()), "error.log", f"{args.command}: {exc!r}")
        return 0


if __name__ == "__main__":
    sys.exit(main())
