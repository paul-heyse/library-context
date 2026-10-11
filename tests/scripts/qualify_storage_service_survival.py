"""Opt-in SM8 acquisition: remove one owned checkout and observe the installed service.

Run through ``just run -- just fixture -- uv run --no-sync python ... --execute``.
This never installs, restarts, backs up, restores, selects or retires native content.
The supplied publication and current approved protected archive must already exist.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import uuid
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

# Standalone execution does not inherit pytest's configured scripts import path.
import runs  # noqa: E402
import storage_service as assets  # noqa: E402
import surrealdb_fixture as fixture  # noqa: E402
import surrealdb_service as service  # noqa: E402
import worktree  # noqa: E402
from storage_lifecycle import (  # noqa: E402
    Storage, admission, durable_json, fsync_directory, physical, private_directory,
)


def validation_handle(value: dict, installation: service.Installation) -> None:
    if (value.get("database") != {"namespace": service.NAMESPACE, "database": "validation"}
            or value.get("service_generation") != bytes(installation.record["service_generation"]).hex()):
        raise service.FixtureBlocked("scope", "supply an exact retained installed validation publication")


def checkout_state(target: Path, branch: str) -> dict:
    """Read Git's owner, including a branch-only or interrupted worktree add."""
    listing = worktree.git("worktree", "list", "--porcelain", "-z", cwd=ROOT).stdout
    registrations = []
    for block in listing.split(b"\0\0"):
        fields = dict(field.decode().split(" ", 1) for field in block.split(b"\0") if b" " in field)
        if fields.get("worktree") == str(target) or fields.get("branch") == "refs/heads/" + branch:
            registrations.append(fields)
    tip = worktree.git("rev-parse", "--verify", "--quiet", "refs/heads/" + branch,
                       cwd=ROOT, check=False)
    return {"path_present": target.exists() or target.is_symlink(),
            "branch_tip": tip.stdout.decode().strip() if tip.returncode == 0 else None,
            "registrations": registrations}


def checkout_intent(target: Path, branch: str, commit: str) -> dict:
    private_directory(target.parent)
    state = checkout_state(target, branch)
    if state != {"path_present": False, "branch_tip": None, "registrations": []}:
        raise RuntimeError("disposable checkout namespace is not fresh")
    return {"target": str(target), "branch": branch, "commit": commit,
            "parent_identity": physical(target.parent), "before": state,
            "common_git_directory": worktree.git("rev-parse", "--path-format=absolute", "--git-common-dir",
                                                 cwd=ROOT).stdout.decode().strip()}


def remove_owned_checkout(intent: dict, report) -> dict:
    """Reconcile partial creation; unknown or changed resources have no deletion authority."""
    target, branch, commit = Path(intent["target"]), intent["branch"], intent["commit"]
    if physical(target.parent) != intent["parent_identity"]:
        raise RuntimeError("owned checkout parent identity changed; cleanup retained")
    state = checkout_state(target, branch)
    if state == {"path_present": False, "branch_tip": None, "registrations": []}:
        return state
    if not state["path_present"]:
        # The generic remover prunes all stale Git registrations when a target
        # is missing. This one-checkout owner cannot authorize that operation.
        raise RuntimeError("missing checkout has retained branch or registration; exact root resolution required")
    if state["branch_tip"] != commit or target.is_symlink():
        raise RuntimeError("owned checkout branch or path identity changed; cleanup retained")
    expected = {"worktree": str(target), "HEAD": commit, "branch": "refs/heads/" + branch}
    if state["registrations"] not in ([], [expected]):
        raise RuntimeError("owned checkout registration changed; cleanup retained")
    if state["registrations"] != [expected]:
        raise RuntimeError("partial checkout path has no exact Git ownership; cleanup retained")
    if "target_identity" in intent and physical(target) != intent["target_identity"]:
        raise RuntimeError("owned checkout physical identity changed; cleanup retained")
    common = worktree.git("rev-parse", "--path-format=absolute", "--git-common-dir", cwd=target).stdout.decode().strip()
    if common != intent["common_git_directory"]:
        raise RuntimeError("owned checkout Git owner changed; cleanup retained")
    if worktree.remove(target.name, root=ROOT, base=target.parent, into=commit, report=report):
        raise RuntimeError("guarded checkout cleanup refused; retained for explicit owner resolution")
    after = checkout_state(target, branch)
    if after != {"path_present": False, "branch_tip": None, "registrations": []}:
        raise RuntimeError("owned checkout cleanup remains incomplete")
    return after


