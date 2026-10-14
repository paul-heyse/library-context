"""Owned SurrealKit development workers; existing services/runs remain the authorities."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path
from typing import Any

import runs
import surrealdb_service as service
from harness import ProcessIdentity, read_json
from storage_lifecycle import Storage, admission, fsync_directory, physical

TOKEN_RESTORE_JOURNAL = "sample-token-restore.json"
TOKEN_MARKER = "LCTX_WORKER_ISSUED_TOKEN"


def worker_build_info(worker: Path) -> dict[str, Any]:
    """Bind a read-only compiled response to this physical worker, without a service."""
    try:
        before = service.file_sha256(worker)
        response = subprocess.run(
            [str(worker), "build-info"], capture_output=True, timeout=40, check=False
        )
        value = json.loads(response.stdout) if response.returncode == 0 else None
        after = service.file_sha256(worker)
    except OSError, ValueError, subprocess.TimeoutExpired:
        raise service.FixtureBlocked(
            "worker", "worker build-info unavailable; rebuild lctx-surreal-dev"
        ) from None
    if after != before or service.development_build_info(value)["status"] != "recorded":
        raise service.FixtureBlocked(
            "worker", "worker build-info invalid or executable changed; rebuild lctx-surreal-dev"
        )
    return {"worker_sha256": before, "recorded_at": runs.now(), "build_info": value}


def protect_token_input(installation: service.Installation, run_path: Path) -> dict[str, str]:
    consumer = f"lifecycle-token-input:{installation.id}:{run_path.name}"
    receipt = Storage().publish(
        run_path, "run-receipt", {"kind": "run", "path": str(run_path)}, consumer, managed=True
    )
    return {"receipt": receipt, "consumer": consumer}


def _token_input_storage(
    installation: service.Installation, run_path: Path, reference: Any
) -> tuple[Storage, str, str]:
    expected_consumer = f"lifecycle-token-input:{installation.id}:{run_path.name}"
    if (
        not isinstance(reference, dict)
        or set(reference) != {"receipt", "consumer"}
        or reference.get("consumer") != expected_consumer
        or not isinstance(reference.get("receipt"), str)
    ):
        raise service.FixtureBlocked("ownership", "token recovery storage consumer differs")
    storage = Storage()
    row = storage.get(reference["receipt"])
    if (
        row.get("path") != str(run_path)
        or row.get("category") != "run-receipt"
        or row.get("owner") != {"kind": "run", "path": str(run_path)}
        or row.get("identity") != physical(run_path)
    ):
        raise service.FixtureBlocked("ownership", "token recovery storage owner differs")
    return storage, reference["receipt"], expected_consumer


def _physical_owned(path: Path, *, directory: bool = False, private: bool = False) -> None:
    if (
        not path.is_absolute()
        or ".." in path.parts
        or any(part.is_symlink() for part in (path, *path.parents))
    ):
        raise service.FixtureBlocked("ownership", "token recovery requires physical owned paths")
    info = path.lstat()
    if (
        info.st_uid != os.getuid()
        or not (stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode))
        or (private and info.st_mode & 0o077)
    ):
        raise service.FixtureBlocked(
            "ownership", "token recovery path ownership or privacy differs"
        )


def restore_token_input(
    installation: service.Installation, ledger: Path, *, parent_owned: bool = False
) -> dict[str, str]:
    """Restore only a ledger-bound worker input, after its actual descendants are drained."""
    service.lifecycle_campaign(installation, installation.id)
    _physical_owned(ledger, directory=True)
    record_path = ledger / "record.json"
    _physical_owned(record_path, private=True)
    row = read_json(record_path)
    generation = bytes(installation.record["service_generation"]).hex()
    if (
        not isinstance(row, dict)
        or row.get("installation_id") != installation.id
        or row.get("server_generation") != generation
        or not isinstance(row.get("run_path"), str)
        or ledger.parent != installation.directory / "development"
    ):
        raise service.FixtureBlocked("ownership", "token recovery ledger differs from campaign")
    run_path = Path(row["run_path"])
    if run_path.name != ledger.name:
        raise service.FixtureBlocked("ownership", "token recovery run differs from ledger")
    _physical_owned(run_path, directory=True)
    project = run_path / "project"
    journal = project / TOKEN_RESTORE_JOURNAL
    # Existing projects never mutated under this protocol have no recovery journal.
    # They are not adopted, scanned for token-like strings or rewritten.
    if any(part.is_symlink() for part in (journal, *journal.parents)):
        raise service.FixtureBlocked("ownership", "token recovery paths cannot be symlinks")
    if not journal.exists() and row.get("credential_storage") is None:
        return {"status": "not_applicable"}
    with admission([run_path, ledger]):
        for path in (run_path / runs.RECORD, run_path / "target-binding.json"):
            _physical_owned(path, private=True)
        run = runs.load_record(run_path)
        if (
            run is None
            or run.get("id") != run_path.name
            or run.get("owner") != row.get("owner")
            or runs.cleanup_protected(run)
        ):
            raise service.FixtureBlocked(
                "drainage", "token recovery requires confirmed child cleanup"
            )
        live_parent = runs.owner_alive(run_path, run)
        if live_parent and not (
            parent_owned and run.get("owner") == ProcessIdentity.of().to_json()
        ):
            raise service.FixtureBlocked("drainage", "token recovery has a live foreign parent")
        binding = read_json(run_path / "target-binding.json")
        expected = {
            "format": 1,
            "purpose": service.LIFECYCLE_PURPOSE,
            "target_kind": "campaign",
            "installation_id": installation.id,
            "server_generation": generation,
            "runtime_config": str(installation.runtime_path("validation", installer=True)),
            "namespace": service.NAMESPACE,
            "database": row.get("database"),
            "allowed_operations": ["test"],
            "project_dir": str(project),
            "report_path": str(project / "test-report.json"),
        }
        if binding != expected or row.get("operation") != "test":
            raise service.FixtureBlocked(
                "ownership", "token recovery binding differs from worker ledger"
            )
        storage, receipt, consumer = _token_input_storage(
            installation, run_path, row.get("credential_storage")
        )
        if not journal.exists():
            storage.release(
                receipt, consumer, "confirmed child drainage; no token recovery journal"
            )
            return {"status": "not_applicable"}
        for path in (project, project / "tests"):
            _physical_owned(path, directory=True)
        config = project / "tests/config.toml"
        for path in (journal, config):
            _physical_owned(path, private=True)
        recovery = read_json(journal)
        if (
            not isinstance(recovery, dict)
            or set(recovery)
            != {
                "schema",
                "purpose",
                "installation_id",
                "server_generation",
                "project_dir",
                "config_path",
                "original",
            }
            or recovery.get("schema") != 1
            or recovery.get("purpose") != "surrealdb-lifecycle-token-restore"
            or recovery.get("installation_id") != installation.id
            or recovery.get("server_generation") != generation
            or recovery.get("project_dir") != str(project)
            or recovery.get("config_path") != str(config)
            or not isinstance(recovery.get("original"), str)
            or TOKEN_MARKER not in recovery["original"]
        ):
            raise service.FixtureBlocked(
                "ownership", "token recovery journal differs from exact owned input"
            )
        # Atomic replacement does not follow a substituted config leaf. All parent
        # directories were authenticated after drainage; the source project is absent.
        fd, temporary = tempfile.mkstemp(prefix=".token-restore-", dir=config.parent)
        try:
            with os.fdopen(fd, "wb") as stream:
                stream.write(recovery["original"].encode())
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temporary, config)
            fsync_directory(config.parent)
        finally:
            Path(temporary).unlink(missing_ok=True)
        journal.unlink()
        fsync_directory(project)
        storage.release(receipt, consumer, "confirmed child drainage and token input restoration")
        return {"status": "passed"}


def restore_campaign_token_inputs(installation: service.Installation) -> None:
    """Idempotent fallback through the existing campaign development inventory."""
    for ledger in sorted((installation.directory / "development").glob("*/record.json")):
        restore_token_input(installation, ledger.parent)


def copy_tests(source: Path, destination: Path) -> None:
    """Copy only declared test inputs; environments and schema authority do not transfer."""
    tests = source / "tests"
    if not tests.is_dir() or tests.is_symlink():
        raise service.FixtureBlocked("inputs", "test project requires a physical tests directory")
    for path in sorted(tests.rglob("*")):
        if path.is_symlink():
            raise service.FixtureBlocked("inputs", "test inputs cannot contain symlinks")
        if path.is_file() and path.suffix in {".surql", ".toml"}:
            target = destination / "tests" / path.relative_to(tests)
            target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
            shutil.copyfile(path, target)
            target.chmod(0o600)


def run_worker(
    installation: service.Installation,
    identity: str,
    operation: str,
    worker: Path,
    *,
    source: Path | None = None,
    modules: str | None = None,
) -> dict[str, Any]:
    service.lifecycle_campaign(installation, identity)
    if operation not in {"sync", "test"}:
        raise service.FixtureBlocked("scope", "unsupported development operation")
    if operation == "sync" and source is not None:
        raise service.FixtureBlocked(
            "scope", "Sync consumes authoritative embedded schema, not a test project"
        )
    worker = worker.absolute()
    if (
        any(path.is_symlink() for path in (worker, *worker.parents))
        or not worker.is_file()
        or not os.access(worker, os.X_OK)
    ):
        raise service.FixtureBlocked(
            "worker", "build the physical worker with cargo build --release -p lctx-surreal-dev"
        )
    resources = [installation.directory, worker]
    if source is not None:
        source = source.absolute()
        if any(path.is_symlink() for path in (source, *source.parents)):
            raise service.FixtureBlocked("inputs", "test project must have a physical path")
        resources.append(source)
    with admission(resources), service.borrow(installation):
        provenance = worker_build_info(worker)
        run_path = runs.new_run_dir()
        with admission([run_path]):
            project = run_path / "project"
            project.mkdir(mode=0o700)
            if source is not None:
                copy_tests(source, project)
            generation = bytes(installation.record["service_generation"]).hex()
            database = "lifecycle_" + uuid.uuid4().hex
            report = project / "test-report.json"
            binding_path = run_path / "target-binding.json"
            service.private_json(
                binding_path,
                {
                    "format": 1,
                    "purpose": service.LIFECYCLE_PURPOSE,
                    "target_kind": "campaign",
                    "installation_id": identity,
                    "server_generation": generation,
                    "runtime_config": str(installation.runtime_path("validation", installer=True)),
                    "namespace": service.NAMESPACE,
                    "database": database,
                    "allowed_operations": [operation],
                    "project_dir": str(project),
                    "report_path": str(report),
                },
            )
            ledger = installation.directory / "development" / run_path.name
            ledger.mkdir(parents=True, mode=0o700)
            row = {
                "schema": 1,
                "installation_id": identity,
                "server_generation": generation,
                "database": database,
                "operation": operation,
                "run_path": str(run_path),
                "owner": ProcessIdentity.of().to_json(),
                "worker_sha256": provenance["worker_sha256"],
                "worker_provenance": provenance,
                "recorded_at": runs.now(),
                "outcome": "unknown",
                "report": {"status": "not_created"},
                "credential_cleanup": {"status": "unknown"},
            }
            if operation == "test":
                row["credential_storage"] = protect_token_input(installation, run_path)
            service.private_json(ledger / "record.json", row)
            argv = [str(worker), operation, "--binding", str(binding_path)]
            if operation == "test":
                argv += ["--prepare-project"]
                if source is None:
                    argv += ["--sample-suite"]
            if modules:
                argv += ["--modules", modules]
            owner = runs.Owner(
                run_path,
                argv,
                label="surrealdb-lifecycle-" + operation,
                cwd=str(service.ROOT),
                mode="foreground",
                stream=True,
            )
            # A recoverable run record and target ledger precede the first DDL effect.
            owner.write()
            try:
                if service.file_sha256(worker) != provenance["worker_sha256"]:
                    raise service.FixtureBlocked("worker", "worker changed after build-info")
                cfg = installation.runtime("validation", installer=True)
                service.sql(installation, f"DEFINE DATABASE {database} STRICT", cfg=cfg)
                code = owner.run()
                record = runs.load_record(run_path)
                cleanup = runs.cleanup_of(record)
                host = runs.host_cleanup(run_path, record)
                row["credential_cleanup"] = restore_token_input(
                    installation, ledger, parent_owned=True
                )
                capture = "not_applicable" if operation != "test" else "failed"
                if report.is_file() and not report.is_symlink():
                    try:
                        parsed = json.loads(report.read_text())
                        if isinstance(parsed, (dict, list)):
                            capture = "passed"
                    except OSError, ValueError:
                        pass
                row.update(
                    recorded_at=runs.now(),
                    outcome="passed"
                    if code == 0
                    and host["status"] == "confirmed"
                    and capture in {"passed", "not_applicable"}
                    else "failed",
                    command_exit=code,
                    child_cleanup=cleanup,
                    host_cleanup=host,
                    report={
                        "status": capture,
                        "path": str(report) if operation == "test" else None,
                    },
                    remote={
                        "status": "unknown",
                        "reason": "target remains owned until whole-executor disposal",
                    },
                )
                service.private_json(ledger / "record.json", row)
                summary = {
                    "outcome": row["outcome"],
                    "installation_id": identity,
                    "run": run_path.name,
                    "command_exit": code,
                    "child_cleanup": cleanup,
                    "host_cleanup": host,
                    "credential_cleanup": row["credential_cleanup"],
                    "report": row["report"],
                    "remote": row["remote"],
                    "disposal": "not_run",
                    "release_route": f"just service dispose-campaign {identity}",
                }
                service.private_json(run_path / "summary.json", summary)
                return summary
            except BaseException:
                row["outcome"] = "failed"
                row["recorded_at"] = runs.now()
                # A pre-spawn DDL failure has no worker descendants. Keep the actual
                # native uncertainty on the target until checked whole-target disposal.
                if owner.record["child"] is None:
                    owner.record.update(
                        phase="ended",
                        termination="completed",
                        cleanup={"status": "confirmed", "survivors": []},
                    )
                    owner.write()
                try:
                    row["credential_cleanup"] = restore_token_input(
                        installation, ledger, parent_owned=True
                    )
                finally:
                    service.private_json(ledger / "record.json", row)
                raise


def inspect_campaign(
    installation: service.Installation, identity: str, indexes: bool
) -> dict[str, Any]:
    service.lifecycle_campaign(installation, identity)
    result: dict[str, Any] = {
        "installation_id": identity,
        "generation": bytes(installation.record["service_generation"]).hex(),
        "owner": "surrealdb_service",
        "source": "installation-and-development-ledgers",
        "observed_at": service.time.time(),
        "phase": installation.record["phase"],
        "workers": service.development_workers(installation),
        "borrowers": service.borrowers(installation),
        "native": {"status": "not_inspected", "route": "docs/surrealdb.md#maintenance-diagnostics"},
        "server": {
            key: installation.record.get("binary", {}).get(key)
            for key in ("version", "sha256", "generation")
        },
        "server_source": "installation-descriptor; use just service status for current host observation",
        "capabilities": {
            "source": "installation-descriptor",
            "declared": installation.record.get("capabilities"),
        },
        "disposal": None,
    }
    disposal = read_json(installation.directory / "disposal.json")
    if disposal is not None:
        result["disposal"] = service._campaign_disposal_projection(disposal)
    if indexes:
        # Fixed diagnostic reads only. EXPLAIN ANALYZE would execute a workload and
        # is deliberately absent from this read-only preset.
        with service.borrow(installation):
            cfg = installation.runtime("validation", installer=True)
            result["native"] = {
                "status": "observed",
                "database": "validation",
                "server": installation.record["binary"]["version"],
                "membership_schema": service.sql(
                    installation, "INFO FOR TABLE compiler_membership", cfg=cfg
                )[0]["result"],
                "membership_plan": service.sql(
                    installation,
                    "SELECT node FROM compiler_membership WITH INDEX contribution_rows "
                    "WHERE contribution=compiler_contribution:diagnostic "
                    "AND relation='TypedModule' ORDER BY semantic_key LIMIT 4096 EXPLAIN",
                    cfg=cfg,
                )[0]["result"],
                "audit_membership_schema": service.sql(
                    installation, "INFO FOR TABLE compiler_membership", cfg=cfg
                )[0]["result"],
                "audit_membership_plan": service.sql(
                    installation,
                    "SELECT * FROM compiler_membership WITH INDEX contribution_nodes "
                    "WHERE contribution=compiler_contribution:diagnostic "
                    "AND node=entity:diagnostic EXPLAIN",
                    cfg=cfg,
                )[0]["result"],
                "retirement_schema": service.sql(
                    installation, "INFO FOR TABLE native_retirement_item", cfg=cfg
                )[0]["result"],
                "retirement_pending_plan": service.sql(
                    installation,
                    "SELECT id,job,object,incarnation,state,successor "
                    "FROM native_retirement_item WITH INDEX retirement_queue_v4 "
                    "WHERE job=native_retirement:diagnostic AND state='pending' "
                    "ORDER BY id LIMIT 128 EXPLAIN",
                    cfg=cfg,
                )[0]["result"],
                "retirement_retiring_plan": service.sql(
                    installation,
                    "SELECT id,job,object,incarnation,state,successor "
                    "FROM native_retirement_item WITH INDEX retirement_queue_v4 "
                    "WHERE job=native_retirement:diagnostic AND state='retiring' "
                    "ORDER BY id LIMIT 128 EXPLAIN",
                    cfg=cfg,
                )[0]["result"],
            }
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="operation", required=True)
    for operation in ("sync", "test", "inspect"):
        command = sub.add_parser(operation)
        command.add_argument("--installation", required=True)
        if operation == "inspect":
            command.add_argument("--indexes", action="store_true")
        else:
            command.add_argument(
                "--worker", type=Path, default=service.ROOT / "target/release/lctx-surreal-dev"
            )
            command.add_argument("--project", type=Path)
            command.add_argument("--modules")
    args = parser.parse_args()
    try:
        installation = service.Installation.load(ready=args.operation != "inspect")
        if args.operation == "inspect":
            value = inspect_campaign(installation, args.installation, args.indexes)
        else:
            value = run_worker(
                installation,
                args.installation,
                args.operation,
                args.worker,
                source=args.project,
                modules=args.modules,
            )
        print(json.dumps(value, indent=2))
        return 0 if value.get("outcome", "passed") == "passed" else 1
    except service.FixtureBlocked as error:
        print(error.message(), file=sys.stderr)
        return service.EXIT_BLOCKED
    except (service.FixtureFailed, OSError, ValueError) as error:
        print(f"surreal-dev: failed: {type(error).__name__}: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
