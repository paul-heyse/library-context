"""S4: compare in-process Pysa structs against the pyrefly 1.3.1 CLI's JSON reports.

Usage: parity.py TREE SITE INPROC_OUT CLI_OUT
Both sides have `module_id` stripped (dependency ids come from a parallel counter; names stay).
"""
import json, subprocess, sys
from pathlib import Path

tree, site, inproc, cli = map(lambda p: Path(p).resolve(), sys.argv[1:5])
cli.mkdir(parents=True, exist_ok=True)
toml = cli / "pyrefly.toml"
toml.write_text(
    f'project-includes = ["{tree}/**/*.py", "{tree}/**/*.pyi"]\n'
    f'search-path = ["{tree}"]\n'
    f'site-package-path = ["{site}"]\n'
    'python-version = "3.14.0"\n'
    'python-platform = "linux"\n'
    "skip-interpreter-query = true\n"
    "disable-search-path-heuristics = true\n"
    "disable-project-excludes-heuristics = true\n"
)
pysa = cli / "pysa"
cmd = ["uv", "run", "--project", "/home/paul/library-context", "pyrefly", "check", "-c", str(toml),
       "--report-pysa", str(pysa), "--report-pysa-format", "json", "--summary=none",
       "--output-format=min-text", "-j", "1"]
r = subprocess.run(cmd, capture_output=True, text=True, cwd="/")
print("cli exit", r.returncode, "stderr tail:", r.stderr.strip().splitlines()[-1:] if r.stderr else "")

def strip(v):
    if isinstance(v, dict):
        v.pop("module_id", None)
        for x in v.values(): strip(x)
    elif isinstance(v, list):
        for x in v: strip(x)
    return v

def canon(v):
    """Lists compared as multisets of canonical JSON, recursively."""
    if isinstance(v, dict):
        return {k: canon(x) for k, x in v.items()}
    if isinstance(v, list):
        return sorted((canon(x) for x in v), key=lambda x: json.dumps(x, sort_keys=True))
    return v

report = {}
for kind in ["definitions", "call_graphs"]:
    ours = {p.stem: json.loads(p.read_text()) for p in (inproc / "pysa" / kind).glob("*.json")}
    theirs = {}
    for p in (pysa / kind).glob("*.json"):
        name = p.name.rsplit(":", 1)[0]
        theirs[name] = strip(json.loads(p.read_text()))
    common = sorted(set(ours) & set(theirs))
    exact = sum(1 for m in common if ours[m] == theirs[m])
    as_sets = sum(1 for m in common if canon(ours[m]) == canon(theirs[m]))
    diffs = [m for m in common if canon(ours[m]) != canon(theirs[m])]
    report[kind] = {
        "inproc_modules": len(ours), "cli_modules": len(theirs),
        "only_inproc": sorted(set(ours) - set(theirs))[:10],
        "only_cli": sorted(set(theirs) - set(ours))[:10],
        "equal_exact": exact, "equal_as_sets": as_sets, "different": diffs[:20],
    }
print(json.dumps(report, indent=2))
(cli / "parity.json").write_text(json.dumps(report, indent=2))
