"""Shell command normalization and classification for transcript mining (stdlib only).

Ported from the pse-arrow W1 miner; repository specifics come from config.py.
"""
import os
import re
import shlex
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import config  # noqa: E402

HEREDOC_RE = re.compile(r"<<(-?)[ \t]*(?:'([^']+)'|\"([^\"]+)\"|\\?([A-Za-z_][A-Za-z0-9_]*))")
ASSIGN_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
NUM_RE = re.compile(r"^\d+(\.\d+)?[smhd]?$")
PKG_RE = config.PKG_RE

SEARCH_READ = {
    "rg", "grep", "egrep", "fgrep", "ugrep", "cat", "head", "tail", "ls", "find", "fd", "fdfind", "wc", "jq",
    "ast-grep", "sg", "tree", "file", "stat", "diff", "cmp", "nl", "sort", "uniq", "cut", "tr", "column",
    "less", "more", "bat", "realpath", "readlink", "basename", "dirname", "du", "md5sum", "sha256sum",
    "sha1sum", "b2sum", "xxd", "od", "strings", "nm", "objdump", "readelf", "comm", "paste", "fold", "rev",
    "yq", "tomlq", "xq", "zcat", "zgrep", "gzip", "unzip", "tar", "printf_read", "look", "sed", "awk", "perl",
    "colordiff", "delta", "difft", "pdftotext", "hexdump", "base64", "iconv", "expand", "fmt", "split", "tac",
    "gunzip", "zstdcat", "xzcat", "bzcat", "ag", "ack", "locate", "glow", "mdcat", "llvm-nm", "llvm-objdump",
    "llvm-readelf", "ldd", "patchelf", "otool",
}
PROCESS_MON = {
    "sleep", "ps", "pgrep", "pkill", "kill", "killall", "top", "htop", "btop", "free", "uptime", "vmstat",
    "iostat", "journalctl", "systemctl", "systemd-cgtop", "watch", "wait", "nvidia-smi", "df", "lsof", "fuser",
    "pidstat", "mpstat", "sar", "dmesg", "loginctl", "busctl", "systemd-cgls", "pstree", "nproc", "lscpu",
    "smem", "iotop", "timeout_only", "jobs", "fg", "bg", "disown", "flock",
}
RAW_TOOLS = {
    "ruff", "pyrefly", "ty", "mypy", "taplo", "typos", "shellcheck", "actionlint", "zizmor", "mdbook",
    "pagefind", "cargo-deny", "cargo-nextest", "cargo-hakari", "rustfmt", "clippy-driver", "rustc", "rustup",
    "rustdoc", "cbindgen", "bindgen", "cmake", "make", "ninja", "meson", "gcc", "g++", "cc", "clang", "clang++",
    "ld", "pkg-config", "lychee", "markdownlint", "prettier", "yamllint", "hyperfine", "perf", "valgrind",
    "heaptrack", "gdb", "lldb", "codespell", "pre-commit", "tox", "nox", "pip", "pip3", "sccache", "cargo-udeps",
    "cargo-mutants", "cargo-machete", "cargo-shear", "cargo-llvm-cov", "miri", "ipopt", "pse-worker",
    "pytest-3", "coverage", "hatch", "mdformat", "ripsecrets", "editorconfig-checker", "jsonschema",
    "datamodel-codegen", "protoc", "flatc", "psql", "pg_ctl", "initdb", "postgres", "surreal",
}
GIT_MUTATING = {"add", "commit", "stash", "checkout", "restore", "reset", "rebase", "cherry-pick", "worktree",
                "merge", "push", "pull", "apply", "rm", "mv", "clean", "switch", "tag", "am", "revert", "fetch",
                "branch", "update-index", "notes", "submodule"}
KEYWORD_SKIP = {"do", "then", "else", "{", "(", "!", "time", "}", ")", "done", "fi", "esac", "in"}
NOOP = {"true", "false", ":", "echo", "printf", "date", "exit", "return", "break", "continue", "test", "[",
        "[[", "local", "declare", "readonly", "unset", "shopt", "trap", "alias", "eval", "read", "let", "shift",
        "wait_noop", "hash", "type", "which", "command", "printenv", "env", "id", "whoami", "hostname", "uname",
        "pwd", "set", "ulimit", "export", "cd", "pushd", "popd", "source", ".", "umask", "builtin", "getconf",
        "tput", "clear", "reset_term", "mktemp", "seq", "yes", "bc", "expr", "dirs"}
PY_WRITE_RE = re.compile(r"\.write_text\(|\.write_bytes\(|open\([^)]*['\"](w|a|wb|ab|w\+|r\+)['\"]|\.write\(|"
                         r"shutil\.(copy|move|rmtree)|os\.(replace|rename|remove|unlink)|\.unlink\(|json\.dump\(|"
                         r"\.rename\(|\.replace\(\s*[A-Za-z_]*path|tomli_w\.dump|toml\.dump|yaml\.dump\(")
REPO_PATH_RE = re.compile(r"['\"/](crates|docs|python|scripts|packages|xtask|tests|benches|\.codex|\.claude|\.agents|\.config)/|justfile|Cargo\.toml|pyproject\.toml|AGENTS\.md")
REDUCERS = {"head", "tail", "grep", "rg", "sort", "uniq", "wc", "sed", "awk", "cut", "less", "jq", "tr", "column",
            "egrep", "fgrep", "ugrep", "tee", "cat", "nl", "fold", "more", "xargs"}


def match_close(s, i, op, cl):
    depth = 0
    j = i
    n = len(s)
    while j < n:
        ch = s[j]
        if ch == "\\":
            j += 2; continue
        if ch == "'":
            k = s.find("'", j + 1); j = (k if k >= 0 else n) + 1; continue
        if ch == '"':
            k = j + 1
            while k < n and s[k] != '"':
                k += 2 if s[k] == "\\" else 1
            j = k + 1; continue
        if ch == op:
            depth += 1
        elif ch == cl:
            depth -= 1
            if depth == 0:
                return j
        j += 1
    return -1


def _strip_line_continuations(s):
    return s.replace("\\\r\n", " ").replace("\\\n", " ")


