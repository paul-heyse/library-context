"""Bounded reuse probe; only its temporary directories are removed."""

import concurrent.futures
import json
import os
import subprocess
import tempfile
import tomllib
from pathlib import Path

root = Path(__file__).resolve().parents[4]
config = (root / ".cargo/config.toml").read_text()
env = dict(os.environ)
env["RUSTUP_TOOLCHAIN"] = tomllib.loads((root / "rust-toolchain.toml").read_text())["toolchain"][
    "channel"
]
for k in (
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_BUILD_BUILD_DIR",
    "LCTX_CARGO_TARGET_DIR",
):
    env.pop(k, None)
with tempfile.TemporaryDirectory(prefix="lctx-shared-probe-") as tmp:
    base = Path(tmp)
    shared = base / "intermediates"
    config = config.replace("{cargo-cache-home}/build/library-context", str(shared))
    paths = []
    for n in ["one", "two"]:
        path = base / n
        (path / "src").mkdir(parents=True)
        (path / ".cargo").mkdir()
        (path / "Cargo.toml").write_text(
            '[package]\nname="lctx-cache-probe"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nitoa="=1.0.18"\n[profile.release.package."*"]\nopt-level=3\nincremental=false\n'
        )
        (path / "src/main.rs").write_text(
            'fn main() { println!("{}", itoa::Buffer::new().format(7)); }\n'
        )
        (path / ".cargo/config.toml").write_text(config)
        paths.append(path)

    def run(path):
        r = subprocess.run(
            ["cargo", "build", "--release", "--offline", "--message-format=json"],
            cwd=path,
            env=env,
            capture_output=True,
            text=True,
            check=True,
        )
        rows = [json.loads(s) for s in r.stdout.splitlines()]
        artifact = next(
            r
            for r in rows
            if r.get("reason") == "compiler-artifact" and r["target"]["name"] == "itoa"
        )
        return {
            "checkout": path.name,
            "itoa_fresh": artifact["fresh"],
            "itoa_artifact": artifact["filenames"][0],
            "output": subprocess.check_output(
                [str(path / "target/release/lctx-cache-probe")], text=True
            ).strip(),
        }

    first = run(paths[0])
    second = run(paths[1])
    assert second["itoa_fresh"] and first["itoa_artifact"] == second["itoa_artifact"]
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        parallel = list(pool.map(run, paths))
    assert all(r["itoa_fresh"] and r["output"] == "7" for r in parallel)
    print(json.dumps({"serial": [first, second], "concurrent": parallel}, indent=2))
