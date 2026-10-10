"""Prepare exact, format-compatible measureme readers outside the product workspace."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "12.0.3"
REVISION = "5ac839c602b59eee9c908b3b35b6d6c0cd1c42f7"
TOOLCHAIN = "nightly-2026-09-29"
FORMAT_VERSION = 9
NAMES = ("summarize", "crox", "flamegraph")
LOCK = ROOT / "tools/compile-profile/measureme.Cargo.lock"
PATCH = ROOT / "tools/compile-profile/measureme-nightly-cpuid.patch"


def tools_root() -> Path:
    return (
        Path(
            os.environ.get(
                "LCTX_COMPILE_PROFILE_TOOLS_ROOT",
                str(Path.home() / ".local/share/compile-profile-tools" / f"measureme-{VERSION}"),
            )
        )
        .expanduser()
        .resolve()
    )


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def check(root: Path | None = None) -> dict:
    root = tools_root() if root is None else Path(root).resolve()
    errors = []
    receipt = {}
    try:
        receipt = json.loads((root / "install.json").read_text())
        for key, value in {
            "version": VERSION,
            "revision": REVISION,
            "format_version": FORMAT_VERSION,
            "toolchain": TOOLCHAIN,
            "lock_sha256": digest(LOCK),
            "patch_sha256": digest(PATCH),
        }.items():
            if receipt.get(key) != value:
                errors.append(f"measureme receipt {key} differs from the selected tool contract")
        for name in NAMES:
            path = root / "bin" / name
            entry = receipt.get("binaries", {}).get(name, {})
            if not path.is_file() or not os.access(path, os.X_OK):
                errors.append(f"missing executable {path}")
            elif entry.get("path") != str(path) or entry.get("sha256") != digest(path):
                errors.append(f"binary identity differs: {path}")
    except (OSError, ValueError, TypeError, AttributeError) as error:
        errors.append(str(error))
    return {
        "status": "blocked" if errors else "passed",
        "root": str(root),
        "errors": errors,
        "receipt": receipt,
        "repair": "just compile-profile-tools sync",
    }


def reader_generations_root() -> Path:
    return (
        Path(
            os.environ.get(
                "LCTX_COMPILE_PROFILE_READERS_ROOT",
                str(Path.home() / ".local/share/compile-profile-reader-generations"),
            )
        )
        .expanduser()
        .absolute()
    )


def reader_binding_valid(binding: dict) -> bool:
    if (
        binding.get("schema") != 1
        or binding.get("format_version") != FORMAT_VERSION
        or not binding.get("readers")
    ):
        return False
    try:
        root = Path(binding["generation_root"])
        if root.is_symlink() or json.loads((root / "record.json").read_text()) != binding:
            return False
        for item in binding["readers"].values():
            path = Path(item["path"])
            if (
                not path.is_relative_to(root / "bin")
                or path.is_symlink()
                or not path.is_file()
                or not os.access(path, os.X_OK)
                or digest(path) != item["sha256"]
            ):
                return False
        return True
    except OSError, KeyError, TypeError, ValueError:
        return False


def prepare_reader_generation(capabilities: tuple[str, ...]) -> dict:
    """Keep exact selected executables independently of mutable PATH/tool installs."""
    import tempfile

    import compile_profile_capture
    from storage_lifecycle import Storage, admission, durable_json

    selected = {}
    measureme = {}
    if any(name.startswith("compiler-") for name in capabilities):
        ready = check()
        if ready["status"] != "passed":
            raise RuntimeError("exact compiler readers unavailable: " + str(ready["errors"]))
        measureme = ready["receipt"]
        selected.update({name: tools_root() / "bin" / name for name in NAMES})
    if any(
        name.startswith("sampled-") or name in {"disassembly", "source-annotation"}
        for name in capabilities
    ):
        selected["perf"] = Path(compile_profile_capture.resolve_perf()).resolve(strict=True)
    if "source-annotation" in capabilities:
        selected["objdump"] = Path(shutil.which("objdump") or "/nonexistent").resolve(strict=True)
    if "sampled-symbolized-import" in capabilities:
        selected["samply"] = Path(shutil.which("samply") or "/nonexistent").resolve(strict=True)
    identities = {
        name: {"source_path": str(path), "sha256": digest(path)} for name, path in selected.items()
    }
    identity = hashlib.sha256(
        json.dumps({"readers": identities, "measureme": measureme}, sort_keys=True).encode()
    ).hexdigest()
    target = reader_generations_root() / identity
    with admission([target], exclusive=True):
        if not target.exists():
            target.parent.mkdir(parents=True, exist_ok=True)
            with tempfile.TemporaryDirectory(
                prefix=".reader-generation-", dir=target.parent
            ) as temporary:
                stage = Path(temporary) / "generation"
                stage.mkdir(mode=0o700)
                (stage / "bin").mkdir()
                for name, source in selected.items():
                    shutil.copy2(source, stage / "bin" / name)
                binding = {
                    "schema": 1,
                    "identity": identity,
                    "generation_root": str(target),
                    "format_version": FORMAT_VERSION,
                    "measureme_receipt": measureme,
                    "readers": {
                        name: {**item, "path": str(target / "bin" / name)}
                        for name, item in identities.items()
                    },
                }
                durable_json(stage / "record.json", binding)
                os.rename(stage, target)
        binding = json.loads((target / "record.json").read_text())
        if not reader_binding_valid(binding):
            raise RuntimeError("recorded exact reader generation missing or changed")
        storage = Storage()
        storage.publish(
            target,
            "native-content",
            {"kind": "native", "path": str(target)},
            "exact-reader-generation:" + identity,
            requires="raw-replay",
            managed=False,
        )
        return binding


def sync(root: Path | None = None) -> dict:
    from storage_lifecycle import admission

    root = tools_root() if root is None else Path(root).resolve()
    with admission([root], exclusive=True):
        return _sync_owned(root)


def _sync_owned(root: Path | None = None) -> dict:
    root = tools_root() if root is None else Path(root).resolve()
    if check(root)["status"] == "passed":
        return check(root)
    if not LOCK.is_file():
        raise RuntimeError(f"missing selected bootstrap lockfile: {LOCK}")
    root.mkdir(parents=True, exist_ok=True)
    source = root / "src"
    if not source.exists():
        subprocess.run(
            [
                "git",
                "clone",
                "--filter=blob:none",
                "--no-checkout",
                "https://github.com/rust-lang/measureme.git",
                str(source),
            ],
            check=True,
        )
        subprocess.run(["git", "-C", str(source), "checkout", "--detach", REVISION], check=True)
    actual = subprocess.check_output(
        ["git", "-C", str(source), "rev-parse", "HEAD"], text=True
    ).strip()
    if actual != REVISION:
        raise RuntimeError(
            f"preserving unexpected measureme checkout {actual}; expected {REVISION}"
        )
    dirty = subprocess.check_output(["git", "-C", str(source), "diff", "--binary", "HEAD"])
    expected = PATCH.read_bytes()
    if not dirty:
        subprocess.run(["git", "-C", str(source), "apply", "--check", str(PATCH)], check=True)
        subprocess.run(["git", "-C", str(source), "apply", str(PATCH)], check=True)
    elif dirty != expected:
        raise RuntimeError("preserving modified measureme sources; use an independent tools root")
    untracked = subprocess.check_output(
        ["git", "-C", str(source), "ls-files", "--others", "--exclude-standard", "-z"]
    )
    if untracked:
        raise RuntimeError("preserving untracked measureme sources; use an independent tools root")
    existing_lock = source / "Cargo.lock"
    if existing_lock.exists() and existing_lock.read_bytes() != LOCK.read_bytes():
        raise RuntimeError(
            "preserving a different measureme lockfile; use an independent tools root"
        )
    shutil.copy2(LOCK, source / "Cargo.lock")
    env = os.environ.copy()
    # Tool acquisition has its own workspace and artifacts. Product paths and wrappers do not
    # select this build; compiler/job/thread settings are never changed here.
    for key in (
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_TARGET_DIR",
        "CARGO_BUILD_BUILD_DIR",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
    ):
        env.pop(key, None)
    command = [
        "cargo",
        f"+{TOOLCHAIN}",
        "build",
        "--locked",
        "--release",
        "--target-dir",
        str(root / "build"),
    ]
    for name in NAMES:
        command.extend(["-p", name])
    subprocess.run(command, cwd=source, env=env, check=True)
    binaries = {}
    (root / "bin").mkdir(exist_ok=True)
    for name in NAMES:
        target = root / "bin" / name
        temporary = target.with_suffix(".installing")
        shutil.copy2(root / "build/release" / name, temporary)
        temporary.replace(target)
        binaries[name] = {"path": str(target), "sha256": digest(target)}
    receipt = {
        "version": VERSION,
        "revision": REVISION,
        "format_version": FORMAT_VERSION,
        "toolchain": TOOLCHAIN,
        "lock_sha256": digest(LOCK),
        "patch_sha256": digest(PATCH),
        "binaries": binaries,
        "command": command,
    }
    temporary = root / "install.json.tmp"
    temporary.write_text(json.dumps(receipt, indent=2) + "\n")
    temporary.replace(root / "install.json")
    return check(root)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("sync", "check"))
    args = parser.parse_args()
    try:
        result = sync() if args.command == "sync" else check()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        result = {"status": "failed", "errors": [str(error)]}
    print(json.dumps(result, indent=2))
    return 0 if result["status"] == "passed" else 75 if result["status"] == "blocked" else 1


if __name__ == "__main__":
    raise SystemExit(main())