def split_script(s):
    """Split a shell script into statements of pipeline stages.

    Returns (statements, heredocs, comments) where statements is a list of dicts
    {stages:[str], bg:bool, conn:str} and heredocs is a list of (delim, body).
    Handles quotes, $(), backticks, heredocs; tolerant of malformed input.
    """
    s = _strip_line_continuations(s)
    stmts = []
    heredocs = []
    pending_hd = []  # (strip_tabs, delim)
    cur = []
    stages = []
    i = 0
    n = len(s)
    depth = 0  # $( ( { nesting
    conn = ""

    def flush_stage():
        txt = "".join(cur).strip()
        cur.clear()
        if txt:
            stages.append(txt)
        return txt

    def flush_stmt(c, bg=False):
        nonlocal stages
        flush_stage()
        if stages:
            stmts.append({"stages": stages, "bg": bg, "conn": c})
        stages = []

    while i < n:
        ch = s[i]
        if ch == "\\" and i + 1 < n:
            cur.append(s[i:i + 2]); i += 2; continue
        if ch == "'":
            j = s.find("'", i + 1)
            if j < 0: j = n - 1
            cur.append(s[i:j + 1]); i = j + 1; continue
        if ch == '"':
            j = i + 1
            while j < n:
                if s[j] == "\\": j += 2; continue
                if s[j] == '"': break
                j += 1
            cur.append(s[i:j + 1]); i = j + 1; continue
        if ch == "`":
            j = s.find("`", i + 1)
            if j < 0: j = n - 1
            cur.append(s[i:j + 1]); i = j + 1; continue
        if ch == "#" and (not cur or cur[-1][-1:].isspace()) and depth == 0:
            j = s.find("\n", i)
            if j < 0: j = n
            i = j; continue
        if ch == "<" and s.startswith("<<", i) and not s.startswith("<<<", i):
            m = HEREDOC_RE.match(s, i)
            if m:
                delim = m.group(2) or m.group(3) or m.group(4)
                pending_hd.append((m.group(1) == "-", delim))
                cur.append("<<HEREDOC")
                i = m.end(); continue
        if ch in "({" and (ch == "{" and (i + 1 < n and s[i + 1] in " \n\t") or ch == "("):
            depth += 1; cur.append(ch); i += 1; continue
        if ch in ")}" and depth > 0:
            depth -= 1; cur.append(ch); i += 1; continue
        if depth == 0:
            if ch == "\n" or ch == ";":
                if ch == ";" and s.startswith(";;", i):
                    i += 2; flush_stmt(";"); continue
                flush_stmt(";" if ch == ";" else "nl")
                i += 1
                if ch == "\n" and pending_hd:
                    # consume heredoc bodies
                    for strip_tabs, delim in pending_hd:
                        body = []
                        while i < n:
                            j = s.find("\n", i)
                            if j < 0: j = n
                            line = s[i:j]
                            i = j + 1
                            if (line.strip() if True else line) == delim:
                                break
                            body.append(line)
                        heredocs.append((delim, "\n".join(body)))
                    pending_hd = []
                continue
            if s.startswith("&&", i):
                flush_stmt("&&"); i += 2; continue
            if s.startswith("||", i):
                flush_stmt("||"); i += 2; continue
            if ch == "|":
                flush_stage(); i += 2 if s.startswith("|&", i) else 1; continue
            if ch == "&":
                prev = s[i - 1] if i > 0 else ""
                nxt = s[i + 1] if i + 1 < n else ""
                if prev in "<>" or nxt == ">":
                    cur.append(ch); i += 1; continue
                flush_stmt("&", bg=True); i += 1; continue
        cur.append(ch); i += 1
    flush_stmt("")
    if pending_hd:
        # heredoc started on last line without body (truncated input)
        for _, delim in pending_hd:
            heredocs.append((delim, ""))
    return stmts, heredocs


REDIR_RE = re.compile(r"^(\d*|&)(>>?|<|>\||&>>?)(&?\d+|.*)$")


def tokenize(stage):
    try:
        lex = shlex.shlex(stage, posix=True, punctuation_chars=False)
        lex.whitespace_split = True
        lex.commenters = ""
        toks = list(lex)
        return toks, True
    except ValueError:
        return stage.split(), False


def extract_redirs(toks):
    argv, redirs = [], []
    i = 0
    while i < len(toks):
        t = toks[i]
        m = REDIR_RE.match(t)
        if m and t not in ("<", ">") or t in ("<", ">", ">>", "2>", "2>>", "&>", "1>", "&>>"):
            op_target = m.group(3) if m else ""
            op = (m.group(1) + m.group(2)) if m else t
            if t in ("<", ">", ">>", "2>", "2>>", "&>", "1>", "&>>") or op_target == "":
                tgt = toks[i + 1] if i + 1 < len(toks) else ""
                redirs.append((t, tgt)); i += 2; continue
            redirs.append((op, op_target)); i += 1; continue
        if t == "<<HEREDOC":
            redirs.append(("<<", "HEREDOC")); i += 1; continue
        if t.startswith("<<<"):
            redirs.append(("<<<", t[3:] or (toks[i + 1] if i + 1 < len(toks) else ""))); i += 1 if t[3:] else 2; continue
        argv.append(t); i += 1
    return argv, redirs


def path_kind(p):
    p = p.strip("'\"")
    if not p: return "<arg>"
    if p in ("justfile", "./justfile", "Justfile") or p.endswith("/justfile"): return "<justfile>"
    if p.endswith("AGENTS.md") or p.endswith("CLAUDE.md"): return "<agents-md>"
    if "docs/dev/" in p: return "<path:docs-dev>"
    if p.startswith("docs/") or "/docs/" in p: return "<path:docs>"
    if "/.cargo/registry" in p or p.startswith("~/.cargo") or "/.cargo/git" in p: return "<path:cargo-registry>"
    if p.startswith("crates/") or "/crates/" in p: return "<path:crates>"
    if p.startswith("python/") or "/" + config.PY_AREA in p: return "<path:python>"
    if p.startswith("scripts/") or "/scripts/" in p: return "<path:scripts>"
    if p.startswith("target/") or "/target/" in p: return "<path:target>"
    if p.startswith("build/") or "/build/" in p: return "<path:build>"
    if p.startswith("/tmp") or "scratchpad" in p: return "<path:tmp>"
    if ".codex/skills" in p or ".claude/skills" in p or "library-skills" in p: return "<path:skills>"
    if p.startswith("external/") or "/external/" in p: return "<path:external>"
    if p.startswith(".claude") or p.startswith(".codex") or p.startswith(".agents") or p.startswith(".config"): return "<path:agent-config>"
    if p.endswith(".toml") or p.endswith(".lock"): return "<path:manifest>"
    if "/" in p or "." in p: return "<path>"
    return "<arg>"


