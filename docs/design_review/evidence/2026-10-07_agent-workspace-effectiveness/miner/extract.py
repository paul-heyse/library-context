"""Stream Claude Code and Codex transcripts into normalized per-tool-call events (stdlib only).

Run order (from this directory, Python 3.14, stdlib only):
    python -I codex_index.py && python -I recipes.py && python -I extract.py \
      && python -I analyze.py && python -I extras.py && python -I report.py && python -I aggregate.py
Everything is written to config.OUTDIR (gitignored; contains command text) except aggregate.py's
metrics.json, which holds counts only. Ported from the pse-arrow W1 miner; config.py holds the
repository specifics.
"""
import collections
import glob
import gzip
import hashlib
import json
import os
import re
import sys
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import config  # noqa: E402
from config import INCLUDED_CWD, cwd_kind  # noqa: E402
from shparse import path_kind, summarize  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OUTDIR = config.OUTDIR
CLAUDE_DIR = config.CLAUDE_DIR
EXCLUDED = config.EXCLUDED_SESSIONS
ASSESSMENT_CUTOFF = config.CUTOFF
PERIOD_BOUNDS = config.PERIOD_BOUNDS


def period(ts):
    if not ts:
        return "P?"
    t = ts[:19]
    for name, bound in PERIOD_BOUNDS:
        if t < bound:
            return name
    return config.LAST_PERIOD


def iso_from_ms(ms):
    return datetime.fromtimestamp(ms / 1000.0, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:23] + "Z"


def parse_ts(ts):
    try:
        return datetime.strptime(ts[:23].rstrip("Z"), "%Y-%m-%dT%H:%M:%S.%f").replace(tzinfo=timezone.utc).timestamp()
    except Exception:
        try:
            return datetime.strptime(ts[:19], "%Y-%m-%dT%H:%M:%S").replace(tzinfo=timezone.utc).timestamp()
        except Exception:
            return None


def wt_id(cwd):
    c = (cwd or "").replace("file://", "")
    m = re.match(re.escape(config.REPO) + r"/\.claude/worktrees/([^/]+)", c)
    return m.group(1) if m else ""


def h(s):
    return hashlib.blake2b(s.encode("utf-8", "replace"), digest_size=6).hexdigest()


# ---------------- outcome heuristics ----------------
def rx(p, f=0):
    return re.compile(p, f)


USAGE = [
    ("usage:just-unknown-recipe", rx(r"Justfile does not contain recipe|Justfile does not contain")),
    ("usage:just-arity", rx(r"Recipe `[^`]+` got \d+ arguments? but takes")),
    ("usage:just-other", rx(r"error: Recipe `[^`]+` has no parameter|error: Unknown (setting|start of token)|error: Expected '")),
    ("usage:cargo-unknown-pkg", rx(r"package ID specification `[^`]+` did not match|error: package\(s\) `[^`]+` not found in workspace|no matching package named|package `[^`]+` cannot be tested because it requires dev-dependencies|error: none of the packages")),
    ("usage:cargo-unknown-feature", rx(r"does not contain this feature|none of the selected packages contains these features|does not have (the |these )?features?|error: Package `[^`]+` does not have")),
    ("usage:cargo-unknown-target", rx(r"no (test|bin|example|bench) target named|error: no library targets found|error: no bin target named")),
    ("usage:cli-arg", rx(r"error: unexpected argument|error: unrecognized subcommand|error: invalid value '|error: Found argument|unrecognized arguments:|error: unrecognized option|[Uu]nknown option|invalid option --|error: the argument '[^']+' cannot be used|error: a value is required for|error: unexpected value|error: no such (subcommand|option)|error: unknown (command|flag)|For more information, try '--help'")),
    ("usage:shell-syntax", rx(r"(?:^|\n)(?:/bin/|/usr/bin/)?(?:ba|z)?sh: [^\n]*(syntax error near unexpected token|unexpected EOF while looking for matching|bad substitution|syntax error: unexpected end of file)")),
    ("usage:nextest-filter", rx(r"failed to parse filterset|error: expected filterset|error: no tests to run|Starting 0 tests|0 tests run")),
    ("usage:pytest-selection", rx(r"no tests ran in|collected 0 items|ERROR: file or directory not found|ERROR: not found:|error: unrecognized arguments")),
]
ENV = [
    ("env:tmp-output-lost", rx(r"Command output was lost")),
    ("env:enospc", rx(r"No space left on device")),
    ("env:ice", rx(r"internal compiler error|the compiler unexpectedly panicked|rustc-ice-")),
    ("env:oom-killed", rx(r"signal: 9\b|SIGKILL|\bKilled\b|[Oo]ut of memory|oom[-_]kill|memory allocation of \d+ bytes failed|exit (status|code):? 137\b")),
    ("env:text-file-busy", rx(r"Text file busy")),
    ("env:lctx-env-missing", rx(r"(?:fixture[^\n]{0,40}|LCTX_[A-Z_]+[^\n]{0,80}): NotPresent|LCTX_[A-Z_]+ (?:is )?(?:not set|unset|required|must be set|missing)|assert [^\n]{0,40}LCTX_[A-Z_]+|KeyError: 'LCTX_[A-Z_]+'")),
    ("env:py312-syntax", rx(r"multiple exception types must be parenthesized")),
    ("env:docker", rx(r"Cannot connect to the Docker daemon|docker: Error response from daemon|Unable to find image|No such image|port is already allocated|Conflict\. The container name|docker image inspect[^\n]{0,80}(No such|Error)|requires the pinned SurrealDB image")),
    ("env:surreal-conn", rx(r"(?i)surreal[^\n]{0,80}(connection refused|failed to connect|timed out|not ready)|readiness[^\n]{0,40}(timed out|failed)")),
    ("env:native-adapter", rx(r"(?:ImportError|ModuleNotFoundError)[^\n]{0,120}(lctx_semantics|_native|lib_native)|undefined symbol: [^\n]{0,80}|lctx_semantics[^\n]{0,80}(stale|mismatch|does not match)")),
    ("env:sccache", rx(r"sccache: error|sccache: caused by|Failed to execute compile|sccache: encountered fatal error")),
    ("env:linker-loader", rx(r"error while loading shared libraries|cannot open shared object file|linker `[^`]+` not found|ld(\.lld)?: (error: )?unable to find library|ld(\.lld)?: cannot find|collect2: error|could not find native static library|LIBCLANG_PATH|Unable to find libclang|mold: fatal")),
    ("env:missing-tool", rx(r"command not found|^(?:/bin/)?(?:ba)?sh: (?:line \d+: )?[^\s:]+: No such file or directory|error: no such command: `|is not installed|executable file not found|could not execute process `|Failed to spawn: `|failed to run `[^`]+`: No such file", re.M)),
    ("env:python-env", rx(r"ModuleNotFoundError|No module named|does not match the project environment path|\.venv/bin/python: No such file|No interpreter found|maturin failed|Failed to build `lctx|VIRTUAL_ENV=[^\n]{0,80}does not match")),
    ("env:network", rx(r"Could not resolve host|getaddrinfo|Temporary failure in name resolution|Network is unreachable|failed to download|certificate verify failed|error sending request|failed to fetch `")),
    ("env:git-index-lock", rx(r"index\.lock': File exists|Another git process seems to be running")),
    ("env:database", rx(r"connection to server [^\n]* failed|could not connect to server|role \"[^\"]+\" does not exist|password authentication failed|DATABASE_URL|pg_ctl: |psql: error")),
    ("env:lockfile", rx(r"lock file [^\n]* needs to be updated|--locked was passed|lockfile at [^\n]* needs to be updated|needs to be updated but --locked|The lockfile at `uv\.lock` needs to be updated")),
    ("env:toolchain", rx(r"toolchain '[^']+' is not installed|rustup could not choose a version|is only accepted on the nightly|requires rustc \d")),
    ("env:fd-limit", rx(r"Too many open files")),
    ("env:permission", rx(r"Permission denied|Operation not permitted|Read-only file system")),
    ("env:lock-wait", rx(r"Blocking waiting for file lock")),
]
CODE = [
    ("code:rust-compile", rx(r"error\[E\d{4}\]|error: could not compile|error: aborting due to|error: cannot find (macro|attribute|derive)")),
    ("code:clippy", rx(r"implied by `-D warnings`|-D clippy::|#\[deny\(clippy")),
    ("code:rust-test", rx(r"test result: FAILED|error: test run failed|\n\s+(FAIL|SIGABRT|SIGSEGV|TIMEOUT|LEAK-FAIL) \[|panicked at|failures:\n")),
    ("code:python-test", rx(r"\b[1-9]\d* (failed|errors?)\b[^\n]*(passed|in \d)|= FAILURES =|= ERRORS =|ERROR collecting|^FAILED [\w/.-]+::", re.M)),
    ("code:python-type-lint", rx(r"^(ERROR|error)\b[^\n]*\[[a-z-]+\]|Found \d+ errors?|^\S+\.py:\d+:\d+: [A-Z]+\d+|would reformat|Would reformat|^Diff in ", re.M)),
    ("code:python-exc", rx(r"Traceback \(most recent call last\)")),
    ("code:check-drift", rx(r"(?i)(is out of date|drift|differs from|stale|not up to date|does not match the generator)")),
    ("code:recipe-failed", rx(r"error: [Rr]ecipe `[^`]+` failed")),
    ("code:generic-fail", rx(r"(?i)\bFAILED\b|\bfailed\b|error:")),
]
MASKED = rx(r"test result: FAILED|error: test run failed|error\[E\d{4}\]|error: could not compile|error: [Rr]ecipe `[^`]+` failed|\b[1-9]\d* failed\b|Justfile does not contain recipe|error: unexpected argument")
LOCKWAIT = rx(r"Blocking waiting for file lock")
UVBUILD = rx(r"(?:Building|Built) lctx-semantics|Built lctx_semantics|Uninstalled \d+ packages?[^\n]*\n[^\n]*Installed")
UVSYNC_NOTE = rx(r"(?:Resolved|Prepared|Installed|Uninstalled) \d+ packages? in")
THERMO_RE = rx(r"(?!x)x")  # no cross-repo exclusion by content in this repository
CRATE_PATH = rx(r"(?:^|[\s(/'\"`:])(?:" + re.escape(config.REPO) + r"/|\./)?(?:[^\s:]*?/)?crates/(" + config.CRATE_RE + r")/")
NEXTEST_CRATE = rx(r"(?:FAIL|SIGABRT|SIGSEGV|TIMEOUT) \[[^\]]+\] (?:\(\s*\d+/\d+\) )?(" + config.CRATE_RE + r")")
COMPILE_CRATE = rx(r"could not compile `(" + config.CRATE_RE + r")`")
PY_PATH = rx(re.escape(config.PY_AREA))
BUILDTEST_CLS = {"just", "raw"}


