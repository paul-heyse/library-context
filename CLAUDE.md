@AGENTS.md

## Claude Code specifics

- **Diagnostics:** the pyright plugin is disabled for this project (`.claude/settings.json`)
  because pyrefly is the type checker. There is no in-session Python language server; pyrefly runs
  in the end-of-turn checks.
- **End-of-turn hooks, no edit hook.** Nothing formats files after Edit/Write. `Stop`,
  `UserPromptSubmit` and a `PreToolUse` guard run `scripts/after_turn.py` (AGENTS.md, Commands;
  ADR-0104). They are silent to the model and need no agent action. A fixer runs as a headless
  `claude -p` on Sonnet 5.5 at the session's effort; subagents stopping don't trigger the hook.
- **Subagents:** [shared roles](.agents/roles/README.md) define responsibility and handoff;
  `.claude/agents/` supplies native model/tool configuration. `implementer` uses the executor
  contract. Use fresh context for independent review; the binding selects formal tier and purpose.
- **Memory:** project memory records the low-friction process preference; the repository binding under ADR-0040 owns current
  review mechanics; AGENTS.md owns agent instructions.
