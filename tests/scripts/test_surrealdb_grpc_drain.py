"""Exact patch/source and isolated native registry controls; no server or database."""

import json
import subprocess
import tomllib
from pathlib import Path

import pytest

import surrealdb_server as server
from storage_lifecycle import Blocked

EVIDENCE = Path(
    "/home/paul/.local/state/library-context/surrealdb.tools/server-build/"
    "3efaf277b46867beb1e75f6cae56eb30dfd40878bbb823ef953213cf1cd84680/source"
)


def command(argv, cwd):
    result = subprocess.run(argv, cwd=cwd, env=server.git_environment(), capture_output=True)
    assert result.returncode == 0, result.stderr.decode()
    return result.stdout


@pytest.fixture
def source(tmp_path):
    if not (EVIDENCE / ".git").exists():
        pytest.skip("exact pinned upstream source is unavailable; no acquisition implied")
    assert command(["git", "rev-parse", "HEAD"], EVIDENCE).decode().strip() == server.COMMIT
    selected = server.recipe(cpu_count=7)
    root = tmp_path / "source"
    root.mkdir()
    patch = selected["source_patches"][0]
    for row in patch["files"]:
        if row["original_sha256"] is None:
            continue
        content = command(["git", "show", server.COMMIT + ":" + row["path"]], EVIDENCE)
        path = root / row["path"]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        assert server.digest(path) == row["original_sha256"]
    for name in ("Cargo.lock", "Cargo.toml", selected["pin"]["patched_file"]):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(command(["git", "show", server.COMMIT + ":" + name], EVIDENCE))
    command(["git", "apply", "--check", str(server.PIN_ROOT / "timeout-tree-stack.patch")], root)
    command(["git", "apply", str(server.PIN_ROOT / "timeout-tree-stack.patch")], root)
    return selected, {"source": root}


def test_exact_successor_patch_applies_retries_and_adds_only_reviewed_lock_edge(source):
    selected, paths = source
    root = paths["source"]
    lock = (root / "Cargo.lock").read_bytes()
    executor = (root / selected["pin"]["patched_file"]).read_bytes()
    patch = selected["source_patches"][0]

    def run(argv, **kwargs):
        command(argv, kwargs["cwd"])

    server.prepare_source_patches(paths, selected, run=run)
    server.prepare_source_patches(
        paths, selected, run=lambda *a, **kw: pytest.fail("exact retry must not replay patch")
    )
    for row in patch["files"]:
        assert server.digest(root / row["path"]) == row["patched_sha256"]
    import hashlib

    assert hashlib.sha256(lock).hexdigest() == server.LOCK_SHA256
    expected = tomllib.loads(lock.decode())
    tokio_util = [row for row in expected["package"] if row["name"] == "tokio-util"]
    assert len(tokio_util) == 1 and tokio_util[0]["version"] == "0.7.18"
    assert "futures-util" not in tokio_util[0]["dependencies"]
    tokio_util[0]["dependencies"].append("futures-util")
    tokio_util[0]["dependencies"].sort()
    assert tomllib.loads((root / "Cargo.lock").read_text()) == expected
    assert server.digest(root / "Cargo.lock") == server.source_lock_sha256(selected)
    assert (root / selected["pin"]["patched_file"]).read_bytes() == executor
    assert (
        server.digest(root / selected["pin"]["patched_file"])
        == selected["pin"]["patched_file_sha256"]
    )
    status = "\n".join(
        [
            " M " + selected["pin"]["patched_file"],
            *(
                (" M " if row["original_sha256"] is not None else "?? ") + row["path"]
                for row in patch["files"]
            ),
        ]
    )

    def observed(argv, **kwargs):
        return server.COMMIT if argv[1] == "rev-parse" else status

    server.verify_source(paths, selected, run=observed)
    changed = root / "surrealdb/server/src/rpc/grpc.rs"
    changed.write_bytes(changed.read_bytes() + b"\n# unreviewed change\n")
    with pytest.raises(
        Blocked,
        match=r"server source differs from composed patch: surrealdb/server/src/rpc/grpc\.rs",
    ):
        server.verify_source(paths, selected, run=observed)
    with pytest.raises(Blocked, match="conflicts"):
        server.prepare_source_patches(paths, selected, run=run)


