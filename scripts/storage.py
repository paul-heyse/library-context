"""Owner-directed storage inventory and effects. Nothing ages out by file mtime."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
import shutil
import stat
import subprocess
import uuid
from pathlib import Path
from typing import Any

from storage_lifecycle import (
    ROOT,
    Blocked,
    Invalid,
    Storage,
    absolute,
    admission,
    durable_json,
    fsync_directory,
    now,
    physical,
    private_directory,
    read_json,
)


def implementation_identity() -> str:
    digest = hashlib.sha256()
    for name in (
        "storage.py",
        "storage_lifecycle.py",
        "storage_owners.py",
        "storage_archive.py",
        "storage_service.py",
        "storage_acquisition.py",
        "harness.py",
        "runs.py",
        "worktree.py",
        "workspace_env.py",
        "compile_profile.py",
        "compile_profile_capture.py",
        "compile_profile_tools.py",
        "compile_profile_wrapper.py",
        "build_environment.py",
        "surrealdb_fixture.py",
        "surrealdb_service.py",
        "docs.py",
    ):
        path = ROOT / "scripts" / name
        digest.update(name.encode())
        if path.exists():
            digest.update(path.read_bytes())
    digest.update((ROOT / ".config/storage.toml").read_bytes())
    for name in (
        "justfile",
        ".python-version",
        "tools/compile-profile/measureme.Cargo.lock",
        "tools/compile-profile/measureme-nightly-cpuid.patch",
        "crates/lctx/src/acquisition.rs",
        "crates/lctx/src/main.rs",
        "crates/lctx/src/compile.rs",
    ):
        if not (ROOT / name).exists():
            continue
        digest.update((ROOT / name).read_bytes())
    return digest.hexdigest()


def known_scopes(store: Storage) -> list[dict]:
    home = Path.home()
    import compile_profile_tools
    import surrealdb_service
    import workspace_env

    paths = [
        (
            home / ".cargo/build",
            "native-cache",
            "shared Cargo intermediates; internal eviction disabled",
        ),
        (
            home / ".cache/library-context-build",
            "unknown",
            "legacy outputs require exact owner adoption",
        ),
        (store.root / "build", "unknown", "mixed outputs; classify independent owner groups"),
        (store.root / "target", "native-cache", "current final artifacts and pending BC3"),
        (workspace_env.environment_path(store.root), "environment", "current checkout environment"),
        (
            workspace_env.vllm_environment_path(store.root),
            "environment",
            "current vLLM service environment",
        ),
        (store.root / workspace_env.EXTENSION_DIRECTORY, "environment", "native extension output"),
        (
            store.root / "build/envs",
            "acquired-environment",
            "default acquisition environments; explicit CLI overrides require descriptors",
        ),
        (
            store.root / "build/sources",
            "acquired-source",
            "default acquisition sources; explicit CLI overrides require descriptors",
        ),
        (
            compile_profile_tools.tools_root(),
            "tool",
            "exact profiling readers and build inputs; current installation protected",
        ),
        (
            compile_profile_tools.reader_generations_root(),
            "native-cache",
            "recorded exact-reader generations; archive dependencies preserved",
        ),
        (home / ".cache/uv", "native-cache", "native uv policy unchanged"),
        (home / ".cache/sccache", "native-cache", "native sccache policy unchanged"),
        (home / ".rustup", "native-cache", "current toolchains preserved"),
        (home / ".cache/huggingface", "native-cache", "current model inputs preserved"),
        (home / ".cache/torch", "native-cache", "native model cache preserved"),
        (home / ".cache/triton", "native-cache", "native kernel cache preserved"),
        (store.root / "build/docs/site", "docs-output", "current published documentation"),
        (
            store.root / "docs/design_review/evidence",
            "tracked-evidence",
            "Git/LFS and named evidence consumers",
        ),
        (
            surrealdb_service.state_root(),
            "service-state",
            "service owner; no private config inspection",
        ),
    ]
    for key, category in (
        ("diagnostic_root", "profile-raw"),
        ("archive_destination", "recovery-archive"),
    ):
        if store.host.get(key):
            paths.append(
                (
                    absolute(Path(store.host[key])),
                    category,
                    "explicit host placement; unregistered children remain protected",
                )
            )
    result = []
    for path, category, reason in paths:
        exists = path.exists()
        row: dict[str, Any] = {
            "path": str(path),
            "category": category,
            "disposition": "external"
            if category in {"native-cache", "service-state", "tracked-evidence"}
            else "unresolved",
            "reason": reason,
            "exists": exists,
        }
        if exists and path.is_dir() and not path.is_symlink():
            try:
                row["top_level"] = [
                    {
                        "name": child.name,
                        "kind": "symlink"
                        if child.is_symlink()
                        else "directory"
                        if child.is_dir()
                        else "file",
                        "disposition": "protected-unclassified",
                    }
                    for child in sorted(path.iterdir())
                ]
            except OSError as error:
                row["error"] = str(error)
        result.append(row)
    return result


def account(paths: list[Path]) -> dict:
    from storage_lifecycle import _mountpoints

    mounts = _mountpoints()
    seen = set()
    apparent = allocated = duplicate_links = 0
    errors = []
    for root in paths:
        if not root.exists():
            continue
        device = root.lstat().st_dev

        def visit(path: Path, device: int = device, scope: Path = root) -> None:
            nonlocal apparent, allocated, duplicate_links
            try:
                info = path.lstat()
                if info.st_dev != device or (path != scope and path in mounts):
                    errors.append(f"mount boundary skipped: {path}")
                    return
                if stat.S_ISLNK(info.st_mode):
                    return
                identity = (info.st_dev, info.st_ino)
                if identity in seen:
                    duplicate_links += 1
                    return
                seen.add(identity)
                apparent += info.st_size
                allocated += info.st_blocks * 512
                if stat.S_ISDIR(info.st_mode):
                    for child in path.iterdir():
                        visit(child)
            except OSError as error:
                errors.append(f"{path}: {error}")

        visit(root)
    return {
        "observed_at": now(),
        "apparent_bytes": apparent,
        "allocated_bytes": allocated,
        "duplicate_links": duplicate_links,
        "errors": errors,
        "limits": [
            "unseen external hardlinks and shared extents cannot be exclusively attributed",
            "concurrent writers can change totals",
        ],
    }


def inventory(store: Storage, *, deep: bool = False, scope: str = "host") -> dict:
    result = scoped_plan(store, (), scope)
    result["categories"] = store.policy["categories"]
    result["coverage"] = [
        row
        for row in known_scopes(store)
        if scope == "host" or Path(row["path"]).is_relative_to(store.root)
    ]
    result["participants"] = [
        {"path": root, "available": Path(root).is_dir()}
        for root in store.host.get("repositories", [])
        if scope == "host" or Path(root) == store.root
    ]
    result["capacity"] = dict(
        zip(("total", "used", "free"), shutil.disk_usage(store.root), strict=True)
    )
    if deep:
        result["accounting"] = account(
            [Path(row["path"]) for row in result["coverage"]]
            + [Path(row["path"]) for row in result["dispositions"]]
        )
    return result


def _write_host(store: Storage, host: dict) -> None:
    permitted = {"schema", "repositories", "archive_destination", "diagnostic_root"}
    if set(host) - permitted:
        raise Invalid(
            "host configuration has fields this command cannot preserve; edit it explicitly"
        )
    private_directory(store.host_path.parent)
    # JSON string/list representations are valid TOML for these explicit scalar arrays.
    text = "\n".join(f"{key} = {json.dumps(value)}" for key, value in host.items()) + "\n"
    target = store.host_path.with_name("." + store.host_path.name + "." + uuid.uuid4().hex)
    fd = os.open(target, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, "w") as stream:
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(target, store.host_path)
        fsync_directory(store.host_path.parent)
    finally:
        target.unlink(missing_ok=True)


def register(store: Storage, path: Path, *, remove: bool = False) -> dict:
    path = absolute(path)
    host = dict(store.host)
    repos = set(host.get("repositories", []))
    if remove:
        if any(
            str(path) in json.dumps(row["owner"]) and not row.get("retired_at")
            for row in store.records()
        ):
            raise Blocked("repository still owns lifecycle records")
        repos.discard(str(path))
    else:
        result = subprocess.run(
            ["git", "-C", str(path), "rev-parse", "--git-common-dir"],
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode:
            raise Invalid("registration requires an available Git repository")
        current = subprocess.run(
            ["git", "-C", str(ROOT), "rev-parse", "--git-common-dir"],
            capture_output=True,
            text=True,
            check=True,
        )

        def common_directory(checkout: Path, output: str) -> Path:
            value = Path(output.strip())
            return (value if value.is_absolute() else checkout / value).resolve()

        if common_directory(path, result.stdout) != common_directory(ROOT, current.stdout):
            raise Blocked("registration is limited to this repository and its own worktrees")
        repos.add(str(path))
        # Register discovered worktrees explicitly so absent checkouts remain visible later.
        listing = subprocess.run(
            ["git", "-C", str(path), "worktree", "list", "--porcelain"],
            capture_output=True,
            text=True,
            check=True,
        )
        repos.update(
            line.removeprefix("worktree ")
            for line in listing.stdout.splitlines()
            if line.startswith("worktree ")
        )
    host["repositories"] = sorted(repos)
    with store.metadata():
        _write_host(store, host)
    return {"outcome": "passed", "repositories": sorted(repos)}


def _qualified(store: Storage) -> dict:
    receipt = (
        read_json(store.state / "acceptance.json")
        if (store.state / "acceptance.json").exists()
        else {}
    )
    if (
        receipt.get("outcome") != "passed"
        or receipt.get("implementation_identity") != implementation_identity()
        or receipt.get("policy_revision") != store.revision
        or not receipt.get("qualify_run")
        or receipt.get("independent_review") != "accepted"
    ):
        raise Blocked(
            "SM8 matching-source qualification and independent review required; "
            "automation remains disabled"
        )
    return receipt


def automation(store: Storage, operation: str) -> dict:
    root = Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config"))) / "systemd/user"
    service, timer = (
        root / "library-context-storage-sweep.service",
        root / "library-context-storage-sweep.timer",
    )
    if operation == "status":
        qualification: dict[str, Any] = {
            "present": (store.state / "acceptance.json").exists(),
            "valid": False,
        }
        try:
            _qualified(store)
            qualification["valid"] = True
        except (Blocked, Invalid, OSError, ValueError, RuntimeError) as error:
            qualification["reason"] = str(error)
        units: dict[str, dict[str, Any]] = {}
        for unit in (service, timer):
            try:
                observed = subprocess.run(
                    [
                        "systemctl",
                        "--user",
                        "show",
                        unit.name,
                        "--property=LoadState,ActiveState,SubState,UnitFileState,Result",
                    ],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                properties = dict(
                    line.split("=", 1) for line in observed.stdout.splitlines() if "=" in line
                )
                units[unit.name] = {
                    "outcome": "passed" if observed.returncode == 0 and properties else "blocked",
                    "properties": properties,
                }
                if observed.returncode or not properties:
                    units[unit.name]["reason"] = (
                        observed.stderr.strip() or "systemd unit status unavailable"
                    )
            except OSError as error:
                units[unit.name] = {"outcome": "blocked", "reason": str(error)}
        timer_state = units[timer.name].get("properties", {})
        return {
            "outcome": "passed",
            "installed": service.is_file() and timer.is_file(),
            "enabled": timer_state.get("UnitFileState") in {"enabled", "enabled-runtime"},
            "active": timer_state.get("ActiveState") == "active",
            "qualification": qualification,
            "systemd": units,
        }
    if operation == "disable":
        subprocess.run(["systemctl", "--user", "disable", "--now", timer.name], check=True)
        (store.state / "hooks-enabled.json").unlink(missing_ok=True)
        return {"outcome": "passed", "enabled": False}
    _qualified(store)
    if not store.host_path.is_file():
        raise Blocked("explicit host configuration required before automation installation")
    uv = shutil.which("uv")
    if uv is None:
        raise Blocked("uv is unavailable")

    def quote(value: str | Path) -> str:
        return '"' + str(value).replace("\\", "\\\\").replace('"', '\\"').replace("%", "%%") + '"'

    root.mkdir(parents=True, exist_ok=True)
    service.write_text(
        "[Unit]\nDescription=Owner-directed managed storage sweep\n[Service]\nType=oneshot\n"
        f"WorkingDirectory={quote(ROOT)}\n"
        f"Environment={quote('LCTX_STORAGE_CONFIG=' + str(store.host_path))}\n"
        f"Environment={quote('LCTX_STORAGE_STATE=' + str(store.state))}\n"
        f"ExecStart={quote(uv)} run --no-project --offline --no-python-downloads "
        f"--python 3.14.7 python {quote(ROOT / 'scripts/storage.py')} "
        "sweep --scheduled --scope host --json\n"
    )
    timer.write_text(
        "[Unit]\nDescription=Daily managed storage catch-up\n[Timer]\nOnCalendar=daily\n"
        "Persistent=true\nRandomizedDelaySec=15min\n[Install]\nWantedBy=timers.target\n"
    )
    subprocess.run(["systemctl", "--user", "daemon-reload"], check=True)
    subprocess.run(["systemctl", "--user", "enable", "--now", timer.name], check=True)
    durable_json(store.state / "hooks-enabled.json", {"schema": 1, "enabled_at": now()})
    return {"outcome": "passed", "installed": True, "enabled": True}


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)

    def options(command, *, defaults=False):
        command.add_argument("--repo", type=Path, default=ROOT if defaults else argparse.SUPPRESS)
        command.add_argument(
            "--scope", choices=("repo", "host"), default="repo" if defaults else argparse.SUPPRESS
        )
        command.add_argument(
            "--json",
            action="store_true",
            default=False if defaults else argparse.SUPPRESS,
            help="emit structured JSON (also the default)",
        )

    options(result, defaults=True)
    commands = result.add_subparsers(dest="action", required=True)
    for name in ("status", "plan"):
        command = commands.add_parser(name)
        command.add_argument("ids", nargs="*")
    command = commands.add_parser("inventory")
    command.add_argument("--deep", action="store_true")
    for name in ("apply", "sweep"):
        command = commands.add_parser(name)
        command.add_argument("ids", nargs="+" if name == "apply" else "*")
        if name == "sweep":
            command.add_argument("--scheduled", action="store_true")
    command = commands.add_parser(
        "adopt", help="protected legacy registration; never invents release or historical deadlines"
    )
    command.add_argument("path", type=Path)
    command.add_argument("--owner", required=True)
    command.add_argument("--owner-path", type=Path)
    command.add_argument("--category", required=True)
    command = commands.add_parser(
        "manage", help="explicitly enroll a qualified legacy owner lifetime; existing holds remain"
    )
    command.add_argument("id")
    command = commands.add_parser("retain")
    command.add_argument("id")
    command.add_argument("--consumer", required=True)
    command.add_argument("--requires", required=True)
    command.add_argument(
        "--operation", action="append", default=[], help="required replay operation, repeatable"
    )
    command = commands.add_parser("release")
    command.add_argument("id")
    command.add_argument("--consumer", required=True)
    command.add_argument("--reason", required=True)
    for name in ("register-repo", "unregister-repo"):
        command = commands.add_parser(name)
        command.add_argument("path", type=Path)
    command = commands.add_parser("archive")
    command.add_argument("id")
    command.add_argument("--destination", type=Path)
    command = commands.add_parser("restore")
    command.add_argument("id")
    command.add_argument("--destination", type=Path, required=True)
    command = commands.add_parser("automation")
    command.add_argument("operation", choices=("install", "status", "disable"))
    for command in commands.choices.values():
        options(command)
    return result


def in_repository(row: dict, root: Path) -> bool:
    owner = row.get("owner", {})
    if owner.get("kind") in {"skill", "tool", "native"}:
        return False
    if owner.get("root"):
        return absolute(Path(owner["root"])) == root
    return any(
        absolute(Path(value)).is_relative_to(root)
        for value in (owner.get("path"), row.get("path"))
        if value
    )


def selected_ids(store: Storage, ids, scope: str) -> list[str]:
    if ids:
        for object_id in ids:
            if scope != "host" and not in_repository(store.get(object_id), store.root):
                raise Blocked("shared or foreign object requires explicit --scope host")
        return list(ids)
    return [
        row["id"] for row in store.records() if scope == "host" or in_repository(row, store.root)
    ]


def scoped_plan(store: Storage, ids, scope: str) -> dict:
    selected = selected_ids(store, ids, scope)
    return store.plan(selected) if selected else store.envelope([])


def apply_objects(store: Storage, ids: list[str], scope: str) -> dict:
    references = store.references()
    actions, outcome = [], "passed"
    for object_id in ids:
        try:
            selected_ids(store, [object_id], scope)
            action = store.retire(object_id, references=references)
            actions.append({**action, "outcome": "passed"})
        except Blocked as error:
            actions.append(
                {"id": object_id, "action": "blocked", "outcome": "blocked", "reason": str(error)}
            )
            if outcome == "passed":
                outcome = "blocked"
        except (Invalid, OSError, ValueError, RuntimeError) as error:
            actions.append(
                {"id": object_id, "action": "failed", "outcome": "failed", "reason": str(error)}
            )
            outcome = "failed"
    return store.envelope([], outcome=outcome, actions=actions)


def restore_verified(store: Storage, row: dict, destination: Path) -> dict:
    archive = row.get("archive") or {}
    if (
        not archive.get("path")
        or not archive.get("replay_qualified")
        or archive.get("replay", {}).get("isolation", {}).get("outcome") != "passed"
    ):
        raise Blocked("object has no qualified archive")
    import compile_profile

    path = absolute(Path(archive["path"]))
    # Keep archive admission through extraction. The fixed restore owner adds shared
    # admission for its freshly allocated receipt/raw roots and owns their lifetime.
    with admission([path], state=store.state):
        if physical(path) != archive.get("physical_identity"):
            raise Blocked("recorded archive physical identity changed")
        info = path.stat()
        if [info.st_size, info.st_mtime_ns] != archive.get("file_signature"):
            raise Blocked("recorded archive file signature changed")
        with path.open("rb") as stream:
            if hashlib.file_digest(stream, "sha256").hexdigest() != archive.get("sha256"):
                raise Blocked("recorded archive digest changed")
        return compile_profile.restore_profile(path, destination)


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    store: Storage | None = None
    try:
        store = Storage(root=args.repo)
        if args.scope != "host" and (
            args.action in {"register-repo", "unregister-repo"}
            or (args.action == "automation" and args.operation != "status")
        ):
            raise Blocked("shared effects require explicit --scope host")
        if args.action in {"manage", "retain", "release", "archive", "restore"}:
            selected_ids(store, [args.id], args.scope)
        if args.action in {"archive", "restore"} and store.root != ROOT:
            raise Blocked("profile effects require the owning checkout's storage CLI")
        if args.action in {"status", "plan"}:
            value = scoped_plan(store, args.ids, args.scope)
            if args.action == "status":
                value["last_sweep"] = (
                    read_json(store.state / "last-sweep.json")
                    if (store.state / "last-sweep.json").exists()
                    else None
                )
                value["capacity"] = dict(
                    zip(("total", "used", "free"), shutil.disk_usage(store.root), strict=True)
                )
                value["automation"] = automation(store, "status")
        elif args.action == "inventory":
            value = inventory(store, deep=args.deep, scope=args.scope)
        elif args.action == "adopt":
            proposed = {
                "path": str(absolute(args.path)),
                "owner": {"kind": args.owner, "path": str(absolute(args.owner_path or args.path))},
            }
            if args.scope != "host" and not in_repository(proposed, store.root):
                raise Blocked("shared adoption requires explicit --scope host")
            object_id = store.publish(
                args.path,
                args.category,
                {
                    "kind": args.owner,
                    "path": str(absolute(args.owner_path or args.path)),
                    "root": str(store.root),
                },
                "legacy-unresolved",
                requires="external-owner",
                managed=False,
            )
            value = store.plan([object_id])
        elif args.action == "manage":
            value = store.envelope([], actions=[store.manage_existing(args.id)])
        elif args.action == "retain":
            value = store.envelope(
                [], actions=[store.retain(args.id, args.consumer, args.requires, args.operation)]
            )
        elif args.action == "release":
            value = store.envelope([], actions=[store.release(args.id, args.consumer, args.reason)])
        elif args.action in {"register-repo", "unregister-repo"}:
            value = register(store, args.path, remove=args.action == "unregister-repo")
        elif args.action == "automation":
            value = automation(store, args.operation)
        elif args.action == "apply":
            value = apply_objects(store, args.ids, args.scope)
        elif args.action == "sweep":
            if args.scheduled:
                _qualified(store)
            private_directory(store.state)
            fd = os.open(store.state / "sweep.lock", os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
            try:
                try:
                    fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    value = store.envelope(
                        [], actions=[{"action": "skipped", "reason": "another sweep is running"}]
                    )
                else:
                    chosen = selected_ids(store, args.ids, args.scope)
                    value = store.sweep(chosen) if chosen else store.envelope([])
                    if not chosen:
                        durable_json(store.state / "last-sweep.json", value)
                        durable_json(store.state / "actions" / f"{uuid.uuid4()}.json", value)
            finally:
                os.close(fd)
        elif args.action == "archive":
            row = store.get(args.id)
            destination = args.destination or store.host.get("archive_destination")
            if not destination:
                raise Blocked("no qualified archive destination selected; original retained")
            if row["owner"]["kind"] != "profile":
                raise Blocked("archive requires a qualified profile owner")
            import compile_profile
            from storage_lifecycle import owner_guard

            raw_path = Path(row["path"])
            destination = absolute(Path(destination))
            if destination.is_dir():
                destination = destination / (args.id + ".tar.zst")
            required = tuple(
                sorted(
                    {
                        cap
                        for item in row["obligations"].values()
                        if item["requires"] == "restorable-replay" and not item.get("released_at")
                        for cap in item.get("operations", [])
                    }
                )
            )
            if not required:
                raise Blocked("explicit restorable consumer replay operations required")
            import compile_profile_tools

            try:
                readers = (
                    read_json(raw_path / "replay-readers.json")
                    if (raw_path / "replay-readers.json").exists()
                    else compile_profile_tools.prepare_reader_generation(required)
                )
            except (OSError, RuntimeError, ValueError) as error:
                raise Blocked("exact reader generation unavailable: " + str(error)) from error
            with (
                admission(
                    [
                        raw_path,
                        destination,
                        compile_profile.tools_root(),
                        Path(readers["generation_root"]),
                    ],
                    exclusive_paths=[raw_path, destination],
                    blocking=False,
                    state=store.state,
                ),
                owner_guard(row, store.host),
            ):
                if any(
                    not item.get("released_at")
                    and item["requires"] in {"immediate-use", "raw-replay"}
                    for item in row["obligations"].values()
                ):
                    raise Blocked("consumer still requires local raw/immediate availability")
                required = tuple(
                    sorted(
                        {
                            cap
                            for item in row["obligations"].values()
                            if item["requires"] == "restorable-replay"
                            and not item.get("released_at")
                            for cap in item.get("operations", [])
                        }
                    )
                )
                if not required:
                    raise Blocked("explicit restorable consumer replay operations required")
                value = compile_profile.archive_profile(
                    raw_path, destination, required, reader_binding=readers
                )
                if value.get("outcome") != "passed":
                    raise Blocked(
                        value.get("reason", "archive qualification failed; original retained")
                    )
                archive_path = Path(value["path"])
                value["physical_identity"] = physical(archive_path)
                info = archive_path.stat()
                value["file_signature"] = [info.st_size, info.st_mtime_ns]
                # Archive publication never deletes local raw. Its separate release is evaluated
                # afresh, including all existing consumer obligations and grace.
                store.publish(
                    archive_path,
                    "recovery-archive",
                    {"kind": "external", "path": str(archive_path)},
                    f"archive:{args.id}",
                    requires="restorable-replay",
                )
                with store.metadata():
                    current = store.get(args.id)
                    current["archive"] = value
                    store.save(current)
        else:
            value = restore_verified(store, store.get(args.id), args.destination)
        value.setdefault("schema", 1)
        value.setdefault("observed_at", now())
        value.setdefault("policy_revision", store.revision)
        value.setdefault("scope", {"repository": str(store.root), "state": str(store.state)})
        value["scope"]["selection"] = args.scope
        value["operation"] = args.action
        print(json.dumps(value, indent=2, default=str))
        return {"passed": 0, "blocked": 75}.get(value.get("outcome", "passed"), 1)
    except Blocked as error:
        print(
            json.dumps(
                {
                    "schema": 1,
                    "outcome": "blocked",
                    "reason": str(error),
                    "operation": args.action,
                    "observed_at": now(),
                    "policy_revision": store.revision if store is not None else None,
                    "scope": {"repository": str(absolute(args.repo)), "selection": args.scope},
                }
            )
        )
        return 75
    except (Invalid, OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(
            json.dumps(
                {
                    "schema": 1,
                    "outcome": "failed",
                    "reason": str(error),
                    "operation": args.action,
                    "observed_at": now(),
                    "policy_revision": store.revision if store is not None else None,
                    "scope": {"repository": str(absolute(args.repo)), "selection": args.scope},
                }
            )
        )
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
