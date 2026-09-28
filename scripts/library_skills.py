"""Run the shared library-skill selector for this repository."""

import os
import runpy
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]
store = Path(os.environ.get("LIBRARY_SKILLS_ROOT", "~/.local/share/library-skills")).expanduser()
manager = store / "manage.py"
if not manager.is_file():
    raise SystemExit(
        f"Shared library skills are not installed at {store}; set LIBRARY_SKILLS_ROOT if moved."
    )
sys.argv[1:1] = ["--repo", str(root)]
runpy.run_path(str(manager), run_name="__main__")
