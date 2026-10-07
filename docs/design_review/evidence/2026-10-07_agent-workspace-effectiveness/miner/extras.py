"""Cross-event W1 metrics: concurrency, context cost, log locations, missing paths, corrections, git.

Reads OUTDIR/{events.jsonl.gz, raw_cmds.jsonl.gz, session_signals.jsonl, human_msgs.jsonl} and writes
OUTDIR/extras.json. Counts only; the raw inputs stay in OUTDIR.
"""
import collections
import gzip
import json
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import config  # noqa: E402

OUT = config.OUTDIR
WIN = 600  # seconds per concurrency window


def pgroup(per):
    return "P4-5" if per in config.CURRENT_GROUP else "P0-3"


def epoch(ts):
    from datetime import datetime, timezone
    try:
        return datetime.strptime(ts[:19], "%Y-%m-%dT%H:%M:%S").replace(tzinfo=timezone.utc).timestamp()
    except Exception:
        return None


def q(v, p):
    if not v:
        return None
    v = sorted(v)
    return v[min(len(v) - 1, int(round(p * (len(v) - 1))))]


def main():
    X = {}
    # ---------- events pass ----------
    ev_meta = {}
    windows = collections.defaultdict(set)          # window -> roots active in the main checkout
    win_any = collections.defaultdict(set)          # window -> roots active anywhere
    win_signals = collections.defaultdict(collections.Counter)
    read_miss = collections.Counter()
    for line in gzip.open(os.path.join(OUT, "events.jsonl.gz"), "rt"):
        e = json.loads(line)
        ev_meta[e["id"]] = (e["rt"], e["ak"], e.get("per"))
        t = epoch(e.get("ts") or "")
        if t is None:
            continue
        w = int(t // WIN)
        root = "%s:%s" % (e["rt"], e.get("root") or e["sid"])
        win_any[w].add(root)
        if e.get("cwdk") == "main":
            windows[w].add(root)
        if e.get("lockwait"):
            win_signals[w]["cargo-lock-wait"] += 1
            read_miss["lockwait|%s|%s|%s-%s" % (e.get("cwdk"), pgroup(e.get("per")), e["rt"], e["ak"])] += 1
        if e.get("shell"):
            read_miss["shell|%s|%s|%s-%s" % (e.get("cwdk"), pgroup(e.get("per")), e["rt"], e["ak"])] += 1
        h = e.get("heur") or ""
        if h == "env:git-index-lock":
            win_signals[w]["git-index-lock"] += 1
        if h in ("tool-error:stale-read",) or h.startswith("patch:"):
            win_signals[w]["edit-conflict-like"] += 1
        if e.get("out") in ("code_failure", "env_failure", "other_failure", "usage_error", "masked-failure", "timeout"):
            win_signals[w]["failures"] += 1
        if e.get("shell"):
            win_signals[w]["shell"] += 1
        if e.get("tool") in ("Read", "Glob", "Grep") and h == "tool-error:missing-file":
            read_miss["%s|%s" % (e["tool"], pgroup(e.get("per")))] += 1
    conc = collections.Counter()
    sig_by = collections.defaultdict(collections.Counter)
    for w, roots in win_any.items():
        n_main = len(windows.get(w, ()))
        k = "main>=2" if n_main >= 2 else ("main=1" if n_main == 1 else "main=0")
        conc[k] += 1
        conc["any>=2" if len(roots) >= 2 else "any<2"] += 1
        conc["max_roots_any"] = max(conc["max_roots_any"], len(roots))
        conc["max_roots_main"] = max(conc["max_roots_main"], n_main)
        sig_by[k].update(win_signals[w])
    X["concurrency"] = {"window_s": WIN, "windows": dict(conc),
                        "signals_by_window_kind": {k: dict(v) for k, v in sig_by.items()}}
    X["by_location"] = dict(read_miss)
    # ---------- context cost ----------
    cc = collections.defaultdict(list)
    codex_tok = collections.defaultdict(list)
    trunc = collections.Counter()
    ss = collections.Counter()
    for line in open(os.path.join(OUT, "session_signals.jsonl")):
        s = json.loads(line)
        if s["rt"] == "claude":
            if s["ak"] == "main" and s.get("first_usage"):
                u = s["first_usage"]
                tot = (u.get("input") or 0) + (u.get("cache_creation") or 0) + (u.get("cache_read") or 0)
                fam = "opus" if "opus" in (s.get("model") or "") else ("sonnet" if "sonnet" in (s.get("model") or "") else "other")
                cc["%s|%s" % (fam, pgroup(s.get("per")))].append(tot)
            ss["sessions|%s" % s["ak"]] += 1
            ss["sessionstart_superpowers|%s" % s["ak"]] += 1 if s.get("sessionstart_superpowers") else 0
            ss["compactions|%s" % s["ak"]] += s.get("compactions") or 0
            ss["denials|%s" % s["ak"]] += s.get("denials") or 0
            ss["posttool_hook_ctx_sessions|%s" % s["ak"]] += 1 if s.get("posttool_hook_ctx") else 0
        else:
            if s.get("tokens_used"):
                codex_tok["%s|%s" % (s["ak"], pgroup(s.get("per")))].append(s["tokens_used"])
            trunc["truncated_outputs|%s|%s" % (s["ak"], pgroup(s.get("per")))] += s.get("truncated_outputs") or 0
            trunc["threads|%s|%s" % (s["ak"], pgroup(s.get("per")))] += 1
    X["claude_first_request_tokens"] = {k: {"n": len(v), "p50": q(v, .5), "min": min(v), "max": max(v)} for k, v in cc.items()}
    X["claude_session_signals"] = dict(ss)
    X["codex_tokens_per_thread"] = {k: {"n": len(v), "p50": q(v, .5), "p90": q(v, .9), "sum_M": round(sum(v) / 1e6, 1)} for k, v in codex_tok.items()}
    X["codex_truncation"] = dict(trunc)
    # ---------- raw command pass: log locations, missing paths ----------
    redir_re = re.compile(r"(?:\d?>>?|&>>?|\btee\s+(?:-a\s+)?)\s*([^\s;|&()]+)")
    loc = collections.Counter()
    miss_re = re.compile(r"(?:rg|sed|cat|head|tail|ls|wc|nl|grep): (?:can't read |cannot access '?)?([^\s:']+)'?: (?:IO error for operation on [^\n]*?: )?No such file or directory|IO error for operation on ([^\s:]+): No such file")
    miss = collections.Counter()
    removed_crates = {"cpg-schema", "lctx-postgres", "cpg-store", "lctx-store"}
    for line in gzip.open(os.path.join(OUT, "raw_cmds.jsonl.gz"), "rt"):
        r = json.loads(line)
        meta = ev_meta.get(r["id"])
        if not meta:
            continue
        rt, ak, per = meta
        g = pgroup(per)
        for m in redir_re.finditer(r["cmd"]):
            tgt = m.group(1).strip("'\"")
            if tgt.startswith("/dev/") or tgt.startswith("&") or not tgt:
                continue
            if "/" not in tgt and not re.search(r"\.(log|txt|out|json|jsonl|xml)$", tgt):
                continue  # comparison operators and arrows inside inline code, not redirects
            if "/.cache/lctx-" in tgt or "/.cache/library-context" in tgt:
                k = "~/.cache/lctx-*"
            elif tgt.startswith("/tmp/claude-") or "scratchpad" in tgt:
                k = "claude-scratchpad"
            elif tgt.startswith("/tmp/"):
                k = "/tmp/*"
            elif tgt.startswith("build/") or "/library-context/build/" in tgt:
                k = "build/"
            elif tgt.startswith("target/") or "/target/" in tgt:
                k = "target/"
            elif tgt.startswith("$"):
                k = "$VAR"
            else:
                k = "other"
            loc["%s|%s|%s" % (rt, g, k)] += 1
        for m in miss_re.finditer(r.get("otail") or ""):
            p = m.group(1) or m.group(2)
            rel = p.replace(config.REPO + "/", "")
            if rel.startswith("/"):
                c = "outside"
            elif "*" in rel or "{" in rel:
                c = "glob"
            else:
                full = os.path.join(config.REPO, rel)
                mc = re.match(r"crates/([a-z-]+)/", rel)
                if os.path.exists(full):
                    c = "exists-at-HEAD"
                elif rel.endswith("/mod.rs") and os.path.exists(full[:-7] + ".rs"):
                    c = "mod.rs-vs-file.rs"
                elif rel.endswith(".rs") and os.path.exists(full[:-3] + "/mod.rs"):
                    c = "file.rs-vs-mod.rs"
                elif mc and (mc.group(1) in removed_crates or not os.path.isdir(os.path.join(config.REPO, "crates", mc.group(1)))):
                    c = "crate-removed-or-absent"
                elif rel.startswith("docs/") and not os.path.exists(full):
                    c = "docs-path-absent(retired-or-renamed)"
                elif os.path.isdir(os.path.dirname(full)):
                    c = "dir-exists-file-absent"
                else:
                    c = "path-absent"
            miss["%s|%s" % (g, c)] += 1
    X["redirect_targets"] = dict(loc)
    X["missing_path_categories"] = dict(miss)
    # ---------- maintainer messages (categories only) ----------
    corr_re = re.compile(r"(?i)\b(no,|don't|do not|stop|wrong|instead|why did you|not what i|revert|undo|shouldn't|should not|never|that's not|incorrect)\b")
    topics = {
        "testing": r"(?i)\b(test|tests|qualif|nextest|pytest|verify|gate|check)\w*",
        "process/ceremony": r"(?i)\b(ceremony|process|adr|review|evidence|plan|status|handoff)\w*",
        "environment/build": r"(?i)\b(build|cargo|uv|sync|venv|docker|sccache|disk|memory|environment|toolchain|compile)\w*",
        "scope": r"(?i)\b(scope|only|just do|stay|focus)\b",
        "git/commit": r"(?i)\b(commit|push|branch|worktree|stash)\w*",
        "speed": r"(?i)\b(slow|speed|faster|took|long time|hours?)\b",
        "concurrency": r"(?i)\b(another agent|other agent|parallel|concurrent|in parallel)\b",
    }
    msg = collections.Counter()
    for line in open(os.path.join(OUT, "human_msgs.jsonl")):
        m = json.loads(line)
        if m.get("auto"):
            msg["auto"] += 1
            continue
        g = pgroup(m.get("per"))
        msg["messages|%s|%s" % (m["rt"], g)] += 1
        txt = m.get("text") or ""
        if corr_re.search(txt[:600]):
            msg["correction-like|%s|%s" % (m["rt"], g)] += 1
            for name, pat in topics.items():
                if re.search(pat, txt[:1500]):
                    msg["correction-topic:%s|%s" % (name, g)] += 1
    X["maintainer_messages"] = dict(msg)
    # ---------- git outcomes ----------
    log = subprocess.run(["git", "log", "--until=%sZ" % config.CUTOFF, "--format=%H%x1f%aI%x1f%B%x1e"], cwd=config.REPO,
                         capture_output=True, text=True).stdout
    gc = collections.Counter()
    for rec in log.split("\x1e"):
        if not rec.strip():
            continue
        parts = rec.strip("\n").split("\x1f")
        if len(parts) < 3:
            continue
        body = parts[2]
        subj = body.strip().split("\n", 1)[0]
        gc["commits"] += 1
        low = body.lower()
        if re.search(r"co-authored-by: claude", low):
            gc["trailer:claude"] += 1
        if re.search(r"co-authored-by: .*(codex|openai|gpt)", low):
            gc["trailer:codex"] += 1
        for word, pat in (("passed", r"\bpass(ed|es)?\b"), ("failed", r"\bfail(ed|s|ure)?\b"), ("pending", r"\bpending\b"),
                          ("not_run", r"not[_ ]run"), ("blocked", r"\bblocked\b"), ("stopped", r"\bstopped\b|interrupted"),
                          ("fix", r"^fix|\bfix(es|ed)?\b|\brepair"), ("revert", r"\brevert"), ("format", r"\bformat")):
            if re.search(pat, subj.lower()):
                gc["subject:" + word] += 1
    X["git"] = dict(gc)
    json.dump(X, open(os.path.join(OUT, "extras.json"), "w"), indent=1)
    print(json.dumps({k: X[k] for k in ("concurrency", "claude_first_request_tokens", "git")}, indent=1)[:4000])


if __name__ == "__main__":
    main()