PROBE_MISS_RE = rx(r"No such file or directory \(os error 2\)|sed: can't read|cat: [^\n]*: No such file|head: cannot open|IO error for operation on|ls: cannot access")
EXIT_MAP = {100: ("code_failure", "code:exit100-tests-failed"), 101: ("code_failure", "code:exit101-build-or-test"),
            94: ("usage_error", "usage:nextest-invalid-filterset(exit94)"), 102: ("other_failure", "nextest:cargo-metadata-failed(exit102)"),
            96: ("env_failure", "env:nextest-setup(exit96)"), 104: ("code_failure", "code:nextest-list-failed(exit104)")}
TESTISH = ("test", "unit", "nextest", "py-", "pytest", "parity", "check", "clippy")


def classify_outcome(cls, sub, exit_code, text, piped, segs=(), redir=False):
    """Return (outcome, heuristic)."""
    t = text or ""
    if exit_code == 0:
        if piped and cls in BUILDTEST_CLS and MASKED.search(t):
            return "masked-failure", "masked:" + MASKED.search(t).group(0)[:30]
        return "success", "exit0"
    if exit_code is None:
        return "unknown", "no-exit"
    if exit_code == 124:
        return "timeout", "exit124"
    for name, r in USAGE:
        if cls not in ("just", "raw", "other", "inline-script") and name != "usage:shell-syntax":
            continue
        if r.search(t):
            return "usage_error", name
    seg_classes = set(x.split("|", 1)[0] for x in segs)
    if cls not in BUILDTEST_CLS and "search-read" in seg_classes and PROBE_MISS_RE.search(t) and not re.search(r"error\[E\d{4}\]|Traceback|panicked", t):
        return "probe_miss", "probe:missing-path(compound)"
    if cls in ("search-read", "git", "just-discovery", "noop", "process-monitor"):
        for name, r in ENV:
            if name in ("env:lock-wait", "env:permission"):
                continue
            if name == "env:missing-tool" and r.search(t):
                return "env_failure", name
        if cls == "search-read":
            if re.search(r"No such file or directory|can't read|does not exist|IO error for operation", t):
                return "probe_miss", "probe:missing-path"
            if exit_code == 1:
                return "probe_miss", "probe:nomatch"
            return "probe_miss", "probe:exit%s" % exit_code
        if cls == "git":
            for name, r in ENV:
                if name == "env:git-index-lock" and r.search(t):
                    return "env_failure", name
            return "other_failure", "git:exit%s" % exit_code
        if cls == "process-monitor" and exit_code in (1, 143, -15, 130):
            return "probe_miss", "monitor:exit%s" % exit_code
    for name, r in ENV:
        if name == "env:lock-wait":
            continue
        if name == "env:oom-killed" and exit_code in (137, -9):
            return "env_failure", name
        if r.search(t):
            if name == "env:permission" and cls in BUILDTEST_CLS and re.search(r"error\[E\d{4}\]|test result: FAILED|panicked at", t):
                break
            if name == "env:python-env" and re.search(r"error\[E\d{4}\]", t):
                break
            return "env_failure", name
    if exit_code in (137, -9):
        return "env_failure", "env:sig9-exit"
    if exit_code in (143, -15, 130, -2):
        return "interrupted", "sigterm-exit%s" % exit_code
    for name, r in CODE:
        if name in ("code:recipe-failed", "code:generic-fail"):
            continue
        if r.search(t):
            return "code_failure", name
    if cls in BUILDTEST_CLS:
        if exit_code in EXIT_MAP:
            return EXIT_MAP[exit_code]
        if exit_code == 4 and ("nextest" in sub or any(k in (sub or "") for k in ("unit", "test"))):
            return "usage_error", "usage:nextest-no-tests(exit4)"
        if exit_code == 5 and ("pytest" in sub or (sub or "").startswith("py-")):
            return "usage_error", "usage:pytest-no-tests(exit5)"
    for name, r in CODE:
        if name in ("code:recipe-failed", "code:generic-fail") and r.search(t):
            return "code_failure", name
    if cls == "inline-script":
        return "code_failure", "inline-script:exit%s" % exit_code
    return "other_failure", "nonzero:exit%s%s" % (exit_code, ":redirected" if (redir and len(t.strip()) == 0) else (":empty" if not t.strip() else ""))