def retain_checkout_run(run: Path, output: Path) -> dict:
    """Protect interrupted creation in both existing run and storage lifecycle owners."""
    storage = Storage()
    rows = [row for row in storage.records() if row["path"] == str(run.resolve())
            and row["category"] == "run-receipt" and not row.get("retired_at")
            and row["owner"] == {"kind": "run", "path": str(run.resolve())}
            and row["identity"] == physical(run)]
    if len(rows) != 1:
        raise RuntimeError("live run has no exact storage lifetime; checkout creation refused")
    consumer = "sm8-checkout-resolution:" + str(output / "receipt.json")
    storage.retain(rows[0]["id"], consumer)
    marker = run / runs.RETAIN
    owned = not (marker.exists() or marker.is_symlink())
    content = {"consumer": consumer, "receipt": str(output / "receipt.json")}
    if owned:
        fd = os.open(marker, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "w") as stream:
            json.dump(content, stream, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        fsync_directory(run)
    return {"storage_id": rows[0]["id"], "consumer": consumer,
            "marker_owned": owned, "marker_content": content}


def release_checkout_run(run: Path, hold: dict) -> None:
    marker = run / runs.RETAIN
    if hold["marker_owned"] and (marker.is_symlink() or json.loads(marker.read_bytes()) != hold["marker_content"]):
        raise RuntimeError("owned run retention marker changed; resolution hold retained")
    Storage().release(hold["storage_id"], hold["consumer"], "exact disposable checkout cleanup confirmed")
    if hold["marker_owned"]:
        marker.unlink()
        fsync_directory(run)


def relation_rows(details: dict, relation: str, limit: int) -> int:
    views = [view for view in details["views"] if view["relation"] == relation]
    if len(views) != 1:
        raise ValueError("the witness must have one exact completed view for the relation")
    count = views[0]["rows"]
    if type(count) is not int or not 0 < count <= limit:
        raise ValueError("use a nonempty retained relation that fits the explicit read limit")
    return count


def same_publication(before: dict, after: dict) -> bool:
    def row_bodies(rows):
        return sorted(json.dumps(row, sort_keys=True, separators=(",", ":")) for row in rows)

    return (
        before["audit"] == after["audit"]
        and before["details"] == after["details"]
        and row_bodies(before["rows"]) == row_bodies(after["rows"])
    )


def command_json(argv: list[str], output: Path, name: str, *, cwd: Path) -> Any:
    result = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, check=False)
    durable_json(output / (name + "-command.json"), {
        "argv": argv, "returncode": result.returncode,
        "stdout": result.stdout, "stderr": result.stderr,
    })
    if result.returncode:
        raise RuntimeError(f"{name} failed with exit {result.returncode}; see owned command receipt")
    return json.loads(result.stdout)


def publication(installer: Path, runtime: Path, handle: Path, relation: str, limit: int,
                output: Path, phase: str, *, cwd: Path) -> dict:
    prefix = [str(installer), "snapshot", "--runtime-config", str(runtime)]
    audit = command_json([*prefix, "audit", str(handle)], output, phase + "-audit", cwd=cwd)
    if audit.get("audit") != "passed" or audit.get("handle") != json.loads(handle.read_bytes()):
        raise RuntimeError("native audit did not establish the exact supplied publication")
    details = command_json([*prefix, "show", "--handle", str(handle)], output,
                           phase + "-show", cwd=cwd)
    count = relation_rows(details, relation, limit)
    rows = command_json([*prefix, "query", relation, "--limit", str(limit), "--handle", str(handle)],
                        output, phase + "-rows", cwd=cwd)
    if not isinstance(rows, list) or len(rows) != count:
        raise RuntimeError("native read did not recover the entire nonempty witness relation")
    return {"audit": audit, "details": details, "rows": rows}


def observe_service(installation: service.Installation) -> dict:
    health = installation.check()
    dependencies = assets.dependencies(installation)
    if not dependencies["stable_installer"]:
        raise service.FixtureBlocked("installer", "service-owned installer transfer required",
                                     "just service maintenance --stabilize-installer")
    installer = Path(installation.record["installer"])
    server = service._installed_binary(installation)
    installer_sha = assets.digest(installer)
    if installer_sha != installation.record["installer_generation"]["sha256"]:
        raise service.FixtureBlocked("identity", "installed maintenance executable changed")
    server_sha = service.file_sha256(server)
    if server_sha != installation.record["binary"]["sha256"]:
        raise service.FixtureBlocked("identity", "installed server executable changed")
    configurations = [installation.directory / "installation.json"]
    configurations += [installation.runtime_path(db, installer=role)
                       for db in service.DATABASES for role in (False, True)]
    return {
        "health": health, "dependencies": dependencies,
        "installer": {"path": str(installer), "sha256": installer_sha},
        "server": {"path": str(server), "sha256": server_sha},
        "daemon": service.unit_properties(service.UNIT, "MainPID", "InvocationID",
                                          "ExecMainStartTimestampMonotonic"),
        "configuration_sha256": {str(path): service.file_sha256(path) for path in configurations},
    }


