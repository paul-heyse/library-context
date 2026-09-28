@AGENTS.md

## Claude Code specifics

- **Diagnostics:** the pyright plugin is disabled for this project (`.claude/settings.json`)
  because pyrefly is the type checker. There is no in-session Python language server; Python
  diagnostics come from `uv run pyrefly check` once functional scope is complete (AGENTS.md,
  Testing rules).
- **No format hook.** Nothing formats files after Edit/Write; formatting runs once, at the end of
  the scope.
- **Subagent:** `.claude/agents/design-reviewer.md` runs the `design-review` skill with fresh
  context. Give it the target and expected change scenarios; the binding selects tier and purpose.
- **Memory:** project memory records the low-friction process preference; the repository binding under ADR-0040 owns current
  review mechanics; AGENTS.md owns agent instructions.
