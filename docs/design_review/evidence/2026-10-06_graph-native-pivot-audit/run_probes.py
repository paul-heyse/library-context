#!/usr/bin/env python3
"""Compile diagnostic probes against current release libraries; use only owned scratch/state."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "scripts"))
from build_environment import normalized_env  # noqa: E402
from surrealdb_fixture import fixture  # noqa: E402


def main() -> None:
    env = normalized_env(dict(os.environ))
    env.update(UV_NO_SYNC="1", INSTA_UPDATE="no")
    with tempfile.TemporaryDirectory(prefix="lctx-graph-audit-") as temporary:
        scratch = Path(temporary)
        metadata = scratch / "build.jsonl"
        with metadata.open("w") as output:
            subprocess.run(
                [
                    "cargo",
                    "build",
                    "--release",
                    "--locked",
                    "-p",
                    "lctx-publisher",
                    "-p",
                    "lctx-serving",
                    "--message-format=json",
                ],
                cwd=ROOT,
                env=env,
                stdout=output,
                check=True,
            )
        libraries = {}
        link_libraries = {}
        directories = set()
        native_paths = set()
        for line in metadata.read_text().splitlines():
            record = json.loads(line)
            if record.get("reason") == "compiler-artifact":
                for name in record["filenames"]:
                    path = Path(name)
                    directories.add(path.parent)
                    if path.suffix == ".rlib":
                        link_libraries[record["target"]["name"]] = path
                    if path.suffix == ".rmeta":
                        libraries[record["target"]["name"]] = path
            elif record.get("reason") == "build-script-executed":
                native_paths.update(record["linked_paths"])
        dependencies = [
            "cpg_core",
            "cpg_extract",
            "datafusion",
            "lctx_model",
            "lctx_publisher",
            "lctx_serving",
            "lctx_surrealdb",
            "serde_json",
            "tempfile",
            "tokio",
        ]
        compile_env = env | {"CARGO_MANIFEST_DIR": str(ROOT / "crates/cpg-core")}
        binaries = {}
        for probe in ("selection_failure", "cold_audit_search"):
            binary = scratch / probe
            command = [
                "rustc",
                "--edition=2024",
                "-C",
                "opt-level=2",
                "-C",
                "linker=clang",
                "-C",
                "link-arg=-fuse-ld=mold",
            ]
            for directory in sorted(directories):
                command += ["-L", "dependency=" + str(directory)]
            for path in sorted(native_paths):
                command += ["-L", path]
            for dependency in dependencies:
                command += [
                    "--extern",
                    dependency + "=" + str(libraries[dependency]),
                    "--extern",
                    dependency + "=" + str(link_libraries[dependency]),
                ]
            command += [str(HERE / (probe + ".rs")), "-o", str(binary)]
            result = subprocess.run(command, cwd=ROOT, env=compile_env, check=False)
            if result.returncode:
                raise RuntimeError(f"{probe}: rustc failed ({result.returncode})")
            binaries[probe] = binary
        subprocess.run([str(binaries["selection_failure"])], cwd=ROOT, env=env, check=True)
        with fixture() as owned:
            subprocess.run(
                [str(binaries["cold_audit_search"])],
                cwd=ROOT,
                env=owned.environment(env),
                check=True,
            )


if __name__ == "__main__":
    main()