def norm_flag(tok):
    if "=" in tok and tok.startswith("-"):
        k, v = tok.split("=", 1)
        if "force-validate" in v: return k + "=<..force-validate..>"
        return k + "=<v>"
    m = re.match(r"^(-[A-Za-z])(\d+)$", tok)
    if m: return m.group(1) + "<N>"
    return tok


VALUE_FLAGS = {
    "cargo": {"-p", "--package", "--features", "-F", "--test", "--bench", "--bin", "--example", "--profile",
              "-j", "--jobs", "--target", "--target-dir", "--manifest-path", "-E", "--filterset",
              "--message-format", "--config", "--color", "-Z", "--exclude", "--depth", "-e", "--edges", "-i",
              "--invert", "--format", "--prefix", "--partition", "--retries", "--test-threads", "--cargo-profile",
              "--format-version", "--filter-platform", "--status-level", "--final-status-level", "--success-output",
              "--failure-output", "--archive-file", "--workspace-remap", "--run-ignored", "--lib-name", "--hide-progress-bar_x",
              "--no-tests", "--threads", "--max-fail", "--timeout"},
    "pytest": {"-k", "-m", "-p", "--maxfail", "-n", "--durations", "--tb", "-o", "--basetemp", "--rootdir",
               "-c", "--junitxml", "--deselect", "--ignore", "--timeout", "-W", "--log-level", "--color"},
    "just": {"-f", "--justfile", "-d", "--working-directory", "--set", "--shell", "--color", "--dump-format",
             "--show", "-s", "--unstable_x"},
    "rg": {"-g", "--glob", "-t", "--type", "-T", "--type-not", "-e", "--regexp", "-A", "-B", "-C", "-m",
           "--max-count", "--max-depth", "-f", "--iglob", "-r", "--replace", "--sort", "--max-columns", "-M",
           "--type-add", "--max-filesize", "-j", "--threads", "--sortr", "--context-separator", "--field-match-separator"},
    "grep": {"-e", "-A", "-B", "-C", "-m", "--include", "--exclude", "--exclude-dir", "-f", "--max-count",
             "--color"},
    "git": {"-C", "-c", "-m", "--format", "--pretty", "-n", "--since", "--until", "--author", "-U", "--grep",
            "--date", "-b", "-B", "--depth", "--max-count", "-S", "-G", "--diff-filter", "--message", "-F", "--file"},
}


