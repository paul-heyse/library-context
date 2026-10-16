#!/usr/bin/env python3
"""Build one exact patched server; never install, restart or inspect a database.

Source and intermediates are retained native caches. Storage admission and the shared
process owner cover acquisition/build children through confirmed cleanup. No automatic
cache retirement, compiler concurrency limit or healthy-command timeout is introduced.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import signal
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
import urllib.request
import uuid
from pathlib import Path, PurePosixPath

from harness import SpawnGuard, observe_exit, spawn_group
from storage_lifecycle import (
    Blocked,
    Storage,
    admission,
    durable_json,
    fsync_directory,
    private_directory,
    read_json,
)
from storage_service import digest, prepare_server, tools_root

ROOT = Path(__file__).resolve().parents[1]
PIN_ROOT = ROOT / "third_party/surrealdb-server"
COMMIT = "238bfeb11f5725bebed370167656748df8067595"
LOCK_SHA256 = "6ec3773437dcad1ce1e354398f8a2f7ead00e185f9ff7afc187655670a0dbbd5"
TOOLCHAIN = "nightly-2026-09-29"


def identity(value: dict) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def recipe(
    *, cpu_count: int | None = None, grpc_drain: bool = True, query_ingress: bool = False
) -> dict:
    pin = read_json(PIN_ROOT / "provenance.json")
    if (
        pin.get("schema") != 1
        or pin.get("upstream_commit") != COMMIT
        or pin.get("upstream_lock_sha256") != LOCK_SHA256
        or pin.get("upstream_repository") != "https://github.com/surrealdb/surrealdb.git"
        or pin.get("toolchain") != TOOLCHAIN
        or pin.get("features") != ["default", "cjk"]
        or pin.get("platform") != "x86_64-unknown-linux-gnu"
        or pin.get("build_metadata") != "lctx.timeout-stack.1"
        or pin.get("cli_version") != "3.3.0+lctx.timeout-stack.1 for linux on x86_64"
        or pin.get("http_version") != "surrealdb-3.3.0+lctx.timeout-stack.1"
        or pin.get("patch_file") != "timeout-tree-stack.patch"
        or digest(PIN_ROOT / pin["patch_file"]) != pin.get("patch_sha256")
    ):
        raise Blocked("server recipe pins or patch identity differ")
    count = (os.process_cpu_count() or os.cpu_count() or 1) if cpu_count is None else cpu_count
    if type(count) is not int or count < 1:
        raise Blocked("available logical CPU count is unavailable")
    dependency = read_json(PIN_ROOT / "diskann-provenance.json")
    if (
        dependency.get("schema") != 1
        or dependency.get("name") != "diskann"
        or dependency.get("version") != "0.56.0"
        or dependency.get("override") != "paths"
        or dependency.get("archive_url")
        != "https://static.crates.io/crates/diskann/diskann-0.56.0.crate"
        or dependency.get("archive_sha256")
        != "3ead115d8a4f3917c1ae9c3db0aa4dd56251841864ef8637329969f8e5145dd3"
        or dependency.get("patch_file") != "diskann-batch-borrow.patch"
        or digest(PIN_ROOT / dependency["patch_file"]) != dependency.get("patch_sha256")
    ):
        raise Blocked("server dependency patch identity differs")
    source_patches: list[dict] = []
    dependency_patches = [dependency]
    selected = {
        "schema": 1,
        "pin": pin,
        "dependency_patches": dependency_patches,
        "build": {
            "profile": "release",
            "opt_level": 3,
            "panic": "abort",
            "lto": "thin",
            "codegen_units": count,
            "incremental": False,
            "strip": True,
            "rustc_wrapper": "sccache",
            "linker": "clang",
            "c_compiler": "clang",
            "cxx_compiler": "clang++",
            "rustflags": [
                "-Z",
                "unstable-options",
                "--jobs-frontend=0",
                "--jobs-backend=0",
                "-C",
                "link-arg=-fuse-ld=mold",
            ],
            "default_features": True,
            "features": ["cjk"],
            "locked": True,
        },
    }
    if grpc_drain:
        patch = read_json(PIN_ROOT / "grpc-drain-provenance.json")
        expected_files = {
            "Cargo.lock",
            "surrealdb/server/Cargo.toml",
            "surrealdb/server/src/rpc/grpc.rs",
            "surrealdb/server/src/rpc/mod.rs",
            "surrealdb/server/src/rpc/query_drain.rs",
            "surrealdb/server/src/ntw/signals.rs",
            "surrealdb/server/src/ntw/mod.rs",
        }
        if (
            patch.get("schema") != 1
            or patch.get("name") != "grpc-execution-drain"
            or patch.get("upstream_commit") != COMMIT
            or patch.get("patch_file") != "grpc-execution-drain.patch"
            or patch.get("build_metadata") != "lctx.timeout-stack.grpc-drain.1"
            or patch.get("cli_version")
            != "3.3.0+lctx.timeout-stack.grpc-drain.1 for linux on x86_64"
            or patch.get("http_version") != "surrealdb-3.3.0+lctx.timeout-stack.grpc-drain.1"
            or patch.get("upstream_lock_sha256") != LOCK_SHA256
            or not re.fullmatch("[a-f0-9]{64}", patch.get("patched_lock_sha256", ""))
            or patch.get("lock_dependency_edges")
            != [{"package": "tokio-util", "version": "0.7.18", "added": ["futures-util"]}]
            or digest(PIN_ROOT / patch["patch_file"]) != patch.get("patch_sha256")
            or {row.get("path") for row in patch.get("files", [])} != expected_files
            or len(patch["files"]) != len(expected_files)
            or any(
                not re.fullmatch("[a-f0-9]{64}", row.get("patched_sha256", ""))
                or (
                    not isinstance(row.get("original_sha256"), str)
                    or not re.fullmatch("[a-f0-9]{64}", row["original_sha256"])
                    if row["path"] != "surrealdb/server/src/rpc/query_drain.rs"
                    else row.get("original_sha256") is not None
                )
                for row in patch["files"]
            )
        ):
            raise Blocked("server execution-drain patch identity differs")
        lock = next(row for row in patch["files"] if row["path"] == "Cargo.lock")
        if (
            lock["original_sha256"] != LOCK_SHA256
            or lock["patched_sha256"] != patch["patched_lock_sha256"]
        ):
            raise Blocked("server execution-drain lock identity differs")
        source_patches.append(patch)
        selected["source_patches"] = source_patches
        selected["pin"] = {
            **pin,
            **{key: patch[key] for key in ("build_metadata", "cli_version", "http_version")},
        }
    if query_ingress:
        if not grpc_drain:
            raise Blocked("query ingress requires the execution-drain predecessor")
        patch = read_json(PIN_ROOT / "grpc-query-bytes-provenance.json")
        protocol = read_json(PIN_ROOT / "protocol-query-bytes-provenance.json")
        predecessor = next(
            row
            for row in source_patches[-1]["files"]
            if row["path"] == "surrealdb/server/src/rpc/grpc.rs"
        )
        if (
            patch.get("schema") != 1
            or patch.get("name") != "grpc-query-bytes"
            or patch.get("upstream_commit") != COMMIT
            or patch.get("requires") != "grpc-execution-drain.patch"
            or patch.get("capability") != "QUERY_BATCH_BYTES_V1"
            or patch.get("patch_file") != "grpc-query-bytes.patch"
            or digest(PIN_ROOT / patch["patch_file"]) != patch.get("patch_sha256")
            or len(patch.get("files", [])) != 3
            or {row.get("path") for row in patch.get("files", [])}
            != {
                "surrealdb/server/src/rpc/grpc.rs",
                "surrealdb/server/tests/grpc.rs",
                "surrealdb/src/engine/remote/grpc.rs",
            }
            or patch["files"][0].get("path") != predecessor["path"]
            or patch["files"][0].get("original_sha256") != predecessor["patched_sha256"]
            or any(
                not re.fullmatch("[a-f0-9]{64}", row.get(key, ""))
                for row in patch["files"]
                for key in ("original_sha256", "patched_sha256")
            )
            or patch.get("build_metadata") != "lctx.timeout-stack.grpc-drain.query-ingress.1"
            or patch.get("cli_version")
            != "3.3.0+lctx.timeout-stack.grpc-drain.query-ingress.1 for linux on x86_64"
            or patch.get("http_version")
            != "surrealdb-3.3.0+lctx.timeout-stack.grpc-drain.query-ingress.1"
            or protocol.get("schema") != 1
            or protocol.get("name") != "surrealdb-protocol"
            or protocol.get("version") != "0.13.1"
            or protocol.get("override") != "paths"
            or protocol.get("archive_url")
            != (
                "https://static.crates.io/crates/surrealdb-protocol/surrealdb-protocol-0.13.1.crate"
            )
            or protocol.get("archive_sha256")
            != "d53501c853fe88d5ead8e50612d310b43389f3c66fda7ef323a1af6c393809c6"
            or protocol.get("patch_file") != "protocol-query-bytes.patch"
            or digest(PIN_ROOT / protocol["patch_file"]) != protocol.get("patch_sha256")
            or protocol.get("query_request_tag") != 6
            or protocol.get("capability") != "QUERY_BATCH_BYTES_V1"
            or any(
                not re.fullmatch("[a-f0-9]{64}", protocol.get(key, ""))
                for key in ("manifest_sha256", "original_file_sha256", "patched_file_sha256")
            )
        ):
            raise Blocked("server query-ingress patch or protocol identity differs")
        source_patches.append(patch)
        dependency_patches.append(protocol)
        selected["query_ingress"] = {"capability": "QUERY_BATCH_BYTES_V1", "query_request_tag": 6}
        selected["pin"] = {
            **selected["pin"],
            **{key: patch[key] for key in ("build_metadata", "cli_version", "http_version")},
        }
    return selected


def source_lock_sha256(selected: dict) -> str:
    """The successor records its reviewed feature edge separately from upstream."""
    return (
        selected["source_patches"][0]["patched_lock_sha256"]
        if selected.get("source_patches")
        else LOCK_SHA256
    )


def original_recipe(selected: dict) -> dict:
    """The first recipe owns unchanged source and mutable native Cargo caches."""
    original = {
        key: value
        for key, value in selected.items()
        if key not in ("dependency_patches", "source_patches", "query_ingress")
    }
    if selected.get("source_patches"):
        original["pin"] = {
            **selected["pin"],
            "build_metadata": "lctx.timeout-stack.1",
            "cli_version": "3.3.0+lctx.timeout-stack.1 for linux on x86_64",
            "http_version": "surrealdb-3.3.0+lctx.timeout-stack.1",
        }
    return original


def cache_paths(directory: Path, selected_recipe: dict) -> dict[str, Path]:
    root = tools_root(directory) / "server-build" / identity(selected_recipe)
    cache = tools_root(directory) / "server-build" / identity(original_recipe(selected_recipe))
    paths = {
        "root": root,
        "cache": cache,
        "cache_record": cache / "record.json",
        "source": (root if selected_recipe.get("source_patches") else cache) / "source",
        "target": cache / "target",
        "intermediates": cache / "intermediates",
        "record": root / "record.json",
        "dependency": root / "overrides/diskann-0.56.0",
        "archive": root / "archives/diskann-0.56.0.crate",
    }
    if selected_recipe.get("query_ingress"):
        paths["protocol_dependency"] = root / "overrides/surrealdb-protocol-0.13.1"
        paths["protocol_archive"] = root / "archives/surrealdb-protocol-0.13.1.crate"
    return paths


def dependency_inputs(paths: dict[str, Path], selected: dict):
    """Reuse the exact archive/patch owner for each deliberately selected package."""
    yield paths, {**selected, "dependency_patches": [selected["dependency_patches"][0]]}
    if selected.get("query_ingress"):
        yield (
            {
                **paths,
                "dependency": paths["protocol_dependency"],
                "archive": paths["protocol_archive"],
            },
            {**selected, "dependency_patches": [selected["dependency_patches"][1]]},
        )


def build_provenance(
    directory: Path,
    selected: dict,
    rustc_version: str,
    cargo_version: str,
    *,
    cache_record_sha256: str,
) -> dict:
    paths = cache_paths(directory, selected)
    result = {
        "recipe": selected,
        "recipe_identity": identity(selected),
        "service_directory": str(directory),
        "source_path": str(paths["source"]),
        "build_path": str(paths["root"]),
        "lock_sha256": source_lock_sha256(selected),
        "patch_sha256": selected["pin"]["patch_sha256"],
        "rustc_version": rustc_version,
        "cargo_version": cargo_version,
        "cache_lineage": {
            "path": str(paths["cache"]),
            "record": str(paths["cache_record"]),
            "recipe_identity": identity(original_recipe(selected)),
            "record_sha256": cache_record_sha256,
        },
        "dependency_overrides": [
            {
                "pin": dependency["dependency_patches"][0],
                "path": str(inputs["dependency"]),
                "archive_path": str(inputs["archive"]),
            }
            for inputs, dependency in dependency_inputs(paths, selected)
        ],
    }
    if selected.get("source_patches"):
        result["source_patches"] = selected["source_patches"]
        result["upstream_lock_sha256"] = LOCK_SHA256
    return result


def validate_provenance(value: dict, *, directory: Path | None = None) -> None:
    try:
        selected = value["recipe"]
        # The exact predecessor remains consumed by installed immutable services and
        # campaigns until explicit owner handoff. This is not a permissive legacy reader.
        if selected != recipe(
            cpu_count=selected["build"]["codegen_units"],
            grpc_drain="source_patches" in selected,
            query_ingress="query_ingress" in selected,
        ):
            raise Blocked("server provenance recipe differs from reviewed pins")
        if value["recipe_identity"] != identity(selected):
            raise Blocked("server recipe identity differs")
        service = Path(value["service_directory"])
        paths = cache_paths(service, selected)
        expected = build_provenance(
            service,
            selected,
            value["rustc_version"],
            value["cargo_version"],
            cache_record_sha256=value["cache_lineage"]["record_sha256"],
        )
        if (
            not service.is_absolute()
            or (directory is not None and service != Path(directory))
            or value["source_path"] != str(paths["source"])
            or value["build_path"] != str(paths["root"])
            or value["lock_sha256"] != expected["lock_sha256"]
            or value.get("upstream_lock_sha256") != expected.get("upstream_lock_sha256")
            or value["patch_sha256"] != selected["pin"]["patch_sha256"]
            or not re.fullmatch("[a-f0-9]{64}", value["cache_lineage"]["record_sha256"])
            or value["cache_lineage"] != expected["cache_lineage"]
            or value["dependency_overrides"] != expected["dependency_overrides"]
            or value.get("source_patches") != expected.get("source_patches")
            or not value["rustc_version"].startswith("rustc ")
            or "commit-date: 2026-09-" not in value["rustc_version"]
            or not value["cargo_version"].startswith("cargo ")
        ):
            raise Blocked("server build provenance is incomplete or differs")
    except (KeyError, TypeError, ValueError) as error:
        raise Blocked("server build provenance is incomplete") from error


def validate_descriptor(path: Path, *, runner=None) -> dict:
    """Keep generation admission through byte validation and owned version-child cleanup."""
    path = Path(path)
    with admission([path.parent]):
        return _validate_descriptor(path, runner=runner)


def _validate_descriptor(path: Path, *, runner=None) -> dict:
    path = Path(path)
    try:
        if any(parent.is_symlink() for parent in (path, *path.parents)):
            raise Blocked("server descriptor must have a physical path")
        row = read_json(path)
        validate_provenance(row["provenance"])
        body = {
            key: row[key]
            for key in ("schema", "kind", "sha256", "size", "version", "http_version", "provenance")
        }
        expected_identity = identity(body)
        directory = Path(row["provenance"]["service_directory"])
        root = tools_root(directory) / "surreal" / expected_identity
        executable = root / "surreal"
        if (
            row.get("schema") != 1
            or row.get("kind") != "library-context-surrealdb-server"
            or row.get("identity") != expected_identity
            or row.get("descriptor_path") != str(root / "record.json")
            or path != root / "record.json"
            or row.get("path") != str(executable)
            or row["version"] != row["provenance"]["recipe"]["pin"]["cli_version"]
            or row["http_version"] != row["provenance"]["recipe"]["pin"]["http_version"]
            or executable.is_symlink()
            or not executable.is_file()
            or executable.stat().st_size != row["size"]
            or executable.stat().st_mode & 0o777 != 0o500
            or path.stat().st_mode & 0o777 != 0o400
            or digest(executable) != row["sha256"]
        ):
            raise Blocked("server generation descriptor or executable identity differs")
        if runner is None or runner is subprocess.run:
            reported = execute(
                [str(executable), "version"], cwd=root, env=dict(os.environ), capture=True
            )
        else:
            # Explicit injected runners serve pure host-tool controls; production uses the
            # shared process owner above, including when a caller passes subprocess.run.
            result = runner(
                [str(executable), "version"], capture_output=True, text=True, check=False
            )
            reported = result.stdout.strip() if not result.returncode else None
        if reported != row["version"]:
            raise Blocked("server generation reports a different version")
        return row
    except (OSError, KeyError, TypeError, ValueError) as error:
        raise Blocked("server generation cannot be verified") from error


def dependencies(descriptor: dict) -> list[dict]:
    validate_provenance(descriptor["provenance"])
    provenance = descriptor["provenance"]
    dependencies = [
        {"path": path, "role": role}
        for path, role in (
            (str(Path(descriptor["path"]).parent), "server-executable-generation"),
            (descriptor["descriptor_path"], "server-generation-provenance"),
            (provenance["source_path"], "server-source-warm-cache"),
            (provenance["build_path"], "server-build-warm-cache"),
            (provenance["cache_lineage"]["path"], "server-borrowed-build-warm-cache"),
            (provenance["cache_lineage"]["record"], "server-original-cache-provenance"),
            (provenance["dependency_overrides"][0]["path"], "server-dependency-source-override"),
            (
                provenance["dependency_overrides"][0]["archive_path"],
                "server-dependency-original-archive",
            ),
        )
    ]
    for override in provenance["dependency_overrides"][1:]:
        dependencies.extend(
            [
                {"path": override["path"], "role": "server-dependency-source-override"},
                {"path": override["archive_path"], "role": "server-dependency-original-archive"},
            ]
        )
    return dependencies


def execute(argv: list[str], *, cwd: Path, env: dict, capture: bool = False) -> str:
    """The original leader stays waitable until its owned descendant group is drained."""
    with tempfile.TemporaryFile(dir=cwd) as output:
        child = spawn_group(
            argv,
            cwd=cwd,
            env=env,
            stdout=output if capture else sys.stderr,
            stderr=sys.stderr,
            death_signal=signal.SIGTERM,
        )
        guard = SpawnGuard(child)
        with guard:
            while observe_exit(child) is None:
                time.sleep(0.05)
        if guard.cleanup.get("status") != "confirmed":
            raise Blocked(
                "server tool child cleanup is unresolved; retained caches remain protected"
            )
        if guard.returncode:
            raise RuntimeError(f"server tool failed ({guard.returncode}): {argv[0]}")
        if capture:
            output.seek(0)
            return output.read().decode().strip()
        return ""


def git_environment() -> dict:
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(
        GIT_CONFIG_NOSYSTEM="1",
        GIT_CONFIG_GLOBAL="/dev/null",
        GIT_ATTR_NOSYSTEM="1",
        GIT_TERMINAL_PROMPT="0",
    )
    return env


def build_environment(selected: dict, paths: dict[str, Path]) -> dict:
    env = dict(os.environ)
    # Do not inherit another checkout's target, profile, flags or compiler job setting.
    for key in list(env):
        if key.startswith("CARGO_PROFILE_") or key in {
            "CARGO_TARGET_DIR",
            "CARGO_BUILD_BUILD_DIR",
            "CARGO_BUILD_JOBS",
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "RUSTC_WORKSPACE_WRAPPER",
            "RUSTC",
            "CARGO_BUILD_RUSTFLAGS",
            "CARGO_INCREMENTAL",
            "MAKEFLAGS",
            "CARGO_MAKEFLAGS",
        }:
            env.pop(key)
    config = selected["build"]
    env.update(
        RUSTC_WRAPPER=config["rustc_wrapper"],
        CC=config["c_compiler"],
        CXX=config["cxx_compiler"],
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=config["linker"],
        CARGO_TARGET_DIR=str(paths["target"]),
        CARGO_PROFILE_RELEASE_OPT_LEVEL="3",
        CARGO_PROFILE_RELEASE_PANIC="abort",
        CARGO_PROFILE_RELEASE_LTO="thin",
        CARGO_PROFILE_RELEASE_INCREMENTAL="false",
        CARGO_PROFILE_RELEASE_CODEGEN_UNITS=str(config["codegen_units"]),
        CARGO_PROFILE_RELEASE_STRIP="true",
        RUSTFLAGS=" ".join(config["rustflags"]),
        SURREAL_BUILD_VERSION="3.3.0",
        SURREAL_BUILD_METADATA=selected["pin"]["build_metadata"],
    )
    return env


def verify_source(paths: dict[str, Path], selected: dict, *, run=execute) -> None:
    source = paths["source"]
    env = git_environment()
    if run(["git", "rev-parse", "HEAD"], cwd=source, env=env, capture=True) != COMMIT:
        raise Blocked("server source HEAD differs from pinned commit")
    pin = selected["pin"]
    for name, expected in (
        ("Cargo.lock", source_lock_sha256(selected)),
        ("Cargo.toml", pin["upstream_manifest_sha256"]),
        (pin["patched_file"], pin["patched_file_sha256"]),
    ):
        if digest(source / name) != expected:
            raise Blocked("server source or lock differs: " + name)
    changed = {pin["patched_file"]}
    added = set()
    final_files = {}
    for patch in selected.get("source_patches", []):
        for row in patch["files"]:
            if (
                row["path"] in final_files
                and row["original_sha256"] != final_files[row["path"]]["patched_sha256"]
            ):
                raise Blocked("server source patch chain has a divergent predecessor")
            final_files[row["path"]] = row
            (changed if row["original_sha256"] is not None else added).add(row["path"])
    for row in final_files.values():
        if digest(source / row["path"]) != row["patched_sha256"]:
            raise Blocked("server source differs from composed patch: " + row["path"])
    status = run(
        ["git", "status", "--porcelain", "--untracked-files=all", "--ignored"],
        cwd=source,
        env=env,
        capture=True,
    )
    observed = {line.strip() for line in status.splitlines() if line.strip()}
    if observed != {"M " + path for path in changed} | {"?? " + path for path in added}:
        raise Blocked("server source has changes beyond the reviewed patch")


def prepare_source_patches(paths: dict[str, Path], selected: dict, *, run=execute) -> None:
    """Apply only on this successor's private source; never mutate its borrowed cache."""
    source = paths["source"]
    patches = selected.get("source_patches", [])
    for index, patch in enumerate(patches):
        states = []
        for row in patch["files"]:
            path = source / row["path"]
            actual = digest(path) if path.is_file() and not path.is_symlink() else None
            later_hashes = {
                later["patched_sha256"]
                for successor in patches[index + 1 :]
                for later in successor["files"]
                if later["path"] == row["path"]
            }
            states.append(
                "patched"
                if actual == row["patched_sha256"] or actual in later_hashes
                else "original"
                if actual == row["original_sha256"] and not path.is_symlink()
                else "conflict"
            )
        if all(state == "patched" for state in states):
            continue
        if not all(state == "original" for state in states):
            raise Blocked("successor source conflicts with exact source patch chain")
        path = str(PIN_ROOT / patch["patch_file"])
        run(["git", "apply", "--check", path], cwd=source, env=git_environment())
        run(["git", "apply", path], cwd=source, env=git_environment())


