#!/usr/bin/env bash
# PostToolUse hook: format only the file just edited. Never blocks; a
# formatter failure (e.g. a syntax error mid-edit) is left for `just check`.
set -u
file=$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("tool_input",{}).get("file_path",""))' 2>/dev/null)
[ -n "$file" ] && [ -f "$file" ] || exit 0
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
case "$file" in
  */.claude/skills/*|*/fixtures/*|*/target/*) exit 0 ;;
  *.rs) rustfmt --edition 2024 --quiet "$file" >/dev/null 2>&1 ;;
  *.py) uv run --quiet ruff format --quiet "$file" >/dev/null 2>&1 ;;
esac
exit 0
