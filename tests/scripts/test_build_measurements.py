from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

import build_measurements as bm


def git(root: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)


def test_snapshot_keeps_dirty_and_untracked_bytes_without_build_output(tmp_path: Path) -> None:
    git(tmp_path, "init", "--quiet")
    (tmp_path / ".gitignore").write_text("/build/\n")
    source = tmp_path / "src/main.rs"
    source.parent.mkdir()
    source.write_text("fn main() {}\n")
    git(tmp_path, "add", ".gitignore", "src/main.rs")
    subprocess.run(
        [
            "git",
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-qm",
            "init",
        ],
        cwd=tmp_path,
        check=True,
    )
    source.write_text('fn main() { println!("dirty"); }\n')
    (tmp_path / ".cargo").mkdir()
    (tmp_path / ".cargo/config.toml").write_text('linker = "clang"\n')
    (tmp_path / "build").mkdir()
    (tmp_path / "build/ignored").write_text("never copied")
    before = bm.inventory(tmp_path)
    campaign = tmp_path / "build/perf/campaign"

    manifest = bm.snapshot(tmp_path, campaign)

    assert manifest["source_sha256"] == bm.source_digest(before)
    assert (campaign / "source/src/main.rs").read_text() == source.read_text()
    assert (campaign / "source/.cargo/config.toml").exists()
    assert not (campaign / "source/build").exists()
    assert bm.inventory(tmp_path) == before
    with pytest.raises(ValueError, match="already exists"):
        bm.snapshot(tmp_path, campaign)


def test_variants_preserve_mold_and_isolate_cache(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("RUSTFLAGS", "-C opt-level=3")
    monkeypatch.setenv("RUSTC_WRAPPER", "/wrong/wrapper")
    monkeypatch.setattr(bm.shutil, "which", lambda name: "/usr/bin/sccache")

    stable = bm.variant_env(tmp_path, "stable", 1)
    nightly = bm.variant_env(tmp_path, "nightly-8x4", 2)

    assert stable["CARGO_ENCODED_RUSTFLAGS"] == "-C\x1flink-arg=-fuse-ld=mold"
    assert stable["RUSTC_WRAPPER"] == ""
    assert stable["CARGO_INCREMENTAL"] == "0"
    assert "RUSTFLAGS" not in nightly
    assert nightly["CARGO_ENCODED_RUSTFLAGS"].endswith("\x1f-Zthreads=4")
    assert nightly["CARGO_BUILD_JOBS"] == "8"
    assert nightly["SCCACHE_DIR"] == str(tmp_path / "cache/nightly-8x4")
    assert nightly["RUSTC_WRAPPER"] == "/usr/bin/sccache"
    assert nightly["CARGO_TARGET_DIR"].endswith("nightly-8x4/trial-2")


def test_cargo_artifact_counts_and_cache_counter_deltas(tmp_path: Path) -> None:
    events = [
        {"reason": "compiler-artifact", "fresh": True},
        {"reason": "compiler-artifact", "fresh": False},
        {"reason": "build-finished", "success": True},
    ]
    path = tmp_path / "cargo.jsonl"
    path.write_text("\n".join(json.dumps(event) for event in events) + "\n")

    assert bm.cargo_artifacts(path) == {"fresh_artifacts": 1, "rebuilt_artifacts": 1}
    assert bm.counter_delta(
        {"hits": 2, "nested": {"misses": 1}}, {"hits": 5, "nested": {"misses": 3}}
    ) == {
        "hits": 3,
        "nested": {"misses": 2},
    }
    path.write_text(json.dumps({"reason": "compiler-artifact", "fresh": False}))
    with pytest.raises(ValueError, match="build-finished"):
        bm.cargo_artifacts(path)


def test_sample_records_a_process_without_running_cargo(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(bm, "competing_builds", lambda: [])
    output = tmp_path / "result"
    target = tmp_path / "target"
    command = [
        sys.executable,
        "-c",
        "import json; print(json.dumps({'reason':'compiler-artifact','fresh':False})); "
        "print(json.dumps({'reason':'build-finished','success':True}))",
    ]

    result = bm.sample(tmp_path, output, target, os.environ.copy(), "tiny", command)

    assert result["exit_code"] == 0
    assert result["rebuilt_artifacts"] == 1
    assert result["wall_seconds"] > 0
    assert json.loads((output / "result.json").read_text()) == result
    with pytest.raises(ValueError, match="already exists"):
        bm.sample(tmp_path, output, target, os.environ.copy(), "tiny", command)


def test_report_keeps_decision_open(tmp_path: Path) -> None:
    bm.write_json(tmp_path / "snapshot.json", {"source_sha256": "abc"})
    bm.write_json(
        tmp_path / "results/stable/trial-1/summary.json",
        {"cold-tests": {"wall_seconds": 10.0, "exit_code": 0}},
    )
    bm.write_json(
        tmp_path / "results/stable/trial-2/summary.json",
        {"cold-tests": {"wall_seconds": 12.0, "exit_code": 0}},
    )

    report = bm.summarize(tmp_path)

    assert report["measurements"]["stable"]["cold-tests"]["median_seconds"] == 11.0
    assert report["measurements"]["stable"]["cold-tests"]["count"] == 2
    assert report["decision"].startswith("not_run")