def approved_recovery(installation: service.Installation) -> tuple[Path, dict]:
    pointer = installation.record.get("recovery")
    if not isinstance(pointer, dict) or not isinstance(pointer.get("archive"), str):
        raise service.FixtureBlocked("recovery", "current approved protected archive required")
    archive = Path(pointer["archive"])
    if not archive.is_absolute():
        raise service.FixtureBlocked("recovery", "approved archive must have an absolute path")
    try:
        info = archive.stat()
        digest = service.file_sha256(archive)
    except OSError as error:
        raise service.FixtureBlocked("recovery", "approved protected archive unavailable") from error
    if digest != pointer.get("sha256"):
        raise service.FixtureBlocked("recovery", "approved protected archive identity differs")
    # Existing owner validates every archived file/configuration/executable and removes
    # only its private staging. It never applies a restore without explicit apply=True.
    observation = service.restore_service(installation, archive, apply=False)
    if observation.get("applied") is not False or observation.get("sha256") != pointer["sha256"]:
        raise RuntimeError("protected recovery dry-run did not validate the approved bytes")
    after = archive.stat()
    def physical(row):
        return row.st_dev, row.st_ino, row.st_size, row.st_mtime_ns, row.st_mode

    if physical(info) != physical(after):
        raise RuntimeError("protected archive physical identity changed during validation")
    observation["physical_identity"] = physical(after)
    return archive, observation


