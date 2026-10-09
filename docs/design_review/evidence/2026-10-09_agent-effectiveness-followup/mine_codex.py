"""Mine attributed native completions; keep command/output material in ignored build/.

This review helper reads state_5.sqlite with mode=ro and original JSONL streams.
It reuses the October 7 miner's shell parser/classifier, but does not use its
fixed cutoff, equal-runtime weighting, or assumed success for function calls.
Run with the repository's Python: uv run --no-sync python PATH --help.
"""

import argparse
import collections
import datetime as dt
import gzip
import hashlib
import importlib.util
import json
import pathlib
import sqlite3
import sys


def parse_time(value):
    return dt.datetime.fromisoformat(value.replace("Z", "+00:00"))


def parent_id(source):
    try:
        return json.loads(source).get("subagent", {}).get("thread_spawn", {}).get("parent_thread_id")
    except (ValueError, AttributeError, TypeError):
        return None


def output_text(value):
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return "\n".join(part.get("text", "") for part in value if isinstance(part, dict))
    return str(value or "")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--start", default="2026-10-06T04:00:00Z")
    parser.add_argument("--cutoff", default="2026-10-09T18:45:09Z")
    parser.add_argument("--exclude", action="append", default=["01a121f2-de58-7573-a6ee-88bd39280f2a"])
    parser.add_argument("--out", type=pathlib.Path, default=pathlib.Path("build/agent-effectiveness-followup/2026-10-09"))
    args = parser.parse_args()
    repo = pathlib.Path(__file__).resolve().parents[4]
    old = repo / "docs/design_review/evidence/2026-10-07_agent-workspace-effectiveness/miner"
    sys.path.insert(0, str(old))
    spec = importlib.util.spec_from_file_location("prior_extractor", old / "extract.py")
    extractor = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(extractor)
    start, cutoff = parse_time(args.start), parse_time(args.cutoff)
    db = pathlib.Path.home() / ".codex/state_5.sqlite"
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    con.row_factory = sqlite3.Row
    rows = [dict(r) for r in con.execute("SELECT id,rollout_path,cwd,source,agent_role,cli_version,model,originator,created_at_ms,updated_at_ms FROM threads WHERE updated_at_ms >= ?", (int(start.timestamp() * 1000),))]
    excluded = set(args.exclude)
    while True:
        children = {r["id"] for r in rows if parent_id(r["source"]) in excluded}
        if children <= excluded:
            break
        excluded |= children
    def own_cwd(cwd):
        return cwd == str(repo) or cwd.startswith(str(repo) + "/") or cwd.startswith(str(repo) + "-wt/") or extractor.config.cwd_kind(cwd) in {"tmp-repo", "cache-copy", "tmp-copy"}
    selected = {r["id"] for r in rows if own_cwd(r["cwd"] or "")}
    while True:
        children = {r["id"] for r in rows if parent_id(r["source"]) in selected}
        if children <= selected:
            break
        selected |= children
    selected -= excluded
    args.out.mkdir(parents=True, exist_ok=True)
    stats = collections.Counter()
    by_day = collections.Counter()
    by_outcome = collections.Counter()
    by_role = collections.Counter()
    by_version = collections.Counter()
    by_origin = collections.Counter()
    by_family = collections.Counter()
    by_mcp = collections.Counter()
    populated = set()
    seen = set()
    index = []
    with gzip.open(args.out / "commands.jsonl.gz", "wt") as commands, gzip.open(args.out / "tool-errors.jsonl.gz", "wt") as errors:
        for meta in rows:
            if meta["id"] not in selected:
                continue
            path = pathlib.Path(meta["rollout_path"])
            if not path.is_file():
                stats["missing_rollouts"] += 1
                continue
            stats["candidate_files"] += 1
            calls = {}
            before = stats["native_completions"]
            with path.open(encoding="utf-8") as source:
                for lineno, line in enumerate(source, 1):
                    try:
                        event = json.loads(line)
                    except json.JSONDecodeError:
                        stats["invalid_json_lines"] += 1
                        continue
                    try:
                        when = parse_time(event.get("timestamp", ""))
                    except ValueError:
                        stats["missing_or_invalid_time"] += 1
                        continue
                    if not start <= when <= cutoff:
                        continue
                    payload = event.get("payload", {})
                    if not isinstance(payload, dict):
                        continue
                    typ = payload.get("type")
                    if typ in {"custom_tool_call", "function_call"}:
                        calls[payload.get("call_id")] = {"line": lineno, "name": payload.get("name"), "input": payload.get("input") or payload.get("arguments")}
                        stats[f"invocation:{payload.get('namespace') or ''}.{payload.get('name')}"] += 1
                    elif typ in {"custom_tool_call_output", "function_call_output"}:
                        text = output_text(payload.get("output"))
                        if "Script failed" in text[:2000] or "Script error" in text[:2000] or text.startswith("Error:"):
                            stats["outer_error_candidates"] += 1
                            errors.write(json.dumps({"thread":meta["id"],"path":str(path),"line":lineno,"timestamp":event["timestamp"],"call_id":payload.get("call_id"),"call":calls.get(payload.get("call_id")),"output_head":text[:12000]}) + "\n")
                    elif typ == "item_completed":
                        item = payload.get("item", {})
                        key = (meta["id"], item.get("id"))
                        if item.get("type") not in {"CommandExecution", "McpToolCall"}:
                            continue
                        if key in seen:
                            stats["duplicate_completion_items"] += 1
                            continue
                        seen.add(key)
                        if item.get("type") == "McpToolCall":
                            by_mcp[f"{item.get('server')}.{item.get('tool')}:{item.get('status')}"] += 1
                            continue
                        cwd = (item.get("cwd") or meta["cwd"] or "").replace("file://", "")
                        # Thread attribution is retained even for a command in a shared
                        # skill/registry directory. No unrelated thread is text-mined.
                        argv = item.get("command") or []
                        if isinstance(argv, list):
                            command = argv[2] if len(argv) >= 3 and pathlib.Path(argv[0]).name in {"bash", "sh", "zsh"} and argv[1] in {"-lc", "-c"} else " ".join(argv)
                        else:
                            command = str(argv)
                        fields, _ = extractor.shell_fields(command)
                        output = item.get("aggregated_output") or (item.get("stdout") or "") + (item.get("stderr") or "")
                        exit_code = item.get("exit_code")
                        outcome, heuristic = extractor.classify_outcome(fields["cls"], fields.get("sub"), exit_code, extractor.text_sample(output), bool(fields.get("pdown")), fields.get("segs", ()), fields.get("redir"))
                        role = meta["agent_role"] or ("worker_unspecified" if parent_id(meta["source"]) else "root")
                        record = {"thread":meta["id"],"role":role,"path":str(path),"line":lineno,"timestamp":event["timestamp"],"started_at_ms":payload.get("started_at_ms"),"completed_at_ms":payload.get("completed_at_ms"),"item_id":item.get("id"),"cwd":cwd,"command":command,"exit_code":exit_code,"status":item.get("status"),"outcome_heuristic":outcome,"heuristic":heuristic,"class":fields["cls"],"sub":fields.get("sub"),"recipe":fields.get("recipe"),"duration":item.get("duration"),"output_bytes":len(output.encode()),"formatted_output_bytes":len((item.get("formatted_output") or "").encode()),"output_head":output[:6000] if exit_code != 0 else "","output_tail":output[-12000:] if exit_code != 0 else ""}
                        commands.write(json.dumps(record) + "\n")
                        stats["native_completions"] += 1
                        if exit_code == 0:
                            stats["exit_zero"] += 1
                        elif exit_code is None:
                            stats["exit_unavailable"] += 1
                        else:
                            stats["exit_nonzero"] += 1
                        populated.add(meta["id"])
                        by_day[when.astimezone(dt.timezone(dt.timedelta(hours=-4))).strftime("%Y-%m-%d")] += 1
                        by_outcome[outcome] += 1
                        by_role[role] += 1
                        by_version[meta["cli_version"]] += 1
                        by_origin[meta["originator"]] += 1
                        by_family[fields["cls"]] += 1
            index.append({"id":meta["id"],"path":str(path),"role":meta["agent_role"],"parent":parent_id(meta["source"]),"cwd":meta["cwd"],"native_completions":stats["native_completions"]-before})
    summary = {"window_start":args.start,"cutoff":args.cutoff,"time_basis":"recorded completion event timestamp","excluded_threads":sorted(excluded),"populated_threads":len(populated),"stats":dict(stats),"by_local_day":dict(by_day),"outcome_heuristics":dict(by_outcome),"by_role":dict(by_role),"by_cli_version":dict(by_version),"by_originator":dict(by_origin),"by_command_class":dict(by_family),"mcp_completions":dict(by_mcp),"reused_extractor_sha256":hashlib.sha256((old/"extract.py").read_bytes()).hexdigest()}
    (args.out / "metrics.json").write_text(json.dumps(summary, indent=2)+"\n")
    (args.out / "index.json").write_text(json.dumps(index, indent=2)+"\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
