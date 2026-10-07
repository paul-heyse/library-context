"""Index the repository's Codex threads from ~/.codex/state_5.sqlite (read-only) into OUTDIR."""
import json
import os
import sqlite3
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import config  # noqa: E402


def main():
    con = sqlite3.connect("file:%s?mode=ro" % config.CODEX_STATE_DB, uri=True)
    where = " or ".join("cwd like ?" for _ in config.CODEX_CWD_LIKE)
    rows = con.execute(
        "select id, rollout_path, created_at_ms, source, cwd, agent_role, agent_nickname, tokens_used, "
        "cli_version, model, reasoning_effort, thread_source, originator from threads where " + where,
        config.CODEX_CWD_LIKE).fetchall()
    out = []
    for r in rows:
        (tid, path, cms, src, cwd, role, nick, tok, cli, model, effort, tsrc, orig) = r
        try:
            src_v = json.loads(src)
        except Exception:
            src_v = src
        from datetime import datetime, timezone
        ts = datetime.fromtimestamp((cms or 0) / 1000, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")
        out.append({"id": tid, "path": path, "ts": ts, "source": src_v, "cwd": cwd, "agent_role": role,
                    "agent_nickname": nick, "tokens_used": tok, "cli_version": cli, "model": model,
                    "effort": effort, "thread_source": tsrc, "originator": orig,
                    "size": os.path.getsize(path) if os.path.exists(path) else None})
    os.makedirs(config.OUTDIR, exist_ok=True)
    json.dump(out, open(os.path.join(config.OUTDIR, "codex_index.json"), "w"))
    print("threads", len(out), "missing rollouts", sum(1 for r in out if r["size"] is None))


if __name__ == "__main__":
    main()
