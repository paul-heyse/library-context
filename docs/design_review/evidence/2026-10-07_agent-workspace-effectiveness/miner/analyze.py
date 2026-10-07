"""Compute W1 friction metrics from events.jsonl.gz (streamed per session)."""
import collections
import gzip
import hashlib
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import config  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OUTDIR = config.OUTDIR
EV = os.path.join(OUTDIR, "events.jsonl.gz")
CAP = 500
FAIL = {"code_failure", "usage_error", "env_failure", "other_failure", "masked-failure", "timeout"}
EDIT_TOOLS = {"Edit", "Write", "MultiEdit", "NotebookEdit", "apply_patch"}
PERIODS = config.PERIODS
ENVNAME = re.compile(r"^(?:[A-Z][A-Z0-9]*_[A-Z0-9_]+|PATH|TZ|CC|CXX|HOME|LANG|RUSTFLAGS|RUSTDOCFLAGS|PYTHONPATH|VIRTUAL_ENV|CFLAGS|CXXFLAGS|LDFLAGS|RUSTFMT|DEBUG)$")


def q(vals, p):
    if not vals:
        return None
    v = sorted(vals)
    k = min(len(v) - 1, max(0, int(round(p * (len(v) - 1)))))
    return round(v[k], 2)


def stratum(e):
    return "%s-%s" % (e["rt"], e["ak"])


def pgroup(e):
    p = e.get("per")
    if p in config.CURRENT_GROUP:
        return "P4-5"
    return "P0-3" if p in PERIODS else "P?"


def keys(e):
    s = stratum(e)
    return ["all", s, "per:" + e.get("per", "?"), s + "|" + pgroup(e), "all|" + pgroup(e)]


class C:
    """Nested counter keyed by stratum keys."""

    def __init__(self):
        self.d = collections.defaultdict(collections.Counter)

    def add(self, e, item, n=1):
        for k in keys(e):
            self.d[k][item] += n

    def to(self, top=None):
        return {k: dict(v.most_common(top)) for k, v in self.d.items()}


class W:
    """Wall-time lists keyed by stratum and item."""

    def __init__(self):
        self.d = collections.defaultdict(lambda: collections.defaultdict(list))

    def add(self, e, item, v):
        if v is None:
            return
        for k in keys(e):
            self.d[k][item].append(v)

    def summary(self):
        out = {}
        for k, items in self.d.items():
            out[k] = {it: {"n": len(v), "sum": round(sum(v), 1), "sum_h": round(sum(v) / 3600, 2), "p50": q(v, .5), "p90": q(v, .9), "max": round(max(v), 1)}
                      for it, v in items.items()}
        return out


def is_edit(e):
    if e.get("tool") in EDIT_TOOLS and e.get("out") in ("success", None):
        return True
    if e.get("shell") and (e.get("cls") == "edit-shell" or e.get("bash_edit") or any(s.startswith("edit-shell|") for s in e.get("segs", []))):
        return True
    return False


def is_discovery(e):
    if e.get("shell"):
        if e.get("cls") == "just-discovery" or any(s.startswith("just-discovery|") for s in e.get("segs", [])):
            return "just-list/show"
        paths = e.get("paths") or []
        if e.get("cls") == "search-read" or True:
            if "<justfile>" in paths:
                return "read-justfile"
            if "<path:docs-dev>" in paths:
                return "read-docs-dev"
            if "<agents-md>" in paths:
                return "read-agents-md"
    else:
        rp = e.get("rpath")
        if rp in ("<justfile>",):
            return "Read-justfile"
        if rp == "<path:docs-dev>":
            return "Read-docs-dev"
        if rp == "<agents-md>":
            return "Read-agents-md"
    return None


def hsel(eid):
    return int(hashlib.md5(eid.encode()).hexdigest()[:8], 16)


def loc_kind(e):
    cd = e.get("cd_abs") or []
    if cd:
        c = cd[-1]
        if "/.claude/worktrees/" in c:
            return "claude-worktree"
        if config.cwd_kind(c) in ("cache-copy", "tmp-copy"):
            return "worktree/copy"
        if c.startswith(config.REPO):
            return "main-checkout"
    ck = e.get("cwdk")
    if ck == "main" or ck == "skill-dir":
        return "main-checkout"
    if ck == "claude-worktree":
        return "claude-worktree"
    return "worktree/copy"


