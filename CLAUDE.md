@AGENTS.md

## Claude Code specifics

- **Diagnostics:** the pyright plugin is disabled for this project (`.claude/settings.json`)
  because pyrefly is the type checker. There is no in-session Python language server; pyrefly runs
  as the applicable `just types` leaf at scope end.
- **End-of-turn hooks, no edit hook.** Nothing formats files after Edit/Write. `Stop` and
  `UserPromptSubmit` run `scripts/after_turn.py` (AGENTS.md, Commands; ADR-0126): formatting,
  generators, readiness and the catalog only. They are silent to the model, never block a prompt
  and fix nothing else; applicable non-functional findings (type errors, Clippy, lint) are
  the agent's at scope end. Subagents stopping don't trigger the hook.
- **Subagents:** [shared roles](.agents/roles/README.md) define responsibility and handoff;
  `.claude/agents/` supplies native model settings and no tool lists (ADR-0113). `implementer` uses the executor
  contract. Use fresh context for independent review; the binding selects formal tier and purpose.
- **Memory:** project memory records the low-friction process preference; the repository binding under ADR-0040 owns current
  review mechanics; AGENTS.md owns agent instructions.