def classify_argv(argv, redirs, heredoc_bodies, feats):
    """Return (cls, sub, norm, info) for a stripped argv."""
    info = {}
    if not argv:
        return ("noop", "empty", "", info)
    a0 = argv[0]
    base = os.path.basename(a0)
    if a0.startswith(".venv/") or "/.venv/" in a0 or a0.startswith(".venv-parity/"):
        feats.add("venv-bin")
    # python variants
    if re.match(r"^python(3(\.\d+)?)?$", base) or base == "pypy3":
        return classify_python(argv, redirs, heredoc_bodies, feats, info)
    if base in ("pytest", "py.test"):
        return classify_pytest(["pytest"] + argv[1:], info, feats, via="pytest")
    if base == "just":
        return classify_just(argv, info, feats)
    if base == "cargo":
        return classify_cargo(argv, info, feats)
    if base == "uv":
        sub = next((t for t in argv[1:] if not t.startswith("-")), "")
        return ("raw", "uv " + sub, "uv " + sub + norm_rest(argv[2:], "uv"), info)
    if base == "maturin":
        sub = argv[1] if len(argv) > 1 else ""
        return ("raw", "maturin " + sub, "maturin " + sub + norm_rest(argv[2:], "maturin"), info)
    if base in ("bash", "sh", "zsh") and len(argv) > 1 and not argv[1].startswith("-"):
        sp = argv[1]
        if "scripts/" in sp or sp.startswith("./"):
            return ("raw", "repo-script:" + os.path.basename(sp), "bash " + "<script:%s>" % os.path.basename(sp) + norm_rest(argv[2:], "x"), info)
        return ("other", "bash-script", "bash <script>" , info)
    if base == "lctx" or (("/target/" in a0 or a0.startswith("target/")) and base == "lctx"):
        sub = next((t for t in argv[1:] if not t.startswith("-")), "")
        return ("raw", "lctx " + sub, "lctx " + sub + norm_rest(argv[2:], "x"), info)
    if (a0.startswith("scripts/") or a0.startswith("./scripts/") or config.REPO + "/scripts/" in a0):
        return ("raw", "repo-script:" + base, "<script:%s>" % base + norm_rest(argv[1:], "x"), info)
    if base == "git":
        return classify_git(argv, redirs, info, feats)
    if base in ("sed",):
        flags = [t for t in argv[1:] if t.startswith("-")]
        if any(t == "-i" or t.startswith("-i") or t == "--in-place" or (re.match(r"^-[a-zA-Z]*i", t) and not t.startswith("--")) for t in flags):
            return ("edit-shell", "sed -i", "sed -i <expr> <path>", info)
        paths = [path_kind(t) for t in argv[1:] if not t.startswith("-")][1:]
        info["paths"] = paths
        return ("search-read", "sed", "sed " + " ".join(sorted(set(f for f in flags))) + " <expr> " + " ".join(sorted(set(paths))), info)
    if base == "perl":
        flags = "".join(t for t in argv[1:] if t.startswith("-"))
        if "i" in flags.replace("-", "") and ("p" in flags or "i" in flags):
            if re.search(r"-[a-zA-Z]*i", " ".join(argv[1:3])):
                return ("edit-shell", "perl -i", "perl -pi <expr> <path>", info)
        return ("search-read", "perl", "perl <expr>", info)
    if base == "awk" or base == "gawk":
        if "-i" in argv and "inplace" in argv:
            return ("edit-shell", "awk -i", "awk -i inplace", info)
        return ("search-read", "awk", "awk <prog> <path>", info)
    if base == "tee":
        files = [t for t in argv[1:] if not t.startswith("-") and t != "/dev/null"]
        if files:
            return ("edit-shell", "tee", "tee <file>", info)
        return ("noop", "tee", "tee", info)
    if base in ("cat", "echo", "printf") and any(op in (">", ">>", "1>") or (op.endswith(">") and not op.startswith("2")) for op, t in redirs if t not in ("/dev/null", "/dev/stderr", "&2", "&1")):
        tgt = [t for op, t in redirs if ">" in op and t not in ("/dev/null", "&2", "&1", "/dev/stderr")]
        hd = any(op == "<<" for op, t in redirs)
        info["write_target"] = path_kind(tgt[0]) if tgt else "<file>"
        if info["write_target"] == "<path:tmp>":
            return ("inline-script", "%s-write-tmp" % base, "%s > <path:tmp>" % base, info)
        return ("edit-shell", "%s-redirect%s" % (base, "-heredoc" if hd else ""), "%s > %s" % (base, info["write_target"]), info)
    if base in ("patch",):
        return ("edit-shell", "patch", "patch", info)
    if base == "apply_patch" or base == "applypatch":
        return ("edit-shell", "apply_patch", "apply_patch", info)
    if base in ("cp", "mv", "rm", "mkdir", "touch", "ln", "chmod", "rmdir", "install", "rsync", "truncate", "chown"):
        return ("other", "fileops:" + base, base + norm_rest(argv[1:], base), info)
    if base in SEARCH_READ:
        flags = [norm_flag(t) for t in argv[1:] if t.startswith("-")]
        pos = [t for t in argv[1:] if not t.startswith("-")]
        if base in ("rg", "grep", "egrep", "ugrep", "ag"):
            paths = [path_kind(t) for t in pos[1:]]
        else:
            paths = [path_kind(t) for t in pos]
        info["paths"] = paths
        if base == "tail" and any(f in ("-f", "-F", "--follow") for f in flags):
            return ("process-monitor", "tail -f", "tail -f <path>", info)
        if base == "ldd":
            return ("other", "env-probe:ldd", "ldd <path>", info)
        pat = " <pat>" if base in ("rg", "grep", "egrep", "ugrep", "ag", "ast-grep", "sg") and pos else ""
        return ("search-read", base, base + (" " + " ".join(sorted(set(flags))) if flags else "") + pat + (" " + " ".join(sorted(set(paths))) if paths else ""), info)
    if base in PROCESS_MON:
        if base == "sleep":
            try:
                v = argv[1] if len(argv) > 1 else "0"
                mult = {"s": 1, "m": 60, "h": 3600}.get(v[-1:], 1)
                info["sleep_s"] = float(v.rstrip("smh")) * mult
            except ValueError:
                info["sleep_s"] = None
            return ("process-monitor", "sleep", "sleep <N>", info)
        return ("process-monitor", base, base + norm_rest(argv[1:], base), info)
    if base in RAW_TOOLS or base.startswith("cargo-"):
        return ("raw", "tool:" + base, base + norm_rest(argv[1:], base), info)
    if base in ("gh",):
        sub = " ".join(argv[1:3])
        return ("other", "gh", "gh " + sub, info)
    if base in ("curl", "wget"):
        return ("other", "net:" + base, base + " <url>", info)
    if base in ("which", "printenv", "ldconfig") or (base == "command" and len(argv) > 1 and argv[1] == "-v") or (base == "type"):
        return ("other", "env-probe:" + base, base + " <arg>", info)
    if base in ("node", "npm", "npx", "deno", "bun"):
        if base == "node" and len(argv) > 1 and argv[1] == "-e":
            return ("inline-script", "node -e", "node -e <code>", info)
        return ("other", "node", base + " " + (argv[1] if len(argv) > 1 else ""), info)
    if base in ("docker", "podman"):
        sub = next((t for t in argv[1:] if not t.startswith("-")), "")
        return ("other", "docker " + sub, base + " " + sub, info)
    if base in ("codex", "claude"):
        return ("other", "agent-cli", base, info)
    if base in ("direnv",):
        return ("other", "direnv", "direnv " + (argv[1] if len(argv) > 1 else ""), info)
    if base in ("xtask",):
        return ("raw", "xtask", "xtask" + norm_rest(argv[1:], "x"), info)
    if base in NOOP:
        if base in ("echo", "printf"):
            return ("noop", base, base + " <text>", info)
        if base in ("test", "[", "[["):
            return ("noop", "test", "test <expr>", info)
        return ("noop", base, base, info)
    if a0.startswith("./target/") or a0.startswith("target/") or "/target/debug/" in a0 or "/target/release/" in a0 or "/target/" in a0 and "/deps/" in a0:
        return ("raw", "test-binary", "<target-binary>" + norm_rest(argv[1:], "x"), info)
    if a0.startswith("/tmp/") or "scratchpad" in a0:
        return ("inline-script", "tmp-exec", "<tmp-exec>", info)
    return ("other", "cmd:" + base[:30], base[:30] + norm_rest(argv[1:], base)[:80], info)


def norm_rest(args, tool):
    out = []
    vflags = VALUE_FLAGS.get(tool, set())
    skip = False
    for idx, t in enumerate(args):
        if skip:
            skip = False
            continue
        if t.startswith("-") and len(t) > 1:
            if t == "--":
                out.append("--"); continue
            if t in vflags:
                v = args[idx + 1] if idx + 1 < len(args) else ""
                if "force-validate" in v:
                    out.append(t + " <..force-validate..>")
                elif PKG_RE.match(v):
                    out.append(t + " <pkg>")
                else:
                    out.append(t + " <v>")
                skip = True
                continue
            out.append(norm_flag(t))
        else:
            if PKG_RE.match(t):
                out.append("<pkg>")
            elif NUM_RE.match(t):
                out.append("<N>")
            elif re.search(r"[()]", t) or "::" in t:
                out.append("<filter>")
            else:
                pk = path_kind(t)
                out.append(pk if pk != "<arg>" else "<arg>")
    # collapse consecutive identical placeholders
    col = []
    for t in out:
        if col and col[-1] == t and t.startswith("<"):
            continue
        col.append(t)
    return (" " + " ".join(col)) if col else ""