def qualify(handle: Path, relation: str, limit: int = 128, *, execute: bool = False) -> dict:
    if not execute:
        return {"outcome": "not_run", "reason": "explicit --execute required after root review"}
    run = runs.current_run()
    if run is None:
        raise service.FixtureBlocked("run", "live just run owner and LCTX_RUN_DIR required")
    if not 0 < limit <= 128:
        raise ValueError("the retained small-case read limit must be in 1..128")
    if any(not os.environ.get(key) for key in (
        "LCTX_SURREAL_TEST_CONFIG", "LCTX_COMPILER_RUNTIME_CONFIG",
    )):
        raise service.FixtureBlocked("scope", "live just fixture validation attachment required")
    installation = service.Installation.load()
    server = fixture.Server.open(installation.id)
    attached = Path(os.environ["LCTX_SURREAL_TEST_CONFIG"]).resolve(strict=True)
    fixture.inspection_scope(server, attached)
    runtime = Path(os.environ["LCTX_COMPILER_RUNTIME_CONFIG"]).resolve(strict=True)
    if runtime != attached.parent / "compiler-runtime.json":
        raise service.FixtureBlocked("scope", "use the live validation attachment runtime")
    expected = {**installation.runtime(),
                "selection": str(attached.parent / "scratch" / "selected.json")}
    if json.loads(runtime.read_bytes()) != expected:
        raise service.FixtureBlocked("scope", "attachment compiler runtime differs from installed validation")
    handle = handle.resolve(strict=True)
    output = run / "sm8-service-survival"
    if output.exists() or output.is_symlink():
        raise ValueError("this run already has survival output; preserve it and use a new run")
    with admission([output, handle]), service.borrow(installation):
        handle_bytes = handle.read_bytes()
        validation_handle(json.loads(handle_bytes), installation)
        private_directory(output)
        witness = output / "handle.json"
        witness.write_bytes(handle_bytes)
        name = "sm8-survival-" + uuid.uuid4().hex[:16]
        base = output / "checkouts"
        target = base / name
        commit = worktree.git("rev-parse", "HEAD", cwd=ROOT).stdout.decode().strip()
        receipt: dict[str, Any] = {
            "schema": 1, "outcome": "running", "source_commit": commit,
            "checkout": str(target), "branch": "wt/" + name,
            "publication_handle": json.loads(handle_bytes), "relation": relation, "limit": limit,
            "checkout_creation_started": False, "checkout_created": False, "checkout_removed": False,
            "scope": "actual checkout removal, exact daemon/executable survival, retained publication read and protected recovery dry-run",
            "outside_scope": ["cold restore application", "service restart", "automation activation"],
        }
        errors: list[BaseException] = []
        messages: list[str] = []
        try:
            before = observe_service(installation)
            receipt["service_before"] = before
            archive, recovery_before = approved_recovery(installation)
            receipt["recovery_before"] = recovery_before
            durable_json(output / "receipt.json", receipt)
            intent = checkout_intent(target, receipt["branch"], commit)
            receipt["checkout_intent"] = intent
            durable_json(output / "receipt.json", receipt)
            receipt["resolution_hold"] = retain_checkout_run(run, output)
            receipt["checkout_creation_started"] = True
            # Durable ownership and indefinite retention precede git worktree add.
            durable_json(output / "receipt.json", receipt)
            code = worktree.create(name, ref=commit, root=ROOT, base=base,
                                   prepare=False, build_dir="shared", report=messages.append)
            if code:
                raise RuntimeError("disposable checkout creation failed; no forced cleanup")
            receipt["checkout_created"] = True
            intent["target_identity"] = physical(target)
            durable_json(output / "receipt.json", receipt)
            with admission([target]):
                copy = target / "target" / "sm8-lctx"
                if worktree.git("check-ignore", "-q", str(copy.relative_to(target)),
                                cwd=target, check=False).returncode:
                    raise RuntimeError("disposable executable path is not ignored by this checkout")
                copy.parent.mkdir(exist_ok=True)
                shutil.copyfile(before["installer"]["path"], copy)
                copy.chmod(0o500)
                if service.file_sha256(copy) != before["installer"]["sha256"]:
                    raise RuntimeError("owned checkout copy differs from the installed executable")
                receipt["publication_before"] = publication(copy, runtime, witness, relation, limit,
                                                            output, "before", cwd=target)
            receipt["checkout_cleanup_after"] = remove_owned_checkout(intent, messages.append)
            receipt["checkout_removed"] = True
            current = service.Installation.load()
            receipt["service_after"] = observe_service(current)
            if receipt["service_after"] != before:
                raise RuntimeError("exact service identity, health or recovery dependencies changed")
            receipt["publication_after"] = publication(Path(current.record["installer"]), runtime,
                                                       witness, relation, limit, output, "after", cwd=ROOT)
            if not same_publication(receipt["publication_before"], receipt["publication_after"]):
                raise RuntimeError("retained publication or complete witness relation changed")
            after_archive, recovery_after = approved_recovery(current)
            receipt["recovery_after"] = recovery_after
            if after_archive != archive or recovery_after != recovery_before:
                raise RuntimeError("approved protected recovery capability changed")
            if handle.read_bytes() != handle_bytes:
                raise RuntimeError("the original supplied publication handle changed")
        except BaseException as error:
            errors.append(error)
        finally:
            if receipt["checkout_creation_started"] and not receipt["checkout_removed"]:
                try:
                    receipt["checkout_cleanup_before"] = checkout_state(target, receipt["branch"])
                    receipt["checkout_cleanup_after"] = remove_owned_checkout(receipt["checkout_intent"], messages.append)
                    receipt["checkout_removed"] = True
                except BaseException as error:
                    errors.append(error)
            blocked = errors and not receipt["checkout_created"] and all(
                isinstance(error, service.FixtureBlocked) for error in errors
            )
            outcome = "blocked" if blocked else "failed" if errors else "passed"
            receipt.update(outcome=outcome, worktree_messages=messages,
                           errors=[f"{type(error).__name__}: {error}" for error in errors])
            durable_json(output / "receipt.json", receipt)
            # Failed qualification retains its named hold even when cleanup succeeded.
            # A successful receipt must already be durable before authorizing release.
            if not errors and receipt["checkout_removed"]:
                try:
                    release_checkout_run(run, receipt["resolution_hold"])
                except BaseException as error:
                    errors.append(error)
                    receipt["resolution_hold"] = retain_checkout_run(run, output)
                    receipt.update(outcome="failed", errors=[f"{type(item).__name__}: {item}" for item in errors])
                    durable_json(output / "receipt.json", receipt)
        if errors:
            if len(errors) == 1:
                raise errors[0]
            raise BaseExceptionGroup("SM8 survival acquisition or owned cleanup failed", errors)
        return {"outcome": "passed", "receipt": str(output / "receipt.json")}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--execute", action="store_true")
    parser.add_argument("--handle", required=True, type=Path)
    parser.add_argument("--relation", required=True)
    parser.add_argument("--limit", type=int, default=128)
    args = parser.parse_args(argv)
    try:
        result = qualify(args.handle, args.relation, args.limit, execute=args.execute)
    except service.FixtureBlocked as error:
        print(json.dumps({"outcome": "blocked", "reason": error.message()}))
        return service.EXIT_BLOCKED
    except Exception as error:
        print(json.dumps({"outcome": "failed", "reason": str(error)}))
        return 1
    print(json.dumps(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
