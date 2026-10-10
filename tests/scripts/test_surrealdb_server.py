"""Pure owned-server tooling controls; no compilation, network or database effects."""

import contextlib
import json
import os
import subprocess
from pathlib import Path

import pytest

import surrealdb_server as server
import storage_service as assets
from storage_lifecycle import Blocked, Storage


@pytest.fixture(autouse=True)
def isolated_storage(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "storage"))
    monkeypatch.setenv("LCTX_STORAGE_CONFIG", str(tmp_path / "no-host-config"))


def provenance(directory):
    selected = server.recipe(cpu_count=7)
    return server.build_provenance(directory, selected,
        "rustc 1.99.0-nightly\ncommit-date: 2026-09-28\nhost: x86_64-unknown-linux-gnu",
        "cargo 1.99.0-nightly", cache_record_sha256="a"*64)


def executable(tmp_path):
    path = tmp_path / "surreal"
    path.write_text("#!/bin/sh\nprintf '3.3.0+lctx.timeout-stack.1 for linux on x86_64\\n'\n")
    path.chmod(0o700)
    return path


def test_recipe_keeps_full_shipped_features_exact_pins_and_available_codegen(monkeypatch):
    monkeypatch.setattr(os, "process_cpu_count", lambda: 73)
    selected = server.recipe()
    assert selected["pin"]["upstream_commit"] == "238bfeb11f5725bebed370167656748df8067595"
    assert selected["pin"]["features"] == ["default", "cjk"]
    assert selected["pin"]["toolchain"] == "nightly-2026-09-29"
    assert selected["build"]["codegen_units"] == 73
    assert selected["build"]["lto"] == "thin"
    assert selected["build"]["opt_level"] == 3
    assert selected["build"]["panic"] == "abort"
    assert selected["build"]["rustflags"][2:4] == ["--jobs-frontend=0", "--jobs-backend=0"]
    patch = (server.PIN_ROOT / selected["pin"]["patch_file"]).read_text()
    assert patch.count("+\t\t\t\t\t\tself.stack = TreeStack::new();") == 2
    assert "@@ -1446" in patch and "@@ -1580" in patch
    assert server.digest(server.PIN_ROOT / selected["pin"]["patch_file"]) == selected["pin"]["patch_sha256"]


def test_recipe_refuses_modified_patch(tmp_path, monkeypatch):
    selected = server.recipe()
    (tmp_path / "provenance.json").write_text(json.dumps(selected["pin"]))
    (tmp_path / "timeout-tree-stack.patch").write_text("different")
    monkeypatch.setattr(server, "PIN_ROOT", tmp_path)
    with pytest.raises(Blocked, match="patch identity"):
        server.recipe()


def test_build_environment_reuses_tools_without_foreign_flags_targets_or_caps(tmp_path, monkeypatch):
    monkeypatch.setenv("CARGO_BUILD_JOBS", "1")
    monkeypatch.setenv("CARGO_ENCODED_RUSTFLAGS", "foreign")
    monkeypatch.setenv("CARGO_TARGET_DIR", "/other-checkout/target")
    monkeypatch.setenv("CARGO_PROFILE_RELEASE_LTO", "fat")
    monkeypatch.setenv("CARGO_INCREMENTAL", "1")
    monkeypatch.setenv("MAKEFLAGS", "-j1")
    monkeypatch.setenv("CARGO_MAKEFLAGS", "--jobserver-auth=99,100")
    selected = server.recipe(cpu_count=45)
    paths = server.cache_paths(tmp_path / "service", selected)
    env = server.build_environment(selected, paths)
    assert "CARGO_BUILD_JOBS" not in env
    assert "CARGO_ENCODED_RUSTFLAGS" not in env
    assert "CARGO_INCREMENTAL" not in env
    assert "MAKEFLAGS" not in env and "CARGO_MAKEFLAGS" not in env
    assert env["CARGO_TARGET_DIR"] == str(paths["target"])
    assert env["CARGO_PROFILE_RELEASE_CODEGEN_UNITS"] == "45"
    assert env["CARGO_PROFILE_RELEASE_LTO"] == "thin"
    assert env["RUSTC_WRAPPER"] == "sccache"
    assert env["CC"] == "clang" and env["CXX"] == "clang++"
    assert env["SURREAL_BUILD_METADATA"] == "lctx.timeout-stack.1"
    assert "-fuse-ld=mold" in env["RUSTFLAGS"]
    assert "CARGO_BUILD_JOBS" in os.environ  # No global environment mutation.