def dependency_archives(package: str = "diskann-0.56.0") -> list[Path]:
    cargo = Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo")))
    return sorted((cargo / "registry/cache").glob("*/" + package + ".crate"))


def archive_files(path: Path, pin: dict) -> dict[str, bytes]:
    """Only exact regular members of the checksum-bound published package are inputs."""
    if digest(path) != pin["archive_sha256"]:
        raise Blocked("dependency archive checksum differs from the locked package")
    result = {}
    prefix = pin["name"] + "-" + pin["version"]
    with tarfile.open(path) as archive:
        for item in archive.getmembers():
            parts = PurePosixPath(item.name).parts
            if not parts or parts[0] != prefix or ".." in parts or item.name.startswith("/"):
                raise Blocked("dependency archive member escapes its exact package")
            if item.isdir():
                continue
            if not item.isfile() or len(parts) < 2:
                raise Blocked("dependency archive requires regular source members")
            relative = str(PurePosixPath(*parts[1:]))
            if relative in result:
                raise Blocked("dependency archive has duplicate source members")
            stream = archive.extractfile(item)
            if stream is None:
                raise Blocked("dependency archive source member is unavailable")
            with stream:
                result[relative] = stream.read()
    if hashlib.sha256(result.get("Cargo.toml", b"")).hexdigest() != pin["manifest_sha256"]:
        raise Blocked("dependency normalized manifest differs from the locked package")
    if (
        hashlib.sha256(result.get(pin["patched_file"], b"")).hexdigest()
        != pin["original_file_sha256"]
    ):
        raise Blocked("dependency original source differs from the reviewed patch")
    return result


