#!/usr/bin/env bash
# Stop hook for Claude Code (.claude/settings.json) and Codex (.codex/hooks.json): once the main
# agent ends its turn, format the tree, then refresh the library catalog in the background.
# Agents never see this. Output goes to logs under the worktree's git directory and the hook
# always exits 0, so a failure can neither block the stop nor reach the agent as work to do.
set -u

root="${CLAUDE_PROJECT_DIR:-$(git -C "$(dirname "$0")" rev-parse --show-toplevel)}"
cd "$root" || exit 0
state="$(git rev-parse --absolute-git-dir)/after-turn"
mkdir -p "$state" || exit 0

# Formatting is quick (about a second on a formatted tree) and runs before the stop completes,
# so it never overlaps the next turn.
{
  echo "== $(date -Is) just fmt"
  just fmt
  rc=$?
  echo "== $(date -Is) exit $rc"
} >"$state/fmt.log" 2>&1 </dev/null

# The catalog takes one to two minutes, so it runs detached. One refresh runs per worktree; a
# stop during a run leaves a pending mark and the running refresh goes round again.
touch "$state/catalog-pending"
setsid bash -c '
  exec 9>"$1/catalog.lock"
  flock -n 9 || exit 0
  while [ -e "$1/catalog-pending" ]; do
    rm -f "$1/catalog-pending"
    {
      echo "== $(date -Is) just library-catalog"
      just library-catalog
      rc=$?
      echo "== $(date -Is) exit $rc"
    } >"$1/catalog.log" 2>&1
  done
' after-turn "$state" </dev/null >/dev/null 2>&1 &

exit 0