def text_sample(s, head=4000, tail=16000):
    if not s:
        return ""
    if len(s) <= head + tail:
        return s
    return s[:head] + "\n...\n" + s[-tail:]


def crates_in(t):
    out = set(m.group(1) for m in CRATE_PATH.finditer(t))
    out |= set(m.group(1) for m in NEXTEST_CRATE.finditer(t))
    out |= set(m.group(1) for m in COMPILE_CRATE.finditer(t))
    if PY_PATH.search(t):
        out.add("python")
    return sorted(out)


def path_crate(p):
    if not p:
        return None
    m = re.search(r"crates/(" + config.CRATE_RE + r")/", p)
    if m:
        return m.group(1)
    if config.PY_AREA in p:
        return "python"
    return None


def path_area(p):
    if not p:
        return "?"
    p2 = re.sub("^" + re.escape(config.REPO) + r"/(\.claude/worktrees/[^/]+/)?", "", p)
    if p2.startswith("/"):
        if p2.startswith("/tmp") or "scratchpad" in p2:
            return "tmp"
        if "/.claude/" in p2 or "/.codex/" in p2:
            return "agent-home"
        if "library-skills" in p2:
            return "library-skills"
        return "outside"
    top = p2.split("/", 1)[0]
    if top == "crates":
        parts = p2.split("/")
        return "crates/" + (parts[1] if len(parts) > 1 else "")
    if top == "docs":
        parts = p2.split("/")
        return "docs/" + (parts[1] if len(parts) > 2 else "")
    return top


# ---------------- writers ----------------
class Out:
    def __init__(self):
        os.makedirs(OUTDIR, exist_ok=True)
        self.ev = gzip.open(os.path.join(OUTDIR, "events.jsonl.gz"), "wt", encoding="utf-8")
        self.raw = gzip.open(os.path.join(OUTDIR, "raw_cmds.jsonl.gz"), "wt", encoding="utf-8")
        self.msgs = open(os.path.join(OUTDIR, "human_msgs.jsonl"), "w", encoding="utf-8")
        self.sess = open(os.path.join(OUTDIR, "session_signals.jsonl"), "w", encoding="utf-8")
        self.stats = collections.Counter()

    def write_events(self, evs, raws):
        evs.sort(key=lambda e: (e.get("ts") or "", e.get("seq", 0)))
        for e in evs:
            self.ev.write(json.dumps(e, separators=(",", ":")) + "\n")
        for r in raws:
            self.raw.write(json.dumps(r, separators=(",", ":")) + "\n")

    def close(self):
        self.ev.close(); self.raw.close(); self.msgs.close(); self.sess.close()


def shell_fields(cmd):
    s = summarize(cmd)
    pi = s["pinfo"]
    pipes = s["pipes"]
    down = sorted(set(d for p in pipes for d in p["down"]))
    up_cls = pipes[0]["up_cls"] if pipes else None
    # pipe whose upstream is primary class, if any
    for p in pipes:
        if p["up_cls"] == s["cls"] or (p["up_cls"] == "git-mut" and s["cls"] == "git"):
            up_cls = p["up_cls"]
            down = sorted(set(p["down"]))
            break
    praw = re.sub(r"\s+", " ", s["praw"]).strip()
    fullnorm = re.sub(r"\s+", " ", cmd).strip()
    pipesig = "|".join(",".join(p["down"]) for p in pipes)
    f = {
        "cls": s["cls"], "sub": s["sub"], "norm": s["norm"],
        "recipe": pi.get("recipe") if s["cls"] in ("just", "just-discovery") else None,
        "rargs": pi.get("rargs"), "pkgs": pi.get("pkgs") or None,
        "ff": pi.get("ff"), "justfile": pi.get("justfile"),
        "toolchain": pi.get("toolchain"),
        "paths": pi.get("paths"),
        "segs": ["%s|%s" % (a, b) for a, b in s["segs"]][:20],
        "envs": s["envs"] or None, "exports": s["exports"] or None, "feats": s["feats"] or None,
        "shellvars": sorted(set(s.get("shellvars") or [])) or None,
        "cdk": sorted(set(path_kind(c) if not c.startswith(config.REPO) else cwd_kind(c) for c in s["cd"])) or None,
        "cd_abs": [c for c in s["cd"] if c.startswith("/")][:3] or None,
        "src": [os.path.basename(x) for x in s["source"]] or None,
        "loop": s["loop"], "poll": s["polling"] or None, "sleep_s": s["sleep_s"] or None,
        "pdown": down or None, "pup": up_cls, "nseg": s["nseg"], "heredoc": s["heredoc"] or None,
        "redir": s["redir_log"] or None, "bgamp": s["bg"] or None, "e2o": s["err2out"] or None,
        "pok": s["parse_ok"], "wtgt": s["write_targets"] or None,
        "h_core": h(praw), "h_full": h(fullnorm), "h_pipe": h(pipesig), "h_envs": h(",".join(s["envs"])),
        "toutp": s["timeout_s"],
        "fam": pi.get("family"), "bnd": pi.get("boundary"), "flt": pi.get("filtered"),
        "beval": 1 if "build_environment.py --shell" in cmd else None,
        "lctxenv": sorted(set(re.findall(r"\bLCTX_[A-Z_]+(?==)", cmd))) or None,
        "cmdlen": len(cmd),
    }
    return {k: v for k, v in f.items() if v is not None}, praw