def classify_python(argv, redirs, heredoc_bodies, feats, info):
    i = 1
    while i < len(argv) and argv[i].startswith("-") and argv[i] not in ("-m", "-c", "-"):
        if argv[i] in ("-X", "-W"):
            i += 2; continue
        i += 1
    if i >= len(argv):
        return ("other", "python-repl", "python", info)
    t = argv[i]
    if t == "-m":
        mod = argv[i + 1] if i + 1 < len(argv) else ""
        if mod == "pytest":
            return classify_pytest(["pytest"] + argv[i + 2:], info, feats, via="python -m")
        if mod in ("json.tool",):
            return ("search-read", "python -m json.tool", "python -m json.tool", info)
        return ("raw", "python -m " + mod, "python -m " + mod + norm_rest(argv[i + 2:], "x"), info)
    if t == "-c":
        code = argv[i + 1] if i + 1 < len(argv) else ""
        if PY_WRITE_RE.search(code):
            return ("edit-shell", "python-c-write", "python -c <code:writes>", info)
        return ("inline-script", "python -c", "python -c <code>", info)
    if t == "-":
        body = "\n".join(heredoc_bodies)
        if PY_WRITE_RE.search(body):
            if re.search(r"['\"]/tmp/", body) and not REPO_PATH_RE.search(body):
                return ("inline-script", "python-heredoc-tmpwrite", "python - <<heredoc:tmp-writes>", info)
            return ("edit-shell", "python-heredoc-write", "python - <<heredoc:writes>", info)
        return ("inline-script", "python-heredoc", "python - <<heredoc>", info)
    sp = t
    if sp.startswith("scripts/") or "/scripts/" in sp:
        bn = os.path.basename(sp)
        rest = argv[i + 1:]
        if bn in ("verify.py", "native_controls.py"):
            fam = next((x for x in rest if not x.startswith("-")), "")
            info["family"] = fam
            if "--command" in rest:
                k = rest.index("--command")
                info["boundary"] = rest[k + 1] if k + 1 < len(rest) else ""
            info["filtered"] = bool([x for x in rest[1:] if x not in ("--command", info.get("boundary"))])
            return ("raw", "%s:%s" % (bn[:-3], fam), "python <script:%s> %s" % (bn, fam) + norm_rest(rest[1:], "cargo"), info)
        return ("raw", "python-script:" + bn, "python <script:%s>" % bn + norm_rest(rest, "x"), info)
    if sp.startswith("/tmp") or "scratchpad" in sp:
        return ("inline-script", "python-tmp-script", "python <tmp-script>", info)
    if "/.codex/skills/" in sp or ".claude/skills" in sp or "library-skills" in sp:
        return ("raw", "python-skill-script", "python <skill-script>", info)
    return ("raw", "python-script:" + os.path.basename(sp)[:40], "python <script:%s>" % os.path.basename(sp)[:40], info)


def classify_pytest(argv, info, feats, via):
    rest = argv[1:]
    info["pytest_via"] = via
    flags = norm_rest(rest, "pytest")
    return ("raw", "pytest", "pytest" + flags, info)


def classify_just(argv, info, feats):
    i = 1
    flags = []
    jf = None
    mode = None
    while i < len(argv):
        t = argv[i]
        if t in ("-f", "--justfile"):
            jf = argv[i + 1] if i + 1 < len(argv) else ""
            i += 2; continue
        if t.startswith("--justfile="):
            jf = t.split("=", 1)[1]; i += 1; continue
        if t in ("-d", "--working-directory", "--set", "--shell", "--color", "--dump-format", "--shell-arg"):
            flags.append(t)
            i += 3 if t == "--set" else 2; continue
        if t in ("-l", "--list", "--summary", "--dump", "--evaluate", "--groups", "--variables", "--choose", "--json"):
            mode = mode or t.lstrip("-").replace("l", "list") if t == "-l" else (mode or t.lstrip("-"))
            i += 1; continue
        if t in ("-s", "--show"):
            mode = "show"; info["show"] = argv[i + 1] if i + 1 < len(argv) else ""
            i += 2; continue
        if t.startswith("-"):
            flags.append(t); i += 1; continue
        break
    if jf is not None:
        info["justfile"] = jf
        feats.add("just-f")
    if mode or (i >= len(argv) and not jf):
        m = mode or "default"
        if m == "l": m = "list"
        info["recipe"] = "--" + m
        return ("just-discovery", "just --" + m, "just --" + m, info)
    recipe = argv[i] if i < len(argv) else ""
    info["recipe"] = recipe
    args = argv[i + 1:]
    shape = []
    pkgs = []
    for a in args:
        if a.startswith("-"):
            shape.append(norm_flag(a))
        elif PKG_RE.match(a):
            shape.append("<pkg>"); pkgs.append(a)
        elif re.search(r"[()&|]", a) or "::" in a:
            shape.append("<filter>")
        elif "=" in a:
            shape.append("<k=v>")
        elif "/" in a:
            shape.append(path_kind(a))
        elif NUM_RE.match(a):
            shape.append("<N>")
        else:
            shape.append("<word>")
    info["rargs"] = " ".join(shape)
    if recipe.startswith("verify-"):
        info["family"] = recipe[len("verify-"):]
        if "--command" in args:
            k = args.index("--command")
            info["boundary"] = args[k + 1] if k + 1 < len(args) else ""
        info["filtered"] = bool([a for a in args if a not in ("--command", info.get("boundary"), "--")])
    info["pkgs"] = pkgs
    if "force-validate" in " ".join(args):
        info["ff"] = True
    return ("just", recipe, ("just -f <jf> " if jf else "just ") + recipe + ((" " + " ".join(shape)) if shape else ""), info)