def verify_dependency_source(paths: dict[str, Path], selected: dict) -> None:
    pin = selected["dependency_patches"][0]
    expected = archive_files(paths["archive"], pin)
    source = paths["dependency"]
    found = set()
    for path in source.rglob("*"):
        if path.is_symlink():
            raise Blocked("dependency override contains a symlink")
        if path.is_dir():
            continue
        relative = str(path.relative_to(source))
        if not path.is_file() or relative not in expected:
            raise Blocked("dependency override contains unreviewed source")
        wanted = (
            pin["patched_file_sha256"]
            if relative == pin["patched_file"]
            else hashlib.sha256(expected[relative]).hexdigest()
        )
        if digest(path) != wanted:
            raise Blocked("dependency override differs from exact archive and patch: " + relative)
        found.add(relative)
    if found != expected.keys():
        raise Blocked("dependency override source is incomplete")


def prepare_dependency(
    paths: dict[str, Path], selected: dict, *, archives: list[Path], run=execute
) -> None:
    pin = selected["dependency_patches"][0]
    lock = tomllib.loads((paths["source"] / "Cargo.lock").read_text())
    entries = [row for row in lock["package"] if row["name"] == pin["name"]]
    if (
        len(entries) != 1
        or entries[0]["version"] != pin["version"]
        or entries[0].get("source") != "registry+https://github.com/rust-lang/crates.io-index"
        or entries[0].get("checksum") != pin["archive_sha256"]
    ):
        raise Blocked("dependency override does not match the unchanged upstream lock")
    private_directory(paths["archive"].parent)
    if not paths["archive"].exists():
        temporary = paths["archive"].with_name(".archive-" + uuid.uuid4().hex)
        cached = next(
            (
                path
                for path in archives
                if not path.is_symlink()
                and path.is_file()
                and digest(path) == pin["archive_sha256"]
            ),
            None,
        )
        try:
            source = cached.open("rb") if cached else urllib.request.urlopen(pin["archive_url"])
            with source, temporary.open("xb") as output:
                while chunk := source.read(1024**2):
                    output.write(chunk)
                output.flush()
                os.fsync(output.fileno())
            if digest(temporary) != pin["archive_sha256"]:
                raise Blocked("acquired dependency archive differs from locked checksum")
            os.link(temporary, paths["archive"])
            fsync_directory(paths["archive"].parent)
        finally:
            temporary.unlink(missing_ok=True)
    members = archive_files(paths["archive"], pin)
    source = paths["dependency"]
    private_directory(source.parent)
    private_directory(source)
    # Recover an interrupted extraction only by adding missing exact archive files. Existing
    # bytes must match the original package, except the already-applied reviewed source patch.
    for relative, data in members.items():
        path = source / relative
        if any(parent.is_symlink() for parent in (path, *path.parents)):
            raise Blocked("dependency override must have physical source paths")
        if path.exists():
            wanted = {hashlib.sha256(data).hexdigest()}
            if relative == pin["patched_file"]:
                wanted.add(pin["patched_file_sha256"])
            if not path.is_file() or digest(path) not in wanted:
                raise Blocked("dependency extraction conflicts with retained source")
            continue
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        temporary = paths["root"] / (".source-" + uuid.uuid4().hex)
        try:
            fd = os.open(temporary, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
            with os.fdopen(fd, "wb") as stream:
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
            os.link(temporary, path)
            fsync_directory(path.parent)
        finally:
            temporary.unlink(missing_ok=True)
    if digest(source / pin["patched_file"]) == pin["original_file_sha256"]:
        patch = str(PIN_ROOT / pin["patch_file"])
        run(["git", "apply", "--check", patch], cwd=source, env=git_environment())
        run(["git", "apply", patch], cwd=source, env=git_environment())
    verify_dependency_source(paths, selected)
    with (source / pin["patched_file"]).open("rb") as stream:
        os.fsync(stream.fileno())
    for path in sorted(
        (p for p in source.rglob("*") if p.is_dir()), key=lambda p: len(p.parts), reverse=True
    ):
        fsync_directory(path)
    for path in (source, source.parent, paths["root"], paths["root"].parent):
        fsync_directory(path)


def cargo_config(paths: dict[str, Path]) -> list[str]:
    overrides = [str(paths["dependency"])]
    if "protocol_dependency" in paths:
        overrides.append(str(paths["protocol_dependency"]))
    return [
        "--config",
        "build.build-dir=" + json.dumps(str(paths["intermediates"])),
        "--config",
        "paths=" + json.dumps(overrides),
    ]


def verify_metadata(paths: dict[str, Path], selected: dict, *, env: dict, run=execute) -> None:
    result = run(
        [
            "rustup",
            "run",
            TOOLCHAIN,
            "cargo",
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            selected["pin"]["platform"],
            "--features",
            "cjk",
            *cargo_config(paths),
        ],
        cwd=paths["source"],
        env=env,
        capture=True,
    )
    for inputs, dependency in dependency_inputs(paths, selected):
        pin = dependency["dependency_patches"][0]
        packages = [row for row in json.loads(result)["packages"] if row["name"] == pin["name"]]
        if (
            len(packages) != 1
            or packages[0]["version"] != pin["version"]
            or Path(packages[0]["manifest_path"]) != inputs["dependency"] / "Cargo.toml"
        ):
            raise Blocked(
                "Cargo did not select the exact owned dependency override: " + pin["name"]
            )
    if digest(paths["source"] / "Cargo.lock") != source_lock_sha256(selected):
        raise Blocked("Cargo metadata changed the selected reviewed lock")


def build(
    directory: Path, *, run=execute, metadata_only: bool = False, query_ingress: bool = False
) -> dict:
    directory = Path(os.path.abspath(directory.expanduser()))
    if platform.system() != "Linux" or platform.machine() != "x86_64":
        raise Blocked("this reviewed server recipe requires Linux x86_64")
    selected = recipe(query_ingress=query_ingress)
    paths = cache_paths(directory, selected)
    if any(
        parent.is_symlink()
        for path in (paths["root"], paths["cache"])
        for parent in (path, *path.parents)
    ):
        raise Blocked("server cache must have a physical path")
    generations = tools_root(directory) / "surreal"
    archives = dependency_archives()
    if query_ingress:
        archives.extend(dependency_archives("surrealdb-protocol-0.13.1"))
    # Admission precedes directories, source download, native compilation and generation copy.
    with admission(
        [paths["root"], paths["cache"], generations, *archives],
        exclusive_paths=[paths["root"], paths["cache"]],
    ):
        for path in (
            tools_root(directory),
            paths["root"].parent,
            paths["root"],
            paths["cache"],
            generations,
        ):
            private_directory(path)
        storage = Storage()
        for path, category, label in (
            (paths["root"], "native-cache", "server-successor-recipe"),
            (paths["cache"], "native-cache", "server-build-warm-cache"),
        ):
            storage.publish(
                path,
                category,
                {"kind": "external", "path": str(path)},
                label + ":" + identity(selected),
                requires="raw-replay",
            )
        if paths["cache_record"].exists():
            if read_json(paths["cache_record"]).get("recipe") != original_recipe(selected):
                raise Blocked("borrowed server cache belongs to a different original recipe")
        else:
            durable_json(
                paths["cache_record"],
                {"schema": 1, "recipe": original_recipe(selected), "phase": "retained-warm-cache"},
            )
        original_record_digest = digest(paths["cache_record"])
        if paths["record"].exists():
            record = read_json(paths["record"])
            if record.get("recipe") != selected:
                raise Blocked("retained server build recipe differs")
            if record.get("descriptor_path"):
                return validate_descriptor(Path(record["descriptor_path"]))
        durable_json(paths["record"], {"schema": 1, "recipe": selected, "phase": "acquisition"})
        source = paths["source"]
        env = git_environment()
        if not (source / "Cargo.lock").exists():
            private_directory(source)
            run(["git", "init", "-q", "--template="], cwd=source, env=env)
            run(
                [
                    "git",
                    "fetch",
                    "-q",
                    "--depth",
                    "1",
                    selected["pin"]["upstream_repository"],
                    COMMIT,
                ],
                cwd=source,
                env=env,
            )
            run(
                ["git", "-c", "core.autocrlf=false", "checkout", "-q", "--detach", COMMIT],
                cwd=source,
                env=env,
            )
        patch_file = source / selected["pin"]["patched_file"]
        if digest(patch_file) == selected["pin"]["original_file_sha256"]:
            patch = str(PIN_ROOT / selected["pin"]["patch_file"])
            run(["git", "apply", "--check", patch], cwd=source, env=env)
            run(["git", "apply", patch], cwd=source, env=env)
        prepare_source_patches(paths, selected, run=run)
        verify_source(paths, selected, run=run)
        storage.publish(
            source,
            "native-cache",
            {"kind": "external", "path": str(source)},
            "server-pinned-source:" + COMMIT,
            requires="raw-replay",
        )
        for name in ("target", "intermediates"):
            private_directory(paths[name])
        for inputs, dependency in dependency_inputs(paths, selected):
            prepare_dependency(inputs, dependency, archives=archives, run=run)
            storage.publish(
                inputs["dependency"],
                "native-cache",
                {"kind": "external", "path": str(inputs["dependency"])},
                "server-exact-dependency-override:" + identity(dependency),
                requires="raw-replay",
            )
        env = build_environment(selected, paths)
        rustc = run(
            ["rustup", "run", TOOLCHAIN, "rustc", "-vV"], cwd=paths["root"], env=env, capture=True
        )
        cargo = run(
            ["rustup", "run", TOOLCHAIN, "cargo", "--version"],
            cwd=paths["root"],
            env=env,
            capture=True,
        )
        provenance = build_provenance(
            directory, selected, rustc, cargo, cache_record_sha256=original_record_digest
        )
        validate_provenance(provenance, directory=directory)
        verify_metadata(paths, selected, env=env, run=run)
        if metadata_only:
            if digest(paths["cache_record"]) != original_record_digest:
                raise Blocked("original cache recipe record changed during successor preparation")
            durable_json(
                paths["record"],
                {
                    "schema": 1,
                    "recipe": selected,
                    "phase": "metadata-checked",
                    "provenance": provenance,
                },
            )
            return {
                "outcome": "passed",
                "phase": "metadata-checked",
                "provenance": provenance,
                "cargo_config": cargo_config(paths),
            }
        durable_json(
            paths["record"],
            {"schema": 1, "recipe": selected, "phase": "building", "provenance": provenance},
        )
        command = [
            "rustup",
            "run",
            TOOLCHAIN,
            "cargo",
            "build",
            "--locked",
            "--release",
            "-p",
            "surreal",
            "--bin",
            "surreal",
            "--features",
            "cjk",
            "--target",
            selected["pin"]["platform"],
            *cargo_config(paths),
        ]
        run(command, cwd=source, env=env)
        verify_source(paths, selected, run=run)
        for inputs, dependency in dependency_inputs(paths, selected):
            verify_dependency_source(inputs, dependency)
        if digest(paths["cache_record"]) != original_record_digest:
            raise Blocked("original cache recipe record changed during successor build")
        executable = paths["target"] / selected["pin"]["platform"] / "release/surreal"
        reported = run([str(executable), "version"], cwd=paths["root"], env=env, capture=True)
        if reported != selected["pin"]["cli_version"]:
            raise Blocked("built server version differs from the exact recipe")
        descriptor = prepare_server(directory, executable, provenance)
        durable_json(
            paths["record"],
            {
                "schema": 1,
                "recipe": selected,
                "phase": "built",
                "provenance": provenance,
                "descriptor_path": descriptor["descriptor_path"],
            },
        )
        return descriptor


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--service-directory", type=Path)
    parser.add_argument(
        "--query-ingress",
        action="store_true",
        help="select the isolated QUERY_BATCH_BYTES_V1 candidate generation",
    )
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("recipe")
    sub.add_parser("build")
    sub.add_parser("metadata")
    verify = sub.add_parser("verify")
    verify.add_argument("descriptor", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "recipe":
            row = recipe(query_ingress=args.query_ingress)
        elif args.command == "verify":
            row = validate_descriptor(args.descriptor)
        else:
            if args.service_directory is None:
                parser.error("build requires an explicit --service-directory")
            row = build(
                args.service_directory,
                metadata_only=args.command == "metadata",
                query_ingress=args.query_ingress,
            )
        print(json.dumps(row, indent=2, sort_keys=True))
        return 0
    except Blocked as error:
        print("surrealdb-server: blocked: " + str(error), file=sys.stderr)
        return 75
    except (OSError, RuntimeError, ValueError) as error:
        print("surrealdb-server: failed: " + str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
