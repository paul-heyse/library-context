@AGENTS.md

## Claude Code specifics

- **Hook:** `scripts/hooks/format_edited.sh` runs after every Edit/Write and formats only the
  edited file (rustfmt / ruff format). It never blocks. If a file changes on disk after your
  edit, that is the formatter.
- **Diagnostics:** the pyright plugin is disabled for this project (`.claude/settings.json`);
  in-session Python diagnostics come from pyrefly.
- **Subagent:** `.claude/agents/design-reviewer.md` runs the `design-review` skill with fresh
  context. Give it the target and the depth.
- **Memory:** project memory records the low-friction process preference; ADR-0001 is the
  authoritative version.
