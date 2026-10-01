---
id: ADR-0104
title: Non-functional checks run in an end-of-turn hook with a fixer agent
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§1.2]
evidence: Tested
revisit: A check's runtime delays the next turn noticeably, a fixer edit changes behaviour, or Claude Code or Codex changes Stop, UserPromptSubmit or PreToolUse hook semantics.
---

## Context

Agents ran formatting, lint, type checks, dependency policy, ADR/agent lint, docs checks and
derived-file generators themselves, mid-work or at scope end, beside functional tests. Formatting
and fixes mid-work move code the agent is reasoning about, forcing it to re-read and re-check
positions. Agents also spent turns troubleshooting failures in these checks and in the library
catalog, which is a temporary aid. The operator decided (2026-09-30) that everything that is not
functional testing, and every action an agent would otherwise start at the beginning of a turn,
runs in one place when the main agent stops; the next turn follows unchanged.
[DESIGN §1.2](../design/DESIGN.md#section-1-2) owns the development loop; the justfile owns the
commands; the review binding's §4 table owns their cadence.

## Options

1. **Keep agent-run gates, add only a Stop-time formatter.** Least machinery, but agents still run
   and troubleshoot lint and type checks, and pay the main model's cost to fix mechanical findings.
2. **Wake the main agent on failure** (a blocking Stop decision or an async rewake). Findings are
   fixed at once, but by the main model, inside the turn it declared finished.
3. **An end-of-turn pipeline with a cheaper fixer agent (chosen).** The hooks run the generators
   and checks; a headless fixer on a cheaper model repairs what it can, confined to re-running its
   own checks; the operator sees the rest; agents run functional tests only.

## Decision

`scripts/after_turn.py`, wired in `.claude/settings.json` and `.codex/hooks.json`:
- **Stop:** runs `skills-sync`, `adr index`, `build-features` (only after a dependency manifest
  changed) and `fmt` synchronously, then starts a background job.
- **The job:** pulls missing PostgreSQL images, reports missing tools and runs every dependency of
  `just hygiene` (the single list of non-functional checks). For failures a fixer can address, it
  starts one headless fixer: `claude -p` on Sonnet 5.5 at the session's effort (high when the hook
  cannot see it), or `codex exec` on `gpt-6.1-sol` at medium effort. It then re-runs those checks
  itself, writes `.git/after-turn/report.json` and refreshes the library catalog last,
  fire-and-forget. A tree whose fingerprint (HEAD, diff and untracked files, excluding the catalog's
  outputs) matches the last complete report is not re-checked.
- **UserPromptSubmit:** holds the next turn until the job is done (at most 25 minutes). It shows
  leftover findings and fixer edits to the operator as a `systemMessage`, never to the model.
- **PreToolUse guard:** in a fixer, allows only `python3 scripts/after_turn.py check <id>` for its
  assigned ids. Every other hook no-ops in a fixer (`LCTX_AFTER_TURN_ROLE`), so a fixer never
  restarts the pipeline.
- **Operator-only failures:** store, gold, tools, images and generator failures never go to the
  fixer.

`just check` and `just test-all` are functional only. Qualification is `just test-all` plus a
clean end-of-turn report for the same tree. `LCTX_AFTER_TURN_FIXER=off` disables the fixer.

## Consequences

- Agents never format, lint or troubleshoot checks. Functional tests stay theirs.
- The next prompt can wait for the checks and fixer after a large change. Clippy and the release
  `lctx` build for `store-check` dominate that wait.
- Codex runs project hooks only once they are trusted. The fixer child passes
  `--dangerously-bypass-hook-trust` so its guard always runs, and uses full access because checks
  need the shared cargo build directory and local PostgreSQL. The guard, not the sandbox, confines
  its commands.
- Verified 2026-09-30:
  - focused tests: `tests/scripts/test_after_turn.py`;
  - a live Claude fixer repaired five failing checks, and the re-run passed;
  - a live Claude child had `just --version` refused by the guard, and its scoped check ran.

## Amendments

- 2026-09-30: No decision changes. Repository facts (sync, ready, operator-only, after steps,
  protected paths, timeouts) moved from the script to `.config/after-turn.toml`. The script is
  shared with project-template, which holds the canonical copy. Its environment variables are now
  `AFTER_TURN_ROLE`, `AFTER_TURN_CHECKS` and `AFTER_TURN_FIXER`. Hooks and the fixer's check command
  run it on a pinned interpreter, `uv run --no-project --python 3.14 python`, not a bare `python3`.
  Root ruff excludes `libraries/` and `services/`, which are separate uv projects.
- 2026-09-30: UserPromptSubmit holds only the next turn of the session whose Stop started the
  job, through per-session marks in `.git/after-turn/sessions/`. A prompt from another session, such
  as a new Codex thread, starts at once; a payload without a session id waits for any job. Codex
  fires Stop only when the agent ends its turn: four user interrupts that day fired none. Steps run
  without `BASH_ENV` and with `SHLVL` of at least 1, because recipe shells under Codex Desktop's
  hooks read `/etc/bash.bashrc`. That file fails under `set -u` (no `PS1`), so recipe comment lines
  failed `fmt` and `deps`. Verified 2026-09-30: `tests/scripts/test_after_turn.py`, 17 passed; its
  comment-line test fails against the previous script.
