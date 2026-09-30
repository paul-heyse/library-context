"""Capture resolved features for the Phase 3 library-fit investigation; never build or edit pins."""

import json
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[4]
NAMES = {
    "datafusion", "datafusion-table-providers-postgres", "datafusion-table-providers-common",
    "arrow-array", "arrow-schema", "arrow-row", "arrow-select", "object_store",
    "sqlx", "sqlx-postgres", "tokio-postgres", "sea-query", "pgpq", "petgraph",
    "serde_arrow", "marrow", "biodivine-lib-bdd", "fixedbitset", "leiden-rs",
    "pyrefly", "ruff_python_ast", "ty_python_core", "salsa",
}
COMMAND = [
    "python3", "scripts/build_environment.py", "--", "cargo", "metadata",
    "--format-version", "1", "--locked", "--offline",
]
metadata = json.loads(subprocess.check_output(COMMAND, cwd=ROOT))
features = {node["id"]: node["features"] for node in metadata["resolve"]["nodes"]}
packages = sorted(
    (
        {
            "name": package["name"],
            "version": package["version"],
            "source": package["source"],
            "features": sorted(features[package["id"]]),
        }
        for package in metadata["packages"]
        if package["name"] in NAMES
    ),
    key=lambda package: (package["name"], package["version"]),
)
result = {
    "date": "2026-09-30",
    "command": COMMAND,
    "scope": "Workspace metadata resolution; not a compile or runtime qualification.",
    "packages": packages,
}
output = Path(__file__).with_name("raw") / "dependencies.json"
output.parent.mkdir(exist_ok=True)
output.write_text(json.dumps(result, indent=2) + "\n")
print(f"Captured {len(packages)} selected resolved packages.")