# ---------------- Claude ----------------
NOTIF_RE = re.compile(r"<task-notification>.*?<tool-use-id>(toolu_[A-Za-z0-9]+)</tool-use-id>.*?<status>([a-z_]+)</status>(?:.*?<summary>(.*?)</summary>)?", re.S)


def claude_text(content):
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts = []
        for c in content:
            if isinstance(c, dict):
                if c.get("type") == "text":
                    parts.append(c.get("text", ""))
                elif c.get("type") == "tool_result":
                    parts.append(claude_text(c.get("content")))
        return "\n".join(parts)
    return ""


def claude_files():
    mains = sorted(glob.glob(os.path.join(CLAUDE_DIR, "*.jsonl")))
    subs = sorted(glob.glob(os.path.join(CLAUDE_DIR, "*", "subagents", "agent-*.jsonl")))
    return mains, subs


def process_claude_file(path, kind, role, root, out, seen_ids):
    sid = os.path.basename(path).replace(".jsonl", "")
    pend = {}
    evs = []
    raws = []
    notifs = {}
    cwds = collections.Counter()
    seq = 0
    bad = 0
    first_human = []
    sig = {"rt": "claude", "sid": sid, "root": root, "ak": kind, "role": role, "first_usage": None, "compactions": 0,
           "sessionstart_superpowers": 0, "sessionstart_other": 0, "skill_listing": 0, "posttool_hook_ctx": 0,
           "denials": 0, "model": None, "first_ts": None, "last_ts": None}
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            try:
                r = json.loads(line)
            except Exception:
                bad += 1
                continue
            tsr = r.get("timestamp")
            if tsr:
                sig["first_ts"] = sig["first_ts"] or tsr
                sig["last_ts"] = tsr
            if r.get("type") == "system" and r.get("subtype") == "compact_boundary":
                sig["compactions"] += 1
                evs.append({"id": "cc-" + h(sid + str(tsr)), "rt": "claude", "sid": sid, "root": root, "ak": kind, "role": role,
                            "ts": tsr, "per": period(tsr), "tool": "_compaction", "seq": seq, "cwdk": cwd_kind(r.get("cwd"))})
            if r.get("type") == "attachment":
                a = r.get("attachment") or {}
                if a.get("type") == "hook_additional_context" and a.get("hookEvent") == "SessionStart":
                    if "superpowers" in json.dumps(a)[:20000]:
                        sig["sessionstart_superpowers"] += 1
                    else:
                        sig["sessionstart_other"] += 1
                elif a.get("type") == "hook_additional_context" and a.get("hookEvent") == "PostToolUse":
                    sig["posttool_hook_ctx"] += 1
                elif a.get("type") == "skill_listing":
                    sig["skill_listing"] += 1
            if r.get("toolDenialKind"):
                sig["denials"] += 1
            if sig["first_usage"] is None and r.get("type") == "assistant":
                u = (r.get("message") or {}).get("usage") or {}
                if u:
                    sig["first_usage"] = {"input": u.get("input_tokens"), "cache_creation": u.get("cache_creation_input_tokens"),
                                          "cache_read": u.get("cache_read_input_tokens")}
                    sig["model"] = (r.get("message") or {}).get("model")
            if "<task-notification>" in line:
                txt = claude_text((r.get("message") or {}).get("content")) if r.get("type") == "user" else (r.get("content") if isinstance(r.get("content"), str) else line)
                for m in NOTIF_RE.finditer(txt or ""):
                    tid = m.group(1)
                    if tid not in notifs:
                        notifs[tid] = (r.get("timestamp"), m.group(2), m.group(3) or "")
            typ = r.get("type")
            if "cwd" in r:
                cwds[r["cwd"]] += 1
            if typ == "assistant":
                for it in (r.get("message") or {}).get("content") or []:
                    if isinstance(it, dict) and it.get("type") == "tool_use":
                        tid = it.get("id")
                        if tid in seen_ids:
                            out.stats["claude_dup_tool_use"] += 1
                            continue
                        seen_ids.add(tid)
                        seq += 1
                        pend[tid] = (r.get("timestamp"), it.get("name"), it.get("input") or {}, r.get("cwd"), seq)
            elif typ == "user":
                msg = r.get("message") or {}
                ct = msg.get("content")
                if kind == "main" and not r.get("isMeta") and not r.get("isSidechain"):
                    if isinstance(ct, str) or (isinstance(ct, list) and not any(isinstance(i, dict) and i.get("type") == "tool_result" for i in ct)):
                        txt = claude_text(ct)
                        if txt and not txt.lstrip().startswith("<") and not txt.startswith("[Request interrupted"):
                            auto = txt.startswith("The end-of-turn checks below failed")
                            first_human.append(auto)
                            out.msgs.write(json.dumps({"rt": "claude", "sid": sid, "ts": r.get("timestamp"), "per": period(r.get("timestamp")), "auto": auto, "cwdk": cwd_kind(r.get("cwd")), "text": txt[:3000]}) + "\n")
                        elif txt.startswith("[Request interrupted"):
                            evs.append({"id": "int-" + h(sid + str(r.get("timestamp"))), "rt": "claude", "sid": sid, "root": root, "ak": kind, "role": role, "ts": r.get("timestamp"), "per": period(r.get("timestamp")), "tool": "_user_interrupt", "seq": seq, "cwdk": cwd_kind(r.get("cwd"))})
                if not isinstance(ct, list):
                    continue
                for it in ct:
                    if not (isinstance(it, dict) and it.get("type") == "tool_result"):
                        continue
                    tid = it.get("tool_use_id")
                    if tid not in pend:
                        continue
                    ts0, name, inp, cwd, sq = pend.pop(tid)
                    te = r.get("timestamp")
                    t0, t1 = parse_ts(ts0 or ""), parse_ts(te or "")
                    wall = (t1 - t0) if (t0 and t1) else None
                    tur = r.get("toolUseResult")
                    ctext = claude_text(it.get("content"))
                    is_err = bool(it.get("is_error"))
                    ev = {"id": tid, "rt": "claude", "sid": sid, "root": root, "ak": kind, "role": role,
                          "ts": ts0, "te": te, "per": period(ts0), "tool": name, "seq": sq,
                          "cwdk": cwd_kind(cwd), "wt": wt_id(cwd) or None, "wall": round(wall, 3) if wall is not None else None,
                          "err": is_err or None}
                    if r.get("toolDenialKind"):
                        ev["denial"] = r.get("toolDenialKind")
                    # generic error heuristics for all tools
                    eh = None
                    if is_err:
                        low = ctext[:600]
                        if "PreToolUse:" in low and "hook error" in low:
                            m = re.search(r"PreToolUse:(\w+) hook error: (.{0,160})", low, re.S)
                            eh = "hook-block:%s" % (m.group(1) if m else "?")
                            ev["block_reason"] = re.sub(r"\s+", " ", re.sub(r"/[\w./-]+", "<path>", re.sub(r"\d+", "N", m.group(2) if m else low[:160])))[:160]
                        elif "This agent is isolated in the worktree" in low:
                            eh = "worktree-isolation"
                        elif low.startswith("Blocked: sleep") or "Blocked: sleep" in low[:40]:
                            eh = "sleep-blocked"
                        elif r.get("toolDenialKind") or "Permission to use" in low or "denied by your permission settings" in low or "doesn't want to proceed" in low:
                            eh = "denied"
                        elif "No such tool available" in low:
                            eh = "tool-error:no-such-tool"
                        elif "has not been read yet" in low:
                            eh = "tool-error:not-read"
                        elif "modified since read" in low:
                            eh = "tool-error:stale-read"
                        elif "InputValidationError" in low or "Invalid tool parameters" in low:
                            eh = "tool-error:input-validation"
                        elif "String to replace not found" in low or "Found 0 matches" in low or "old_string" in low and "not found" in low:
                            eh = "tool-error:edit-nomatch"
                        elif "matches of the string to replace" in low or "Found 2 matches" in low or re.search(r"Found \d+ matches", low):
                            eh = "tool-error:edit-multimatch"
                        elif "File does not exist" in low or "EISDIR" in low:
                            eh = "tool-error:missing-file"
                        elif "File content (" in low and "exceeds maximum" in low or "exceeds maximum allowed" in low:
                            eh = "tool-error:too-large"
                    if eh:
                        ev["eh"] = eh
                    if THERMO_RE.search(json.dumps(inp)[:4000]):
                        ev["thermo_ref"] = 1
                    if name == "Bash":
                        cmd = inp.get("command") or ""
                        sf, praw = shell_fields(cmd)
                        ev.update(sf)
                        ev["shell"] = 1
                        if inp.get("run_in_background"):
                            ev["bg"] = 1
                        if inp.get("timeout"):
                            ev["tparam"] = inp.get("timeout")
                        # output
                        if isinstance(tur, dict):
                            so = (tur.get("stdout") or "") + ("\n" + tur.get("stderr") if tur.get("stderr") else "")
                            ob = tur.get("persistedOutputSize") or len(so)
                            if tur.get("persistedOutputPath"):
                                ev["spill"] = 1
                            if tur.get("interrupted"):
                                ev["interrupted"] = 1
                            if tur.get("returnCodeInterpretation"):
                                ev["rci"] = tur.get("returnCodeInterpretation")[:40]
                            if tur.get("bashEditDiff"):
                                files = (tur.get("bashEditDiff") or {}).get("files") or []
                                ev["bash_edit"] = 1
                                ev["edit_crates"] = sorted(set(c for c in (path_crate(f.get("filePath")) for f in files) if c)) or None
                                ev["edit_areas"] = sorted(set(path_area(f.get("filePath")) for f in files))[:6]
                            if tur.get("backgroundTaskId"):
                                ev["bg"] = 1
                            text = so
                        else:
                            text = tur if isinstance(tur, str) else ctext
                            ob = len(text or "")
                        text = text_sample(text or ctext)
                        ev["obytes"] = ob
                        exit_code = 0
                        if is_err:
                            m = re.match(r"\s*(?:Error: )?Exit code (-?\d+)", text or ctext)
                            exit_code = int(m.group(1)) if m else None
                        ev["exit"] = exit_code
                        piped = bool(sf.get("pdown"))
                        if eh in ("hook-block:Bash", "worktree-isolation", "sleep-blocked", "denied") or (eh or "").startswith("hook-block"):
                            ev["out"] = "blocked" if eh != "denied" else "denied"
                            ev["heur"] = eh
                        elif ev.get("interrupted") or "[Request interrupted by user" in (ctext or "")[:200]:
                            ev["out"], ev["heur"] = "interrupted", "tur.interrupted"
                        elif re.search(r"Command timed out|timed out after \d", (ctext or "")[:300]):
                            ev["out"], ev["heur"] = "timeout", "claude-timeout"
                        else:
                            o, hh = classify_outcome(sf["cls"], sf["sub"], exit_code, text, piped, sf.get("segs", ()), sf.get("redir"))
                            if exit_code is None and is_err:
                                o, hh = "other_failure", "is_error-no-exit"
                            ev["out"], ev["heur"] = o, hh
                        if LOCKWAIT.search(text or ""):
                            ev["lockwait"] = 1
                        if UVBUILD.search(text or ""):
                            ev["uvbuild"] = 1
                        elif UVSYNC_NOTE.search(text or ""):
                            ev["uvsync"] = 1
                        if ev.get("spill") or (ob or 0) > 30000:
                            ev["bigout"] = 1
                        if ev["out"] in ("code_failure", "masked-failure", "usage_error", "env_failure", "other_failure"):
                            full = text
                            if ev.get("spill") and isinstance(tur, dict) and tur.get("persistedOutputPath") and os.path.exists(tur["persistedOutputPath"]):
                                try:
                                    with open(tur["persistedOutputPath"], encoding="utf-8", errors="replace") as fh2:
                                        full = fh2.read(2_000_000)
                                except Exception:
                                    pass
                            ev["ecr"] = crates_in(full) or None
                        raws.append({"id": tid, "cmd": cmd[:4000], "desc": (inp.get("description") or "")[:200], "otail": (text or "")[-600:]})
                    elif name in ("Edit", "Write", "MultiEdit", "NotebookEdit"):
                        fp = inp.get("file_path") or inp.get("notebook_path") or ""
                        c = path_crate(fp)
                        ev["edit_crates"] = [c] if c else None
                        ev["edit_areas"] = [path_area(fp)]
                        ev["edit"] = 1
                        if is_err:
                            ev["out"] = "blocked" if (eh or "").startswith("hook-block") or eh == "worktree-isolation" else ("denied" if eh == "denied" else "tool_error")
                            ev["heur"] = eh or "edit-error"
                        else:
                            ev["out"] = "success"
                    else:
                        if name == "Read":
                            fp = inp.get("file_path") or ""
                            ev["rpath"] = path_kind(fp) if path_kind(fp) != "<path>" else path_area(fp)
                        elif name == "Grep":
                            ev["rpath"] = path_kind(inp.get("path") or "")
                        elif name == "ToolSearch":
                            ev["q"] = (inp.get("query") or "")[:100]
                        elif name in ("Agent", "Task"):
                            ev["subtype"] = inp.get("subagent_type") or "general-purpose"
                            if inp.get("isolation"):
                                ev["isolation"] = inp.get("isolation")
                        elif name == "Skill":
                            ev["skill"] = (inp.get("skill") or "")[:60]
                        elif name == "Monitor":
                            ev["q"] = (inp.get("description") or inp.get("command") or "")[:0]
                        elif name in ("TaskOutput", "BashOutput"):
                            pass
                        ev["out"] = ("blocked" if (eh or "").startswith("hook-block") or eh in ("worktree-isolation", "sleep-blocked") else ("denied" if eh == "denied" else "tool_error")) if is_err else "success"
                        if eh:
                            ev["heur"] = eh
                    evs.append({k: v for k, v in ev.items() if v is not None})
    # unmatched tool_uses (no result)
    for tid, (ts0, name, inp, cwd, sq) in pend.items():
        ev = {"id": tid, "rt": "claude", "sid": sid, "root": root, "ak": kind, "role": role, "ts": ts0, "per": period(ts0),
              "tool": name, "seq": sq, "cwdk": cwd_kind(cwd), "out": "no_result", "heur": "no-tool-result"}
        if name == "Bash":
            sf, praw = shell_fields(inp.get("command") or "")
            ev.update(sf); ev["shell"] = 1
            raws.append({"id": tid, "cmd": (inp.get("command") or "")[:4000], "desc": "", "otail": ""})
        evs.append(ev)
    # background completions
    for e in evs:
        if e.get("bg") and e["id"] in notifs:
            nts, st, summ = notifs[e["id"]]
            t0, t1 = parse_ts(e.get("ts") or ""), parse_ts(nts or "")
            if t0 and t1:
                e["wall_bg"] = round(t1 - t0, 3)
            e["bg_status"] = st
            m = re.search(r"exit code (-?\d+)", summ or "")
            if m:
                e["bg_exit"] = int(m.group(1))
    out.stats["claude_bad_lines"] += bad
    out.claude_sig = sig
    if kind == "main" and first_human and first_human[0]:
        for e in evs:
            e["role"] = "eot-fixer"
        out.stats["claude_eot_fixer_sessions"] += 1
    return evs, raws, cwds


