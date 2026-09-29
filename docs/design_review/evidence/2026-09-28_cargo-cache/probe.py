"""Compare Rust cache reuse across Cargo roots without touching existing targets or servers."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[4]
OUTPUT = Path(__file__).parent / "raw" / "probe.json"
MANIFEST = '''[package]
name = "lctx_cache_probe"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
itoa = "=1.0.18"
[profile.dev]
opt-level = 2
incremental = true
debug = 0
[profile.dev.package."*"]
opt-level = 3
incremental = false
'''


def call(args, cwd, env):
    return subprocess.run(args, cwd=cwd, env=env, text=True, capture_output=True, check=True)


def counts(stats):
    result = {}
    for field in ("cache_hits", "cache_misses"):
        per_language = stats["stats"][field]
        result[field] = per_language.get("counts", {}).get("Rust", 0)
    return result


def main():
    base = dict(os.environ)
    for key in ("CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_BUILD_DIR",
                "SCCACHE_BASEDIRS", "SCCACHE_SERVER_PORT"):
        base.pop(key, None)
    base["RUSTC_WRAPPER"] = shutil.which("sccache")
    records = []
    with tempfile.TemporaryDirectory(prefix="lctx-cache-probe-", dir=ROOT / "build") as temporary:
        folder = Path(temporary)
        for mode in ("exported", "unset", "cargo_config"):
            work = folder / mode
            cache_env = {**base, "SCCACHE_DIR": str(work / "cache"),
                         "SCCACHE_CACHE_SIZE": "64M", "SCCACHE_IDLE_TIMEOUT": "0",
                         "SCCACHE_SERVER_UDS": f"/tmp/lctx-cache-probe-{os.getpid()}-{mode}.sock"}
            work.mkdir()
            call(["sccache", "--start-server"], ROOT, cache_env)
            try:
                for project in ("a", "b"):
                    cwd = work / project
                    (cwd / "src").mkdir(parents=True)
                    (cwd / "Cargo.toml").write_text(MANIFEST)
                    (cwd / "src/main.rs").write_text('fn main() { println!("{}", itoa::Buffer::new().format(42)); }\n')
                    env = dict(cache_env)
                    cmd = ["cargo"]
                    if mode == "exported":
                        env["CARGO_TARGET_DIR"] = str(cwd / "target")
                    elif mode == "cargo_config":
                        cmd += ["--config", f'build.target-dir="{cwd / "chosen-target"}"']
                    cmd += ["build", "--offline", "--quiet", "--jobs", "1"]
                    before = json.loads(call(["sccache", "--show-stats", "--stats-format=json"], ROOT, cache_env).stdout)
                    call(cmd, cwd, env)
                    after = json.loads(call(["sccache", "--show-stats", "--stats-format=json"], ROOT, cache_env).stdout)
                    a, b = counts(before), counts(after)
                    artifact, = cwd.glob("*/debug/deps/libitoa-*.rlib")
                    records.append({"mode": mode, "project": project,
                                    "counts": {key: b[key] - a[key] for key in a},
                                    "artifact": artifact.name,
                                    "sha256": hashlib.sha256(artifact.read_bytes()).hexdigest(),
                                    "stats": after})
                    print(json.dumps({key: value for key, value in records[-1].items() if key != "stats"}), flush=True)
            finally:
                call(["sccache", "--stop-server"], ROOT, cache_env)
    result = {"outcome": "passed", "sccache": call(["sccache", "--version"], ROOT, base).stdout.strip(),
              "rustc": call(["rustc", "-Vv"], ROOT, base).stdout.strip(), "records": records}
    second = {r["mode"]: r for r in records if r["project"] == "b"}
    assert second["exported"]["counts"] == {"cache_hits": 0, "cache_misses": 1}
    for mode in ("unset", "cargo_config"):
        assert second[mode]["counts"] == {"cache_hits": 1, "cache_misses": 0}
    OUTPUT.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