def test_server_generation_retains_owned_bytes_provenance_and_warm_dependencies(tmp_path):
    directory = tmp_path / "service"
    source = executable(tmp_path)
    row = assets.prepare_server(directory, source, provenance(directory))
    source.unlink()
    assert server.validate_descriptor(Path(row["descriptor_path"])) == row
    assert row["path"] == str(assets.tools_root(directory) / "surreal" / row["identity"] / "surreal")
    assert Path(row["path"]).stat().st_mode & 0o777 == 0o500
    assert Path(row["descriptor_path"]).stat().st_mode & 0o777 == 0o400
    assert {r["role"] for r in server.dependencies(row)} == {
        "server-executable-generation", "server-generation-provenance", "server-source-warm-cache",
        "server-build-warm-cache", "server-borrowed-build-warm-cache", "server-original-cache-provenance",
        "server-dependency-source-override", "server-dependency-original-archive"}
    records = Storage().records()
    assert len(records) == 1
    assert records[0]["owner"]["kind"] == "native"
    assert records[0]["category"] == "native-content"


def test_server_generation_refuses_corruption_and_provenance_relabeling(tmp_path):
    directory = tmp_path / "service"
    source = executable(tmp_path)
    row = assets.prepare_server(directory, source, provenance(directory))
    assert assets.prepare_server(directory, source, provenance(directory)) == row
    marker = Path(row["descriptor_path"])
    changed = json.loads(marker.read_text())
    changed["provenance"]["lock_sha256"] = "0" * 64
    marker.chmod(0o600)
    marker.write_text(json.dumps(changed))
    with pytest.raises(Blocked, match="provenance"):
        server.validate_descriptor(marker)
    with pytest.raises(Blocked, match="provenance.*conflicts"):
        assets.prepare_server(directory, source, provenance(directory))


def test_server_generation_refuses_foreign_path_symlinks_and_wrong_version(tmp_path):
    directory = tmp_path / "service"
    source = executable(tmp_path)
    row = assets.prepare_server(directory, source, provenance(directory))
    wrong = lambda *a, **kw: subprocess.CompletedProcess(a, 0, stdout="other-version")
    with pytest.raises(Blocked, match="version"):
        server.validate_descriptor(Path(row["descriptor_path"]), runner=wrong)
    link = tmp_path / "symlink"
    link.symlink_to(source)
    with pytest.raises(Blocked, match="physical"):
        assets.prepare_server(directory, link, provenance(directory))
    other = tmp_path / "record.json"
    other.write_text(json.dumps(row))
    other.chmod(0o400)
    with pytest.raises(Blocked, match="identity differs"):
        server.validate_descriptor(other)


def test_source_validation_refuses_other_changes_and_modified_lock(tmp_path, monkeypatch):
    selected = server.recipe()
    paths = server.cache_paths(tmp_path / "service", selected)
    calls = []
    def run(argv, **kw):
        calls.append(argv)
        return server.COMMIT if argv[1] == "rev-parse" else " M " + selected["pin"]["patched_file"]
    hashes = {"Cargo.lock": server.LOCK_SHA256, "Cargo.toml": selected["pin"]["upstream_manifest_sha256"],
              "executor.rs": selected["pin"]["patched_file_sha256"]}
    monkeypatch.setattr(server, "digest", lambda p: hashes[p.name])
    server.verify_source(paths, selected, run=run)
    hashes["Cargo.lock"] = "changed"
    with pytest.raises(Blocked, match="Cargo.lock"):
        server.verify_source(paths, selected, run=run)
    hashes["Cargo.lock"] = server.LOCK_SHA256
    with pytest.raises(Blocked, match="beyond"):
        server.verify_source(paths, selected, run=lambda argv, **kw: server.COMMIT if argv[1] == "rev-parse" else " M other")