def main():
    rj = json.load(open(os.path.join(OUTDIR, "recipes.json")))
    recipes_now = set(rj["now"])
    recipes_all = set(rj["ever"])
    # library-context specific counters
    fam_c = C(); fam_wall = W(); uv_c = C(); uvbuild_wall = W(); wrap_c = C(); bare_c = C(); py_c = C()
    loc_c = C(); timeout_c = C(); tparam_c = C(); bigout_c = C(); docker_c = C(); lctx_c = C()
    nonshell_eh = C(); compaction_c = C(); codex_fn = C(); edit_err = C()
    M = {}
    n_events = C()
    n_shell = C()
    n_shell_cap = C()
    cls_c = C(); cls_cap = C()
    out_c = C(); heur_c = C(); out_by_cls = C()
    recipe_c = C(); recipe_cap = C(); recipe_ok = C(); recipe_wall = W(); recipe_args = C()
    raw_c = C(); raw_cap = C(); raw_sub = C(); cargo_ff = C()
    disc_kind = C()
    env_c = C(); export_c = C(); feat_c = C(); src_c = C(); envs_cap = C()
    pipe_purpose = C(); pipe_down = C()
    obytes = W(); spill = C()
    poll_c = C(); sleep_tot = W(); lockwait_wall = W()
    block_c = C(); block_next = C(); block_reason = C(); denial_c = C(); design_edit = C()
    git_c = C()
    tool_c = C(); toolsearch_q = collections.Counter(); mcp_c = C(); agent_types = C(); skills_c = C()
    wall_cls = W(); wall_cls_cap = W()
    longest = []
    retry_a = C(); retry_b = C(); retry_a_examples = collections.Counter(); retry_wall = W(); retry_a_ident_ex = collections.Counter()
    trunc_c = C(); trunc_wall = W(); trunc_ex = collections.Counter()
    fix_calls = W(); fix_wall = W(); fix_none = C()
    disc_sessions = []
    sess_summ = []
    pass
    role_c = collections.Counter(); role_shell = collections.Counter(); role_fail = collections.Counter(); role_out = collections.defaultdict(collections.Counter)
    usage_ex = collections.Counter(); env_ex = collections.Counter(); masked_ex = collections.Counter()
    bg_c = C(); bg_wall = W(); redir_c = C(); just_edit_sessions = set()
    probe_miss_paths = C()
    per_session_counts = []

    def process_session(sess):
        if not sess:
            return
        sess.sort(key=lambda e: (e.get("ts") or "", e.get("seq", 0)))
        sh = [e for e in sess if e.get("shell")]
        capset = set()
        if len(sh) > CAP:
            capset = set(sorted((e["id"] for e in sh), key=hsel)[:CAP])
        else:
            capset = set(e["id"] for e in sh)
        e0 = sess[0]
        per_session_counts.append((e0["rt"], e0["ak"], e0.get("role"), len(sh)))
        first_disc = None
        disc_n = 0
        first_ok_just = None
        for idx, e in enumerate(sess):
            n_events.add(e, "events")
            tool = e.get("tool")
            tool_c.add(e, tool)
            role = ("%s-%s:%s" % (e["rt"], e["ak"], e.get("role")))
            role_c[role] += 1
            if tool == "ToolSearch" and e.get("q"):
                toolsearch_q[e["q"][:80]] += 1
            if tool and tool.startswith("mcp__"):
                mcp_c.add(e, tool)
            if tool in ("Agent", "Task"):
                agent_types.add(e, e.get("subtype"))
            if tool == "Skill":
                skills_c.add(e, e.get("skill"))
            if tool in ("Monitor", "TaskOutput", "BashOutput", "TaskStop", "KillShell") or tool in ("fn:clock.sleep", "fn:wait", "fn:collaboration.wait_agent", "collab:wait"):
                poll_c.add(e, "tool:" + tool)
                if tool == "fn:clock.sleep" and e.get("sleep_s"):
                    sleep_tot.add(e, "codex clock.sleep", e["sleep_s"])
            if tool == "ToolSearch" and "onitor" in (e.get("q") or ""):
                poll_c.add(e, "ToolSearch:Monitor")
            # blocks / denials (any tool)
            if e.get("out") in ("blocked", "denied") or e.get("denial"):
                h = e.get("heur") or ("denied:" + str(e.get("denial")))
                block_c.add(e, h)
                if e.get("block_reason"):
                    block_reason.add(e, (h + " :: " + e["block_reason"])[:200])
                if e.get("denial"):
                    denial_c.add(e, e["denial"])
                # next action
                nxt = sess[idx + 1] if idx + 1 < len(sess) else None
                if nxt is None:
                    na = "end-of-session"
                elif nxt.get("shell"):
                    na = "shell:" + nxt.get("cls", "?") + (":edit" if is_edit(nxt) else "")
                else:
                    na = "tool:" + str(nxt.get("tool"))
                block_next.add(e, h + " -> " + na)
            if tool == "_compaction":
                compaction_c.add(e, "compaction")
            if tool and tool.startswith("fn:"):
                codex_fn.add(e, tool)
            if not e.get("shell") and e.get("heur"):
                nonshell_eh.add(e, "%s|%s" % (tool, e.get("heur")))
            if tool in ("Edit", "Write", "MultiEdit", "apply_patch", "_patch_fail") and e.get("out") not in ("success", None):
                edit_err.add(e, "%s|%s" % (tool, e.get("heur")))
            if not e.get("shell"):
                continue
            # ---------- shell metrics ----------
            cap_in = e["id"] in capset
            n_shell.add(e, "shell")
            if cap_in:
                n_shell_cap.add(e, "shell")
            cls = e.get("cls")
            role_shell[role] += 1
            role_out[role][e.get("out")] += 1
            if e.get("out") in FAIL:
                role_fail[role] += 1
            cls_c.add(e, cls)
            if cap_in:
                cls_cap.add(e, cls)
            out_c.add(e, e.get("out"))
            heur_c.add(e, e.get("heur"))
            out_by_cls.add(e, "%s|%s" % (cls, e.get("out")))
            loc_c.add(e, e.get("cwdk"))
            fam = e.get("fam")
            if fam:
                k = "%s|%s|%s|%s" % (e.get("cls"), fam, e.get("bnd") or "-", "filtered" if e.get("flt") else "unfiltered")
                fam_c.add(e, k + "|" + str(e.get("out")))
                fam_wall.add(e, "%s|%s" % (fam, e.get("bnd") or "-"), None if e.get("bg") else e.get("wall"))
            feats = e.get("feats") or []
            for f in feats:
                if f.startswith("uv-"):
                    uv_c.add(e, f + "|" + str(e.get("out")))
                if f.startswith("wrapper:") or f.startswith("python:") or f == "flock":
                    wrap_c.add(e, f)
                if f == "flock":
                    fam_wall.add(e, "agent-flock:" + cls, None if e.get("bg") else e.get("wall"))
            if e.get("beval"):
                wrap_c.add(e, "eval build_environment --shell")
            ran_uv = any(f.startswith("uv-") for f in feats) or (e.get("sub") or "").startswith("uv ") or cls == "just" or bool(e.get("fam"))
            if e.get("uvbuild") and not ran_uv:
                uv_c.add(e, "native-wheel-build-seen-in-log-read")
            if e.get("uvbuild") and ran_uv:
                uv_c.add(e, "native-wheel-build-in-output")
                uvbuild_wall.add(e, "uvbuild", None if e.get("bg") else e.get("wall"))
            if e.get("uvsync"):
                uv_c.add(e, "sync-activity-in-output")
            sub = e.get("sub") or ""
            if cls == "raw" and sub in ("cargo nextest run", "cargo test", "cargo nextest list"):
                wrapped = any(f.startswith("wrapper:") for f in feats)
                envset = bool(e.get("lctxenv")) or any(v.startswith("LCTX_") for v in (e.get("envs") or []))
                bare_c.add(e, "%s|%s|%s" % (sub, "wrapped" if wrapped else ("lctx-env-inline" if envset else "bare"), e.get("out")))
            for sg in e.get("segs", []):
                c2, s2 = sg.split("|", 1)
                if s2.startswith("python") or s2.startswith("verify:") or s2.startswith("native_controls:"):
                    pass
            if any(f.startswith("python:") for f in feats) or (e.get("norm") or "").startswith("python"):
                py_c.add(e, "python-invocation")
            if e.get("heur") == "env:py312-syntax":
                py_c.add(e, "py312-syntax-failure")
            if sub.startswith("docker"):
                docker_c.add(e, sub + "|" + str(e.get("out")))
            if sub.startswith("lctx "):
                lctx_c.add(e, sub + "|" + str(e.get("out")))
            if e.get("out") == "timeout":
                timeout_c.add(e, cls)
            if e.get("tparam"):
                tparam_c.add(e, "timeout-param:%s" % ("<=120s" if e["tparam"] <= 120000 else ("<=600s" if e["tparam"] <= 600000 else ">600s")))
            if e.get("bigout"):
                bigout_c.add(e, cls)
            wall = e.get("wall")
            if e.get("bg"):
                bg_c.add(e, "bg-run")
                if e.get("wall_bg") is not None:
                    bg_wall.add(e, cls, e["wall_bg"])
            else:
                wall_cls.add(e, cls, wall)
                if cap_in:
                    wall_cls_cap.add(e, cls, wall)
            if wall is not None and not e.get("bg"):
                longest.append((wall, e["rt"], e["ak"], e.get("role"), e.get("per"), e.get("norm", "")[:120], e.get("out"), e.get("heur")))
                if len(longest) > 400:
                    longest.sort(reverse=True)
                    del longest[60:]
            if e.get("obytes") is not None:
                obytes.add(e, cls, e["obytes"])
            if e.get("spill"):
                spill.add(e, cls)
            # recipes (all just segments)
            for sg in e.get("segs", []):
                c2, sub = sg.split("|", 1)
                if c2 == "just":
                    recipe_c.add(e, sub)
                    if cap_in:
                        recipe_cap.add(e, sub)
                    if e.get("cls") == "just" and e.get("recipe") == sub:
                        recipe_ok.add(e, "%s|%s" % (sub, e.get("out")))
                        recipe_wall.add(e, sub, None if e.get("bg") else wall)
                        recipe_args.add(e, e.get("norm", "")[:120])
                if c2 == "raw":
                    raw_sub.add(e, sub)
                if c2 == "git":
                    git_c.add(e, "%s|%s" % (loc_kind(e), sub))
            if cls == "raw":
                raw_c.add(e, e.get("norm", "")[:140])
                if cap_in:
                    raw_cap.add(e, e.get("norm", "")[:140])
                sub = e.get("sub", "")
                if sub.startswith("cargo "):
                    cs = sub.split(" ", 1)[1]
                    if cs in ("check", "build", "test", "nextest run", "clippy", "nextest list", "run"):
                        cargo_ff.add(e, "%s|%s" % (cs, "force-validate" if e.get("ff") else "no-force-validate"))
            # discovery
            dk = is_discovery(e)
            if dk:
                disc_kind.add(e, dk)
            # env
            for v in e.get("envs", []) or []:
                if not ENVNAME.match(v):
                    continue
                env_c.add(e, v)
                if cap_in:
                    envs_cap.add(e, v)
                pass
            for v in e.get("exports", []) or []:
                export_c.add(e, v)
                pass
            for f in e.get("feats", []) or []:
                feat_c.add(e, f)
            for sname in e.get("src", []) or []:
                src_c.add(e, sname)
            if e.get("justfile"):
                feat_c.add(e, "just -f " + os.path.basename(e["justfile"]))
            if e.get("toolchain"):
                feat_c.add(e, "cargo " + e["toolchain"])
            if cls in ("just", "raw") and e.get("redir") and not e.get("bg"):
                redir_c.add(e, "build/test redirected to file")
                follow = 0
                for j in range(idx + 1, min(len(sess), idx + 6)):
                    x = sess[j]
                    if x.get("shell") and x.get("cls") in ("search-read", "process-monitor") and any(pk in (x.get("paths") or []) for pk in ("<path:tmp>", "<path:build>")):
                        follow += 1
                if follow:
                    redir_c.add(e, "followed by log read(s)")
                    redir_c.add(e, "log reads (within 5 calls)", follow)
            if cls in ("just", "raw") and e.get("pdown") and not e.get("redir"):
                redir_c.add(e, "build/test piped to reducer")
            if any(sg in ("process-monitor|ps", "process-monitor|pgrep") for sg in e.get("segs", [])):
                poll_c.add(e, "process-check(ps/pgrep)")
            # pipes
            if e.get("pdown"):
                up = e.get("pup") or "?"
                kind = "output-reduction(build/test)" if up in ("just", "raw") else ("file-read-filter" if up == "search-read" else ("git-filter" if up in ("git", "git-mut") else "other:" + up))
                pipe_purpose.add(e, kind + (" +2>&1" if e.get("e2o") else ""))
                for d in e["pdown"]:
                    pipe_down.add(e, "%s|%s" % (kind, d))
            # polling
            if e.get("poll"):
                poll_c.add(e, "shell-poll-loop:" + str(e.get("loop")))
            if any(s == "process-monitor|sleep" for s in e.get("segs", [])):
                poll_c.add(e, "shell-sleep")
                if e.get("sleep_s"):
                    sleep_tot.add(e, "shell sleep", e["sleep_s"])
            if e.get("heur") == "sleep-blocked":
                poll_c.add(e, "Blocked: sleep")
            if e.get("lockwait") and cls in ("just", "raw"):
                poll_c.add(e, "cargo-lock-wait-in-output")
                lockwait_wall.add(e, "lockwait", wall)
            if e.get("out") == "probe_miss" and e.get("heur", "").startswith("probe:missing"):
                probe_miss_paths.add(e, cls)
            if e.get("out") == "usage_error":
                usage_ex[(e.get("heur"), e.get("norm", "")[:100])] += 1
            if e.get("out") == "env_failure":
                env_ex[(e.get("heur"), e.get("norm", "")[:100])] += 1
            if e.get("out") == "masked-failure":
                masked_ex[(e.get("heur", "")[:40], e.get("norm", "")[:100], ",".join(e.get("pdown") or []))] += 1
            # discovery cost
            if dk:
                disc_n += 1
                if first_disc is None:
                    first_disc = idx
            if first_disc is not None and first_ok_just is None and cls == "just" and e.get("out") == "success" and idx > first_disc:
                first_ok_just = idx
        # Read-tool discovery (non-shell)
        for idx, e in enumerate(sess):
            if not e.get("shell"):
                dk = is_discovery(e)
                if dk:
                    disc_kind.add(e, dk)
                    disc_n += 1
                    if first_disc is None or idx < first_disc:
                        first_disc = idx
        if first_disc is not None:
            ok_idx = None
            for j in range(first_disc + 1, len(sess)):
                x = sess[j]
                if x.get("shell") and x.get("cls") == "just" and x.get("out") == "success":
                    ok_idx = j
                    break
            from_ts = sess[first_disc].get("ts"); to_ts = sess[ok_idx].get("ts") if ok_idx is not None else None
            edited_justfile = any("justfile" in (x.get("edit_areas") or []) or (x.get("shell") and x.get("cls") == "edit-shell" and "<justfile>" in (x.get("wtgt") or [])) for x in sess)
            disc_sessions.append({"rt": e0["rt"], "ak": e0["ak"], "role": e0.get("role"), "per": sess[first_disc].get("per"), "edited_justfile": edited_justfile,
                                  "disc_calls": disc_n, "calls_to_ok_just": (ok_idx - first_disc) if ok_idx is not None else None,
                                  "secs_to_ok_just": _dt(from_ts, to_ts)})
        # ---------- retry chains / truncation reruns / fix effort ----------
        for i, f in enumerate(sess):
            if not f.get("shell"):
                continue
            fcls = f.get("cls")
            if fcls in ("just", "raw", "inline-script", "other") and f.get("out") in FAIL:
                edit_between = False
                for j in range(i + 1, min(len(sess), i + 12)):
                    r = sess[j]
                    if is_edit(r):
                        edit_between = True
                    if not r.get("shell"):
                        continue
                    same_core = r.get("h_core") == f.get("h_core")
                    same_family = r.get("cls") == fcls and r.get("sub") == f.get("sub") and (r.get("recipe") == f.get("recipe")) and fcls in ("just", "raw")
                    if not (same_core or same_family):
                        continue
                    if r.get("h_full") == f.get("h_full"):
                        kind = "identical"
                    elif same_core and r.get("h_pipe") != f.get("h_pipe") and r.get("h_envs") == f.get("h_envs"):
                        kind = "pipe-only"
                    elif same_core and r.get("h_envs") != f.get("h_envs"):
                        kind = "env-changed"
                    elif same_core:
                        kind = "prefix/redirect-changed"
                    elif r.get("norm") == f.get("norm"):
                        kind = "args-changed(same-shape)"
                    else:
                        kind = "flags/shape-changed"
                    if edit_between:
                        retry_b.add(f, kind)
                    else:
                        retry_a.add(f, kind)
                        retry_wall.add(f, kind, r.get("wall"))
                        retry_a_examples[(kind, f.get("heur", "")[:40], (f.get("norm") or "")[:90])] += 1
                    break
            # truncation reruns: same build/test core, different pipe, no edit between
            if fcls in ("just", "raw"):
                for j in range(i + 1, min(len(sess), i + 6)):
                    r = sess[j]
                    if is_edit(r):
                        break
                    if r.get("shell") and r.get("h_core") == f.get("h_core"):
                        if r.get("h_pipe") != f.get("h_pipe") and (f.get("pdown") or r.get("pdown")):
                            trunc_c.add(f, "rerun-different-pipe")
                            trunc_wall.add(f, "rerun-different-pipe", r.get("wall"))
                            trunc_ex[((f.get("norm") or "")[:90], ",".join(f.get("pdown") or []) + " -> " + ",".join(r.get("pdown") or []))] += 1
                        break
            # failure -> fix effort
            if fcls in ("just", "raw") and f.get("out") in ("code_failure", "masked-failure") and f.get("ecr"):
                tgt = set(f["ecr"])
                found = None
                calls = 0
                for j in range(i + 1, len(sess)):
                    r = sess[j]
                    calls += 1
                    ec = set(r.get("edit_crates") or [])
                    if is_edit(r) and ec & tgt:
                        found = j
                        break
                if found is not None:
                    fix_calls.add(f, "calls", calls)
                    fix_wall.add(f, "secs", _dt(f.get("te") or f.get("ts"), sess[found].get("ts")))
                    fix_none.add(f, "fixed-in-session")
                else:
                    fix_none.add(f, "no-matching-edit")

    def _dt(a, b):
        from datetime import datetime
        if not a or not b:
            return None
        try:
            ta = datetime.strptime(a[:19], "%Y-%m-%dT%H:%M:%S")
            tb = datetime.strptime(b[:19], "%Y-%m-%dT%H:%M:%S")
            return max(0.0, (tb - ta).total_seconds())
        except Exception:
            return None

    globals()["_dt"] = _dt
    cur, cur_sid = [], None
    nlines = 0
    for line in gzip.open(EV, "rt", encoding="utf-8"):
        nlines += 1
        e = json.loads(line)
        if e["sid"] != cur_sid:
            process_session(cur)
            cur, cur_sid = [], e["sid"]
        cur.append(e)
    process_session(cur)

    # ---------- assemble ----------
    used = set()
    for k, v in recipe_c.d.items():
        if k == "all":
            used = set(v)
    used_p45 = set(recipe_c.d.get("all|P4-5", {}))
    M["n_events_total"] = nlines
    M["events"] = n_events.to()
    M["shell"] = n_shell.to()
    M["shell_capped"] = n_shell_cap.to()
    M["classes"] = cls_c.to()
    M["classes_capped"] = cls_cap.to()
    M["outcomes"] = out_c.to()
    M["heuristics"] = heur_c.to(80)
    M["outcome_by_class"] = out_by_cls.to(80)
    M["recipes"] = {"used": recipe_c.to(), "used_capped": recipe_cap.to(), "outcomes": recipe_ok.to(), "wall": recipe_wall.summary(),
                    "arg_shapes": recipe_args.to(60),
                    "current_count": len(recipes_now), "historical_only": sorted(recipes_all - recipes_now),
                    "used_not_current": sorted(used - recipes_now),
                    "never_used_current": sorted(recipes_now - used),
                    "never_used_current_p45": sorted(recipes_now - used_p45)}
    M["raw"] = {"patterns": raw_c.to(80), "patterns_capped": raw_cap.to(80), "subs": raw_sub.to(60), "cargo_force_validate": cargo_ff.to()}
    M["discovery"] = {"kinds": disc_kind.to(), "sessions": disc_sessions}
    M["env"] = {"inline": env_c.to(60), "inline_capped": envs_cap.to(60), "exports": export_c.to(60), "features": feat_c.to(60), "sourced": src_c.to(40)}
    M["pipes"] = {"purpose": pipe_purpose.to(), "down": pipe_down.to(60), "redirect_vs_pipe": redir_c.to()}
    M["output"] = {"bytes": obytes.summary(), "spill": spill.to()}
    M["polling"] = {"counts": poll_c.to(), "sleep": sleep_tot.summary(), "lockwait": lockwait_wall.summary(), "bg": bg_c.to(), "bg_wall": bg_wall.summary()}
    M["blocks"] = {"kinds": block_c.to(), "reasons": block_reason.to(40), "next": block_next.to(60), "denials": denial_c.to(), "design_edit": design_edit.to()}
    M["git"] = git_c.to(80)
    M["tools"] = {"by_stratum": tool_c.to(80), "toolsearch_queries": dict(toolsearch_q.most_common(30)), "mcp": mcp_c.to(), "agent_types": agent_types.to(), "skills": skills_c.to(40)}
    M["wall_by_class"] = wall_cls.summary()
    M["wall_by_class_capped"] = wall_cls_cap.summary()
    longest.sort(reverse=True)
    M["longest"] = [dict(zip(["wall_s", "rt", "ak", "role", "per", "norm", "out", "heur"], x)) for x in longest[:30]]
    M["retry"] = {"no_edit": retry_a.to(), "with_edit": retry_b.to(), "no_edit_wall": retry_wall.summary(),
                  "examples": [{"kind": k[0], "heur": k[1], "norm": k[2], "n": n} for k, n in retry_a_examples.most_common(40)]}
    M["truncation"] = {"counts": trunc_c.to(), "wall": trunc_wall.summary(), "examples": [{"norm": k[0], "pipes": k[1], "n": n} for k, n in trunc_ex.most_common(25)]}
    M["fix_effort"] = {"calls": fix_calls.summary(), "secs": fix_wall.summary(), "resolution": fix_none.to()}
    M["lctx"] = {"families": fam_c.to(), "family_wall": fam_wall.summary(), "uv": uv_c.to(), "uvbuild_wall": uvbuild_wall.summary(),
                 "wrappers": wrap_c.to(), "bare_cargo_tests": bare_c.to(), "python": py_c.to(), "locations": loc_c.to(),
                 "timeouts": timeout_c.to(), "timeout_params": tparam_c.to(), "big_output": bigout_c.to(), "docker": docker_c.to(),
                 "lctx_cli": lctx_c.to(), "nonshell_errors": nonshell_eh.to(60), "compactions": compaction_c.to(),
                 "codex_fn": codex_fn.to(), "edit_errors": edit_err.to()}
    M["roles"] = {"events": dict(role_c), "shell": dict(role_shell), "fail": dict(role_fail), "outcomes": {k: dict(v) for k, v in role_out.items()}}
    M["usage_examples"] = [{"heur": k[0], "norm": k[1], "n": n} for k, n in usage_ex.most_common(40)]
    M["env_examples"] = [{"heur": k[0], "norm": k[1], "n": n} for k, n in env_ex.most_common(40)]
    M["masked_examples"] = [{"heur": k[0], "norm": k[1], "pipes": k[2], "n": n} for k, n in masked_ex.most_common(30)]
    M["probe_miss"] = probe_miss_paths.to()
    # sessions over cap
    big = [x for x in per_session_counts if x[3] > CAP]
    M["sessions"] = {"n": len(per_session_counts), "over_cap": len(big), "shell_in_over_cap": sum(x[3] for x in big),
                     "max_shell": max((x[3] for x in per_session_counts), default=0)}
    json.dump(M, open(os.path.join(OUTDIR, "metrics.json"), "w"), indent=1, default=str)
    print("events", nlines, "sessions", len(per_session_counts), "over_cap", len(big))


if __name__ == "__main__":
    main()