def classify_cargo(argv, info, feats):
    i = 1
    while i < len(argv) and (argv[i].startswith("+") or argv[i] in ("--locked", "--offline", "--frozen", "-q", "--quiet", "-v", "-vv", "--verbose") or argv[i].startswith("--config") or argv[i] == "-Z" or argv[i].startswith("--color")):
        if argv[i].startswith("+"):
            feats.add("toolchain-override")
            info["toolchain"] = argv[i]
        if argv[i] in ("-Z", "--config", "--color"):
            i += 2; continue
        i += 1
    sub = argv[i] if i < len(argv) else ""
    rest = argv[i + 1:]
    if sub == "nextest" and rest:
        sub = "nextest " + rest[0]
        rest = rest[1:]
    if sub == "xtask":
        return ("raw", "xtask", "cargo xtask" + norm_rest(rest, "cargo"), info)
    joined = " ".join(argv)
    info["ff"] = "force-validate" in joined
    pk = []
    for j, t in enumerate(rest):
        if t in ("-p", "--package") and j + 1 < len(rest):
            pk.append(rest[j + 1])
        elif t.startswith("--package="):
            pk.append(t.split("=", 1)[1])
    info["pkgs"] = pk
    info["cargo_sub"] = sub
    return ("raw", "cargo " + sub, "cargo " + sub + norm_rest(rest, "cargo"), info)


def classify_git(argv, redirs, info, feats):
    i = 1
    while i < len(argv) and argv[i].startswith("-"):
        if argv[i] in ("-C", "-c", "--git-dir", "--work-tree"):
            if argv[i] == "-C": info["git_C"] = argv[i + 1] if i + 1 < len(argv) else ""
            i += 2; continue
        i += 1
    sub = argv[i] if i < len(argv) else ""
    rest = argv[i + 1:]
    detail = sub
    if sub == "add":
        if any(t in ("-A", "--all", ".", "-u", "--update", ":/") for t in rest):
            detail = "add -A/." if any(t in ("-A", "--all", ".", ":/") for t in rest) else "add -u"
        elif "-p" in rest or "--patch" in rest:
            detail = "add -p"
        else:
            detail = "add <paths>"
    elif sub == "stash":
        detail = "stash " + (rest[0] if rest and not rest[0].startswith("-") else "push")
    elif sub == "checkout":
        if "--" in rest or (rest and rest[0] in (".",)):
            detail = "checkout -- <paths>"
        elif "-b" in rest or "-B" in rest:
            detail = "checkout -b"
        else:
            detail = "checkout <ref>"
    elif sub == "reset":
        detail = "reset --hard" if "--hard" in rest else ("reset --soft" if "--soft" in rest else "reset")
    elif sub == "restore":
        detail = "restore --staged" if "--staged" in rest and "--worktree" not in rest else "restore"
    elif sub == "worktree":
        detail = "worktree " + (rest[0] if rest else "")
    elif sub == "commit":
        detail = "commit" + (" -a" if any(t in ("-a", "--all", "-am") for t in rest) else "") + (" --amend" if "--amend" in rest else "")
    elif sub == "clean":
        detail = "clean"
    cls = "git"
    mut_prefixes = ("add", "commit", "stash push", "stash pop", "stash apply", "stash drop", "stash save", "stash -", "checkout", "restore",
                    "reset", "rebase", "cherry-pick", "worktree add", "worktree remove", "worktree prune", "worktree move", "merge",
                    "push", "pull", "apply", "rm", "mv", "clean", "switch", "am", "revert", "update-index")
    info["git_mut"] = detail.startswith(mut_prefixes) and not detail.startswith("stash list") and not detail.startswith("stash show")
    return (cls, "git " + detail, "git " + detail, info)