def test_build_admits_before_creation_persists_warm_cache_and_uses_locked_full_recipe(tmp_path, monkeypatch):
    directory = tmp_path / "service"
    selected = server.recipe()
    initial = server.cache_paths(directory, selected)
    for path in (assets.tools_root(directory), initial["cache"].parent, initial["cache"]):
        server.private_directory(path)
    old_record = {"schema": 1, "recipe": server.original_recipe(selected), "phase": "building",
                  "evidence": "original failed build"}
    server.durable_json(initial["cache_record"], old_record)
    original_record = initial["cache_record"].read_bytes()
    initial["target"].mkdir(mode=0o700)
    (initial["target"] / "warm-product").write_bytes(b"retained compiled bytes")
    active = []
    real_admission = server.admission
    @contextlib.contextmanager
    def admitted(paths, **kw):
        assert not server.cache_paths(directory, server.recipe())["root"].exists()
        assert set(kw["exclusive_paths"]) == {initial["root"], initial["cache"]}
        with real_admission(paths, **kw):
            active.append(True)
            try:
                yield
            finally:
                active.pop()
    monkeypatch.setattr(server, "admission", admitted)
    monkeypatch.setattr(server, "verify_source", lambda *a, **kw: None)
    monkeypatch.setattr(server, "prepare_dependency", lambda paths, *a, **kw:
                        paths["dependency"].mkdir(parents=True))
    monkeypatch.setattr(server, "verify_dependency_source", lambda *a, **kw: None)
    pin = server.recipe()["pin"]
    real_digest = server.digest
    monkeypatch.setattr(server, "digest", lambda p: pin["original_file_sha256"] if p.name == "executor.rs"
                        else server.LOCK_SHA256 if p.name == "Cargo.lock" else real_digest(p))
    calls = []
    def run(argv, *, cwd, env, capture=False):
        assert active
        calls.append((argv, env))
        if argv[:2] == ["git", "init"]:
            (cwd / ".git").mkdir()
        if "checkout" in argv:
            p = cwd / pin["patched_file"]
            p.parent.mkdir(parents=True)
            p.write_text("fake upstream bytes")
            (cwd / "Cargo.lock").write_text("fake lock")
        if "-vV" in argv:
            return "rustc 1.99.0-nightly\ncommit-date: 2026-09-28"
        if "--version" in argv:
            return "cargo 1.99.0-nightly"
        if "metadata" in argv:
            return json.dumps({"packages": [{"name": "diskann", "version": "0.56.0", "manifest_path":
                str(server.cache_paths(directory, server.recipe())["dependency"] / "Cargo.toml")}]})
        if "build" in argv:
            selected = server.recipe()
            target = server.cache_paths(directory, selected)["target"] / pin["platform"] / "release"
            target.mkdir(parents=True)
            executable(target)
        if argv[-1] == "version":
            return pin["cli_version"]
        return ""
    row = server.build(directory, run=run)
    command, env = next(c for c in calls if "build" in c[0])
    assert "--locked" in command and "--features" in command
    assert command[command.index("--features")+1] == "cjk"
    assert "--no-default-features" not in command
    assert command[:3] == ["rustup", "run", "nightly-2026-09-29"]
    assert "CARGO_BUILD_JOBS" not in env
    record = server.read_json(server.cache_paths(directory, server.recipe())["record"])
    assert record["phase"] == "built"
    assert row["descriptor_path"] == record["descriptor_path"]
    assert initial["cache_record"].read_bytes() == original_record
    assert (initial["target"] / "warm-product").read_bytes() == b"retained compiled bytes"
    assert row["provenance"]["cache_lineage"]["record_sha256"] == assets.digest(initial["cache_record"])
    assert {r["category"] for r in Storage().records()} == {"native-cache", "native-content"}
    monkeypatch.setattr(server, "admission", real_admission)
    assert server.build(directory, run=lambda *a, **kw: pytest.fail("retained generation must not rebuild")) == row


