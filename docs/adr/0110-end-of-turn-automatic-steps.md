---
id: ADR-0110
title: End-of-turn hooks run only automatic steps; agents run the non-functional checks at scope end
status: accepted
date: 2026-10-01
supersedes: [ADR-0104]
superseded-by: null
design: [§1.2]
evidence: Tested
revisit: A non-functional check repeatedly fails late enough at scope end to cost rework, or Claude Code or Codex changes Stop or UserPromptSubmit hook semantics.
---

## Context

ADR-0104 moved formatting, generators and every non-functional check (`just hygiene`) into an
end-of-turn hook. When a check failed, the hook started a headless fixer agent (`claude -p` or
`codex exec`) confined by a PreToolUse guard. The fixer proved brittle. It edited code another
session was still developing. It needed harness-specific trust, effort and permission plumbing.
Waiting for it froze sessions, so the prompt wait was already removed on 2026-10-01. The operator
decided (2026-10-01) to remove the fixer, and with it the checks whose findings need targeted
fixes such as type errors, clippy and lint. The hook keeps only steps that complete or repair
themselves. [DESIGN §1.2](../design/DESIGN.md#section-1-2) owns the development loop; the
justfile owns the commands; the review binding's §4 table owns their cadence.

## Options

1. **Keep the fixer and harden it** (scope it to the stopping session's files, make it
   cancellable). Each hardening adds machinery to a component whose failures are hard to see, and
   it still repairs code it did not write.
2. **Run every check in the hook and report the findings to the operator only.** Nothing edits
   code unasked, but the findings have no owner who knows the change, and the background run still
   costs a clippy and release build after every turn.
3. **Surface the findings to the main agent on its next prompt.** The author fixes them, but
   mid-work findings interrupt turns that are not finished, and the agent re-reads moved code.
4. **The hook runs only automatic steps; agents run `just hygiene` once at scope end (chosen).**

## Decision

`scripts/after_turn.py`, configured in `.config/after-turn.toml` and wired in
`.claude/settings.json` and `.codex/hooks.json`, is the same script in every repository. Its
canonical copy is in project-template.
- **Stop:** runs `skills-sync`, `adr-index`, `build-features` (only after a dependency manifest
  changed) and `fmt`, which includes ruff's safe auto-fixes. It then starts a background job.
- **The job:** runs `images-ready` and `doctor-check`, then writes `.git/after-turn/report.json`
  with every step's result. It then refreshes the library catalog without anything waiting on it.
  A burst of stops runs once more after the current run, not once per stop.
- **UserPromptSubmit:** never waits. It shows failed steps from the last report to the operator as
  a `systemMessage`, once, and never to the model.
- **No fixer, no guard, no checks.** The hooks never run a `just hygiene` check and never repair
  anything beyond the formatters' own fixes. There is no PreToolUse hook.
- **Non-functional checks are the agent's.** Once all functional scope in a plan is implemented,
  the agent runs `just hygiene` beside `just test-all`, fixes what fails and re-runs single checks
  with `just <id>`. Agents still neither format nor run hygiene checks mid-work, so auto-fixes and
  findings do not move code they are reasoning about. Qualification is `just test-all` plus
  `just hygiene` passing for the same tree.

Steps run without `BASH_ENV` and with `SHLVL` of at least 1. Recipe shells under Codex Desktop's
hooks otherwise read `/etc/bash.bashrc`, which fails under `set -u`, so recipe comment lines fail.

## Consequences

- Nothing edits code that its author did not ask for; the next prompt never waits.
- Lint, type and policy findings surface only at scope end, so they can accumulate during a long
  scope. Fixing them is ordinary scope-end work for the agent that made the change.
- Codex re-trusts hooks by content hash, so a changed `.codex/hooks.json` needs re-trusting once.
- Verified 2026-10-01: `tests/scripts/test_after_turn.py` (stdlib unittest), 9 passed; the
  project-template render test runs Stop and checks the report and a silent prompt hook.