def strip_prefixes(argv, feats, envs, exports, ctx):
    """Strip wrappers; may return ('recurse', script) for bash -c."""
    guard = 0
    while argv and guard < 40:
        guard += 1
        t = argv[0]
        b = os.path.basename(t)
        if ASSIGN_RE.match(t):
            envs.append(t.split("=", 1)[0]); argv = argv[1:]; continue
        if t in KEYWORD_SKIP:
            if t in ("do", "then", "else"):
                pass
            argv = argv[1:]; continue
        if t in ("for", "select"):
            ctx["loop"] = ctx.get("loop") or "for"
            return []
        if t in ("while", "until"):
            ctx["loop"] = t
            argv = argv[1:]; continue
        if t in ("if", "elif"):
            argv = argv[1:]; continue
        if t == "case":
            return []
        if t == "export":
            for a in argv[1:]:
                if not a.startswith("-"):
                    exports.append(a.split("=", 1)[0])
            return []
        if t in ("cd", "pushd"):
            ctx.setdefault("cd", []).append(argv[1] if len(argv) > 1 else "~")
            return []
        if t in ("source", "."):
            ctx.setdefault("source", []).append(argv[1] if len(argv) > 1 else "")
            return []
        if t == "set" or t in ("ulimit", "shopt", "trap", "unset", "alias", "local", "declare", "readonly", "popd", "umask"):
            if t == "set": feats.add("set-flags")
            return []
        if b == "timeout":
            feats.add("timeout")
            j = 1
            while j < len(argv) and argv[j].startswith("-"):
                if argv[j] in ("-s", "-k", "--signal", "--kill-after"):
                    j += 2
                else:
                    j += 1
            if j < len(argv):
                ctx["timeout_s"] = argv[j]
            argv = argv[j + 1:]; continue
        if b == "env" and len(argv) > 1:
            j = 1
            while j < len(argv):
                a = argv[j]
                if a in ("-u", "--unset", "-C", "--chdir", "-S"):
                    if a in ("-u", "--unset"): feats.add("env-unset")
                    j += 2; continue
                if a in ("-i", "--ignore-environment", "-", "--"):
                    feats.add("env-i") if a in ("-i", "--ignore-environment") else None
                    j += 1; continue
                if ASSIGN_RE.match(a):
                    envs.append(a.split("=", 1)[0]); j += 1; continue
                break
            feats.add("env-cmd")
            argv = argv[j:]; continue
        if b in ("nice", "ionice", "chrt", "taskset"):
            feats.add(b)
            j = 1
            while j < len(argv) and argv[j].startswith("-"):
                j += 2 if argv[j] in ("-n", "-c", "-p") else 1
            if b == "taskset" and j < len(argv): j += 1
            argv = argv[j:]; continue
        if b in ("time", "nohup", "setsid", "exec", "stdbuf", "unbuffer", "sudo", "caffeinate", "systemd-inhibit"):
            feats.add(b)
            j = 1
            while j < len(argv) and argv[j].startswith("-"):
                j += 1
            argv = argv[j:]; continue
        if b == "command" and len(argv) > 1 and argv[1] != "-v":
            argv = argv[1:]; continue
        # python[3] scripts/<wrapper>.py [opts] -- CMD runs CMD inside the wrapper's environment
        WRAPS = ("build_environment.py", "surrealdb_fixture.py")
        if re.match(r"^python(3(\.\d+)?)?$", b) and len(argv) > 2 and os.path.basename(argv[1]) in WRAPS and "--" in argv:
            feats.add("wrapper:" + os.path.basename(argv[1]))
            feats.add("python:" + b)
            argv = argv[argv.index("--") + 1:]
            continue
        if os.path.basename(t) in WRAPS and "--" in argv:
            feats.add("wrapper:" + os.path.basename(t))
            argv = argv[argv.index("--") + 1:]
            continue
        if b == "systemd-run":
            feats.add("systemd-run")
            j = 1
            while j < len(argv) and argv[j].startswith("-"):
                a = argv[j]
                if a == "--":
                    j += 1; break
                if a in ("-p", "--property", "-u", "--unit", "--slice", "-E", "--setenv", "--uid", "--gid", "-M", "--machine", "--description", "--working-directory", "-t"):
                    if a in ("-E", "--setenv") and j + 1 < len(argv):
                        envs.append(argv[j + 1].split("=", 1)[0])
                    j += 2; continue
                if a.startswith("--setenv="):
                    envs.append(a.split("=", 1)[1].split("=", 1)[0])
                j += 1
            argv = argv[j:]; continue
        if b == "direnv" and len(argv) > 2 and argv[1] == "exec":
            feats.add("direnv-exec")
            argv = argv[3:]; continue
        if b in ("bash", "sh", "zsh", "dash") and len(argv) > 1 and argv[1].startswith("-") and "c" in argv[1]:
            # bash -c / -lc / -ec 'script'
            j = 1
            while j < len(argv) and argv[j].startswith("-"):
                if argv[j] == "-o": j += 2; continue
                if "c" in argv[j]: j += 1; break
                j += 1
            if j < len(argv):
                feats.add("bash-c")
                return ("recurse", argv[j])
            return []
        if b in ("bash", "sh") and len(argv) > 1 and argv[1].startswith("-") and "c" not in argv[1]:
            # bash -euo pipefail script.sh or bash -x script
            j = 1
            while j < len(argv) and argv[j].startswith("-"):
                j += 2 if argv[j] == "-o" else 1
            argv = [argv[0]] + argv[j:]
            if len(argv) == 1: return []
            continue
        if b == "uv" and len(argv) > 1 and argv[1] == "run":
            feats.add("uv-run")
            flags = []
            j = 2
            while j < len(argv) and argv[j].startswith("-"):
                a = argv[j]
                flags.append(a)
                if a in ("--python", "-p", "--with", "--project", "--directory", "--group", "--extra", "--env-file", "--package", "--with-requirements", "--index", "--only-group", "--no-group"):
                    j += 2; continue
                if a == "--":
                    j += 1; break
                j += 1
            if "--no-sync" in flags:
                feats.add("uv-no-sync")
            elif "--no-project" in flags:
                feats.add("uv-no-project")
            elif "UV_NO_SYNC" in envs:
                feats.add("uv-no-sync(env)")
            else:
                feats.add("uv-run-syncs")
            if "--frozen" in flags:
                feats.add("uv-frozen")
            if j < len(argv):
                argv = argv[j:]; continue
            return ["uv", "run"]
        if b == "flock" and len(argv) > 2:
            # flock [-opts] <lockfile> <cmd...> | flock [-opts] <lockfile> -c '<cmd>': an agent-made build lock
            feats.add("flock")
            j = 1
            while j < len(argv) and argv[j].startswith("-") and argv[j] not in ("-c", "--command"):
                j += 2 if argv[j] in ("-w", "--timeout", "-E", "--conflict-exit-code") else 1
            j += 1  # lock file
            if j < len(argv) and argv[j] in ("-c", "--command") and j + 1 < len(argv):
                return ("recurse", argv[j + 1])
            argv = argv[j:]; continue
        if b == "xargs":
            feats.add("xargs")
            j = 1
            while j < len(argv) and argv[j].startswith("-"):
                j += 2 if argv[j] in ("-n", "-I", "-P", "-L", "-d", "-a", "-s", "-E") else 1
            argv = argv[j:]; continue
        if b == "watch":
            feats.add("watch")
            ctx["loop"] = ctx.get("loop") or "watch"
            j = 1
            while j < len(argv) and argv[j].startswith("-"):
                j += 2 if argv[j] in ("-n", "--interval") else 1
            argv = argv[j:]; continue
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*\(\)$", t) or (len(argv) > 1 and argv[1] == "()"):
            return []
        break
    return argv


PRIORITY = {"just": 0, "raw": 1, "edit-shell": 2, "inline-script": 3, "git-mut": 4, "just-discovery": 5,
            "process-monitor": 6, "git": 7, "search-read": 8, "other": 9, "noop": 10}