def test_owned_tool_process_capture_and_failure_preserve_admitted_cache(tmp_path):
    import sys

    with server.admission([tmp_path]):
        assert server.execute([sys.executable, "-c", "print('exact-output')"],
                              cwd=tmp_path, env=dict(os.environ), capture=True) == "exact-output"
        with pytest.raises(RuntimeError, match="failed \\(3\\)"):
            server.execute([sys.executable, "-c", "raise SystemExit(3)"],
                           cwd=tmp_path, env=dict(os.environ))
    assert tmp_path.is_dir()


def test_failed_acquisition_retains_owner_recipe_and_cache_for_explicit_retry(tmp_path):
    directory = tmp_path / "service"
    def fail(argv, **kw):
        raise RuntimeError("acquisition unavailable")
    with pytest.raises(RuntimeError, match="unavailable"):
        server.build(directory, run=fail)
    paths = server.cache_paths(directory, server.recipe())
    assert paths["root"].is_dir()
    assert server.read_json(paths["record"])["phase"] == "acquisition"
    assert any(r["path"] == str(paths["root"]) and r["category"] == "native-cache"
               for r in Storage().records())


def test_default_descriptor_version_child_uses_shared_owner_under_admission(tmp_path, monkeypatch):
    directory = tmp_path / "service"
    row = assets.prepare_server(directory, executable(tmp_path), provenance(directory))
    calls = []
    def owned(argv, *, cwd, env, capture):
        admitted = json.loads(os.environ["LCTX_STORAGE_ADMISSION"])
        assert any(item["path"] == str(Path(row["path"]).parent) for item in admitted)
        assert capture and cwd == Path(row["path"]).parent
        calls.append(argv)
        return row["version"]
    monkeypatch.setattr(server, "execute", owned)
    assert server.validate_descriptor(Path(row["descriptor_path"])) == row
    assert server.validate_descriptor(Path(row["descriptor_path"]), runner=subprocess.run) == row
    assert calls == [[row["path"], "version"], [row["path"], "version"]]


def test_matching_interrupted_descriptor_mode_is_repaired_and_synced(tmp_path, monkeypatch):
    directory = tmp_path / "service"
    source = executable(tmp_path)
    row = assets.prepare_server(directory, source, provenance(directory))
    marker = Path(row["descriptor_path"])
    marker.chmod(0o600)  # Crash immediately after durable_json, before immutable-mode publication.
    calls = []
    real_fchmod, real_fsync = os.fchmod, os.fsync
    def chmod(fd, mode):
        if Path(os.readlink(f"/proc/self/fd/{fd}")) == marker:
            calls.append(("chmod", mode))
        return real_fchmod(fd, mode)
    def fsync(fd):
        if Path(os.readlink(f"/proc/self/fd/{fd}")) == marker:
            calls.append(("fsync", None))
        return real_fsync(fd)
    monkeypatch.setattr(os, "fchmod", chmod)
    monkeypatch.setattr(os, "fsync", fsync)
    assert assets.prepare_server(directory, source, provenance(directory)) == row
    assert calls[:2] == [("chmod", 0o400), ("fsync", None)]
    assert server.validate_descriptor(marker) == row


