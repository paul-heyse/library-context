@AGENTS.md

## Claude Code specifics

- **Diagnostics:** the pyright plugin is disabled for this project (`.claude/settings.json`)
  because pyrefly is the type checker. There is no in-session Python language server; pyrefly runs
  as the applicable `just types` leaf at scope end.
- **No hooks.** Nothing formats files after Edit/Write or at stop. Run `just turn-end` at the end of
  a turn that changed files, `just ready` after an environment change and applicable
  non-functional leaves at scope end (AGENTS.md, Commands; ADR-0126).
- **Subagents:** [shared roles](.agents/roles/README.md) define responsibility and handoff;
  `.claude/agents/` supplies native model settings and no tool lists (ADR-0113). `implementer` uses the executor
  contract. Use fresh context for independent review; the binding selects formal tier and purpose.
- **Memory:** project memory records the low-friction process preference; the repository binding under ADR-0040 owns current
  review mechanics; AGENTS.md owns agent instructions.