def parse_command(cmd, depth=0):
    """Parse a full shell command string. Returns a dict with segments and features."""
    res = {"segs": [], "envs": [], "exports": [], "feats": set(), "ctx": {}, "pipes": [], "heredoc": 0,
           "parse_ok": True, "redir_log": False, "bg": False, "write_targets": []}
    try:
        stmts, heredocs = split_script(cmd)
    except Exception:
        res["parse_ok"] = False
        stmts, heredocs = [{"stages": [cmd], "bg": False, "conn": ""}], []
    res["heredoc"] = len(heredocs)
    hd_bodies = [b for _, b in heredocs]
    for st in stmts:
        if st["bg"]:
            res["bg"] = True
        stage_infos = []
        for si, stage in enumerate(st["stages"]):
            st_text = stage.strip()
            grp = None
            if st_text.startswith("(") and not st_text.startswith("(("):
                c = match_close(st_text, 0, "(", ")")
                if c > 0:
                    grp = st_text[1:c]
            elif st_text.startswith("{ ") or st_text.startswith("{\n"):
                c = match_close(st_text, 0, "{", "}")
                if c > 0:
                    grp = st_text[1:c]
            if grp is not None and depth < 4:
                r = ("recurse", grp)
                tail_redirs = st_text[c + 1:]
                if ">" in tail_redirs:
                    res["redir_log"] = True
            else:
                r = None
            if r is None:
                toks, ok = tokenize(st_text)
                if not ok:
                    res["parse_ok"] = False
                argv, redirs = extract_redirs(toks)
                envs, exports = [], []
                r = strip_prefixes(argv, res["feats"], envs, exports, res["ctx"])
                if not r and envs and argv and all(ASSIGN_RE.match(t) for t in argv):
                    res["ctx"].setdefault("shellvars", []).extend(envs)
                    envs = []
                res["envs"].extend(envs)
                res["exports"].extend(exports)
            if isinstance(r, tuple) and r and r[0] == "recurse":
                if depth < 3:
                    sub = parse_command(r[1], depth + 1)
                    res["segs"].extend(sub["segs"])
                    res["envs"].extend(sub["envs"]); res["exports"].extend(sub["exports"])
                    res["feats"] |= sub["feats"]; res["pipes"].extend(sub["pipes"])
                    for k, v in sub["ctx"].items():
                        if isinstance(v, list):
                            res["ctx"].setdefault(k, []).extend(v)
                        else:
                            res["ctx"].setdefault(k, v)
                    res["heredoc"] += sub["heredoc"]
                    res["bg"] = res["bg"] or sub["bg"]
                    res["redir_log"] = res["redir_log"] or sub["redir_log"]
                    res["write_targets"].extend(sub["write_targets"])
                    if not sub["parse_ok"]: res["parse_ok"] = False
                    if sub["segs"]:
                        ps = min(sub["segs"], key=lambda x: PRIORITY.get(x["pcls"], 11))
                        stage_infos.append((ps["cls"], ps["sub"], ps["norm"], ps["info"], [], ps["pcls"]))
                continue
            argv = r
            if not argv:
                continue
            cls, sub, norm, info = classify_argv(argv, redirs, hd_bodies, res["feats"])
            if cls == "edit-shell" and sub == "tee":
                files = [t for t in argv[1:] if not t.startswith("-")]
                if si > 0 and (not stage_infos or stage_infos[0][1] not in ("echo", "printf", "cat")) or all(path_kind(f) in ("<path:tmp>", "<path:build>", "<path:target>") for f in files):
                    cls, sub, norm = "noop", "tee-log", "tee <log>"
                    res["redir_log"] = True
            if cls == "git" and info.get("git_mut"):
                pcls = "git-mut"
            else:
                pcls = cls
            out_redirs = [(op, t) for op, t in redirs if ">" in op and t not in ("/dev/null", "&1", "&2", "/dev/stderr", "/dev/stdout")]
            if out_redirs and cls not in ("edit-shell",):
                res["redir_log"] = True
            if cls == "edit-shell" and info.get("write_target"):
                res["write_targets"].append(info["write_target"])
            stage_infos.append((cls, sub, norm, info, redirs, pcls))
            res["segs"].append({"cls": cls, "pcls": pcls, "sub": sub, "norm": norm, "info": info,
                                "stage": si, "nstages": len(st["stages"]),
                                "raw": " ".join(argv)[:300],
                                "err2out": any(op in ("2>&1", "&>", "&>>") or (op.startswith("2>") and t in ("&1",)) for op, t in redirs)})
        if len(stage_infos) > 1:
            up = stage_infos[0]
            downs = [os.path.basename(s[3].get("_a0", s[1].split(" ")[0].split(":")[-1])) for s in stage_infos[1:]]
            downs = [s[1].split(" ")[0] for s in stage_infos[1:]]
            res["pipes"].append({"up_cls": up[5], "up_sub": up[1], "down": downs})
    return res


def summarize(cmd):
    p = parse_command(cmd)
    segs = p["segs"]
    if segs:
        prim = min(segs, key=lambda s: (PRIORITY.get(s["pcls"], 11), 0))
    else:
        prim = {"cls": "noop", "pcls": "noop", "sub": "empty", "norm": "", "info": {}, "raw": ""}
    # loops containing sleep => polling
    has_sleep = any(s["sub"] == "sleep" for s in segs)
    loop = p["ctx"].get("loop")
    out = {
        "cls": prim["cls"], "sub": prim["sub"], "norm": prim["norm"][:200], "pinfo": prim["info"],
        "praw": prim.get("raw", "")[:300],
        "segs": [(s["cls"], s["sub"]) for s in segs],
        "envs": sorted(set(p["envs"])), "exports": sorted(set(p["exports"])), "feats": sorted(p["feats"]),
        "shellvars": p["ctx"].get("shellvars", []),
        "cd": p["ctx"].get("cd", []), "source": p["ctx"].get("source", []), "loop": loop,
        "has_sleep": has_sleep, "timeout_s": p["ctx"].get("timeout_s"), "pipes": p["pipes"],
        "heredoc": p["heredoc"], "parse_ok": p["parse_ok"], "redir_log": p["redir_log"], "bg": p["bg"],
        "nseg": len(segs), "write_targets": p["write_targets"],
        "err2out": any(s.get("err2out") for s in segs),
        "sleep_s": sum((s["info"].get("sleep_s") or 0) for s in segs if s["sub"] == "sleep"),
    }
    if loop and has_sleep:
        out["polling"] = True
    elif loop in ("while", "until"):
        out["polling"] = True
    else:
        out["polling"] = False
    return out