def test_generation_never_chmods_a_symlinked_or_foreign_owner_marker(tmp_path, monkeypatch):
    from types import SimpleNamespace

    directory = tmp_path / "service"
    source = executable(tmp_path)
    row = assets.prepare_server(directory, source, provenance(directory))
    marker = Path(row["descriptor_path"])
    foreign = tmp_path / "foreign-record.json"
    foreign.write_bytes(marker.read_bytes())
    foreign.chmod(0o600)
    marker.unlink()
    marker.symlink_to(foreign)
    with pytest.raises(Blocked, match="owned regular"):
        assets.prepare_server(directory, source, provenance(directory))
    assert foreign.stat().st_mode & 0o777 == 0o600
    marker.unlink()
    marker.write_bytes(foreign.read_bytes())
    marker.chmod(0o600)
    real_fstat = os.fstat
    def fstat(fd):
        row = real_fstat(fd)
        if Path(os.readlink(f"/proc/self/fd/{fd}")) == marker:
            return SimpleNamespace(st_mode=row.st_mode, st_uid=os.getuid()+1)
        return row
    monkeypatch.setattr(os, "fstat", fstat)
    with pytest.raises(Blocked, match="owned regular"):
        assets.prepare_server(directory, source, provenance(directory))
    assert marker.stat().st_mode & 0o777 == 0o600


def dependency_fixture(tmp_path):
    import copy
    import hashlib
    import io
    import tarfile

    selected = copy.deepcopy(server.recipe(cpu_count=7))
    pin = selected["dependency_patches"][0]
    original = ("                    // The task assigned to each round of `set_element`.\n"
                "                    let future = async move {\n"
                "                        self_clone\n"
                "                            .set_chunk(&context_clone, &batch_clone, &ids_clone, r)\n"
                "                            .await\n"
                "                    };\n\n").encode()
    patched = original.replace(b"&batch_clone,", b"batch_clone.as_ref(),")
    manifest = b'[package]\nname="diskann"\nversion="0.56.0"\n[dependencies]\n'
    archive = tmp_path / "published.crate"
    with tarfile.open(archive, "w:gz") as stream:
        for relative, data in (("Cargo.toml", manifest), ("src/graph/index.rs", original)):
            member = tarfile.TarInfo("diskann-0.56.0/" + relative)
            member.size = len(data)
            stream.addfile(member, io.BytesIO(data))
    pin.update(archive_sha256=server.digest(archive), manifest_sha256=hashlib.sha256(manifest).hexdigest(),
               original_file_sha256=hashlib.sha256(original).hexdigest(),
               patched_file_sha256=hashlib.sha256(patched).hexdigest())
    paths = server.cache_paths(tmp_path / "service", selected)
    paths["root"].mkdir(parents=True, mode=0o700)
    paths["source"].mkdir(parents=True, mode=0o700)
    (paths["source"] / "Cargo.lock").write_text('version=4\n[[package]]\nname="diskann"\nversion="0.56.0"\n'
        'source="registry+https://github.com/rust-lang/crates.io-index"\nchecksum="' + pin["archive_sha256"] + '"\n')
    return selected, paths, archive, original, patched, manifest


def test_owned_dependency_patch_preserves_manifest_lock_and_original_archive(tmp_path):
    selected, paths, archive, original, patched, manifest = dependency_fixture(tmp_path)
    lock_before = (paths["source"] / "Cargo.lock").read_bytes()
    archive_before = archive.read_bytes()
    with server.admission([paths["root"], paths["cache"], archive],
                          exclusive_paths=[paths["root"], paths["cache"]]):
        server.prepare_dependency(paths, selected, archives=[archive])
        assert (paths["dependency"] / "src/graph/index.rs").read_bytes() == patched
        assert (paths["dependency"] / "Cargo.toml").read_bytes() == manifest
        server.verify_dependency_source(paths, selected)
        server.prepare_dependency(paths, selected, archives=[archive])  # Exact retry, no replay patch.
    assert (paths["source"] / "Cargo.lock").read_bytes() == lock_before
    assert archive.read_bytes() == archive_before
    assert paths["source"].is_relative_to(paths["cache"])
    assert paths["dependency"].is_relative_to(paths["root"])
    assert paths["cache"] != paths["root"]