# ---------------- Codex ----------------
BLOCK_RE = re.compile(r"Command blocked by PreToolUse hook: BLOCKED: (\S+?): ([^.\n]{0,200})")
PATCH_FAIL_RE = re.compile(r"(apply_patch verification failed|Failed to find expected lines|Invalid patch|invalid hunk|Failed to find context)")
JS_TOOL_RE = re.compile(r"tools\.(\w+)\s*\(")


def process_codex_file(meta, out, seen_ids):
    path = meta["path"]
    src = meta.get("source")
    if isinstance(src, dict) and "subagent" in src:
        kind = "sub"
        ts_ = (src.get("subagent") or {}).get("thread_spawn") or {}
        role = meta.get("agent_role") or ts_.get("agent_role") or "subagent"
        root = ts_.get("parent_thread_id") or ""
    else:
        kind = "main"
        role = "main"
        root = meta.get("id")
    sid = meta.get("id") or os.path.basename(path)
    evs, raws = [], []
    jscount = collections.Counter()
    bad = 0
    seq = 0
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            # cheap prefilter: skip token/reasoning records
            hd = line[:220]
            if ('"token_count"' in hd or '"reasoning"' in hd or '"token_usage_record"' in hd or '"world_state"' in hd
                    or '"turn_context"' in hd or '"compacted"' in hd or '"session_meta"' in hd or '"inter_agent_communication_metadata"' in hd
                    or '"agent_message"' in hd or '"type":"message"' in hd or '"function_call_output"' in hd):
                continue
            if '"custom_tool_call_output"' in hd and "Warning: truncated output" in line[:600]:
                jscount["_truncated_output"] += 1
            if '"custom_tool_call_output"' in hd and not ("blocked by PreToolUse" in line or "Script failed" in line or "Script error" in line or "verification failed" in line or "Failed to find" in line or "Invalid patch" in line):
                continue
            if '"custom_tool_call"' in hd:
                out.stats["codex_custom_tool_calls_seen"] += 1
            try:
                r = json.loads(line)
            except Exception:
                bad += 1
                continue
            pl = r.get("payload")
            if not isinstance(pl, dict):
                continue
            pt = pl.get("type")
            ts = r.get("timestamp")
            if pt == "item_completed":
                it = pl.get("item") or {}
                itype = it.get("type")
                iid = it.get("id")
                if iid and iid in seen_ids and itype in ("CommandExecution", "FileChange", "McpToolCall"):
                    out.stats["codex_dup_items"] += 1
                    continue
                if iid:
                    seen_ids.add(iid)
                st_ms = pl.get("started_at_ms")
                en_ms = pl.get("completed_at_ms")
                t0 = iso_from_ms(st_ms) if st_ms else ts
                seq += 1
                if itype == "CommandExecution":
                    argv = it.get("command") or []
                    if isinstance(argv, list):
                        if len(argv) >= 3 and os.path.basename(argv[0]) in ("bash", "sh", "zsh") and argv[1] in ("-lc", "-c"):
                            cmd = argv[2]
                        else:
                            cmd = " ".join(argv)
                    else:
                        cmd = str(argv)
                    cwd = (it.get("cwd") or "").replace("file://", "")
                    if cwd_kind(cwd) not in INCLUDED_CWD:
                        out.stats["codex_othercwd:" + "/".join(cwd.split("/")[:4])] += 1
                    elif cwd_kind(cwd) in ("cache-copy", "tmp-copy"):
                        out.stats["codex_copycwd:" + "/".join(cwd.split("/")[:5])[:70]] += 1
                    du = it.get("duration") or {}
                    wall = (du.get("secs") or 0) + (du.get("nanos") or 0) / 1e9 if du else None
                    agg = it.get("aggregated_output") or ((it.get("stdout") or "") + (it.get("stderr") or ""))
                    sf, praw = shell_fields(cmd)
                    if it.get("status") == "failed" and it.get("exit_code") is None:
                        sf["nexit"] = 1
                    ev = {"id": iid, "rt": "codex", "sid": sid, "root": root, "ak": kind, "role": role, "ts": t0,
                          "te": iso_from_ms(en_ms) if en_ms else ts, "per": period(t0), "tool": "exec_command", "seq": seq,
                          "cwdk": cwd_kind(cwd), "wt": wt_id(cwd) or None, "wall": round(wall, 3) if wall is not None else None,
                          "shell": 1, "exit": it.get("exit_code"), "obytes": len(agg),
                          "ptypes": sorted(set(p.get("type") for p in (it.get("parsed_cmd") or []) if isinstance(p, dict))) or None}
                    ev.update(sf)
                    text = text_sample(agg)
                    o, hh = classify_outcome(sf["cls"], sf["sub"], it.get("exit_code"), text, bool(sf.get("pdown")), sf.get("segs", ()), sf.get("redir"))
                    ev["out"], ev["heur"] = o, hh
                    if LOCKWAIT.search(text):
                        ev["lockwait"] = 1
                    if UVBUILD.search(agg):
                        ev["uvbuild"] = 1
                    elif UVSYNC_NOTE.search(agg):
                        ev["uvsync"] = 1
                    if o in ("code_failure", "masked-failure", "usage_error", "env_failure", "other_failure"):
                        ev["ecr"] = crates_in(agg[:2_000_000]) or None
                    evs.append({k: v for k, v in ev.items() if v is not None})
                    raws.append({"id": iid, "cmd": cmd[:4000], "desc": "", "otail": (text or "")[-600:]})
                elif itype == "FileChange":
                    ch = it.get("changes") or {}
                    paths = list(ch.keys()) if isinstance(ch, dict) else []
                    ev = {"id": iid, "rt": "codex", "sid": sid, "root": root, "ak": kind, "role": role, "ts": t0,
                          "per": period(t0), "tool": "apply_patch", "seq": seq, "edit": 1, "out": "success",
                          "edit_crates": sorted(set(c for c in (path_crate(p) for p in paths) if c)) or None,
                          "edit_areas": sorted(set(path_area(p) for p in paths))[:6], "nfiles": len(paths),
                          "cwdk": "main", "epathk": cwd_kind(paths[0] if paths else "")}
                    evs.append({k: v for k, v in ev.items() if v is not None})
                elif itype == "McpToolCall":
                    ev = {"id": iid, "rt": "codex", "sid": sid, "root": root, "ak": kind, "role": role, "ts": t0,
                          "per": period(t0), "tool": "mcp__%s__%s" % (it.get("server"), it.get("tool")), "seq": seq,
                          "out": "success" if it.get("status") == "completed" else "tool_error", "cwdk": "main"}
                    du = it.get("duration") or {}
                    if du:
                        ev["wall"] = round((du.get("secs") or 0) + (du.get("nanos") or 0) / 1e9, 3)
                    evs.append(ev)
                elif itype == "UserMessage" and kind == "main":
                    txt = " ".join(c.get("text", "") for c in (it.get("content") or []) if isinstance(c, dict))
                    out.msgs.write(json.dumps({"rt": "codex", "sid": sid, "ts": t0, "per": period(t0), "auto": False, "cwdk": "main", "text": txt[:3000]}) + "\n")
                elif itype in ("CollabAgentToolCall",):
                    evs.append({"id": iid, "rt": "codex", "sid": sid, "root": root, "ak": kind, "role": role, "ts": t0,
                                "per": period(t0), "tool": "collab:%s" % it.get("tool"), "seq": seq, "out": "success", "cwdk": "main"})
                elif itype == "ContextCompaction":
                    evs.append({"id": iid or ("cc-" + h(sid + str(t0))), "rt": "codex", "sid": sid, "root": root, "ak": kind, "role": role,
                                "ts": t0, "per": period(t0), "tool": "_compaction", "seq": seq, "cwdk": "main"})
            elif pt == "function_call":
                cid = pl.get("call_id") or pl.get("id")
                if cid and cid in seen_ids:
                    out.stats["codex_dup_fn"] += 1
                    continue
                if cid:
                    seen_ids.add(cid)
                seq += 1
                name = "%s.%s" % (pl.get("namespace"), pl.get("name")) if pl.get("namespace") else pl.get("name")
                ev = {"id": pl.get("call_id") or pl.get("id"), "rt": "codex", "sid": sid, "root": root, "ak": kind, "role": role,
                      "ts": ts, "per": period(ts), "tool": "fn:%s" % name, "seq": seq, "out": "success", "cwdk": "main"}
                if pl.get("name") == "sleep":
                    try:
                        a = json.loads(pl.get("arguments") or "{}")
                        v = a.get("seconds") or a.get("duration_seconds") or a.get("duration_ms", 0) / 1000.0 or a.get("ms", 0) / 1000.0
                        ev["sleep_s"] = v
                    except Exception:
                        pass
                evs.append(ev)
            elif pt == "custom_tool_call":
                inp = pl.get("input") or ""
                names = JS_TOOL_RE.findall(inp)
                for n in names:
                    jscount[n] += 1
                jscount["_scripts"] += 1
                if "yield_time_ms" in inp:
                    jscount["_yield_time_ms"] += 1
                if "max_output_tokens" in inp:
                    jscount["_max_output_tokens"] += 1
            elif pt == "custom_tool_call_output":
                o = pl.get("output")
                txt = " ".join(x.get("text", "") for x in o if isinstance(x, dict)) if isinstance(o, list) else str(o or "")
                head = txt[:400]
                if "Script failed" in head or "Script error" in txt[:2000]:
                    jscount["_script_failed"] += 1
                for m in BLOCK_RE.finditer(txt):
                    seq += 1
                    reason = re.sub(r"\d+", "N", m.group(2)).strip()
                    evs.append({"id": "blk-" + h(sid + str(ts) + m.group(1)), "rt": "codex", "sid": sid, "root": root, "ak": kind, "role": role,
                                "ts": ts, "per": period(ts), "tool": "_hook_block", "seq": seq, "out": "blocked", "heur": "hook-block:apply_patch",
                                "block_reason": reason[:160], "edit_areas": [path_area(m.group(1))], "cwdk": "main", "epathk": cwd_kind(m.group(1))})
                pm = PATCH_FAIL_RE.search(txt[:4000])
                if pm and "apply_patch" not in txt[:0]:
                    seq += 1
                    evs.append({"id": "pf-" + h(sid + str(ts) + str(seq)), "rt": "codex", "sid": sid, "root": root, "ak": kind, "role": role,
                                "ts": ts, "per": period(ts), "tool": "_patch_fail", "seq": seq, "out": "tool_error", "heur": "patch:" + pm.group(1)[:40], "cwdk": "main"})
            elif r.get("type") == "event_msg" and pt == "turn_aborted":
                seq += 1
                evs.append({"id": "ab-" + h(sid + str(ts)), "rt": "codex", "sid": sid, "root": root, "ak": kind, "role": role,
                            "ts": ts, "per": period(ts), "tool": "_user_interrupt", "seq": seq, "cwdk": "main"})
    out.stats["codex_bad_lines"] += bad
    return evs, raws, jscount, kind, role


