"""Per-repository configuration for the transcript miner (library-context).

This is the only file that names a repository. To mine another repository, copy it and edit the
values: paths, the session exclusions, the cutoff, the periods and the package pattern. Nothing
the miner writes under OUTDIR is meant for version control: it contains command text.
"""
import os
import re

REPO = "/home/paul/library-context"
OUTDIR = os.path.join(REPO, "build/agent-effectiveness/w1")
CLAUDE_DIR = os.path.expanduser("~/.claude/projects/-home-paul-library-context")
CODEX_STATE_DB = os.path.expanduser("~/.codex/state_5.sqlite")
# A Codex thread belongs to the corpus when its cwd matches one of these (SQL LIKE patterns).
CODEX_CWD_LIKE = ["/home/paul/library-context", "/home/paul/library-context/%",
                  "/tmp/%library-context%", "/tmp/library-context-role-trial-%"]
# Sessions to exclude: the assessment session itself. Drill sessions start after CUTOFF.
EXCLUDED_SESSIONS = {"229442b4-20d0-4a30-a853-4f01b41581c7"}
CUTOFF = "2026-10-07T22:42:00"  # UTC; the assessment started here

# Configuration periods (UTC upper bounds), from the git history of the harness files.
PERIOD_BOUNDS = [
    ("P0", "2026-09-29T02:49:00"),  # before shared nightly Cargo builds (ADR-0079)
    ("P1", "2026-09-30T21:15:00"),  # before end-of-turn hooks
    ("P2", "2026-10-01T16:18:00"),  # hooks; before ADR-0110 automatic-only hook
    ("P3", "2026-10-05T05:08:00"),  # before verify.py / ADR-0126 scoped verification (hooks removed 08:09Z)
    ("P4", "2026-10-06T02:41:00"),  # verify.py; before native_controls.py + Docker fixture
]
LAST_PERIOD = "P5"  # current surface
PERIODS = [p for p, _ in PERIOD_BOUNDS] + [LAST_PERIOD]
CURRENT_GROUP = ("P4", "P5")  # the verify-family era

PKG_RE = re.compile(r"^(lctx|cpg)(-[a-z]+)*$")
CRATE_RE = r"(?:lctx|cpg)(?:-[a-z]+)*"
PY_AREA = "python/lctx_"  # first-party Python packages


def cwd_kind(cwd):
    if not cwd:
        return "unknown"
    c = cwd.replace("file://", "")
    if c.startswith(REPO + "/.claude/worktrees/"):
        return "claude-worktree"
    if c.startswith(REPO + "/.claude/skills") or c.startswith(REPO + "/.agents/skills"):
        return "skill-dir"
    if c == REPO or c.startswith(REPO + "/"):
        return "main"
    if c.startswith("/tmp/") and "library-context" in c:
        return "tmp-repo"
    if c.startswith("/home/paul/.cache/lctx-") or c.startswith("/home/paul/.cache/library-context"):
        return "cache-copy"  # agent-made copies, worktrees and scratch under ~/.cache
    if c.startswith("/tmp/lctx-"):
        return "tmp-copy"  # agent-made worktrees and scratch under /tmp
    if "library-skills" in c:
        return "library-skills"
    return "other"


INCLUDED_CWD = {"main", "claude-worktree", "skill-dir", "tmp-repo", "cache-copy", "tmp-copy"}


def outside_repo(path):
    """True for an absolute cd target that leaves the repository (other projects, $HOME)."""
    p = path.rstrip("/")
    if p == "/home/paul":
        return True
    if not p.startswith("/home/paul/"):
        return False
    return not (p.startswith(REPO) or p.startswith("/home/paul/.cargo") or p.startswith("/home/paul/.rustup")
                or p.startswith("/home/paul/.cache") or p.startswith("/home/paul/.local/share/library-skills")
                or p.startswith("/tmp"))
