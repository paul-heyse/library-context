"""Current recipes (`just --dump`) and every recipe name the justfile ever defined (git history)."""
import json
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import config  # noqa: E402

RECIPE_LINE = re.compile(r"^@?([a-z_][a-z0-9_-]*)(?:\s+[^:=]*)?:(?!=)", re.M)


def main():
    dump = json.loads(subprocess.run(["just", "--dump", "--dump-format", "json"], cwd=config.REPO,
                                     capture_output=True, text=True, check=True).stdout)
    now = sorted(dump["recipes"])
    revs = subprocess.run(["git", "log", "--format=%H", "--", "justfile"], cwd=config.REPO,
                          capture_output=True, text=True, check=True).stdout.split()
    ever = set(now)
    for rev in revs:
        txt = subprocess.run(["git", "show", "%s:justfile" % rev], cwd=config.REPO, capture_output=True, text=True).stdout
        ever |= set(m.group(1) for m in RECIPE_LINE.finditer(txt) if m.group(1) not in ("set", "import", "mod", "alias", "export"))
    os.makedirs(config.OUTDIR, exist_ok=True)
    json.dump({"now": now, "ever": sorted(ever), "justfile_revisions": len(revs)},
              open(os.path.join(config.OUTDIR, "recipes.json"), "w"), indent=1)
    print("now", len(now), "ever", len(ever), "revisions", len(revs))


if __name__ == "__main__":
    main()