def main():
    limit = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    out = Out()
    out.claude_sig = None
    seen = set()
    sess_index = []
    excluded = collections.Counter()
    # Claude
    mains, subs = claude_files()
    allc = mains + subs
    if limit:
        allc = allc[:limit]
    after_cutoff_roots = set()
    for p in allc:
        is_sub = "/subagents/" in p
        sid = os.path.basename(p).replace(".jsonl", "")
        root = p.split("/")[-3] if is_sub else sid
        if sid in EXCLUDED or root in EXCLUDED:
            excluded["claude_excluded_session_files"] += 1
            continue
        if is_sub:
            mp = p.replace(".jsonl", ".meta.json")
            try:
                m = json.load(open(mp))
            except Exception:
                m = {}
                out.stats["claude_missing_meta"] += 1
            role = m.get("agentType") or "unknown"
            kind = "sub"
        else:
            role, kind = "main", "main"
        out.claude_sig = None
        try:
            evs, raws, cwds = process_claude_file(p, kind, role, root, out, seen)
        except Exception as ex:
            out.stats["claude_file_errors"] += 1
            print("ERR", p, ex, file=sys.stderr)
            continue
        first_ts = min((e["ts"] for e in evs if e.get("ts")), default="")
        if not is_sub and first_ts and first_ts[:19] >= ASSESSMENT_CUTOFF:
            after_cutoff_roots.add(sid)
        if root in after_cutoff_roots:
            excluded["claude_after_cutoff_session_events"] += len(evs)
            continue
        keep, drop = [], 0
        keep_ids = set()
        for e in evs:
            ck = e.get("cwdk", "unknown")
            if (e.get("ts") or "")[:19] >= ASSESSMENT_CUTOFF:
                drop += 1; excluded["claude_after_cutoff_events"] += 1; continue
            if e.get("tool") and ck not in INCLUDED_CWD and ck != "unknown":
                drop += 1; excluded["claude_%s_cwd_events" % ck] += 1; continue
            if e.get("cd_abs") and any(config.outside_repo(c) for c in e["cd_abs"]):
                drop += 1; excluded["claude_cd_outside_events"] += 1; continue
            keep.append(e); keep_ids.add(e["id"])
        out.write_events(keep, [r for r in raws if r["id"] in keep_ids])
        sig = out.claude_sig or {}
        sig.update({"n": len(keep), "dropped": drop, "per": period(first_ts), "major_cwd_kind": cwd_kind(cwds.most_common(1)[0][0]) if cwds else None})
        out.sess.write(json.dumps(sig) + "\n")
        sess_index.append({"rt": "claude", "sid": sid, "root": root, "ak": kind, "role": role, "n": len(keep), "dropped": drop,
                           "first_ts": first_ts or None,
                           "last_ts": max((e.get("te") or e["ts"] for e in evs if e.get("ts")), default=None)})
        out.stats["claude_files"] += 1
    # Codex
    idx = json.load(open(os.path.join(OUTDIR, "codex_index.json")))
    sel = [r for r in idx if r.get("size") and cwd_kind(r["cwd"]) in INCLUDED_CWD and (r.get("ts") or "") < ASSESSMENT_CUTOFF]
    excluded["codex_threads_after_cutoff_or_missing"] = len(idx) - len(sel)
    if limit:
        sel = sorted(sel, key=lambda r: r["size"])[:limit]
    js_total = collections.Counter()
    js_by = collections.defaultdict(collections.Counter)
    for meta in sorted(sel, key=lambda r: r.get("ts") or ""):
        try:
            evs, raws, jsc, kind, role = process_codex_file(meta, out, seen)
        except Exception as ex:
            out.stats["codex_file_errors"] += 1
            print("ERR", meta["path"], ex, file=sys.stderr)
            continue
        keep, drop = [], 0
        keep_ids = set()
        for e in evs:
            ck = e.get("cwdk", "main")
            if (e.get("ts") or "")[:19] >= ASSESSMENT_CUTOFF:
                drop += 1; excluded["codex_after_cutoff_events"] += 1; continue
            if ck not in INCLUDED_CWD:
                drop += 1; excluded["codex_%s_cwd_events" % ck] += 1; continue
            if e.get("cd_abs") and any(config.outside_repo(c) for c in e["cd_abs"]):
                drop += 1; excluded["codex_cd_outside_events"] += 1; continue
            keep.append(e); keep_ids.add(e["id"])
        out.write_events(keep, [r for r in raws if r["id"] in keep_ids])
        js_total.update(jsc)
        per = period(meta.get("ts"))
        js_by["%s|%s" % (kind, per)].update(jsc)
        root = meta.get("id") if kind == "main" else ((meta.get("source") or {}).get("subagent", {}).get("thread_spawn", {}).get("parent_thread_id") if isinstance(meta.get("source"), dict) else None)
        tss = [e["ts"] for e in keep if e.get("ts")]
        out.sess.write(json.dumps({"rt": "codex", "sid": meta.get("id"), "root": root, "ak": kind, "role": role, "n": len(keep),
                                   "dropped": drop, "per": per, "tokens_used": meta.get("tokens_used"), "model": meta.get("model"),
                                   "effort": meta.get("effort"), "truncated_outputs": jsc.get("_truncated_output", 0),
                                   "first_ts": min(tss) if tss else meta.get("ts"), "last_ts": max(tss) if tss else None,
                                   "source": meta.get("source") if isinstance(meta.get("source"), str) else "subagent"}) + "\n")
        sess_index.append({"rt": "codex", "sid": meta.get("id"), "root": root, "ak": kind, "role": role, "n": len(keep), "dropped": drop,
                           "cli": meta.get("cli_version"), "first_ts": meta.get("ts"), "js": dict(jsc)})
        out.stats["codex_files"] += 1
    out.close()
    json.dump({"stats": out.stats, "excluded": excluded, "js_total": js_total, "js_by": js_by}, open(os.path.join(OUTDIR, "extract_stats.json"), "w"), indent=1)
    json.dump(sess_index, open(os.path.join(OUTDIR, "sessions.json"), "w"))
    print(json.dumps({"stats": out.stats, "excluded": excluded}, indent=1))


if __name__ == "__main__":
    main()