def test_exact_predecessor_is_valid_for_live_installed_consumer_and_source_is_separate(tmp_path):
    old = server.recipe(cpu_count=7, grpc_drain=False)
    successor = server.recipe(cpu_count=7)
    assert successor["dependency_patches"] == old["dependency_patches"]
    assert successor["pin"]["patch_sha256"] == old["pin"]["patch_sha256"]
    before = server.cache_paths(tmp_path, old)
    after = server.cache_paths(tmp_path, successor)
    assert before["cache"] == after["cache"] and before["target"] == after["target"]
    assert before["source"] != after["source"] and after["source"] == after["root"] / "source"
    for selected in (old, successor):
        receipt = server.build_provenance(
            tmp_path,
            selected,
            "rustc 1.99.0-nightly\ncommit-date: 2026-09-28",
            "cargo 1.99.0-nightly",
            cache_record_sha256="a" * 64,
        )
        server.validate_provenance(receipt, directory=tmp_path)
        assert receipt["lock_sha256"] == server.source_lock_sha256(selected)
        if selected.get("source_patches"):
            assert receipt["upstream_lock_sha256"] == server.LOCK_SHA256
            wrong_lock = {**receipt, "lock_sha256": server.LOCK_SHA256}
            with pytest.raises(Blocked, match="provenance"):
                server.validate_provenance(wrong_lock, directory=tmp_path)
            wrong_upstream = {**receipt, "upstream_lock_sha256": "b" * 64}
            with pytest.raises(Blocked, match="provenance"):
                server.validate_provenance(wrong_upstream, directory=tmp_path)
        receipt["recipe"]["pin"]["http_version"] = "unreviewed-generation"
        with pytest.raises(Blocked, match="reviewed pins"):
            server.validate_provenance(receipt, directory=tmp_path)


def test_actual_native_registry_waits_for_owned_finalizers_and_records_lost_finality(
    source, tmp_path
):
    selected, paths = source
    server.prepare_source_patches(
        paths, selected, run=lambda argv, **kwargs: command(argv, kwargs["cwd"])
    )
    project = tmp_path / "registry-control"
    (project / "src").mkdir(parents=True)
    module = paths["source"] / "surrealdb/server/src/rpc/query_drain.rs"
    # Include both real patched modules; only database/WebSocket sweeps are
    # substituted in this isolated lifecycle control (not server acceptance).
    harness = Path(__file__).with_name("surrealdb_grpc_drain_control.rs").read_text()
    signals = paths["source"] / "surrealdb/server/src/ntw/signals.rs"
    (project / "src/lib.rs").write_text(
        harness.replace('"QUERY_DRAIN_PATH"', json.dumps(str(module))).replace(
            '"SIGNALS_PATH"', json.dumps(str(signals))
        )
    )
    (project / "Cargo.toml").write_text(
        '[package]\nname="lctx-grpc-registry-control"\nversion="0.0.0"\nedition="2024"\n'
        "[workspace]\n[dependencies]\n"
        'tokio={version="=1.52.3",features=["rt","macros","sync","signal","time"]}\n'
        'tokio-util={version="=0.7.18",features=["rt"]}\n'
        'uuid={version="=1.23.1",features=["v4"]}\nfutures="=0.3.32"\n'
        'axum-server="=0.8.0"\nanyhow="=1.0.104"\ntracing="=0.1.44"\n'
    )
    (project / "Cargo.lock").write_bytes((paths["source"] / "Cargo.lock").read_bytes())
    # Compile the actual patched registry and its native tests, not a test-only copy.
    result = subprocess.run(
        [
            "rustup",
            "run",
            server.TOOLCHAIN,
            "cargo",
            "test",
            "--offline",
            "--release",
            "--manifest-path",
            str(project / "Cargo.toml"),
            "--lib",
        ],
        cwd=project,
        env={**server.git_environment(), "CARGO_TARGET_DIR": str(project / "target")},
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "6 passed" in result.stdout
    locked = {
        (row["name"], row["version"])
        for row in tomllib.loads((paths["source"] / "Cargo.lock").read_text())["package"]
    }
    assert all(
        (row["name"], row["version"]) in locked
        for row in tomllib.loads((project / "Cargo.lock").read_text())["package"]
        if row["name"] != "lctx-grpc-registry-control"
    )