def test_dependency_override_rejects_manifest_graph_changes_and_extra_files(tmp_path):
    selected, paths, archive, original, patched, manifest = dependency_fixture(tmp_path)
    with server.admission([paths["root"], paths["cache"], archive],
                          exclusive_paths=[paths["root"], paths["cache"]]):
        server.prepare_dependency(paths, selected, archives=[archive])
        target = paths["dependency"] / "Cargo.toml"
        target.write_bytes(manifest + b'new_dependency="1"\n')
        with pytest.raises(Blocked, match="exact archive"):
            server.verify_dependency_source(paths, selected)
        target.write_bytes(manifest)
        (paths["dependency"] / "unreviewed.rs").write_text("extra source")
        with pytest.raises(Blocked, match="unreviewed source"):
            server.verify_dependency_source(paths, selected)
        assert (paths["dependency"] / "unreviewed.rs").exists()  # Never delete conflicting retained input.


def test_dependency_override_refuses_wrong_lock_before_creation(tmp_path):
    selected, paths, archive, *_ = dependency_fixture(tmp_path)
    lock = paths["source"] / "Cargo.lock"
    lock.write_text(lock.read_text().replace('version="0.56.0"', 'version="0.57.0"'))
    with pytest.raises(Blocked, match="unchanged upstream lock"):
        server.prepare_dependency(paths, selected, archives=[archive])
    assert not paths["dependency"].exists() and not paths["archive"].exists()


def test_successor_recipe_preserves_original_cache_identity_and_names_all_holds(tmp_path):
    selected = server.recipe(cpu_count=7)
    base = server.original_recipe(selected)
    paths = server.cache_paths(tmp_path / "service", selected)
    assert paths["cache"].name == server.identity(base)
    assert paths["root"].name == server.identity(selected)
    assert paths["target"] == paths["cache"] / "target"
    assert paths["intermediates"] == paths["cache"] / "intermediates"
    value = provenance(tmp_path / "service")
    assert value["cache_lineage"]["recipe_identity"] == server.identity(base)
    value["cache_lineage"]["path"] = "/foreign/cache"
    with pytest.raises(Blocked, match="provenance"):
        server.validate_provenance(value)


def test_locked_metadata_requires_owned_override_and_unchanged_lock(tmp_path, monkeypatch):
    import hashlib

    selected = server.recipe(cpu_count=7)
    paths = server.cache_paths(tmp_path / "service", selected)
    paths["source"].mkdir(parents=True)
    lock = paths["source"] / "Cargo.lock"
    lock.write_bytes(b"exact locked dependency graph")
    monkeypatch.setattr(server, "LOCK_SHA256", hashlib.sha256(lock.read_bytes()).hexdigest())
    seen = []
    def run(argv, **kw):
        seen.append(argv)
        return json.dumps({"packages": [{"name": "diskann", "version": "0.56.0",
            "manifest_path": str(paths["dependency"] / "Cargo.toml")}]})
    server.verify_metadata(paths, selected, env={}, run=run)
    assert "--locked" in seen[0] and "--offline" not in seen[0]
    assert "paths=" + json.dumps([str(paths["dependency"])]) in seen[0]
    with pytest.raises(Blocked, match="exact owned dependency"):
        server.verify_metadata(paths, selected, env={}, run=lambda *a, **kw:
            json.dumps({"packages": [{"name": "diskann", "version": "0.56.0",
            "manifest_path": "/shared-registry/diskann/Cargo.toml"}]}))
    def altered(argv, **kw):
        result = run(argv, **kw)
        lock.write_bytes(b"different graph")
        return result
    with pytest.raises(Blocked, match="changed the pinned upstream lock"):
        server.verify_metadata(paths, selected, env={}, run=altered)


def test_corrupt_retained_dependency_archive_is_preserved_and_refused(tmp_path):
    selected, paths, archive, *_ = dependency_fixture(tmp_path)
    paths["archive"].parent.mkdir(parents=True, mode=0o700)
    paths["archive"].write_bytes(b"corrupt retained archive")
    with pytest.raises(Blocked, match="archive checksum"):
        server.prepare_dependency(paths, selected, archives=[archive])
    assert paths["archive"].read_bytes() == b"corrupt retained archive"
    assert not paths["dependency"].exists()
